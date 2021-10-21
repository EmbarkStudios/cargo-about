use super::ClarificationFile;
use anyhow::Context as _;

pub fn get(krate: &crate::Krate) -> anyhow::Result<Option<super::Clarification>> {
    if ![
        "cranelift-bforest",
        "cranelift-codegen",
        "cranelift-codegen-meta",
        "cranelift-codegen-shared",
        "cranelift-entity",
        "cranelift-frontend",
        "cranelift-native",
        "cranelift-wasm",
        "wasi-cap-std-sync",
        "wasi-common",
        "wasmtime",
        "wasmtime-environ",
        "wasmtime-jit",
        "wasmtime-runtime",
        "wasmtime-types",
        "wasmtime-wasi",
        "wast",
        "wiggle",
        "wiggle-generate",
        "wiggle-macro",
        "winx",
    ]
    .contains(&krate.name.as_str())
    {
        return Ok(None);
    }

    // fixed in https://github.com/bytecodealliance/wasmtime/commit/b5e289d319b2788bb4b6133792546007f7900443#diff-42013ab1aca14e65a6a2b70d5808c75ea3dd331e7436e2cd8b756fa6b96c3296,
    // but at the time of writing, unreleased
    if krate.name == "wasmtime-types" {
        Ok(Some(super::Clarification {
            license: spdx::Expression::parse("Apache-2.0 WITH LLVM-exception")
                .context("failed to parse license expression")?,
            git: vec![ClarificationFile {
                path: "LICENSE".into(),
                license: None,
                checksum: "268872b9816f90fd8e85db5a28d33f8150ebb8dd016653fb39ef1f94f2686bc5"
                    .to_owned(),
                start: None,
                end: None,
            }],
            files: Vec::new(),
        }))
    } else {
        Ok(Some(super::Clarification {
            license: spdx::Expression::parse("Apache-2.0 WITH LLVM-exception")
                .context("failed to parse license expression")?,
            files: vec![
                // Both clearlydefined and askalono don't handle license exceptions it seems, so we need to clarify
                // the file otherwise we will think we won't find the license we expected
                ClarificationFile {
                    path: "LICENSE".into(),
                    license: None,
                    checksum: "268872b9816f90fd8e85db5a28d33f8150ebb8dd016653fb39ef1f94f2686bc5"
                        .to_owned(),
                    start: None,
                    end: None,
                },
            ],
            git: Vec::new(),
        }))
    }
}
