use anyhow::Context as _;
use cargo_about::{generate::generate, licenses};
use codespan_reporting::term;
use krates::{Utf8Path as Path, Utf8PathBuf as PathBuf};
use std::fmt;

#[derive(clap::ValueEnum, Copy, Clone, Debug, Default)]
pub enum OutputFormat {
    /// Uses one or more handlebars templates to transform JSON to the output
    #[default]
    Handlebars,
    /// Outputs the raw JSON of the discovered licenses
    Json,
}

impl fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Handlebars => f.write_str("handlebars"),
            Self::Json => f.write_str("json"),
        }
    }
}

#[derive(clap::Parser, Debug)]
pub struct Args {
    /// Path to the config to use
    ///
    /// Defaults to `<manifest_root>/about.toml` if not specified
    #[clap(short, long)]
    config: Option<PathBuf>,
    /// The confidence threshold required for license files to be positively identified: 0.0 - 1.0
    #[clap(long, default_value = "0.8")]
    threshold: f32,
    /// The name of the template to use when rendering.
    ///
    /// If only passing a single template file to `templates` this is not used.
    #[clap(short, long)]
    name: Option<String>,
    /// A file to write the generated output to, typically an .html file.
    #[clap(short, long)]
    output_file: Option<PathBuf>,
    /// Space-separated list of features to activate
    #[clap(long)]
    features: Vec<String>,
    /// Activate all available features
    #[clap(long)]
    all_features: bool,
    /// Do not activate the `default` feature
    #[clap(long)]
    no_default_features: bool,
    /// The target triples to use for dependency graph filtering.
    ///
    /// Overrides the `targets` configuration value, and note that unlike cargo
    /// itself this can take multiple targets instead of just one.
    #[clap(long)]
    target: Vec<String>,
    /// Run without accessing the network.
    ///
    /// In addition to cargo not fetching crates, this will mean that only
    /// local files will be crawled for license information.
    /// 1. clearlydefined.io will not be used, so some more ambiguous/complicated
    ///    license files might be ignored
    /// 2. Crates that are improperly packaged and don't contain their LICENSE
    ///    file(s) will fallback to the default license file, missing eg.
    ///    copyright information in the license that would be retrieved from
    ///    the original git repo for the crate in question
    #[arg(long)]
    offline: bool,
    /// Assert that `Cargo.lock` will remain unchanged
    #[arg(long)]
    locked: bool,
    /// Equivalent to specifying both `--locked` and `--offline`
    #[arg(long)]
    frozen: bool,
    /// The path of the Cargo.toml for the root crate.
    ///
    /// Defaults to the current crate or workspace in the current working directory
    #[clap(short, long)]
    manifest_path: Option<PathBuf>,
    /// Scan licenses for the entire workspace, not just the active package
    #[clap(long)]
    workspace: bool,
    /// Exit with a non-zero exit code when failing to read, synthesize, or
    /// clarify a license expression for a crate
    #[clap(long)]
    fail: bool,
    /// The format of the output, defaults to `handlebars`.
    #[clap(long, default_value_t)]
    format: OutputFormat,
    /// The template(s) or template directory to use.
    ///
    /// Must either be a `.hbs` file, or have at least one `.hbs` file in it if
    /// it is a directory.
    ///
    /// Required if `--format` is not `json`
    templates: Option<PathBuf>,
}

fn load_config(manifest_path: &Path) -> anyhow::Result<cargo_about::licenses::config::Config> {
    let mut parent = manifest_path.parent();

    // Move up directories until we find an about.toml, to handle
    // cases where eg in a workspace there is a top-level about.toml
    // but the user is only getting a listing for a particular crate from it
    while let Some(p) = parent {
        // We _could_ limit ourselves to only directories that also have a Cargo.toml
        // in them, but there could be cases where someone has multiple
        // rust projects in subdirectories with a single top level about.toml that is
        // used across all of them, we could also introduce a metadata entry for the
        // relative path of the about.toml to use for the crate/workspace

        // if !p.join("Cargo.toml").exists() {
        //     parent = p.parent();
        //     continue;
        // }

        let about_toml = p.join("about.toml");

        if about_toml.exists() {
            let contents = std::fs::read_to_string(&about_toml)?;
            let cfg = toml::from_str(&contents)?;

            log::info!("loaded config from '{about_toml}'");
            return Ok(cfg);
        }

        parent = p.parent();
    }

    log::warn!("no 'about.toml' found, falling back to default configuration");
    Ok(cargo_about::licenses::config::Config::default())
}

pub fn cmd(args: Args, color: crate::Color) -> anyhow::Result<()> {
    let manifest_path = if let Some(mp) = args.manifest_path.clone() {
        mp
    } else {
        let cwd =
            std::env::current_dir().context("unable to determine current working directory")?;
        let mut cwd = PathBuf::from_path_buf(cwd).map_err(|pb| {
            anyhow::anyhow!(
                "current working directory '{}' is not a utf-8 path",
                pb.display()
            )
        })?;

        cwd.push("Cargo.toml");
        cwd
    };

    anyhow::ensure!(
        manifest_path.exists(),
        "cargo manifest path '{manifest_path}' does not exist"
    );

    let cfg = match &args.config {
        Some(cfg_path) => {
            let cfg_str = std::fs::read_to_string(cfg_path)
                .with_context(|| format!("unable to read '{cfg_path}'"))?;
            toml::from_str(&cfg_str)
                .with_context(|| format!("unable to deserialize config from '{cfg_path}'"))?
        }
        None => load_config(&manifest_path)?,
    };

    let mut all_crates = None;
    let mut store = None;
    let mut templates = None;

    anyhow::ensure!(
        matches!(args.format, OutputFormat::Json) || args.templates.is_some(),
        "handlebars template(s) must be specified when using handlebars output format"
    );

    // Check if the parent process is powershell, if it is, assume that it will
    // screw up the output https://github.com/EmbarkStudios/cargo-about/issues/198
    // and inform the user about the -o, --output-file option
    let redirect_stdout =
        args.output_file.is_none() || args.output_file.as_deref() == Some(Path::new("-"));
    if redirect_stdout {
        anyhow::ensure!(
            !cargo_about::is_powershell_parent(),
            "cargo-about should not redirect its output in powershell, please use the -o, --output-file option to redirect to a file to avoid powershell encoding issues"
        );
    }

    rayon::scope(|s| {
        s.spawn(|_| {
            log::info!("gathering crates for {manifest_path}");
            all_crates = Some(cargo_about::get_all_crates(
                &manifest_path,
                args.no_default_features,
                args.all_features,
                args.features.clone(),
                args.workspace,
                krates::LockOptions {
                    frozen: args.frozen,
                    locked: args.locked,
                    offline: args.offline,
                },
                &cfg,
                &args.target,
            ));
        });
        s.spawn(|_| {
            log::info!("loading license store");
            store = Some(cargo_about::licenses::store_from_cache());
        });
        s.spawn(|_| {
            let Some(template_path) = args.templates.as_ref() else {
                return;
            };

            let load_templates = || -> anyhow::Result<_> {
                let mut reg = Handlebars::new();

                anyhow::ensure!(template_path.exists(), "template(s) path '{template_path}' does not exist");

                use handlebars::*;

                reg.register_helper(
                    "json",
                    Box::new(
                        |h: &Helper<'_, >,
                         _r: &Handlebars<'_>,
                         _c: &Context,
                         _rc: &mut RenderContext<'_, '_>,
                         out: &mut dyn Output|
                         -> HelperResult {
                            let param = h
                                .param(0)
                                .ok_or_else(|| RenderErrorReason::ParamNotFoundForIndex("json", 0))?;

                            match serde_json::to_string_pretty(param.value()) {
                                Ok(json) => Ok(out.write(&json)?),
                                Err(err) => {
                                    Err(RenderErrorReason::Other(err.to_string()).into())
                                }
                            }
                        },
                    ),
                );

                if template_path.is_dir() {
                    reg.register_templates_directory(template_path, handlebars::DirectorySourceOptions::default())?;

                    anyhow::ensure!(!reg.get_templates().is_empty(), "template path '{template_path}' did not contain any hbs files");

                    Ok((reg, args.name.context("specified a directory for templates, but did not provide the name of the template to use")?))
                } else {
                    // Ignore the extension, if the user says they want to use a specific file, that's on them
                    reg.register_template_file("tmpl", template_path)?;

                    Ok((reg, "tmpl".to_owned()))
                }
            };

            templates = Some(load_templates());
        });
    });

    let krates = all_crates.unwrap()?;
    let store = store.unwrap()?;

    log::info!("gathered {} crates", krates.len());

    let client = if !args.offline && !args.frozen {
        Some(reqwest::blocking::ClientBuilder::new().build()?)
    } else {
        None
    };

    let summary = licenses::Gatherer::with_store(std::sync::Arc::new(store))
        .with_confidence_threshold(args.threshold)
        .with_max_depth(cfg.max_depth.map(|md| md as _))
        .gather(&krates, &cfg, client);

    let (files, resolved) =
        licenses::resolution::resolve(&summary, &cfg.accepted, &cfg.crates, args.fail);

    use term::termcolor::ColorChoice;

    let stream = term::termcolor::StandardStream::stderr(match color {
        crate::Color::Auto => {
            // The termcolor crate doesn't check the stream to see if it's a TTY
            // which doesn't really fit with how the rest of the coloring works
            use std::io::IsTerminal;
            if std::io::stderr().is_terminal() {
                ColorChoice::Auto
            } else {
                ColorChoice::Never
            }
        }
        crate::Color::Always => ColorChoice::Always,
        crate::Color::Never => ColorChoice::Never,
    });

    let input = generate(&summary, &resolved, &files, Some(stream))?;
    let output = if let Some(templates) = templates {
        let (registry, template_name) = templates?;
        registry.render(&template_name, &input)?
    } else {
        serde_json::to_string(&input)?
    };

    if let Some(path) = &args.output_file.filter(|_| !redirect_stdout) {
        std::fs::write(path, output)
            .with_context(|| format!("output file {path} could not be written"))?;
    } else {
        println!("{output}");
    }

    Ok(())
}
