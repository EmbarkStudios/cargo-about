use anyhow::Context as _;
use cargo_about::licenses;
use cargo_about::licenses::LicenseInfo;
use codespan_reporting::term;
use krates::cm::Package;
use krates::{Utf8Path as Path, Utf8PathBuf as PathBuf};
use serde::Serialize;
use std::{collections::BTreeMap, fmt};

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
    /// The confidence threshold required for license files
    /// to be positively identified: 0.0 - 1.0
    #[clap(long, default_value = "0.8")]
    threshold: f32,
    /// The name of the template to use when rendering. If only passing a
    /// single template file to `templates` this is not used.
    #[clap(short, long)]
    name: Option<String>,
    /// A file to write the generated output to. Typically an .html file.
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
    /// The path of the Cargo.toml for the root crate, defaults to the
    /// current crate or workspace in the current working directory
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
    /// The template(s) or template directory to use. Must either be a `.hbs`
    /// file, or have at least one `.hbs` file in it if it is a directory.
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

    rayon::scope(|s| {
        s.spawn(|_| {
            log::info!("gathering crates for {manifest_path}");
            all_crates = Some(cargo_about::get_all_crates(
                &manifest_path,
                args.no_default_features,
                args.all_features,
                args.features.clone(),
                args.workspace,
                &cfg,
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
                    reg.register_templates_directory( template_path, handlebars::DirectorySourceOptions::default())?;

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

    let client = reqwest::blocking::ClientBuilder::new()
        .timeout(std::time::Duration::from_secs(
            cfg.clearly_defined_timeout_secs.unwrap_or(30),
        ))
        .build()?;
    let summary = licenses::Gatherer::with_store(std::sync::Arc::new(store), client.into())
        .with_confidence_threshold(args.threshold)
        .with_max_depth(cfg.max_depth.map(|md| md as _))
        .gather(&krates, &cfg);

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

    let output = if let Some(templates) = templates {
        let (registry, template_name) = templates?;
        let input = generate(&summary, &resolved, &files, stream)?;
        registry.render(&template_name, &input)?
    } else {
        let input = generate(&summary, &resolved, &files, stream)?;
        serde_json::to_string(&input)?
    };

    match args.output_file.as_ref() {
        None => println!("{output}"),
        Some(path) if path == Path::new("-") => println!("{output}"),
        Some(path) => {
            std::fs::write(path, output)
                .with_context(|| format!("output file {path} could not be written"))?;
        }
    }

    Ok(())
}

#[derive(Clone, Serialize)]
struct UsedBy<'a> {
    #[serde(rename = "crate")]
    krate: &'a krates::cm::Package,
    path: Option<PathBuf>,
}

#[derive(Clone, Serialize)]
struct License<'a> {
    /// The full name of the license
    name: String,
    /// The SPDX short identifier for the license
    id: String,
    /// True if this is the first license of its kind in the flat array
    first_of_kind: bool,
    /// The full license text
    text: String,
    /// The path where the license text was sourced from
    source_path: Option<PathBuf>,
    /// The list of crates this license was applied to
    used_by: Vec<UsedBy<'a>>,
}

#[derive(Serialize)]
struct LicenseSet {
    count: usize,
    name: String,
    id: String,
    indices: Vec<usize>,
    text: String,
}

#[derive(Serialize)]
struct Input<'a> {
    overview: Vec<LicenseSet>,
    licenses: Vec<License<'a>>,
    crates: Vec<PackageLicense<'a>>,
}

fn generate<'kl>(
    nfos: &[licenses::KrateLicense<'kl>],
    resolved: &[Option<licenses::Resolved>],
    files: &licenses::resolution::Files,
    stream: term::termcolor::StandardStream,
) -> anyhow::Result<Input<'kl>> {
    use cargo_about::licenses::resolution::Severity;

    let mut num_errors = 0;

    let diag_cfg = term::Config::default();

    let mut licenses = {
        let mut licenses = BTreeMap::new();
        for (krate_license, resolved) in nfos
            .iter()
            .zip(resolved.iter())
            .filter_map(|(kl, res)| res.as_ref().map(|res| (kl, res)))
        {
            if !resolved.diagnostics.is_empty() {
                let mut streaml = stream.lock();

                for diag in &resolved.diagnostics {
                    if diag.severity >= Severity::Error {
                        num_errors += 1;
                    }

                    term::emit(&mut streaml, &diag_cfg, files, diag)?;
                }
            }

            let license_iter = resolved.licenses.iter().flat_map(|license| {
                let mut license_texts = Vec::new();
                match license.license {
                    spdx::LicenseItem::Spdx { id, .. } => {
                        // Attempt to retrieve the actual license file from the crate, note that in some cases
                        // _sigh_ there are actually multiple license texts for the same license with different
                        // copyright holders/authors/attribution so we can't just return 1
                        license_texts.extend(krate_license
                            .license_files
                            .iter()
                            .filter_map(|lf| {
                                // Check if this is the actual license file we want
                                if !lf
                                    .license_expr
                                    .evaluate(|ereq| ereq.license.id() == Some(id))
                                {
                                    return None;
                                }

                                match &lf.kind {
                                    licenses::LicenseFileKind::Text(text)
                                    | licenses::LicenseFileKind::AddendumText(text, _) => {
                                        let license = License {
                                            name: id.full_name.to_owned(),
                                            id: id.name.to_owned(),
                                            text: text.clone(),
                                            source_path: Some(lf.path.clone()),
                                            used_by: Vec::new(),
                                            first_of_kind: false,
                                        };
                                        Some(license)
                                    }
                                    licenses::LicenseFileKind::Header => None,
                                }
                            }));

                        if license_texts.is_empty() {
                            log::debug!(
                                "unable to find text for license '{license}' for crate '{}', falling back to canonical text",
                                krate_license.krate
                            );

                            // If the crate doesn't have the actual license file,
                            // fallback to the canonical license text and emit a warning
                            license_texts.push(License {
                                name: id.full_name.to_owned(),
                                id: id.name.to_owned(),
                                text: id.text().to_owned(),
                                source_path: None,
                                used_by: Vec::new(),
                                first_of_kind: false,
                            });
                        }
                    }
                    spdx::LicenseItem::Other { .. } => {
                        log::warn!(
                            "{license} has no license file for crate '{}'",
                            krate_license.krate
                        );
                    }
                }

                license_texts
            });

            for license in license_iter {
                let entry = licenses
                    .entry(license.name.clone())
                    .or_insert_with(BTreeMap::new);

                let lic = entry.entry(license.text.clone()).or_insert_with(|| license);
                lic.used_by.push(UsedBy {
                    krate: krate_license.krate,
                    path: None,
                });
            }
        }

        let mut licenses: Vec<_> = licenses
            .into_iter()
            .flat_map(|(_, v)| v.into_values())
            .collect();

        // Sort the krates that use a license lexicographically
        for lic in &mut licenses {
            lic.used_by.sort_by(|a, b| a.krate.id.cmp(&b.krate.id));
        }

        licenses.sort_by(|a, b| a.id.cmp(&b.id));
        licenses
    };

    if num_errors > 0 {
        anyhow::bail!(
            "encountered {num_errors} errors resolving licenses, unable to generate output"
        );
    }

    let mut overview: Vec<LicenseSet> = Vec::with_capacity(256);

    for (ndx, lic) in licenses.iter_mut().enumerate() {
        match overview.binary_search_by(|i| i.id.cmp(&lic.id)) {
            Ok(i) => {
                let ov = &mut overview[i];
                ov.indices.push(ndx);
                ov.count += lic.used_by.len();
            }
            Err(i) => {
                let mut ls = LicenseSet {
                    count: lic.used_by.len(),
                    name: lic.name.clone(),
                    id: lic.id.clone(),
                    indices: Vec::with_capacity(10),
                    text: lic.text.clone(),
                };

                ls.indices.push(ndx);
                overview.insert(i, ls);
                lic.first_of_kind = true;
            }
        }
    }

    // Show the most used licenses first
    overview.sort_by(|a, b| b.count.cmp(&a.count));

    let crates = nfos
        .iter()
        .filter(|nfo| !matches!(nfo.lic_info, LicenseInfo::Ignore))
        .map(|nfo| PackageLicense {
            package: &nfo.krate.0,
            license: nfo.lic_info.to_string(),
        })
        .collect();
    Ok(Input {
        overview,
        licenses,
        crates,
    })
}

#[derive(Serialize)]
struct PackageLicense<'a> {
    package: &'a Package,
    license: String,
}
