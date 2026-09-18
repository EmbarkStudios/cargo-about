use crate::{
    Krate,
    licenses::{KrateLicense, LicenseInfo, LicenseSource, config},
};
use spdx::{Expression, LicenseReq, Licensee};
use std::fmt;
type Label = codespan_reporting::diagnostic::Label<codespan::FileId>;
use codespan_reporting::diagnostic::LabelStyle;

pub use codespan_reporting::diagnostic::Severity;
pub type Diagnostic = codespan_reporting::diagnostic::Diagnostic<codespan::FileId>;
pub type Files = codespan::Files<String>;

struct Accepted<'acc> {
    global: &'acc [Licensee],
    krate: Option<&'acc [Licensee]>,
}

impl<'acc> Accepted<'acc> {
    #[inline]
    fn satisfies(&self, req: &spdx::LicenseReq) -> bool {
        self.iter().any(|licensee| licensee.satisfies(req))
    }

    #[inline]
    fn iter(&'acc self) -> impl Iterator<Item = &'acc Licensee> {
        self.global
            .iter()
            .chain(self.krate.iter().flat_map(|o| o.iter()))
    }
}

impl fmt::Display for Accepted<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "global: [")?;
        for (id, val) in self.global.iter().enumerate() {
            write!(f, "{val}")?;
            if id + 1 < self.global.len() {
                write!(f, ", ")?;
            }
        }
        write!(f, "]")?;

        if let Some(krate) = self.krate {
            write!(f, "\ncrate: [")?;
            for (id, val) in krate.iter().enumerate() {
                write!(f, "{val}")?;
                if id + 1 < krate.len() {
                    write!(f, ", ")?;
                }
            }
            write!(f, "]")?;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct Resolved {
    /// The minimum license requirements that are required
    pub licenses: Vec<LicenseReq>,
    /// Diagnostics emitted during the course of the license resolution, may
    /// include errors
    pub diagnostics: Vec<Diagnostic>,
}

/// Synthesizes a package manifest for a krate with the specified license expression
fn synthesize_manifest(
    krate: &Krate,
    existing: Option<String>,
    expression: &spdx::Expression,
) -> (String, usize) {
    use std::fmt::Write;

    if let Some(mut existing) = existing
        && let Some(pkg) = existing.find("[package]")
    {
        // Find the first empty line, or next table
        let mut start = 0;
        let base = pkg + 5;
        let s = &existing[base..];
        for l in memchr::memchr_iter(b'\n', s.as_bytes()) {
            let line = &s[start..l];

            if line.trim().is_empty() || line.starts_with('[') {
                let insert = base + start;
                existing.insert_str(insert, "license = \"");
                let offset = insert + 11;
                existing.insert_str(offset, expression.as_ref());
                existing.insert_str(offset + expression.as_ref().len(), "\"\n");
                return (existing, offset);
            }

            start = l + 1;
        }

        if !existing.ends_with('\n') {
            existing.push('\n');
        }

        existing.push_str("license = \"");
        let offset = existing.len();
        writeln!(&mut existing, "{expression}\"").unwrap();

        (existing, offset)
    } else {
        let mut doc = String::with_capacity(256);

        doc.push_str("[package]\n");
        writeln!(&mut doc, "name = \"{}\"", krate.name).unwrap();
        writeln!(&mut doc, "version = \"{}\"", krate.version).unwrap();
        doc.push_str("authors = [");

        if let Some(single) = krate.authors.first()
            && krate.authors.len() == 1
        {
            write!(&mut doc, "\"{single}\"").unwrap();
        } else {
            for author in &krate.authors {
                writeln!(&mut doc, "    \"{author}\"").unwrap();
            }
        }
        doc.push_str("]\nlicense = \"");

        let offset = doc.len();
        writeln!(&mut doc, "{expression}\"").unwrap();

        (doc, offset)
    }
}

#[cfg(test)]
// Kept next to `synthesize_manifest` (rather than at the end of the file)
// so this stays mergeable with sibling fixes in `resolve`
#[allow(clippy::items_after_test_module)]
mod synthesize_offset_tests {
    use super::*;
    use krates::cm;

    fn synth_krate() -> Krate {
        Krate(cm::Package {
            name: "synth".to_owned(),
            version: semver::Version::new(0, 1, 0),
            authors: Vec::new(),
            id: cm::PackageId {
                repr: "synth 0.1.0 (path+file:///synth)".to_owned(),
            },
            source: None,
            description: None,
            dependencies: Vec::new(),
            license: None,
            license_file: None,
            targets: Vec::new(),
            features: std::collections::BTreeMap::new(),
            manifest_path: "Cargo.toml".into(),
            categories: Vec::new(),
            keywords: Vec::new(),
            readme: None,
            repository: None,
            homepage: None,
            documentation: None,
            edition: cm::Edition::E2021,
            metadata: serde_json::Value::Null,
            links: None,
            publish: None,
            default_run: None,
            rust_version: None,
        })
    }

    /// Offsets computed relative to the `&existing[pkg + 5..]` subslice must
    /// be rebased before indexing `existing`, otherwise the insert lands
    /// `pkg + 5` bytes early and can split a UTF-8 character, see #314
    #[test]
    fn inserts_at_package_table_end_with_multibyte_description() {
        let expr = spdx::Expression::parse("MIT").unwrap();
        let manifest = "[package]\r\nname = \"repro\"\r\nversion = \"0.1.0\"\r\nedition = \"2021\"\r\ndescription = \"日本語の説明\"\r\n\r\n[dependencies]\r\n".to_owned();

        let (synthesized, offset) = synthesize_manifest(&synth_krate(), Some(manifest), &expr);

        assert_eq!(
            synthesized,
            "[package]\r\nname = \"repro\"\r\nversion = \"0.1.0\"\r\nedition = \"2021\"\r\ndescription = \"日本語の説明\"\r\nlicense = \"MIT\"\n\r\n[dependencies]\r\n"
        );

        // The offset is used to map diagnostics back into the manifest, so it
        // still needs to point at the start of the expression
        assert_eq!(&synthesized[offset..offset + expr.as_ref().len()], "MIT");
    }

    /// Without multibyte characters the same bug silently corrupts an
    /// unrelated value instead of panicking, see #314
    #[test]
    fn inserts_at_package_table_end_without_corrupting_values() {
        let expr = spdx::Expression::parse("MIT").unwrap();
        let manifest =
            "[package]\nname = \"repro\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\n"
                .to_owned();

        let (synthesized, offset) = synthesize_manifest(&synth_krate(), Some(manifest), &expr);

        assert_eq!(
            synthesized,
            "[package]\nname = \"repro\"\nversion = \"0.1.0\"\nedition = \"2021\"\nlicense = \"MIT\"\n\n[dependencies]\n"
        );

        assert_eq!(&synthesized[offset..offset + expr.as_ref().len()], "MIT");

        let parsed =
            toml_span::parse(&synthesized).expect("synthesized manifest is not valid TOML");

        assert_eq!(
            parsed
                .pointer("/package/license")
                .and_then(toml_span::Value::as_str),
            Some("MIT")
        );
    }
}

/// Find the minimal set of required licenses for each crate.
pub fn resolve(
    licenses: &[KrateLicense<'_>],
    accepted: &[Licensee],
    krate_cfg: &std::collections::BTreeMap<String, config::KrateConfig>,
    files: &mut codespan::Files<String>,
    fail_on_missing: bool,
) -> Vec<Option<Resolved>> {
    licenses
        .iter()
        .map(|kl| {
            let mut resolved = Resolved {
                licenses: Vec::new(),
                diagnostics: Vec::new(),
            };

            // For published crates the manifest will be a sanitized one that is more uniform, but we could use the original
            // one that is in the same directory with .orig instead
            let manifest = std::fs::read_to_string(&kl.krate.manifest_path)
                .map_err(|e| {
                    log::error!(
                        "failed to read manifest path {} for crate '{}': {e}",
                        kl.krate.manifest_path,
                        kl.krate,
                    );
                    e
                })
                .ok();

            let expr = match &kl.lic_info {
                LicenseInfo::Expr(expr) => std::borrow::Cow::Borrowed(expr),
                LicenseInfo::Ignore => {
                    return None;
                }
                LicenseInfo::Unknown => {
                    // Find all of the unique license expressions that were discovered
                    // and concatenate them together
                    let mut unique_ids = Vec::new();

                    if kl.license_files.is_empty() {
                        let msg = format!("unable to synthesize license expression for '{}': no `license` specified, and no license files were found", kl.krate);

                        if fail_on_missing {
                            resolved.diagnostics.push(Diagnostic::new(Severity::Error).with_message(msg));
                        } else {
                            log::warn!("{msg}");
                        }

                        return Some(resolved);
                    }

                    for file in &kl.license_files {
                        let LicenseSource::Detected(id) = &file.license else { continue; };

                        if let Err(i) = unique_ids.binary_search(id) {
                            unique_ids.insert(i, *id);
                        }
                    }

                    let mut concat_expr = String::new();
                    for (i, id) in unique_ids.into_iter().enumerate() {
                        if i > 0 {
                            concat_expr.push_str(" AND ");
                        }

                        concat_expr.push('(');
                        concat_expr.push_str(id.name);
                        concat_expr.push(')');
                    }

                    match Expression::parse(&concat_expr) {
                        Ok(expr) => std::borrow::Cow::Owned(expr),
                        Err(e) => {
                            let span = e.span;
                            let reason = e.reason;

                            let failed_expr_id =
                                files.add(format!("{}.license", kl.krate), concat_expr);

                            resolved.diagnostics.push(
                                Diagnostic::new(Severity::Error)
                                    .with_message("failed to parse synthesized license expression")
                                    .with_labels(vec![Label::new(
                                        LabelStyle::Primary,
                                        failed_expr_id,
                                        span,
                                    )
                                    .with_message(reason.to_string())]),
                            );

                            return Some(resolved);
                        }
                    }
                }
            };

            let expr_offset =
                if let (LicenseInfo::Expr(expr), Some(manifest)) = (&kl.lic_info, &manifest) {
                    manifest.find(expr.as_ref())
                } else {
                    None
                };

            // If we don't have an expression offset either because we don't have a manifest, or the expression wasn't
            // there to begin with, we need to synthesize one instead
            let (manifest, expr_offset) = match (manifest, expr_offset) {
                (Some(manifest), Some(expr_offset)) => (manifest, expr_offset),
                (manifest, None) => {
                    synthesize_manifest(kl.krate, manifest, &expr)
                }
                (None, Some(_)) => unreachable!(),
            };

            // Retrieve additional crate specific licenses
            let accepted = match krate_cfg.get(&kl.krate.name) {
                Some(kcfg) => {
                    if kcfg.accepted.is_empty() {
                        Accepted {
                            global: accepted,
                            krate: None,
                        }
                    } else {
                        Accepted {
                            global: accepted,
                            krate: Some(&kcfg.accepted),
                        }
                    }
                }
                None => Accepted {
                    global: accepted,
                    krate: None,
                },
            };

            let manifest_file_id = files.add(kl.krate.manifest_path.clone(), manifest);

            // Evaluates the expression against the accepted licenses to ensure it can
            // be satisfied according to the user's configuration
            if let Err(failed) = expr.evaluate_with_failures(|req| accepted.satisfies(req)) {
                resolved.diagnostics.push(
                    Diagnostic::new(Severity::Error)
                        .with_message("failed to satisfy license requirements")
                        .with_labels(
                            failed
                                .into_iter()
                                .map(|fr| {
                                    let span = fr.span.start as usize + expr_offset
                                        ..fr.span.end as usize + expr_offset;
                                    Label::new(LabelStyle::Secondary, manifest_file_id, span)
                                })
                                .collect(),
                        ),
                );

                return Some(resolved);
            }

            // Attempt to  find the minimal set of licenses needed to satisfy the
            // license requirements, in priority order
            match expr.minimized_requirements(accepted.iter()) {
                Ok(min_reqs) => {
                    resolved.licenses = min_reqs;
                }
                Err(e) => {
                    log::warn!("failed to minimize license requirements: {e}");
                }
            }

            Some(resolved)
        })
        .collect()
}

#[cfg(test)]
mod test {
    use super::*;
    use krates::cm;

    fn synth_krate() -> Krate {
        Krate(cm::Package {
            name: "synth".to_owned(),
            version: semver::Version::new(0, 1, 0),
            authors: Vec::new(),
            id: cm::PackageId {
                repr: "synth 0.1.0 (path+file:///synth)".to_owned(),
            },
            source: None,
            description: None,
            dependencies: Vec::new(),
            license: None,
            license_file: None,
            targets: Vec::new(),
            features: std::collections::BTreeMap::new(),
            manifest_path: "Cargo.toml".into(),
            categories: Vec::new(),
            keywords: Vec::new(),
            readme: None,
            repository: None,
            homepage: None,
            documentation: None,
            edition: cm::Edition::E2021,
            metadata: serde_json::Value::Null,
            links: None,
            publish: None,
            default_run: None,
            rust_version: None,
        })
    }

    /// The append path is only taken when the `[package]` table runs to the end
    /// of the manifest, ie. there is no blank line or subsequent table to insert
    /// the synthesized `license` field before, see #315
    #[test]
    fn synthesizes_a_terminated_license_field_when_appending() {
        let expr = spdx::Expression::parse("MIT").unwrap();
        let manifest = "[package]\nname = \"synth\"\nversion = \"0.1.0\"".to_owned();

        let (synthesized, offset) = synthesize_manifest(&synth_krate(), Some(manifest), &expr);

        assert_eq!(
            synthesized,
            "[package]\nname = \"synth\"\nversion = \"0.1.0\"\nlicense = \"MIT\"\n"
        );

        // The offset is used to map diagnostics back into the manifest, so it
        // still needs to point at the start of the expression
        assert_eq!(&synthesized[offset..offset + expr.as_ref().len()], "MIT");

        let parsed =
            toml_span::parse(&synthesized).expect("synthesized manifest is not valid TOML");

        assert_eq!(
            parsed
                .pointer("/package/license")
                .and_then(toml_span::Value::as_str),
            Some("MIT")
        );
    }
}
