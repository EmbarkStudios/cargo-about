use super::{LicenseFile, LicenseFileKind};
use krates::{Utf8Path as Path, Utf8PathBuf as PathBuf};
use rayon::prelude::*;

pub(crate) fn scan_files(
    root_dir: &Path,
    strat: &askalono::ScanStrategy<'_>,
    threshold: f32,
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

            let contents = read_file(&path)?;

            check_is_license_file(path, contents, strat, threshold)
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

pub(crate) fn check_is_license_file(
    path: PathBuf,
    contents: String,
    strat: &askalono::ScanStrategy<'_>,
    threshold: f32,
) -> Option<LicenseFile> {
    match scan_text(&contents, strat, threshold) {
        ScanResult::Header(ided) => {
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
                kind: LicenseFileKind::Text(contents),
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
