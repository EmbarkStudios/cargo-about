use anyhow::{bail, Context, Error};
use cargo_about::{licenses, Krate, Krates};
use handlebars::Handlebars;
use serde::Serialize;
use std::{collections::HashMap, path::PathBuf};
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
                 _r: &Handlebars,
                 _: &Context,
                 _rc: &mut RenderContext<'_>,
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

        // reg.register_helper(
        //     "sanitize-html",
        //     Box::new(
        //         |h: &Helper,
        //          r: &Handlebars,
        //          _: &Context,
        //          rc: &mut RenderContext<'_>,
        //          out: &mut dyn Output|
        //          -> HelperResult {
        //             let param = h.param(0).ok_or(RenderError::new("param not found"))?;

        //             let val = param
        //                 .value()
        //                 .as_str()
        //                 .ok_or_else(|| RenderError::new("expected string"))?;

        //             let cleaned = ammonia::clean(val);

        //             if val != cleaned {
        //                 println!("{}", difference::Changeset::new(val, &cleaned, ""));
        //                 //return Err(RenderError::new("HOLY CRAP WE DID IT"));
        //             }
        //             out.write(&cleaned)?;
        //             Ok(())
        //         },
        //     ),
        // );

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

    let mut summary = licenses::Gatherer::with_store(std::sync::Arc::new(store))
        .with_confidence_threshold(args.threshold)
        .gather(&krates.krates, &cfg);

    // for (krate_id, licenses) in licenses::resolve(&summary.nfos, &cfg.accepted)? {
    //     let name = &summary.nfos[krate_id].krate.name;
    //     log::info!("{}", name);
    //     for license in licenses {
    //         log::info!("    {:?}", license);
    //     }
    // }
    let resolved = licenses::resolve(&summary.nfos, &cfg.accepted)?;
    let output = generate(&summary, &resolved, &registry, &template)?;

    println!("{}", output);

    Ok(())
}

#[derive(Serialize)]
struct UsedBy<'a> {
    #[serde(rename = "crate")]
    krate: &'a Krate,
    path: Option<PathBuf>,
}

#[derive(Serialize)]
struct License<'a> {
    /// The full name of the license
    name: String,
    /// The SPDX short identifier for the license
    id: String,
    /// The full license text
    text: String,
    /// The path where the license text was sourced from
    source_path: PathBuf,
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
    summary: &licenses::Summary<'_>,
    resolved: &licenses::Resolved,
    hbs: &Handlebars,
    template_name: &str,
) -> Result<String, Error> {
    let licenses = {
        let mut licenses = HashMap::new();

        for k in &summary.nfos {
            if let licenses::LicenseInfo::Expr(ref expr) = k.lic_info {
                for req in expr.requirements().filter_map(|r| {
                    if let spdx::LicenseItem::SPDX { id, .. } = r.req.license {
                        Some(id)
                    } else {
                        None
                    }
                }) {
                    if let Some(nfo) = k.license_files.iter().find(|lf| {
                        if lf.id != req {
                            return false;
                        }

                        match lf.info {
                            licenses::LicenseFileInfo::Text(_)
                            | licenses::LicenseFileInfo::AddendumText(_, _) => true,
                            _ => false,
                        }
                    }) {
                        let entry = licenses.entry(req.name).or_insert_with(HashMap::new);

                        let contents = match nfo.info {
                            licenses::LicenseFileInfo::Text(ref s) => s,
                            licenses::LicenseFileInfo::AddendumText(ref s, _) => s,
                            _ => unreachable!(),
                        };

                        let lic = entry.entry(contents).or_insert_with(|| License {
                            name: req.full_name.to_owned(),
                            id: req.name.to_owned(),
                            text: contents.clone(),
                            used_by: Vec::new(),
                            source_path: nfo.path.clone(),
                        });

                        match nfo.info {
                            licenses::LicenseFileInfo::Text(_) => {
                                lic.used_by.push(UsedBy {
                                    krate: k.krate,
                                    path: None,
                                });
                            }
                            licenses::LicenseFileInfo::AddendumText(_, ref p) => {
                                lic.used_by.push(UsedBy {
                                    krate: k.krate,
                                    path: Some(p.clone()),
                                })
                            }
                            _ => unreachable!(),
                        }
                    }
                }
            }
        }

        let mut licenses = licenses
            .into_iter()
            .flat_map(|(_, v)| v.into_iter().map(|(_, v)| v))
            .collect::<Vec<_>>();

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

    let nput = Input { licenses, overview };

    Ok(hbs.render(template_name, &nput)?)
}
