use anyhow::{self, bail, Context as _};
use cargo_about::licenses;
use codespan_reporting::term;
use handlebars::Handlebars;
use krates::{Utf8Path as Path, Utf8PathBuf as PathBuf};
use serde::Serialize;
use std::collections::BTreeMap;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub struct Args {
    /// Path to the config to use
    ///
    /// Defaults to <manifest_root>/about.toml if not specified
    #[structopt(short, long)]
    config: Option<PathBuf>,
    /// The confidence threshold required for license files
    /// to be positively identified: 0.0 - 1.0
    #[structopt(long, default_value = "0.8")]
    threshold: f32,
    /// Include local crates beneath one or more directories, local crates
    /// are disregarded by default
    #[structopt(short, long)]
    include_local: Vec<PathBuf>,
    /// The name of the template to use when rendering. If only passing a
    /// single template file to `templates` this is not used.
    #[structopt(short, long)]
    name: Option<String>,
    /// The template or template directory to use. Must either be a .hbs or
    /// have at least
    templates: PathBuf,
    /// A file to write the generated output to.  Typically an .html file.
    #[structopt(short = "o", long = "output-file")]
    output_file: Option<PathBuf>,
    /// Space-separated list of features to activate
    #[structopt(long)]
    features: Vec<String>,
    /// Activate all available features
    #[structopt(long)]
    all_features: bool,
    /// Do not activate the `default` feature
    #[structopt(long)]
    no_default_features: bool,
    /// The path of the Cargo.toml for the root crate, defaults to the
    /// current crate or workspace in the current working directory
    #[structopt(short, long = "manifest-path")]
    manifest_path: Option<PathBuf>,
    /// Scan licenses for the entire workspace, not just the active package
    #[structopt(long)]
    workspace: bool,
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

            log::info!("loaded config from {}", about_toml);
            return Ok(cfg);
        }

        parent = p.parent();
    }

    log::warn!("no 'about.toml' found, falling back to default configuration");
    Ok(cargo_about::licenses::config::Config::default())
}

pub fn cmd(args: Args, color: crate::Color) -> anyhow::Result<()> {
    let manifest_path = match args.manifest_path.clone() {
        Some(mp) => mp,
        None => {
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
        }
    };

    if !manifest_path.exists() {
        bail!("cargo manifest path '{}' does not exist", manifest_path);
    }

    let cfg = match &args.config {
        Some(cfg_path) => {
            let cfg_str = std::fs::read_to_string(cfg_path)
                .with_context(|| format!("unable to read {}", cfg_path))?;
            toml::from_str(&cfg_str)
                .with_context(|| format!("unable to deserialize config from {}", cfg_path))?
        }
        None => load_config(&manifest_path)?,
    };

    let (all_crates, store) = rayon::join(
        || {
            log::info!("gathering crates for {}", manifest_path);
            cargo_about::get_all_crates(
                &manifest_path,
                args.no_default_features,
                args.all_features,
                args.features.clone(),
                args.workspace,
                &cfg,
            )
        },
        || {
            log::info!("loading license store");
            cargo_about::licenses::store_from_cache()
        },
    );

    let krates = all_crates?;
    let store = store?;

    log::info!("gathered {} crates", krates.len());

    let (registry, template) = {
        let mut reg = Handlebars::new();

        if !args.templates.exists() {
            bail!("template(s) path {} does not exist", args.templates);
        }

        use handlebars::*;

        reg.register_helper(
            "json",
            Box::new(
                |h: &Helper<'_, '_>,
                 _r: &Handlebars<'_>,
                 _: &Context,
                 _rc: &mut RenderContext<'_, '_>,
                 out: &mut dyn Output|
                 -> HelperResult {
                    let param = h
                        .param(0)
                        .ok_or_else(|| RenderError::new("param not found"))?;

                    out.write(&serde_json::to_string_pretty(param.value())?)?;
                    Ok(())
                },
            ),
        );

        if args.templates.is_dir() {
            reg.register_templates_directory(".hbs", &args.templates)?;

            if reg.get_templates().is_empty() {
                bail!(
                    "template path {} did not contain any hbs files",
                    args.templates
                );
            }

            (reg, args.name.context("specified a directory for templates, but did not provide the name of the template to use")?)
        } else {
            // Ignore the extension, if the user says they want to use a specific file, that's on them
            reg.register_template_file("tmpl", args.templates)?;

            (reg, "tmpl".to_owned())
        }
    };

    let client = cd::client::Client::new();
    let summary = licenses::Gatherer::with_store(std::sync::Arc::new(store), client)
        .with_confidence_threshold(args.threshold)
        .gather(&krates, &cfg);

    let (files, resolved) = licenses::resolution::resolve(&summary, &cfg.accepted, &cfg.crates);

    use term::termcolor::ColorChoice;

    let stream = term::termcolor::StandardStream::stderr(match color {
        crate::Color::Auto => {
            // The termcolor crate doesn't check the stream to see if it's a TTY
            // which doesn't really fit with how the rest of the coloring works
            if atty::is(atty::Stream::Stderr) {
                ColorChoice::Auto
            } else {
                ColorChoice::Never
            }
        }
        crate::Color::Always => ColorChoice::Always,
        crate::Color::Never => ColorChoice::Never,
    });

    let output = generate(&summary, &resolved, &files, stream, &registry, &template)?;

    match args.output_file.as_ref() {
        None => println!("{}", output),
        Some(path) if path == Path::new("-") => println!("{}", output),
        Some(path) => {
            std::fs::write(path, output)
                .with_context(|| format!("output file {} could not be written", path))?;
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
}

#[derive(Serialize)]
struct Input<'a> {
    overview: Vec<LicenseSet>,
    licenses: Vec<License<'a>>,
}

fn generate(
    nfos: &[licenses::KrateLicense<'_>],
    resolved: &[licenses::Resolved],
    files: &licenses::resolution::Files,
    stream: term::termcolor::StandardStream,
    hbs: &Handlebars<'_>,
    template_name: &str,
) -> anyhow::Result<String> {
    use cargo_about::licenses::resolution::Severity;

    let mut num_errors = 0;

    let diag_cfg = term::Config::default();

    let licenses = {
        let mut licenses = BTreeMap::new();
        for (krate_license, resolved) in nfos.iter().zip(resolved.iter()) {
            if !resolved.diagnostics.is_empty() {
                let mut streaml = stream.lock();

                for diag in &resolved.diagnostics {
                    if diag.severity >= Severity::Error {
                        num_errors += 1;
                    }

                    term::emit(&mut streaml, &diag_cfg, files, &diag)?;
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
                                        };
                                        Some(license)
                                    }
                                    licenses::LicenseFileKind::Header => None,
                                }
                            }));

                        if license_texts.is_empty() {
                            log::warn!(
                                "unable to find text for license '{}' for crate '{}', falling back to canonical text",
                                license,
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
                            });
                        }
                    }
                    spdx::LicenseItem::Other { .. } => {
                        log::warn!(
                            "{} has no license file for crate '{}'",
                            license,
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
            .flat_map(|(_, v)| v.into_iter().map(|(_, v)| v))
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
            "encountered {} errors resolving licenses, unable to generate output",
            num_errors
        );
    }

    let mut overview: Vec<LicenseSet> = Vec::with_capacity(256);

    for (ndx, lic) in licenses.iter().enumerate() {
        match overview.binary_search_by(|i| i.id.cmp(&lic.id)) {
            Ok(i) => overview[i].indices.push(ndx),
            Err(i) => {
                let mut ls = LicenseSet {
                    count: 0,
                    name: lic.name.clone(),
                    id: lic.id.clone(),
                    indices: Vec::with_capacity(10),
                };

                ls.indices.push(ndx);
                overview.insert(i, ls);
            }
        }
    }

    overview.iter_mut().for_each(|i| i.count = i.indices.len());
    // Show the most used licenses first
    overview.sort_by(|a, b| b.count.cmp(&a.count));

    let nput = Input { overview, licenses };

    Ok(hbs.render(template_name, &nput)?)
}
