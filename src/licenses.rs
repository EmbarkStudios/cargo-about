use crate::Krate;
use anyhow::{bail, Error};
use rayon::prelude::*;
use spdx::LicenseId;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

const LICENSE_CACHE: &[u8] = include_bytes!("../spdx_cache.bin.gz");

pub struct LicenseStore {
    store: askalono::Store,
}

impl LicenseStore {
    pub fn from_cache() -> Result<Self, Error> {
        let store = askalono::Store::from_cache(LICENSE_CACHE)
            .map_err(|e| anyhow::anyhow!("failed to load license store: {}", e))?;

        Ok(Self { store })
    }
}

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum LicenseInfo {
    Expr(spdx::Expression),
    Unknown,
}

/// The contents of a file with license info in it
pub enum LicenseFileInfo {
    /// The license file is the canonical text of the license
    Text(String),
    /// The file just has a license header, and presumably
    /// also contains other text in it (like, you know, code)
    Header,
}

pub struct LicenseFile {
    /// The SPDX identifier for the license in the file
    pub id: LicenseId,
    /// Full path of the file which had license data in it
    pub path: PathBuf,
    /// The contents of the file
    pub info: LicenseFileInfo,
}

pub struct KrateLicense<'a> {
    pub krate: &'a Krate,
    pub lic_info: LicenseInfo,
    pub license_files: Vec<LicenseFile>,
}

pub struct Summary<'a> {
    store: Arc<LicenseStore>,
    pub nfos: Vec<KrateLicense<'a>>,
}

impl<'a> Summary<'a> {
    fn new(store: Arc<LicenseStore>) -> Self {
        Self {
            store,
            nfos: Vec::new(),
        }
    }
}

pub struct Gatherer {
    store: Arc<LicenseStore>,
    threshold: f32,
}

impl Gatherer {
    pub fn with_store(store: Arc<LicenseStore>) -> Self {
        Self {
            store,
            threshold: 0.8,
        }
    }

    pub fn with_confidence_threshold(mut self, threshold: f32) -> Self {
        self.threshold = if threshold > 1.0 {
            1.0
        } else if threshold < 0.0 {
            0.0
        } else {
            threshold
        };
        self
    }

    pub fn gather<'k>(self, krates: &'k [crate::Krate]) -> Summary<'k> {
        let mut summary = Summary::new(self.store);

        let threshold = self.threshold;
        let min_threshold = threshold - 0.5;

        let strategy = askalono::ScanStrategy::new(&summary.store.store)
            .mode(askalono::ScanMode::Elimination)
            .confidence_threshold(if min_threshold < 1.0 {
                1.0
            } else {
                min_threshold
            })
            .optimize(false)
            .max_passes(1);

        krates
            .par_iter()
            .map(|krate| {
                let info = match krate.license {
                    Some(ref license_field) => {
                        //. Reasons this can fail:
                        // * Empty! The rust crate used to validate this field has a bug
                        // https://github.com/rust-lang-nursery/license-exprs/issues/23
                        // * It also just does basic lexing, so parens, duplicate operators,
                        // unpaired exceptions etc can all fail validation

                        match spdx::Expression::parse(license_field) {
                            Ok(validated) => LicenseInfo::Expr(validated),
                            Err(err) => {
                                log::error!(
                                    "unable to parse license expression for '{} - {}': {}",
                                    krate.name,
                                    krate.version,
                                    err
                                );
                                LicenseInfo::Unknown
                            }
                        }
                    }
                    None => {
                        log::warn!(
                            "crate '{} - {}' doesn't have a license field",
                            krate.name,
                            krate.version,
                        );
                        LicenseInfo::Unknown
                    }
                };

                let root_path = krate.manifest_path.parent().unwrap();
                let license_files = match scan_files(&root_path, &strategy, threshold) {
                    Ok(files) => files,
                    Err(err) => {
                        log::error!(
                            "unable to scan for license files for crate '{} - {}': {}",
                            krate.name,
                            krate.version,
                            err
                        );

                        Vec::new()
                    }
                };

                KrateLicense {
                    krate,
                    lic_info: info,
                    license_files,
                }
            })
            .collect_into_vec(&mut summary.nfos);

        summary
    }
}

fn is_hidden(entry: &walkdir::DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with("."))
        .unwrap_or(false)
}

fn scan_files(
    root_dir: &Path,
    strat: &askalono::ScanStrategy,
    threshold: f32,
) -> Result<Vec<LicenseFile>, Error> {
    use walkdir::WalkDir;

    let walker = WalkDir::new(root_dir).into_iter();

    let files: Vec<_> = walker
        .filter_entry(|e| !is_hidden(e))
        .filter_map(|e| e.ok())
        .collect();

    let license_files: Vec<_> = files
        .into_par_iter()
        .filter_map(|file| {
            if file.file_type().is_dir() {
                return None;
            }

            let contents = match std::fs::read_to_string(file.path()) {
                Err(e) => {
                    log::error!("failed to read '{}': {}", file.path().display(), e);
                    return None;
                }
                Ok(c) => c,
            };

            let text = askalono::TextData::new(&contents);
            match strat.scan(&text) {
                Ok(lic_match) => {
                    match lic_match.license {
                        Some(identified) => {
                            // askalano doesn't report any matches below the confidence threshold
                            // but we want to see what it thinks the license is if the confidence
                            // is somewhat ok at least
                            if lic_match.score >= threshold {
                                match spdx::license_id(&identified.name) {
                                    Some(id) => {
                                        return Some(LicenseFile {
                                            id,
                                            path: file.into_path(),
                                            info: match identified.kind {
                                                askalono::LicenseType::Header => LicenseFileInfo::Header,
                                                askalono::LicenseType::Original => LicenseFileInfo::Text(contents),
                                                askalono::LicenseType::Alternate => unimplemented!("I guess askalono uses this now"),
                                            },
                                        });
                                    }
                                    None => {
                                        log::error!("found a license '{}' in '{}', but it is not a valid SPDX identifier", identified.name, file.path().display());
                                        return None;
                                    }
                                }
                            }

                            None
                        }
                        None => {
                            None
                        }
                    }
                }
                Err(e) => {
                    // the elimination strategy can't currently fail
                    unimplemented!(
                        "I guess askalano's elimination strategy can now fail: {}",
                        e
                    );
                }
            }
        })
        .collect();

    Ok(license_files)
}

pub fn sanity_check(summary: &Summary) -> Result<(), Error> {
    let num_errors = summary
        .nfos
        .par_iter()
        .fold(
            || 0,
            |acc, krate_license| {
                unimplemented!();
                // Check that the licenses found by scanning the crate contents match what was stated
                // in the license expression
                match krate_license.lic_info {
                    LicenseInfo::Expr(ref expr) => {
                        let spdx_reqs = expr
                            .requirements()
                            .filter_map(|req| {
                                if let spdx::LicenseItem::SPDX { id, .. } = req.req.license {
                                    Some(id)
                                } else {
                                    None
                                }
                            })
                            .collect::<smallvec::SmallVec<[LicenseId; 2]>>();

                        acc + 0
                    }
                    LicenseInfo::Unknown => acc + 0,
                }
            },
        )
        .sum::<u32>();

    if num_errors > 0 {
        bail!(
            "encountered {} error sanity checking crate licenses",
            num_errors
        );
    } else {
        Ok(())
    }
}
