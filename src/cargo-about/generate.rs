use anyhow::{anyhow, bail, Context, Error};
use cargo_about::{licenses, Krates};
use handlebars::Handlebars;
use serde::Serialize;
use std::{collections::BTreeMap, path::Path, path::PathBuf};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub struct Args {
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
    #[structopt(parse(from_os_str))]
    templates: PathBuf,
    /// A file to write the generated output to.  Typically an .html file.
    #[structopt(short = "o", long = "output-file", parse(from_os_str))]
    output_file: Option<PathBuf>,
}

pub fn cmd(
    args: Args,
    cfg: licenses::config::Config,
    krates: Krates,
    store: licenses::LicenseStore,
) -> Result<(), Error> {
    let (registry, template) = {
        let mut reg = Handlebars::new();

        if !args.templates.exists() {
            bail!(
                "template(s) path {} does not exist",
                args.templates.display()
            );
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
                    args.templates.display()
                );
            }

            (reg, args.name.context("specified a directory for templates, but did not provide the name of the template to use")?)
        } else {
            // Ignore the extension, if the user says they want to use a specific file, that's on them
            reg.register_template_file("tmpl", args.templates)?;

            (reg, "tmpl".to_owned())
        }
    };

    let summary = licenses::Gatherer::with_store(std::sync::Arc::new(store))
        .with_confidence_threshold(args.threshold)
        .gather(&krates, &cfg);

    let resolved = licenses::Resolved::resolve(&summary.nfos, &cfg.accepted)?;
    let output = generate(&summary.nfos, &resolved, &registry, &template)?;

    match args.output_file.as_ref() {
        None => println!("{}", output),
        Some(path) if path == Path::new("-") => println!("{}", output),
        Some(path) => {
            std::fs::write(path, output).map_err(|err| {
                anyhow!(
                    "output file {} could not be written: {}",
                    path.display(),
                    err
                )
            })?;
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
    resolved: &licenses::Resolved,
    hbs: &Handlebars<'_>,
    template_name: &str,
) -> Result<String, Error> {
    let licenses = {
        let mut licenses = BTreeMap::new();
        for (krate_id, license_list) in &resolved.0 {
            let krate_license = &nfos[*krate_id];
            let license_iter = license_list
                .iter()
                .filter_map(|license| match license.license {
                    spdx::LicenseItem::Spdx { id, .. } => {
                        let file = krate_license.license_files.iter().find_map(move |lf| {
                            if lf.id != id {
                                return None;
                            }
                            match &lf.info {
                                licenses::LicenseFileInfo::Text(text)
                                | licenses::LicenseFileInfo::AddendumText(text, _) => {
                                    let license = License {
                                        name: lf.id.full_name.to_owned(),
                                        id: lf.id.name.to_owned(),
                                        text: text.clone(),
                                        source_path: None,
                                        used_by: Vec::new(),
                                    };
                                    Some(license)
                                }
                                _ => None,
                            }
                        });
                        file.or_else(|| {
                            let license = license::from_id(id.name).map(|lic| License {
                                name: lic.name().to_string(),
                                id: lic.id().to_string(),
                                text: lic.text().to_string(),
                                source_path: None,
                                used_by: Vec::new(),
                            });
                            if license.is_none() {
                                log::warn!(
                                    "No licene file or license text found for {} in crate {}",
                                    id.name,
                                    krate_license.krate.name
                                );
                            }
                            license
                        })
                    }
                    _ => {
                        log::warn!(
                            "{} has no license file for crate '{}'",
                            license,
                            krate_license.krate.name
                        );
                        None
                    }
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

        let mut licenses = licenses
            .into_iter()
            .flat_map(|(_, v)| v.into_iter().map(|(_, v)| v))
            .collect::<Vec<_>>();

        // Sort the used_by krates lexicographically
        for lic in &mut licenses {
            lic.used_by.sort_by(|a, b| a.krate.id.cmp(&b.krate.id));
        }

        licenses.sort_by(|a, b| a.used_by[0].krate.id.cmp(&b.used_by[0].krate.id));
        licenses.sort_by(|a, b| a.id.cmp(&b.id));
        licenses
    };

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

    let nput = Input { licenses, overview };

    Ok(hbs.render(template_name, &nput)?)
}
