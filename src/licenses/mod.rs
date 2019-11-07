use crate::Krate;
use anyhow::{bail, Error};
use rayon::prelude::*;
use spdx::LicenseId;
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

impl LicenseFileInfo {
    fn is_addendum(&self) -> bool {
        if let LicenseFileInfo::AddendumText(_, _) = self {
            true
        } else {
            false
        }
    }
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
                        log::warn!(
                            "crate '{}({})' doesn't have a license field",
                            krate.name,
                            krate.version,
                        );
                        LicenseInfo::Unknown
                    }
                };

                let root_path = krate.manifest_path.parent().unwrap();
                let krate_cfg = cfg.inner.get(&krate.name);

                let license_files = match scan_files(
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
    strat: &askalono::ScanStrategy,
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

fn scan_text(contents: &str, strat: &askalono::ScanStrategy, threshold: f32) -> ScanResult {
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

pub fn sanitize(summary: &mut Summary) -> Result<(), Error> {
    let num_errors = summary
        .nfos
        .par_iter_mut()
        .fold(
            || 0,
            |acc, krate_license| {
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

                        log::info!(
                            "crate {}({}) has license(s) {:?} in its `license` field",
                            krate_license.krate.name,
                            krate_license.krate.version,
                            spdx_reqs
                        );

                        for lf in &krate_license.license_files {
                            if !lf.info.is_addendum() && !spdx_reqs.contains(&lf.id) {
                                log::warn!(
                                    "mismatching license found for {}: license '{}' in path '{}'",
                                    krate_license.krate.name,
                                    lf.id.name,
                                    lf.path.display()
                                );
                            }
                        }

                        acc
                    }
                    LicenseInfo::Unknown => {
                        let mut found = smallvec::SmallVec::<[(LicenseId, u32); 2]>::new();

                        for lf in &krate_license.license_files {
                            match found.iter_mut().find(|lic| lic.0 == lf.id) {
                                Some(lic) => lic.1 += 1,
                                None => found.push((lf.id, 1)),
                            }
                        }

                        let expr_s = {
                            let mut expr_s = String::new();

                            for (i, name) in found.iter().map(|l| l.0.name).enumerate() {
                                if i > 0 {
                                    expr_s.push_str(" AND ");
                                }

                                expr_s.push_str(name);
                            }

                            expr_s
                        };

                        if found.is_empty() {
                            log::error!("unable to find any license files for crate {}({})", krate_license.krate.name, krate_license.krate.version);
                            return acc + 1;
                        }

                        let expr = match spdx::Expression::parse(&expr_s) {
                            Ok(e) => e,
                            Err(e) => {
                                log::error!("failed to parse SPDX license expression from synthesized string '{}': {}", expr_s, e);
                                return acc + 1;
                            }
                        };

                        log::warn!("crate {}({}) had no license field, now using SPDX license expression '{}'", krate_license.krate.name, krate_license.krate.version, expr);
                        krate_license.lic_info = LicenseInfo::Expr(expr);

                        acc
                    },
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
