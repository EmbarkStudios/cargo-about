use super::ClarificationFile;
use anyhow::Context as _;

pub fn get(krate: &crate::Krate) -> anyhow::Result<Option<super::Clarification>> {
    if ![
        "atk-sys",
        "cairo-sys-rs",
        "gdk-pixbuf-sys",
        "gdk-sys",
        "gio-sys",
        "glib-sys",
        "gobject-sys",
        "gtk-sys",
    ]
    .contains(&krate.name.as_str())
    {
        return Ok(None);
    }

    Ok(Some(super::Clarification {
        license: spdx::Expression::parse("MIT").context("failed to parse license expression")?,
        override_git_commit: None,
        git: vec![ClarificationFile {
            path: "LICENSE".into(),
            license: None,
            checksum: "8cf56d10131ce201cf69ab74b111d3ebac1acca3833d7efb39ae357224b70edb".to_owned(),
            start: None,
            end: None,
        }],
        files: Vec::new(),
    }))
}
