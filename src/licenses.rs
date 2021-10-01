use crate::{Krate, Krates};
use anyhow::{bail, Context, Error};
use rayon::prelude::*;
use spdx::{LicenseReq, Licensee};
use std::{
    cmp, fmt,
    sync::Arc,
};
use krates::{Utf8Path as Path, Utf8PathBuf as PathBuf};

pub mod config;

const LICENSE_CACHE: &[u8] = include_bytes!("../spdx_cache.bin.zstd");

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
            cmp::Ordering::Equal => {
                o.confidence
                    .partial_cmp(&self.confidence)
                    .expect("NaN encountered comparing license confidences")
            }
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
        self.krate.cmp(&o.krate)
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

    pub fn gather<'k>(self, krates: &'k Krates, cfg: &config::Config) -> Summary<'k> {
        let mut summary = Summary::new(self.store.clone());

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

        // First attempt to gather license information from clearly-defined.io so
        // we can get previously gathered license information + any possible
        // curations so that we only need to fallback to scanning local crate
        // sources if it's not already in clearly-defined
        let mut cded = self.gather_clearly_defined(krates, cfg);

        let mut gathered: Vec<_> = krates
            .krates()
            .par_bridge()
            .filter_map(|kn| {
                let krate = &kn.krate;

                // Ignore crates that we've already gathered from clearlydefined
                if cded.binary_search_by(|cd_lic| cd_lic.krate.cmp(krate)).is_ok() {
                    return None;
                }

                let info = krate.get_license_expression();

                let root_path = krate.manifest_path.parent().unwrap();
                let krate_cfg = cfg.crates.get(&krate.name);

                let mut license_files = match scan_files(
                    root_path,
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

        gathered.append(&mut cded);
        summary.nfos = gathered;

        summary
    }

    fn gather_clearly_defined<'k>(
        &self,
        krates: &'k Krates,
        cfg: &config::Config,
    ) -> Vec<KrateLicense<'k>> {
        if cfg.disallow_clearly_defined {
            return Vec::new();
        }

        let reqs = cd::definitions::get(10, krates.krates().filter_map(|krate| {
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
        }));

        let mut gathered = Vec::with_capacity(krates.len() / 2);

        //let threshold = std::cmp::min(std::cmp::max(10, (self.threshold * 100.0) as u8), 100);

        for req in reqs {
            match self.cd_client.execute::<cd::definitions::GetResponse>(req) {
                Ok(response) => {
                    gathered.extend(response.definitions.into_iter().filter_map(|def| {
                        if def.described.is_none() {
                            log::info!("the definition for {} has not been harvested", def.coordinates);
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
                                    // clearly defined will attach a license identifier to any file
                                    // with a license or SPDX identifier, but like askalono it won't
                                    // detect all licenses if there are multiple in a single file
                                    match cd_file.license {
                                        Some(lic) => {
                                            // NOASSERTION is not yet? (https://github.com/spdx/spdx-spec/issues/50)
                                            // a part of the SPDX spec, so we basically just treat it as a "nope"
                                            if lic == "NOASSERTION" {
                                                return None;
                                            }

                                            let license_expr = match spdx::Expression::parse_mode(&lic, spdx::ParseMode::Lax) {
                                                Ok(expr) => expr,
                                                Err(err) => {
                                                    log::warn!("clearly defined detected license '{}' in '{}' for crate '{}', but it can't be parsed: {}", lic, cd_file.path, krate, err);
                                                    return None;
                                                }
                                            };
    
                                            let lic_file_info = if cd_file.natures.iter().any(|s| s == "license") {
                                                let root_path = krate.manifest_path.parent().unwrap();
                                                let path = root_path.join(&cd_file.path);
                                                match std::fs::read_to_string(&path) {
                                                    Ok(text) => {
                                                        // TODO: verify the sha256 matches
                                                        LicenseFileKind::Text(text)
                                                    }
                                                    Err(err) => {
                                                        log::warn!("failed to read license from '{}' for crate '{}': {}", path, krate, err);
                                                        return None;
                                                    }
                                                }
                                            } else {
                                                LicenseFileKind::Header
                                            };
    
                                            Some(LicenseFile {
                                                license_expr,
                                                path: cd_file.path.into(),
                                                confidence,
                                                kind: lic_file_info,
                                            })
                                        }
                                        None => None,
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
                    }));
                }
                Err(err) => {
                    log::warn!(
                        "failed to request license information from clearly defined: {:#}",
                        err
                    );
                }
            }
        }

        gathered.sort();
        gathered
    }
}

fn scan_files(
    root_dir: &Path,
    strat: &askalono::ScanStrategy<'_>,
    threshold: f32,
    krate_cfg: Option<(&config::KrateConfig, &str)>,
) -> Result<Vec<LicenseFile>, Error> {
    let types = {
        let mut tb = ignore::types::TypesBuilder::new();
        tb.add_defaults();
        tb.select("all");
        tb.build()?
    };

    let walker = ignore::WalkBuilder::new(root_dir)
        .standard_filters(true)
        .follow_links(true)
        .types(types)
        .build();

    let files: Vec<_> = walker.filter_map(|e| e.ok()).collect();

    let license_files: Vec<_> = files
        .into_par_iter()
        .filter_map(|file| {
            log::trace!("scanning file {}", file.path().display());

            if let Some(ft) = file.file_type() {
                if ft.is_dir() {
                    return None;
                }
            }

            // Check for pipes on unix just in case
            #[cfg(unix)]
            {
                use std::os::unix::fs::FileTypeExt;

                if let Ok(md) = file.metadata() {
                    if md.file_type().is_fifo() {
                        log::error!("skipping FIFO {}", file.path().display());
                        return None;
                    }
                }
            }

            let pb = file.into_path();
            let path = match PathBuf::from_path_buf(pb) {
                Ok(pb) => pb,
                Err(e) => {
                    log::warn!("skipping path {}, not a valid utf-8 path", e.display());
                    return None;
                }
            };

            let mut contents = match read_file(&path) {
                Some(c) => c,
                None => return None,
            };

            let expected = match krate_cfg {
                Some(krate_cfg) => {
                    let relative = match path.strip_prefix(root_dir) {
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
                            Some((ignore.license.clone(), None))
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
                                        path,
                                        additional.root,
                                    );
                                    return None;
                                }
                            }

                            addendum
                                .map(|addendum| (addendum.license.clone(), Some(&addendum.license_file)))
                        }
                    }
                }
                None => None,
            };

            check_is_license_file(path, contents, strat, threshold, expected)
        })
        .collect();

    Ok(license_files)
}

fn read_file(path: &Path) -> Option<String> {
    match std::fs::read_to_string(path) {
        Err(ref e) if e.kind() == std::io::ErrorKind::InvalidData => {
            // If we fail due to invaliddata, it just means the file in question was
            // probably binary and didn't have valid utf-8 data, so we can ignore it
            log::debug!("binary file {} detected", path);
            None
        }
        Err(e) => {
            log::error!("failed to read '{}': {}", path, e);
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

fn check_is_license_file(
    path: PathBuf,
    contents: String,
    strat: &askalono::ScanStrategy<'_>,
    threshold: f32,
    expected: Option<(spdx::Expression, Option<&PathBuf>)>,
) -> Option<LicenseFile> {
    match scan_text(&contents, strat, threshold) {
        ScanResult::Header(ided) => {
            if let Some((expected_expr, addendum)) = expected {
                if !expected_expr.evaluate(|req| req.license.id() == Some(ided.id)) {
                    log::error!(
                        "expected license '{}' in path '{}', but found '{}'",
                        expected_expr,
                        path,
                        ided.id.name
                    );
                } else if addendum.is_none() {
                    log::debug!(
                        "ignoring '{}', matched license '{}'",
                        path,
                        ided.id.name
                    );
                    return None;
                }
            }

            // askalono only detects single license identifiers, not license
            // expressions, so we need to construct one from a single identifier,
            // this should be made into in infallible function in spdx itself
            let license_expr = match spdx::Expression::parse(ided.id.name) {
                Ok(expr) => expr,
                Err(err) => {
                    log::error!("failed to parse license '{}' into a valid expression: {}", ided.id.name, err);
                    return None;
                }
            };

            Some(LicenseFile {
                license_expr,
                confidence: ided.confidence,
                path,
                kind: LicenseFileKind::Header,
            })
        }
        ScanResult::Text(ided) => {
            let kind = if let Some((expected_expr, addendum)) = expected {
                if !expected_expr.evaluate(|req| req.license.id() == Some(ided.id)) {
                    log::error!(
                        "expected license '{}' in path '{}', but found '{}'",
                        expected_expr,
                        path,
                        ided.id.name
                    );
                }

                match addendum {
                    Some(path) => LicenseFileKind::AddendumText(contents, path.clone()),
                    None => {
                        log::debug!(
                            "ignoring '{}', matched license '{}'",
                            path,
                            ided.id.name
                        );
                        return None;
                    }
                }
            } else {
                LicenseFileKind::Text(contents)
            };

            let license_expr = match spdx::Expression::parse(ided.id.name) {
                Ok(expr) => expr,
                Err(err) => {
                    log::error!("failed to parse license '{}' into a valid expression: {}", ided.id.name, err);
                    return None;
                }
            };

            Some(LicenseFile {
                license_expr,
                confidence: ided.confidence,
                path,
                kind,
            })
        }
        ScanResult::UnknownId(id_str) => {
            log::error!(
                "found unknown SPDX identifier '{}' scanning '{}'",
                id_str,
                path,
            );
            None
        }
        ScanResult::LowLicenseChance(ided) => {
            log::debug!(
                "found '{}' scanning '{}' but it only has a confidence score of {}",
                ided.id.name,
                path,
                ided.confidence,
            );
            None
        }
        ScanResult::NoLicense => None,
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
    let text = askalono::TextData::new(contents);
    match strat.scan(&text) {
        Ok(lic_match) => {
            match lic_match.license {
                Some(identified) => {
                    let lic_id = match spdx::license_id(identified.name) {
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
                                panic!("Alternate license detected")
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
            panic!("askalalono elimination strategy failed: {}", e);
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
                            .flat_map(|file| {
                                file.license_expr.requirements().map(|ereq| ereq.req.clone())
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
