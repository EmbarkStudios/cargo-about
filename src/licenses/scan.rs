use crate::licenses::LicenseSource;

use super::{LicenseFile, LicenseFileKind};
use krates::{Utf8Path as Path, Utf8PathBuf as PathBuf};
use rayon::prelude::*;
use spdx::detection::scan::Scanner;

pub(crate) fn scan_files(
    root_dir: &Path,
    scanner: &Scanner<'_>,
    threshold: f32,
    max_depth: Option<usize>,
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
        .max_depth(max_depth)
        .types(types)
        .build();

    let files: Vec<_> = walker.filter_map(|e| e.ok()).collect();

    let license_files: Vec<_> = files
        .into_par_iter()
        .filter_map(|file| {
            log::trace!("scanning file {}", file.path().display());

            if let Some(ft) = file.file_type()
                && ft.is_dir()
            {
                return None;
            }

            // Check for pipes on unix just in case
            #[cfg(unix)]
            {
                use std::os::unix::fs::FileTypeExt;

                if let Ok(md) = file.metadata()
                    && md.file_type().is_fifo()
                {
                    log::error!("skipping FIFO {}", file.path().display());
                    return None;
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

            check_is_license_file(path, contents, scanner, threshold)
        })
        .collect();

    Ok(license_files)
}

fn read_file(path: &Path) -> Option<String> {
    match std::fs::read_to_string(path) {
        Err(ref e) if e.kind() == std::io::ErrorKind::InvalidData => {
            // If we fail due to invaliddata, it just means the file in question was
            // probably binary and didn't have valid utf-8 data, so we can ignore it
            log::debug!("binary file '{path}' detected");
            None
        }
        Err(e) => {
            log::error!("failed to read '{path}': {e}");
            None
        }
        Ok(c) => Some(c),
    }
}

pub(crate) fn check_is_license_file(
    path: PathBuf,
    contents: String,
    scanner: &Scanner<'_>,
    threshold: f32,
) -> Option<LicenseFile> {
    // askalono only detects single license identifiers, not license
    // expressions
    let scan_res = scan_text(&contents, scanner, threshold);

    match scan_res {
        ScanResult::Header(ided) => Some(LicenseFile {
            license: LicenseSource::Detected(ided.id),
            confidence: ided.confidence,
            path,
            kind: LicenseFileKind::Header,
        }),
        ScanResult::Text(ided) => Some(LicenseFile {
            license: LicenseSource::Detected(ided.id),
            confidence: ided.confidence,
            path,
            kind: LicenseFileKind::Text(contents),
        }),
        ScanResult::UnknownId(id_str) => {
            log::error!("found unknown SPDX identifier '{id_str}' scanning '{path}'");
            None
        }
        ScanResult::LowLicenseChance(ided) => {
            log::debug!(
                "found '{}' scanning '{path}' but it only has a confidence score of {}",
                ided.id.name,
                ided.confidence,
            );
            None
        }
        ScanResult::NoLicense => None,
    }
}

#[derive(Debug)]
struct Identified {
    confidence: f32,
    id: spdx::LicenseId,
}

#[derive(Debug)]
enum ScanResult {
    Header(Identified),
    Text(Identified),
    UnknownId(String),
    LowLicenseChance(Identified),
    NoLicense,
}

fn scan_text(contents: &str, strat: &Scanner<'_>, threshold: f32) -> ScanResult {
    let text = spdx::detection::TextData::new(contents);
    let lic_match = strat.scan(&text);

    let Some(identified) = lic_match.license else {
        return ScanResult::NoLicense;
    };

    // The Pixar license is an almost exact copy of Apache-2.0, to the point
    // that an Apache-2.0 license text without its appendix is detected as
    // Pixar. The real Pixar license is titled "Modified Apache 2.0 License",
    // so if that is missing, it is actually Apache-2.0.
    let name = if identified.name == "Pixar"
        && !contents
            .trim_start()
            .starts_with("Modified Apache 2.0 License")
    {
        "Apache-2.0"
    } else {
        identified.name
    };

    let lic_id = match spdx::license_id(name) {
        Some(id) => Identified {
            confidence: lic_match.score,
            id,
        },
        None => return ScanResult::UnknownId(name.to_owned()),
    };

    use spdx::detection::LicenseType;

    // askalano doesn't report any matches below the confidence threshold
    // but we want to see what it thinks the license is if the confidence
    // is somewhat ok at least
    if lic_match.score >= threshold {
        match identified.kind {
            LicenseType::Header => ScanResult::Header(lic_id),
            LicenseType::Original => ScanResult::Text(lic_id),
            LicenseType::Alternate => {
                panic!("Alternate license detected")
            }
        }
    } else {
        ScanResult::LowLicenseChance(lic_id)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn scan(text: &str) -> spdx::LicenseId {
        let store = crate::licenses::store_from_cache().unwrap();
        let scanner = Scanner::new(&store)
            .confidence_threshold(0.8)
            .optimize(false)
            .max_passes(1);

        let lf = check_is_license_file("LICENSE".into(), text.to_owned(), &scanner, 0.8)
            .expect("license not detected");

        match lf.license {
            LicenseSource::Detected(id) => id,
            LicenseSource::Clarified(_) => panic!("unexpected license source"),
        }
    }

    fn text_of(name: &str) -> &'static str {
        spdx::text::LICENSE_TEXTS
            .iter()
            .find_map(|(n, t)| (*n == name).then_some(*t))
            .unwrap()
    }

    // Apache-2.0 without the appendix is extremely similar to Pixar
    #[test]
    fn apache_without_appendix_is_not_pixar() {
        let apache = text_of("Apache-2.0");
        let end = apache.find("END OF TERMS AND CONDITIONS").unwrap();

        assert_eq!(scan(&apache[..end]).name, "Apache-2.0");
    }

    #[test]
    fn detects_actual_pixar() {
        assert_eq!(scan(text_of("Pixar")).name, "Pixar");
    }
}
