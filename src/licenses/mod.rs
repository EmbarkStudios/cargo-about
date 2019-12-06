use crate::Krate;
use anyhow::{bail, Context, Error};
use rayon::prelude::*;
use spdx::{LicenseId, LicenseItem, LicenseReq, Licensee};
use std::fmt;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

pub mod config;

const LICENSE_CACHE: &[u8] = include_bytes!("../../spdx_cache.bin.zstd");

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
    /// The license file is the canonical text, and applies to
    /// a path root
    AddendumText(String, PathBuf),
    /// The file just has a license header, and presumably
    /// also contains other text in it (like, you know, code)
    Header,
}

pub struct LicenseFile {
    /// The SPDX identifier for the license in the file
    pub id: LicenseId,
    /// Full path of the file which had license data in it
    pub path: PathBuf,
    /// The confidence score for the license, the closer to the canonical
    /// license text it is, the closert it approaches 1.0
    pub confidence: f32,
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

    pub fn gather<'k>(self, krates: &'k [crate::Krate], cfg: &config::Config) -> Summary<'k> {
        let mut summary = Summary::new(self.store);

        let threshold = self.threshold;
        let min_threshold = threshold - 0.5;

        let strategy = askalono::ScanStrategy::new(&summary.store.store)
            .mode(askalono::ScanMode::Elimination)
            .confidence_threshold(if min_threshold < 0.1 {
                0.1
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
                        log::info!(
                            "crate '{}({})' doesn't have a license field",
                            krate.name,
                            krate.version,
                        );
                        LicenseInfo::Unknown
                    }
                };

                let root_path = krate.manifest_path.parent().unwrap();
                let krate_cfg = cfg.inner.get(&krate.name);

                let mut license_files = match scan_files(
                    &root_path,
                    &strategy,
                    threshold,
                    krate_cfg.map(|kc| (kc, krate.name.as_str())),
                ) {
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

                // Condense each license down to the best candidate if
                // multiple are found

                license_files.sort_by(|a, b| {
                    use std::cmp::Ordering as Ord;
                    match a.id.cmp(&b.id) {
                        Ord::Equal => {
                            // We want the highest confidence on top
                            b.confidence
                                .partial_cmp(&a.confidence)
                                .expect("uhoh looks like we've got a NaN")
                        }
                        o => o,
                    }
                });

                let mut id = None;
                license_files.retain(|lf| match id {
                    Some(cur) => {
                        if cur != lf.id {
                            id = Some(lf.id);
                            true
                        } else {
                            false
                        }
                    }
                    None => {
                        id = Some(lf.id);
                        true
                    }
                });

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

fn is_ignored(entry: &walkdir::DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| {
            // Ignore hidden directories
            if s.starts_with('.') {
                log::debug!("ignoring hidden directory {}", entry.path().display());
                return true;
            }

            // Include typical files
            if entry.file_type().is_file() {
                if s.starts_with("LICENSE") {
                    return false;
                }

                // Filter out typical binary files
                if let Some(ext) = entry.path().extension() {
                    return match ext.to_string_lossy().as_ref() {
                        // Binary artifacts
                        "a" | "o" | "lib" | "obj" | "pyc" | "dll" | "exe" | "so" => true,
                        // Binary sources
                        "ttf" | "ico" | "dfa" | "rc" => true,
                        // Test data
                        "png" | "spv" | "vert" | "wasm" | "zip" | "gz" | "wav" | "jpg" | "bin"
                        | "zlib" | "p8" | "deflate" => true,
                        // Misc binary
                        "der" | "metallib" | "pdf" => true,
                        _ => false,
                    };
                }
            }

            false
        })
        .unwrap_or(false)
}

fn scan_files(
    root_dir: &Path,
    strat: &askalono::ScanStrategy<'_>,
    threshold: f32,
    krate_cfg: Option<(&config::KrateConfig, &str)>,
) -> Result<Vec<LicenseFile>, Error> {
    use walkdir::WalkDir;

    let walker = WalkDir::new(root_dir).into_iter();

    let files: Vec<_> = walker
        .filter_entry(|e| !is_ignored(e))
        .filter_map(|e| e.ok())
        .collect();

    let license_files: Vec<_> = files
        .into_par_iter()
        .filter_map(|file| {
            log::trace!("scanning file {}", file.path().display());

            if file.file_type().is_dir() {
                return None;
            }

            let mut contents = match read_file(file.path()) {
                Some(c) => c,
                None => return None,
            };

            let expected = match krate_cfg {
                Some(krate_cfg) => {
                    let relative = match file.path().strip_prefix(root_dir) {
                        Ok(rel) => rel,
                        Err(_) => return None,
                    };

                    match krate_cfg
                        .0
                        .ignore
                        .iter()
                        .find(|i| relative == i.license_file)
                    {
                        Some(ignore) => {
                            contents =
                                snip_contents(contents, ignore.license_start, ignore.license_end);
                            Some((ignore.license, None))
                        }
                        None => {
                            let mut addendum = None;

                            for additional in &krate_cfg.0.additional {
                                if relative == additional.license_file {
                                    addendum = Some(additional);
                                    break;
                                }

                                if relative.starts_with(&additional.root) {
                                    log::trace!(
                                        "skipping {} due to addendum for root {}",
                                        file.path().display(),
                                        additional.root.display()
                                    );
                                    return None;
                                }
                            }

                            addendum
                                .map(|addendum| (addendum.license, Some(&addendum.license_file)))
                        }
                    }
                }
                None => None,
            };

            let path = file.into_path();

            match scan_text(&contents, strat, threshold) {
                ScanResult::Header(ided) => {
                    if let Some((exp_id, addendum)) = expected {
                        if exp_id != ided.id {
                            log::error!(
                                "expected license '{}' in path '{}', but found '{}'",
                                exp_id.name,
                                path.display(),
                                ided.id.name
                            );
                        } else if addendum.is_none() {
                            log::debug!(
                                "ignoring '{}', matched license '{}'",
                                path.display(),
                                ided.id.name
                            );
                            return None;
                        }
                    }

                    Some(LicenseFile {
                        id: ided.id,
                        confidence: ided.confidence,
                        path,
                        info: LicenseFileInfo::Header,
                    })
                }
                ScanResult::Text(ided) => {
                    let info = if let Some((exp_id, addendum)) = expected {
                        if exp_id != ided.id {
                            log::error!(
                                "expected license '{}' in path '{}', but found '{}'",
                                exp_id.name,
                                path.display(),
                                ided.id.name
                            );
                        }

                        match addendum {
                            Some(path) => LicenseFileInfo::AddendumText(contents, path.clone()),
                            None => {
                                log::debug!(
                                    "ignoring '{}', matched license '{}'",
                                    path.display(),
                                    ided.id.name
                                );
                                return None;
                            }
                        }
                    } else {
                        LicenseFileInfo::Text(contents)
                    };

                    Some(LicenseFile {
                        id: ided.id,
                        confidence: ided.confidence,
                        path,
                        info,
                    })
                }
                ScanResult::UnknownId(id_str) => {
                    log::error!(
                        "found unknown SPDX identifier '{}' scanning '{}'",
                        id_str,
                        path.display()
                    );
                    None
                }
                ScanResult::LowLicenseChance(ided) => {
                    log::debug!(
                        "found '{}' scanning '{}' but it only has a confidence score of {}",
                        ided.id.name,
                        path.display(),
                        ided.confidence
                    );
                    None
                }
                ScanResult::NoLicense => None,
            }
        })
        .collect();

    Ok(license_files)
}

fn read_file(path: &Path) -> Option<String> {
    match std::fs::read_to_string(path) {
        Err(ref e) if e.kind() == std::io::ErrorKind::InvalidData => {
            // If we fail due to invaliddata, it just means the file in question was
            // probably binary and didn't have valid utf-8 data, so we can ignore it
            log::debug!("binary file {} detected", path.display());
            None
        }
        Err(e) => {
            log::error!("failed to read '{}': {}", path.display(), e);
            None
        }
        Ok(c) => Some(c),
    }
}

fn snip_contents(contents: String, start: Option<usize>, end: Option<usize>) -> String {
    let rng = start.unwrap_or(0)..end.unwrap_or(std::usize::MAX);

    if rng.start == 0 && rng.end == std::usize::MAX {
        contents
    } else {
        let mut snipped_contents = String::with_capacity(contents.len());
        for (i, line) in contents.lines().enumerate() {
            if i >= rng.start && i < rng.end {
                snipped_contents.push_str(line);
                snipped_contents.push('\n');
            }
        }

        snipped_contents
    }
}

struct Identified {
    confidence: f32,
    id: spdx::LicenseId,
}

enum ScanResult {
    Header(Identified),
    Text(Identified),
    UnknownId(String),
    LowLicenseChance(Identified),
    NoLicense,
}

fn scan_text(contents: &str, strat: &askalono::ScanStrategy<'_>, threshold: f32) -> ScanResult {
    let text = askalono::TextData::new(&contents);
    match strat.scan(&text) {
        Ok(lic_match) => {
            match lic_match.license {
                Some(identified) => {
                    let lic_id = match spdx::license_id(&identified.name) {
                        Some(id) => Identified {
                            confidence: lic_match.score,
                            id,
                        },
                        None => return ScanResult::UnknownId(identified.name.to_owned()),
                    };

                    // askalano doesn't report any matches below the confidence threshold
                    // but we want to see what it thinks the license is if the confidence
                    // is somewhat ok at least
                    if lic_match.score >= threshold {
                        match identified.kind {
                            askalono::LicenseType::Header => ScanResult::Header(lic_id),
                            askalono::LicenseType::Original => ScanResult::Text(lic_id),
                            askalono::LicenseType::Alternate => {
                                unimplemented!("I guess askalono uses this now")
                            }
                        }
                    } else {
                        ScanResult::LowLicenseChance(lic_id)
                    }
                }
                None => ScanResult::NoLicense,
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
}

type KrateId = usize;

pub struct ResolveError<'a> {
    pub krate: &'a Krate,
    pub required: Vec<LicenseReq>,
}

impl fmt::Display for ResolveError<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Krate '{}' requires", self.krate.name)?;
        f.debug_list().entries(self.required.iter()).finish()?;
        writeln!(
            f,
            " , which were not specified as 'accepted' licenses in the 'about.toml' file"
        )
    }
}

/// Simple wrapper to display a slice of licensees
pub struct DisplayList<'a, T>(pub &'a [T]);

impl<T: fmt::Display> fmt::Display for DisplayList<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[")?;
        for (id, val) in self.0.iter().enumerate() {
            write!(f, "{}", val)?;
            if id + 1 < self.0.len() {
                write!(f, ", ")?;
            }
        }
        write!(f, "]")
    }
}

pub struct Resolved(pub Vec<(KrateId, Vec<LicenseReq>)>);

impl Resolved {
    /// Find the minimal required licenses for each crate.
    pub fn resolve<'a>(
        licenses: &'a [KrateLicense<'_>],
        accepted: &'a [Licensee],
    ) -> Result<Resolved, Error> {
        let res: Result<Vec<_>, Error> = licenses
        .par_iter()
        .enumerate()
        .map(move |(id, krate_license)| {
            // Check that the licenses found by scanning the crate contents match what was stated
            // in the license expression
            match krate_license.lic_info {
                LicenseInfo::Expr(ref expr) => {
                    let req = accepted.iter().find_map(|licensee| {
                        expr.requirements().find(|expr| licensee.satisfies(&expr.req))
                    }).map(|expr| expr.req.clone())
                    .context(format!(
                        "Crate '{}': Unable to satisfy [{}], with the following accepted licenses {}", krate_license.krate.name,
                        expr, DisplayList(accepted)
                    ))?;
                    Ok((id, vec![req]))
                }
                // If the license is unknown, we will concatenate all the licenses
                LicenseInfo::Unknown => {
                    let license_reqs: Vec<_> = krate_license
                        .license_files
                        .iter()
                        .map(|file| {
                            LicenseReq {
                                license: LicenseItem::SPDX {
                                    id: file.id,
                                    or_later: false,
                                },
                                exception: None,
                            }
                        })
                        .collect();

                    let failed_licenses: Vec<_> = license_reqs
                        .iter()
                        .cloned()
                        .filter(|license| !accepted.iter().any(|a| a.satisfies(license)))
                        .collect();

                    if failed_licenses.is_empty() {
                        Ok((id, license_reqs))
                    } else {
                        bail!("Crate '{}': These licenses {}, could not be satisfied with the following accepted licenses {}",
                            krate_license.krate.name,
                            DisplayList(failed_licenses.as_slice()),
                            DisplayList(accepted));
                    }
                }
            }
        })
        .collect();
        Ok(Resolved(res?))
    }
}
