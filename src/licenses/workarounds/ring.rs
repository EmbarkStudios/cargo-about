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
