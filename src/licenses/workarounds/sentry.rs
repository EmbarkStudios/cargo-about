use super::ClarificationFile;
use anyhow::Context as _;

pub fn get(krate: &crate::Krate) -> anyhow::Result<Option<super::Clarification>> {
    if !krate.name.starts_with("sentry-") {
        return Ok(None);
    }

    Ok(Some(super::Clarification {
        license: spdx::Expression::parse("MIT").context("failed to parse license expression")?,
        // None of the sentry packages include the .cargo_vcs_info.json metadata file
        // so we pin it to the 0.23.0 commit for now
        override_git_commit: Some("151b2c08eeff8994c65285ae0aab77b60d1ea1dc".to_owned()),
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
