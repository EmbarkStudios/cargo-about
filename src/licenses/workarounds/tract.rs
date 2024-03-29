use super::ClarificationFile;
use anyhow::Context as _;

pub fn get(krate: &crate::Krate) -> anyhow::Result<Option<super::Clarification>> {
    if !krate.name.starts_with("tract-") {
        return Ok(None);
    }

    Ok(Some(super::Clarification {
        license: spdx::Expression::parse("Apache-2.0 OR MIT")
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
                checksum: "23f18e03dc49df91622fe2a76176497404e46ced8a715d9d2b67a7446571cca3"
                    .to_owned(),
                start: None,
                end: None,
            },
        ],
        files: Vec::new(),
    }))
}
