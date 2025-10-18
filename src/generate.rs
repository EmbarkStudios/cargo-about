use std::collections::BTreeMap;

use crate::licenses::{self, LicenseInfo};
use serde::Serialize;
use krates::{Utf8PathBuf as PathBuf};
use krates::cm::Package;
use codespan_reporting::term;


#[derive(Clone, Serialize)]
pub struct UsedBy<'a> {
    #[serde(rename = "crate")]
    pub krate: &'a krates::cm::Package,
    pub path: Option<PathBuf>,
}

#[derive(Clone, Serialize)]
pub struct License<'a> {
    /// The full name of the license
    pub name: String,
    /// The SPDX short identifier for the license
    pub id: String,
    /// True if this is the first license of its kind in the flat array
    pub first_of_kind: bool,
    /// The full license text
    pub text: String,
    /// The path where the license text was sourced from
    pub source_path: Option<PathBuf>,
    /// The list of crates this license was applied to
    pub used_by: Vec<UsedBy<'a>>,
}

#[derive(Serialize)]
pub struct LicenseSet {
    pub count: usize,
    pub name: String,
    pub id: String,
    pub indices: Vec<usize>,
    pub text: String,
}

#[derive(Serialize)]
pub struct Input<'a> {
    pub overview: Vec<LicenseSet>,
    pub licenses: Vec<License<'a>>,
    pub crates: Vec<PackageLicense<'a>>,
}

pub fn generate<'kl>(
    nfos: &[licenses::KrateLicense<'kl>],
    resolved: &[Option<licenses::Resolved>],
    files: &licenses::resolution::Files,
    stream: Option<term::termcolor::StandardStream>,
) -> anyhow::Result<Input<'kl>> {
    use licenses::resolution::Severity;

    let mut num_errors = 0;

    let term_and_diag = stream.map(|s| (s, term::Config::default()));

    let mut licenses = {
        let mut licenses = BTreeMap::new();
        for (krate_license, resolved) in nfos
            .iter()
            .zip(resolved.iter())
            .filter_map(|(kl, res)| res.as_ref().map(|res| (kl, res)))
        {
            match &term_and_diag {
                Some((stream, diag_cfg)) if !resolved.diagnostics.is_empty() => {
                    let mut streaml = stream.lock();

                    for diag in &resolved.diagnostics {
                        if diag.severity >= Severity::Error {
                            num_errors += 1;
                        }

                        term::emit(&mut streaml, &diag_cfg, files, diag)?;
                    }
                }
                _ => {}
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
pub struct PackageLicense<'a> {
    pub package: &'a Package,
    pub license: String,
}
