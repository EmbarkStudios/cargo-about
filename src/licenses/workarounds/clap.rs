use super::ClarificationFile;
use anyhow::Context as _;

pub fn get(krate: &crate::Krate) -> anyhow::Result<Option<super::Clarification>> {
    if !["clap", "clap_derive", "clap_generate"].contains(&krate.name.as_str()) {
        return Ok(None);
    }

    Ok(Some(super::Clarification {
        license: spdx::Expression::parse("MIT OR Apache-2.0")
            .context("failed to parse license expression")?,
        override_git_commit: None,
        git: vec![
            ClarificationFile {
                path: "LICENSE-APACHE".into(),
                license: Some(
                    spdx::Expression::parse("Apache-2.0")
                        .context("failed to parse license expression")?,
                ),
                checksum: "c71d239df91726fc519c6eb72d318ec65820627232b2f796219e87dcf35d0ab4"
                    .to_owned(),
                start: None,
                end: None,
            },
            ClarificationFile {
                path: "LICENSE-MIT".into(),
                license: Some(
                    spdx::Expression::parse("MIT").context("failed to parse license expression")?,
                ),
                checksum: "6725d1437fc6c77301f2ff0e7d52914cf4f9509213e1078dc77d9356dbe6eac5"
                    .to_owned(),
                start: None,
                end: None,
            },
        ],
        files: Vec::new(),
    }))
}
