pub mod config;
pub mod fetch;
pub mod resolution;
mod scan;
mod workarounds;

use crate::{Krate, Krates};
use anyhow::Context as _;
use krates::Utf8PathBuf as PathBuf;
use rayon::prelude::*;
pub use resolution::Resolved;
use std::{cmp, sync::Arc};

const LICENSE_CACHE: &[u8] = include_bytes!("../spdx_cache.bin.zstd");

pub type LicenseStore = askalono::Store;

#[inline]
pub fn store_from_cache() -> anyhow::Result<LicenseStore> {
    askalono::Store::from_cache(LICENSE_CACHE)
        .map_err(|e| e.compat())
        .context("failed to load license store")
}

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum LicenseInfo {
    Expr(spdx::Expression),
    Unknown,
}

/// The contents of a file with license info in it
pub enum LicenseFileKind {
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
    /// The SPDX requirement expression detected for the file
    pub license_expr: spdx::Expression,
    /// Full path of the file which had license data in it
    pub path: PathBuf,
    /// The confidence score for the license, the closer to the canonical
    /// license text it is, the closer it approaches 1.0
    pub confidence: f32,
    /// The contents of the file
    pub kind: LicenseFileKind,
}

impl Ord for LicenseFile {
    #[inline]
    fn cmp(&self, o: &Self) -> cmp::Ordering {
        match self.license_expr.as_ref().cmp(o.license_expr.as_ref()) {
            cmp::Ordering::Equal => o
                .confidence
                .partial_cmp(&self.confidence)
                .expect("NaN encountered comparing license confidences"),
            ord => ord,
        }
    }
}

impl PartialOrd for LicenseFile {
    #[inline]
    fn partial_cmp(&self, o: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(o))
    }
}

impl PartialEq for LicenseFile {
    #[inline]
    fn eq(&self, o: &Self) -> bool {
        self.cmp(o) == cmp::Ordering::Equal
    }
}

impl Eq for LicenseFile {}

pub struct KrateLicense<'krate> {
    pub krate: &'krate Krate,
    pub lic_info: LicenseInfo,
    pub license_files: Vec<LicenseFile>,
}

impl<'krate> Ord for KrateLicense<'krate> {
    #[inline]
    fn cmp(&self, o: &Self) -> cmp::Ordering {
        self.krate.cmp(o.krate)
    }
}

impl<'krate> PartialOrd for KrateLicense<'krate> {
    #[inline]
    fn partial_cmp(&self, o: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(o))
    }
}

impl<'krate> PartialEq for KrateLicense<'krate> {
    #[inline]
    fn eq(&self, o: &Self) -> bool {
        self.cmp(o) == cmp::Ordering::Equal
    }
}

impl<'krate> Eq for KrateLicense<'krate> {}

pub struct Gatherer {
    store: Arc<LicenseStore>,
    cd_client: cd::client::Client,
    threshold: f32,
}

impl Gatherer {
    pub fn with_store(store: Arc<LicenseStore>, client: cd::client::Client) -> Self {
        Self {
            store,
            threshold: 0.8,
            cd_client: client,
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

    pub fn gather<'krate>(
        self,
        krates: &'krate Krates,
        cfg: &config::Config,
    ) -> Vec<KrateLicense<'krate>> {
        let mut licensed_krates = Vec::with_capacity(krates.len());

        let threshold = self.threshold;
        let min_threshold = threshold - 0.5;

        let strategy = askalono::ScanStrategy::new(&self.store)
            .mode(askalono::ScanMode::Elimination)
            .confidence_threshold(if min_threshold < 0.1 {
                0.1
            } else {
                min_threshold
            })
            .optimize(false)
            .max_passes(1);

        let git_cache = fetch::GitCache::default();

        // Workarounds are built-in to cargo-about to deal with issues that certain
        // common crates have
        workarounds::apply_workarounds(krates, cfg, &git_cache, &mut licensed_krates);

        // Clarifications are user supplied and thus take precedence over any
        // machine gathered data
        self.gather_clarified(krates, cfg, &git_cache, &mut licensed_krates);

        // Attempt to gather license information from clearly-defined.io so we
        // can get previously gathered license information + any possible
        // curations so that we only need to fallback to scanning local crate
        // sources if it's not already in clearly-defined
        self.gather_clearly_defined(krates, cfg, &strategy, &mut licensed_krates);

        // Finally, crawl the crate sources on disk to try and determine licenses
        self.gather_file_system(krates, cfg, &strategy, &mut licensed_krates);

        licensed_krates.sort();
        licensed_krates
    }

    #[allow(clippy::unused_self)]
    fn gather_clarified<'k>(
        &self,
        krates: &'k Krates,
        cfg: &config::Config,
        gc: &fetch::GitCache,
        licensed_krates: &mut Vec<KrateLicense<'k>>,
    ) {
        for (krate, clarification) in krates.krates().filter_map(|kn| {
            cfg.crates
                .get(&kn.krate.name)
                .and_then(|kc| kc.clarify.as_ref())
                .map(|cl| (&kn.krate, cl))
        }) {
            if let Err(i) = binary_search(licensed_krates, krate) {
                match apply_clarification(gc, krate, clarification) {
                    Ok(lic_files) => {
                        log::debug!(
                            "applying clarification expression '{}' to crate {}",
                            clarification.license,
                            krate
                        );
                        licensed_krates.insert(
                            i,
                            KrateLicense {
                                krate,
                                lic_info: LicenseInfo::Expr(clarification.license.clone()),
                                license_files: lic_files,
                            },
                        );
                    }
                    Err(e) => {
                        log::warn!("failed to validate all files specified in clarification for crate {}: {}", krate, e);
                    }
                }
            }
        }
    }

    fn gather_clearly_defined<'k>(
        &self,
        krates: &'k Krates,
        cfg: &config::Config,
        strategy: &askalono::ScanStrategy<'_>,
        licensed_krates: &mut Vec<KrateLicense<'k>>,
    ) {
        if cfg.disallow_clearly_defined {
            return;
        }

        let reqs = cd::definitions::get(
            10,
            krates.krates().filter_map(|krate| {
                if binary_search(licensed_krates, &krate.krate).is_ok() {
                    return None;
                }

                // Ignore local and git sources in favor of scanning those on the local disk
                if krate
                    .krate
                    .source
                    .as_ref()
                    .map_or(false, |src| src.is_crates_io())
                {
                    Some(cd::Coordinate {
                        shape: cd::Shape::Crate,
                        provider: cd::Provider::CratesIo,
                        // Rust crates, at least on crates.io, don't have a namespace
                        namespace: None,
                        name: krate.krate.name.clone(),
                        version: cd::CoordVersion::Semver(krate.krate.version.clone()),
                        // TODO: maybe set this if it's overriden in the config? seems messy though
                        curation_pr: None,
                    })
                } else {
                    None
                }
            }),
        );

        //let threshold = std::cmp::min(std::cmp::max(10, (self.threshold * 100.0) as u8), 100);
        let collected: Vec<_> = reqs.par_bridge().filter_map(|req| {
            match self.cd_client.execute::<cd::definitions::GetResponse>(req) {
                Ok(response) => {
                    Some(response.definitions.into_iter().filter_map(|def| {
                        if def.described.is_none() {
                            log::warn!("the definition for {} has not been harvested", def.coordinates);
                            return None;
                        }

                        // Since we only ever retrieve license information for crates on crates.io
                        // they _should_ always have a valid semver
                        let version = match &def.coordinates.revision {
                            cd::CoordVersion::Semver(vers) => vers.clone(),
                            cd::CoordVersion::Any(vers) => {
                                log::warn!(
                                    "the definition for {} does not have a valid semver '{}'",
                                    def.coordinates,
                                    vers,
                                );
                                return None;
                            }
                        };

                        // If the score is too low, bail
                        // if def.scores.effective < threshold {
                        //     log::warn!(
                        //         "the definition for {} score {} is below threshold {}",
                        //         def.coordinates,
                        //         def.scores.effective,
                        //         threshold,
                        //     );
                        //     return None;
                        // }

                        match krates.krates_by_name(&def.coordinates.name).find_map(|(_, kn)| {
                            if kn.krate.version == version {
                                Some(&kn.krate)
                            } else {
                                None
                            }})
                        {
                            Some(krate) => {
                                let info = krate.get_license_expression();

                                // clearly defined doesn't provide per-file scores, so we just use
                                // the overall score for the entire crate
                                let confidence = def.scores.effective as f32 / 100.0;

                                let license_files = def.files.into_iter().filter_map(|cd_file| {
                                    // Retrieve (and validate) the text of the file if clearlydefined thinks it is a license file
                                    let license_text = if cd_file.natures.iter().any(|s| s == "license") {
                                        let root_path = krate.manifest_path.parent().unwrap();
                                        let path = root_path.join(&cd_file.path);
                                        match std::fs::read_to_string(&path) {
                                            Ok(text) => {
                                                if let Some(expected) = cd_file.hashes.as_ref().and_then(|hashes| hashes.sha256.as_ref()) {
                                                    if let Err(err) = crate::validate_sha256(&text, expected) {
                                                        log::warn!("file '{}' for crate '{}' marked as a license but the sha256 hash could not be verified: {}", path, krate, err);
                                                        return None;
                                                    }
                                                }

                                                Some(text)
                                            }
                                            Err(err) => {
                                                log::warn!("failed to read license from '{}' for crate '{}': {}", path, krate, err);
                                                return None;
                                            }
                                        }
                                    } else {
                                        None
                                    };

                                    let path = cd_file.path;

                                    // clearly defined will attach a license identifier to any file
                                    // with a license or SPDX identifier, but like askalono it won't
                                    // detect all licenses if there are multiple in a single file
                                    match (cd_file.license, license_text) {
                                        (Some(lic), license_text) => {
                                            let license_expr = match spdx::Expression::parse_mode(&lic, spdx::ParseMode::Lax) {
                                                Ok(expr) => expr,
                                                Err(err) => {
                                                    log::warn!("clearlydefined detected license '{}' in '{}' for crate '{}', but it can't be parsed: {}", lic, path, krate, err);
                                                    return None;
                                                }
                                            };

                                            Some(LicenseFile {
                                                license_expr,
                                                path,
                                                confidence,
                                                kind: license_text.map_or(LicenseFileKind::Header, LicenseFileKind::Text),
                                            })
                                        }
                                        (None, Some(license_text)) => {
                                            // For some reason, clearlydefined will correctly identify text as being a
                                            // license but won't give it an expression, so we have to figure out what it
                                            // is, but at least have high confidence that it will result in a match
                                            scan::check_is_license_file(path.clone(), license_text, strategy, self.threshold, None)
                                                .or_else(|| {
                                                    log::warn!("clearlydefined detected license in '{}' for crate '{}', but it we failed to determine what its license was", path, krate);
                                                    None
                                                })
                                        }
                                        _ => None,
                                    }
                                }).collect();

                                Some(KrateLicense {
                                    krate,
                                    lic_info: info,
                                    license_files,
                                })
                            }
                            None => None,
                        }
                    }).collect::<Vec<_>>())
                }
                Err(err) => {
                    log::warn!(
                        "failed to request license information from clearly defined: {:#}",
                        err
                    );
                    None
                }
            }
        }).collect();

        for mut set in collected {
            licensed_krates.append(&mut set);
        }
        licensed_krates.sort();
    }

    fn gather_file_system<'k>(
        &self,
        krates: &'k Krates,
        cfg: &config::Config,
        strategy: &askalono::ScanStrategy<'_>,
        licensed_krates: &mut Vec<KrateLicense<'k>>,
    ) {
        let threshold = self.threshold;

        let mut gathered: Vec<_> = krates
            .krates()
            .par_bridge()
            .filter_map(|kn| {
                let krate = &kn.krate;

                // Ignore crates that we've already gathered
                if binary_search(licensed_krates, krate).is_ok() {
                    return None;
                }

                let info = krate.get_license_expression();

                let root_path = krate.manifest_path.parent().unwrap();
                let krate_cfg = cfg.crates.get(&krate.name);

                let mut license_files = match scan::scan_files(
                    root_path,
                    strategy,
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
                license_files.sort();

                let mut expr = None;
                license_files.retain(|lf| match &expr {
                    Some(cur) => {
                        if *cur != lf.license_expr {
                            expr = Some(lf.license_expr.clone());
                            true
                        } else {
                            false
                        }
                    }
                    None => {
                        expr = Some(lf.license_expr.clone());
                        true
                    }
                });

                Some(KrateLicense {
                    krate,
                    lic_info: info,
                    license_files,
                })
            })
            .collect();

        licensed_krates.append(&mut gathered);
    }
}

pub(crate) fn apply_clarification<'krate>(
    git_cache: &fetch::GitCache,
    krate: &'krate crate::Krate,
    clarification: &config::Clarification,
) -> anyhow::Result<Vec<LicenseFile>> {
    anyhow::ensure!(
        !clarification.files.is_empty() || !clarification.git.is_empty(),
        "clarification for crate '{}' does not specify any valid LICENSE files to checksum",
        krate.id
    );

    let root = krate.manifest_path.parent().unwrap();

    let mut lic_files = Vec::with_capacity(clarification.files.len() + clarification.git.len());

    let mut push = |contents: &str, cf: &config::ClarificationFile, license_path| {
        anyhow::ensure!(
            !contents.is_empty(),
            "clarification file '{}' is empty",
            license_path
        );

        let start = match &cf.start {
            Some(starts) => contents.find(starts).with_context(|| {
                format!(
                    "failed to find subsection starting with '{}' in {}",
                    starts, license_path
                )
            })?,
            None => 0,
        };

        let end = match &cf.end {
            Some(ends) => {
                contents[start..].find(ends).with_context(|| {
                    format!(
                        "failed to find subsection ending with '{}' in {}",
                        ends, license_path
                    )
                })? + start
                    + ends.len()
            }
            None => contents.len(),
        };

        let text = &contents[start..end];

        crate::validate_sha256(text, &cf.checksum)?;

        let text = text.to_owned();

        lic_files.push(LicenseFile {
            path: cf.path.clone(),
            confidence: 1.0,
            license_expr: cf
                .license
                .as_ref()
                .unwrap_or(&clarification.license)
                .clone(),
            kind: LicenseFileKind::Text(text),
        });

        Ok(())
    };

    for file in &clarification.files {
        let license_path = root.join(&file.path);
        let file_contents = std::fs::read_to_string(&license_path)
            .with_context(|| format!("unable to read path '{}'", license_path))?;

        push(&file_contents, file, license_path)?;
    }

    for file in &clarification.git {
        let license_path = &file.path;

        let contents = git_cache
            .retrieve(krate, file, &clarification.override_git_commit)
            .with_context(|| {
                format!(
                    "unable to retrieve '{}' for crate '{}' from remote git host",
                    license_path, krate
                )
            })?;

        push(&contents, file, license_path.clone())?;
    }

    Ok(lic_files)
}

#[inline]
pub fn binary_search<'krate>(
    kl: &'krate [KrateLicense<'krate>],
    krate: &Krate,
) -> Result<(usize, &'krate KrateLicense<'krate>), usize> {
    kl.binary_search_by(|k| k.krate.cmp(krate))
        .map(|i| (i, &kl[i]))
}
