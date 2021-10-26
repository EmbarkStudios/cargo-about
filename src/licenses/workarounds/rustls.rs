use super::ClarificationFile;
use anyhow::Context as _;

pub fn get(krate: &crate::Krate) -> anyhow::Result<Option<super::Clarification>> {
    if krate.name != "rustls" {
        return Ok(None);
    }

    Ok(Some(super::Clarification {
        license: spdx::Expression::parse("Apache-2.0 OR MIT OR ISC")
            .context("failed to parse license expression")?,
        override_git_commit: None,
        git: vec![
            ClarificationFile {
                path: "LICENSE-APACHE".into(),
                license: Some(
                    spdx::Expression::parse("Apache-2.0")
                        .context("failed to parse license expression")?,
                ),
                checksum: "a60eea817514531668d7e00765731449fe14d059d3249e0bc93b36de45f759f2"
                    .to_owned(),
                start: None,
                end: None,
            },
            ClarificationFile {
                path: "LICENSE-MIT".into(),
                license: Some(
                    spdx::Expression::parse("MIT").context("failed to parse license expression")?,
                ),
                checksum: "709e3175b4212f7b13aa93971c9f62ff8c69ec45ad8c6532a7e0c41d7a7d6f8c"
                    .to_owned(),
                start: None,
                end: None,
            },
            ClarificationFile {
                path: "LICENSE-ISC".into(),
                license: Some(
                    spdx::Expression::parse("ISC").context("failed to parse license expression")?,
                ),
                checksum: "7cfafc877eccc46c0e346ccbaa5c51bb6b894d2b818e617d970211e232785ad4"
                    .to_owned(),
                start: None,
                end: None,
            },
        ],
        files: Vec::new(),
    }))
}
