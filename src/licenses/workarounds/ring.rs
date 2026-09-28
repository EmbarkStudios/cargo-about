use super::ClarificationFile;
use anyhow::Context as _;
use semver::Version;

pub fn get(krate: &crate::Krate) -> anyhow::Result<Option<super::Clarification>> {
    if krate.name != "ring" {
        return Ok(None);
    }

    // Older versions of ring tend to get yanked so instead of covering all versions
    // we just cover the current stable version
    let min_version = Version::new(0, 16, 0);
    anyhow::ensure!(
        krate.version >= min_version,
        "version {} is not covered, please file a PR to add it",
        krate.version
    );

    // Starting with 0.17.10, ring declares `Apache-2.0 AND ISC` and splits its
    // license texts into separate files, with LICENSE only pointing at them
    if krate.version >= Version::new(0, 17, 10) {
        return Ok(Some(super::Clarification {
            license: spdx::Expression::parse("Apache-2.0 AND ISC")
                .context("failed to parse license expression")?,
            override_git_commit: None,
            files: vec![
                // This is the ISC license that applies to the new code in ring
                ClarificationFile {
                    path: "LICENSE-other-bits".into(),
                    license: Some(
                        spdx::Expression::parse("ISC")
                            .context("failed to parse license expression")?,
                    ),
                    checksum: "f025ccfb7dfb6bdfedc75ca0f67acc69e6fb4998143d834f7c2f38a29989680f"
                        .to_owned(),
                    start: None,
                    end: None,
                },
                // This is the license for the code sourced from BoringSSL
                ClarificationFile {
                    path: "LICENSE-BoringSSL".into(),
                    license: Some(
                        spdx::Expression::parse("Apache-2.0")
                            .context("failed to parse license expression")?,
                    ),
                    checksum: "43070e2d4e532684de521b885f385d0841030efa2b1a20bafb76133a5e1379c1"
                        .to_owned(),
                    start: Some("                                 Apache License".to_owned()),
                    end: Some("limitations under the License.".to_owned()),
                },
            ],
            git: Vec::new(),
        }));
    }

    Ok(Some(super::Clarification {
        license: spdx::Expression::parse("ISC AND OpenSSL AND MIT")
            .context("failed to parse license expression")?,
        override_git_commit: None,
        files: vec![
            // This is the ISC license that actually applies to most/all of the rust code
            ClarificationFile {
                path: "LICENSE".into(),
                license: Some(
                    spdx::Expression::parse("ISC").context("failed to parse license expression")?,
                ),
                checksum: "ad5273d2df002d688c00405426acc3eaeba3d83333c61fc0bad7e878a889a65c"
                    .to_owned(),
                start: Some("   Copyright 2015-2016 Brian Smith.".to_owned()),
                end: Some("CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.".to_owned()),
            },
            // This is the license that applies to most of the code inherited from OpenSSL, via BoringSSL
            ClarificationFile {
                path: "LICENSE".into(),
                license: Some(
                    spdx::Expression::parse("OpenSSL")
                        .context("failed to parse license expression")?,
                ),
                checksum: "53552a9b197cd0db29bd085d81253e67097eedd713706e8cd2a3cc6c29850ceb"
                    .to_owned(),
                start: Some(
                    "/* ===================================================================="
                        .to_owned(),
                ),
                end: Some("\n * Hudson (tjh@cryptsoft.com).\n *\n */".to_owned()),
            },
            // This is the license for "new" code in BoringSSL
            ClarificationFile {
                path: "LICENSE".into(),
                license: Some(
                    spdx::Expression::parse("ISC").context("failed to parse license expression")?,
                ),
                checksum: "5dd6bae8b7ee15b1234a4ec7c01d9413e050cb1102e52e4ccae8de26ef63e2aa"
                    .to_owned(),
                start: Some("/* Copyright (c) 2015, Google Inc.".to_owned()),
                end: Some(
                    "\n * CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE. */\n".to_owned(),
                ),
            },
            // This is the license for the code in third_party/fiat
            ClarificationFile {
                path: "LICENSE".into(),
                license: Some(
                    spdx::Expression::parse("MIT").context("failed to parse license expression")?,
                ),
                checksum: "58f60c5a20faa9c92a535bf497d055233e46aa69e0301f6de1b7b7e4a2c5322f"
                    .to_owned(),
                start: Some("Copyright (c) 2015-2016 the fiat-crypto authors (see".to_owned()),
                end: Some("\nSOFTWARE.\n".to_owned()),
            },
        ],
        git: Vec::new(),
    }))
}

#[cfg(test)]
mod test {
    use super::*;
    use krates::cm;

    fn ring_krate(version: Version) -> crate::Krate {
        crate::Krate(cm::Package {
            name: "ring".to_owned(),
            version: version.clone(),
            authors: Vec::new(),
            id: cm::PackageId {
                repr: format!(
                    "ring {version} (registry+https://github.com/rust-lang/crates.io-index)"
                ),
            },
            source: None,
            description: None,
            dependencies: Vec::new(),
            license: None,
            license_file: None,
            targets: Vec::new(),
            features: std::collections::BTreeMap::new(),
            manifest_path: "Cargo.toml".into(),
            categories: Vec::new(),
            keywords: Vec::new(),
            readme: None,
            repository: None,
            homepage: None,
            documentation: None,
            edition: cm::Edition::E2021,
            metadata: serde_json::Value::Null,
            links: None,
            publish: None,
            default_run: None,
            rust_version: None,
        })
    }

    fn clarify(version: Version) -> super::super::Clarification {
        get(&ring_krate(version))
            .expect("ring version should be covered")
            .expect("ring should always be clarified")
    }

    /// Versions before 0.17.10 keep all of the license texts in a single LICENSE file
    #[test]
    fn uses_single_license_file_before_0_17_10() {
        let clarification = clarify(Version::new(0, 17, 9));

        assert_eq!(clarification.license.as_ref(), "ISC AND OpenSSL AND MIT");
        assert_eq!(clarification.files.len(), 4);
        assert!(clarification.files.iter().all(|cf| cf.path == "LICENSE"));
    }

    /// 0.17.10 moved the license texts into separate files, see #316
    #[test]
    fn uses_split_license_files_from_0_17_10() {
        for version in [Version::new(0, 17, 10), Version::new(0, 17, 14)] {
            let clarification = clarify(version);

            assert_eq!(clarification.license.as_ref(), "Apache-2.0 AND ISC");

            let files: Vec<_> = clarification
                .files
                .iter()
                .map(|cf| {
                    (
                        cf.path.as_str(),
                        cf.license.as_ref().map(|lic| lic.as_ref().to_owned()),
                    )
                })
                .collect();

            assert_eq!(
                files,
                [
                    ("LICENSE-other-bits", Some("ISC".to_owned())),
                    ("LICENSE-BoringSSL", Some("Apache-2.0".to_owned())),
                ]
            );
        }
    }
}
