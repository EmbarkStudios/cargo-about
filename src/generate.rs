use std::collections::BTreeMap;

use crate::licenses::{self, LicenseInfo};
use codespan_reporting::term;
use krates::Utf8PathBuf as PathBuf;
use krates::cm::Package;
use serde::{Serialize, Serializer};

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
    /// Number of packages that use this license.
    pub count: usize,
    /// This license's human-readable name (e.g. "Apache License 2.0").
    pub name: String,
    /// This license's SPDX identifier (e.g. "Apache-2.0").
    pub id: String,
    /// Indices (in [`Input::crates`]) of the crates that use this license.
    pub indices: Vec<usize>,
    /// This license's text. Currently taken from the first crate that uses the license.
    pub text: String,
}

fn serialize_as_string<T: std::fmt::Display, S: Serializer>(
    value: &T,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.collect_str(value)
}

#[derive(Serialize)]
pub struct PackageLicense<'a> {
    /// The package itself.
    pub package: &'a Package,
    /// The package's license: either a SPDX license identifier, "Unknown", or "Ignore".
    #[serde(serialize_with = "serialize_as_string")]
    pub license: &'a LicenseInfo,
}

#[derive(Serialize)]
pub struct LicenseList<'a> {
    /// All license types (e.g. Apache, MIT) and the indices (in [`Input::crates`]) of the crates that use them.
    pub overview: Vec<LicenseSet>,
    /// All unique license *texts* (which may differ by e.g. copyright string, even among licenses of the same type),
    /// and the crates that use them.
    pub licenses: Vec<License<'a>>,
    /// All input packages/crates.
    pub crates: Vec<PackageLicense<'a>>,
}

/// Generate a list of all licenses from a list of crates gathered from [`licenses::Gatherer`] and a list of resolved
/// licenses and files from [`licenses::resolution::resolve`].
pub fn generate<'kl>(
    nfos: &'kl [licenses::KrateLicense<'kl>],
    resolved: &[Option<licenses::Resolved>],
    files: &licenses::resolution::Files,
    stream: Option<term::termcolor::StandardStream>,
) -> anyhow::Result<LicenseList<'kl>> {
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

                        term::emit_to_io_write(&mut streaml, &diag_cfg, files, diag)?;
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

    let mut overview: BTreeMap<&str, LicenseSet> = BTreeMap::new();

    for (ndx, lic) in licenses.iter_mut().enumerate() {
        let ls = overview.entry(&lic.id).or_insert_with(|| {
            lic.first_of_kind = true;
            LicenseSet {
                count: 0,
                name: lic.name.clone(),
                id: lic.id.clone(),
                indices: Vec::with_capacity(10),
                text: lic.text.clone(),
            }
        });
        ls.indices.push(ndx);
        ls.count += lic.used_by.len();
    }

    let mut overview = overview.into_values().collect::<Vec<_>>();
    // Show the most used licenses first
    overview.sort_by(|a, b| b.count.cmp(&a.count));

    let crates = nfos
        .iter()
        .filter(|nfo| !matches!(nfo.lic_info, LicenseInfo::Ignore))
        .map(|nfo| PackageLicense {
            package: &nfo.krate.0,
            license: &nfo.lic_info,
        })
        .collect();
    Ok(LicenseList {
        overview,
        licenses,
        crates,
    })
}
