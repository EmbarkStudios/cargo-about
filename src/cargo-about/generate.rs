use anyhow::{Context, Error};
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
    #[structopt(short, long, parse(from_os_str))]
    templates: PathBuf,
}

pub fn cmd(
    args: Args,
    cfg: licenses::config::Config,
    krates: Krates,
    store: licenses::LicenseStore,
) -> Result<(), Error> {
    let registry = {
        let mut reg = Handlebars::new();
        reg.register_templates_directory("hbs", args.templates)?;
        reg
    };

    let mut summary = licenses::Gatherer::with_store(std::sync::Arc::new(store))
        .with_confidence_threshold(args.threshold)
        .gather(&krates.krates, &cfg);

    licenses::sanitize(&mut summary)?;

    let output = generate(&summary, &registry)?;

    println!("{}", output);

    Ok(())
}

#[derive(Serialize)]
enum KrateOrExternal<'a> {
    Krate(&'a Krate),
    External {
        #[serde(rename = "crate")]
        krate: &'a Krate,
        path: PathBuf,
    },
}

#[derive(Serialize)]
struct License<'a> {
    /// The full name of the license
    name: String,
    /// The SPDX short identifier for the license
    id: String,
    /// The full license text
    text: String,
    /// The list of crates this license was applied to
    used_by: Vec<KrateOrExternal<'a>>,
}

#[derive(Serialize)]
struct Input<'a> {
    licenses: Vec<License<'a>>,
}

fn generate(summary: &licenses::Summary, tmp: &Handlebars) -> Result<String, Error> {
    let krates = vec![&summary.nfos[1], &summary.nfos[200]];

    let licenses = {
        let mut licenses = HashMap::new();

        for k in krates {
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

                        if let licenses::LicenseFileInfo::Text(ref s) = lf.info {
                            return true;
                        }

                        false
                    }) {
                        let mut entry = licenses.entry(req.name).or_insert_with(|| HashMap::new());

                        if let licenses::LicenseFileInfo::Text(ref s) = nfo.info {
                            let mut lic = entry.entry(s).or_insert_with(|| License {
                                name: req.name.to_owned(),
                                id: req.name.to_owned(),
                                text: s.clone(),
                                used_by: Vec::new(),
                            });

                            lic.used_by.push(KrateOrExternal::Krate(k.krate));

                            // lic.used_by.push(KrateOrExternal::External {
                            //     krate: k.krate,
                            //     path: nfo.path.clone(),
                            // });
                        }
                    }
                }
            }
        }

        licenses
            .into_iter()
            .flat_map(|(_, v)| v.into_iter().map(|(_, v)| v))
            .collect::<Vec<_>>()
    };

    let nput = Input { licenses };

    unimplemented!();
}
