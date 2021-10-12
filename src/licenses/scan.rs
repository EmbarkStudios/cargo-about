use super::{config, LicenseFile, LicenseFileKind};
use krates::{Utf8Path as Path, Utf8PathBuf as PathBuf};
use rayon::prelude::*;

pub(crate) fn scan_files(
    root_dir: &Path,
    strat: &askalono::ScanStrategy<'_>,
    threshold: f32,
    krate_cfg: Option<(&config::KrateConfig, &str)>,
) -> anyhow::Result<Vec<LicenseFile>> {
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

            let path = match PathBuf::from_path_buf(file.into_path()) {
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

                            addendum.map(|addendum| {
                                (addendum.license.clone(), Some(&addendum.license_file))
                            })
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
                    log::debug!("ignoring '{}', matched license '{}'", path, ided.id.name);
                    return None;
                }
            }

            // askalono only detects single license identifiers, not license
            // expressions, so we need to construct one from a single identifier,
            // this should be made into in infallible function in spdx itself
            let license_expr = match spdx::Expression::parse(ided.id.name) {
                Ok(expr) => expr,
                Err(err) => {
                    log::error!(
                        "failed to parse license '{}' into a valid expression: {}",
                        ided.id.name,
                        err
                    );
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
                        log::debug!("ignoring '{}', matched license '{}'", path, ided.id.name);
                        return None;
                    }
                }
            } else {
                LicenseFileKind::Text(contents)
            };

            let license_expr = match spdx::Expression::parse(ided.id.name) {
                Ok(expr) => expr,
                Err(err) => {
                    log::error!(
                        "failed to parse license '{}' into a valid expression: {}",
                        ided.id.name,
                        err
                    );
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
            log::trace!(
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
