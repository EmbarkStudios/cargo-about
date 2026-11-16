use crate::licenses::LicenseSource;

use super::{LicenseFile, LicenseFileKind};
use krates::{Utf8Path as Path, Utf8PathBuf as PathBuf};
use rayon::prelude::*;
use spdx::detection::scan::Scanner;

/// Globs, matched against the file name, for files that might contain license
/// text but aren't guaranteed to match any of the `ignore` crate's default types
const LICENSE_GLOBS: &[&str] = &[
    "[Ll][Ii][Cc][Ee][Nn][CcSs][Ee]*",
    "*[-._][Ll][Ii][Cc][Ee][Nn][CcSs][Ee]*",
    "[Uu][Nn][Ll][Ii][Cc][Ee][Nn][CcSs][Ee]*",
    "[Cc][Oo][Pp][Yy][Ii][Nn][Gg]*",
    "[Cc][Oo][Pp][Yy][Rr][Ii][Gg][Hh][Tt]*",
];

pub(crate) fn scan_files(
    root_dir: &Path,
    scanner: &Scanner<'_>,
    threshold: f32,
    max_depth: Option<usize>,
) -> anyhow::Result<Vec<LicenseFile>> {
    let types = {
        let mut tb = ignore::types::TypesBuilder::new();
        tb.add_defaults();
        // Selecting types drops every file that matches none of them, and the
        // default `license` type misses common spellings such as `license-mit`
        // or `LICENSE_MIT`, so add our own case-insensitive globs to it
        for glob in LICENSE_GLOBS {
            tb.add("license", glob)?;
        }
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

    let lic_id = match spdx::license_id(identified.name) {
        Some(id) => Identified {
            confidence: lic_match.score,
            id,
        },
        None => return ScanResult::UnknownId(identified.name.to_owned()),
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

    const MIT_TEXT: &str = "MIT License

Copyright (c) 2024 Example Author

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the \"Software\"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED \"AS IS\", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
";

    /// File names that crates on crates.io use for their license files but that
    /// the `ignore` crate's default types don't match, eg. `windows-core` ships
    /// `license-mit` and `tauri-plugin-opener` ships `LICENSE_MIT`
    #[test]
    fn scans_license_files_with_unrecognized_names() {
        // Names must differ ignoring case, for case-insensitive file systems
        let names = [
            "license-mit",
            "LICENSE_MIT",
            "Licence_Other",
            "MIT_LICENSE",
            "mit-license",
            "mit.licence",
            "unlicense_mit",
            "copying-mit",
            "copyright_mit",
            // Already matched by the default types
            "LICENSE",
        ];

        let temp = assert_fs::TempDir::new().unwrap();
        for name in names {
            std::fs::write(temp.path().join(name), MIT_TEXT).unwrap();
        }

        let store = crate::licenses::store_from_cache().unwrap();
        let scanner = Scanner::new(&store)
            .confidence_threshold(0.5)
            .optimize(false)
            .max_passes(1);

        let root = Path::from_path(temp.path()).unwrap();
        let files = scan_files(root, &scanner, 0.8, None).unwrap();

        let mut found: Vec<_> = files
            .iter()
            .map(|lf| {
                assert!(
                    matches!(&lf.license, LicenseSource::Detected(id) if id.name == "MIT"),
                    "'{}' wasn't detected as MIT",
                    lf.path
                );
                lf.path.file_name().unwrap()
            })
            .collect();
        found.sort_unstable();

        let mut expected = names.to_vec();
        expected.sort_unstable();

        assert_eq!(found, expected);
    }
}
