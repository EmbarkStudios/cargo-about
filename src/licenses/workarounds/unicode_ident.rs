use super::ClarificationFile;
use anyhow::Context as _;

pub fn get(krate: &crate::Krate) -> anyhow::Result<Option<super::Clarification>> {
    if krate.name != "unicode-ident" {
        return Ok(None);
    }

    Ok(Some(super::Clarification {
        license: spdx::Expression::parse("(MIT OR Apache-2.0) AND Unicode-DFS-2016")
            .context("failed to parse license expression")?,
        override_git_commit: None,
        git: Vec::new(),
        files: vec![
            ClarificationFile {
                path: "LICENSE-UNICODE".into(),
                license: Some(
                    spdx::Expression::parse("Unicode-DFS-2016")
                        .context("failed to parse license expression")?,
                ),
                checksum: "68f5b9f5ea36881a0942ba02f558e9e1faf76cc09cb165ad801744c61b738844"
                    .to_owned(),
                start: None,
                end: None,
            },
            ClarificationFile {
                path: "LICENSE-APACHE".into(),
                license: Some(
                    spdx::Expression::parse("Apache-2.0")
                        .context("failed to parse license expression")?,
                ),
                checksum: "62c7a1e35f56406896d7aa7ca52d0cc0d272ac022b5d2796e7d6905db8a3636a"
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
    }))
}
