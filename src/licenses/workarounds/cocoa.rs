use super::ClarificationFile;
use anyhow::Context as _;

pub fn get(krate: &crate::Krate) -> anyhow::Result<Option<super::Clarification>> {
    // The other crates in this repo are correct
    // "cocoa", "core-graphics", "core-text"

    if ![
        "cocoa-foundation",
        "core-foundation",
        "core-foundation-sys",
        "core-graphics-types",
    ]
    .contains(&krate.name.as_str())
    {
        return Ok(None);
    }

    // It seems core-graphics-types was published from a branch and the commit
    // was nuked, so we override the git commit to the current HEAD in that case
    // for now, until it is hopefully fixed
    let override_git_commit = if krate.name == "core-graphics-types" {
        Some("3841d2bb3aa76dec2ea6319e757603fb923b5a50".to_owned())
    } else {
        None
    };

    Ok(Some(super::Clarification {
        license: spdx::Expression::parse("MIT OR Apache-2.0")
            .context("failed to parse license expression")?,
        override_git_commit,
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
                checksum: "62065228e42caebca7e7d7db1204cbb867033de5982ca4009928915e4095f3a3"
                    .to_owned(),
                start: None,
                end: None,
            },
        ],
        files: Vec::new(),
    }))
}
