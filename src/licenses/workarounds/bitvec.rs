use super::ClarificationFile;
use anyhow::Context as _;

pub fn get(krate: &crate::Krate) -> anyhow::Result<Option<super::Clarification>> {
    let checksum = match krate.name.as_str() {
        "bitvec" => "411781fd38700f2357a14126d0ab048164ab881f1dcb335c1bb932e232c9a2f5",
        "wyz" => "43fb7b0d1c6fa07d1ffe65d574dc53830cc31027d7c171e4b65f128d74190d94",
        _ => return Ok(None),
    }
    .to_owned();

    Ok(Some(super::Clarification {
        license: spdx::Expression::parse("MIT").context("failed to parse license expression")?,
        override_git_commit: None,
        git: vec![ClarificationFile {
            path: "LICENSE.txt".into(),
            license: None,
            checksum,
            start: None,
            end: None,
        }],
        files: Vec::new(),
    }))
}
