use super::ClarificationFile;
use anyhow::Context as _;

pub fn get(krate: &crate::Krate) -> anyhow::Result<Option<super::Clarification>> {
    if krate.name != "tonic" && krate.name != "tonic-build" {
        return Ok(None);
    }

    Ok(Some(super::Clarification {
        license: spdx::Expression::parse("MIT").context("failed to parse license expression")?,
        override_git_commit: None,
        git: vec![ClarificationFile {
            path: "LICENSE".into(),
            license: None,
            checksum: "4f38e3a425725eb447213c75c0d8ae9f0d1f2ebc4f3183e2106aaf07c23f4b20".to_owned(),
            start: None,
            end: None,
        }],
        files: Vec::new(),
    }))
}
