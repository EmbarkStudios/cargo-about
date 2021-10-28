use super::ClarificationFile;
use anyhow::Context as _;

pub fn get(krate: &crate::Krate) -> anyhow::Result<Option<super::Clarification>> {
    if ![
        "sentry",
        "sentry-backtrace",
        "sentry-contexts",
        "sentry-core",
        "sentry-debug-images",
        "sentry-types",
    ]
    .contains(&krate.name.as_str())
    {
        return Ok(None);
    }

    Ok(Some(super::Clarification {
        license: spdx::Expression::parse("MIT").context("failed to parse license expression")?,
        override_git_commit: Some(krate.version.to_string()),
        git: vec![ClarificationFile {
            path: "LICENSE".into(),
            license: None,
            checksum: "cfc7749b96f63bd31c3c42b5c471bf756814053e847c10f3eb003417bc523d30".to_owned(),
            start: None,
            end: None,
        }],
        files: Vec::new(),
    }))
}
