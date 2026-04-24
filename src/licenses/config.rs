use krates::Utf8PathBuf as PathBuf;
use spdx::Expression;
use std::collections::BTreeMap;
use toml_span::{Deserialize, Value, de_helpers as de};

const MODE: spdx::ParseMode = spdx::ParseMode {
    allow_deprecated: true,
    allow_slash_as_or_operator: false,
    allow_imprecise_license_names: false,
    allow_postfix_plus_on_gpl: false,
    allow_unknown: false,
};

#[inline]
fn parse_expr<'de>(
    th: &mut de::TableHelper<'de>,
    key: &'static str,
) -> Result<Expression, toml_span::Error> {
    let s = th.required_s::<std::borrow::Cow<'de, str>>(key)?;
    Expression::parse_mode(&s.value, MODE).map_err(|err| toml_span::Error {
        kind: toml_span::ErrorKind::Custom(err.reason.to_string().into()),
        span: (s.span.start + err.span.start..s.span.start + err.span.end).into(),
        line_info: None,
    })
}

#[inline]
fn parse_path<'de>(
    th: &mut de::TableHelper<'de>,
    key: &'static str,
) -> Result<PathBuf, toml_span::Error> {
    let s = th.required::<String>(key)?;
    Ok(PathBuf::from(s))
}

pub struct Additional {
    pub root: PathBuf,
    pub license: Expression,
    pub license_file: PathBuf,
    pub license_start: Option<usize>,
    pub license_end: Option<usize>,
}

impl<'de> Deserialize<'de> for Additional {
    fn deserialize(value: &mut Value<'de>) -> Result<Self, toml_span::DeserError> {
        let mut tab = de::TableHelper::new(value)?;

        let root = parse_path(&mut tab, "root")?;
        let license = parse_expr(&mut tab, "license-file")?;
        let license_file = parse_path(&mut tab, "license-file")?;
        let license_start = tab.optional("license-start");
        let license_end = tab.optional("license-end");

        tab.finalize(None)?;

        Ok(Self {
            root,
            license,
            license_file,
            license_start,
            license_end,
        })
    }
}

pub struct Ignore {
    pub license: Expression,
    pub license_file: PathBuf,
    pub license_start: Option<usize>,
    pub license_end: Option<usize>,
}

impl<'de> Deserialize<'de> for Ignore {
    fn deserialize(value: &mut Value<'de>) -> Result<Self, toml_span::DeserError> {
        let mut tab = de::TableHelper::new(value)?;

        let license = parse_expr(&mut tab, "license")?;
        let license_file = parse_path(&mut tab, "license-file")?;
        let license_start = tab.optional("license-start");
        let license_end = tab.optional("license-end");

        tab.finalize(None)?;

        Ok(Self {
            license,
            license_file,
            license_start,
            license_end,
        })
    }
}

pub struct ClarificationFile {
    /// The crate relative path to the file
    pub path: PathBuf,
    /// The SHA-256 checksum of the file in hex
    pub checksum: String,
    /// The license applied to the file. Defaults to the license of the parent
    /// clarification if not specified.
    pub license: Option<Expression>,
    /// The beginning of the text to checksum
    pub start: Option<String>,
    /// The end of the text to checksum
    pub end: Option<String>,
}

impl<'de> Deserialize<'de> for ClarificationFile {
    fn deserialize(value: &mut Value<'de>) -> Result<Self, toml_span::DeserError> {
        let mut tab = de::TableHelper::new(value)?;

        let path = parse_path(&mut tab, "path")?;
        let checksum = tab.required("checksum")?;
        let license = if let Some(lic) = tab.optional_s::<std::borrow::Cow<'de, str>>("license") {
            Some(
                Expression::parse(&lic.value).map_err(|err| toml_span::Error {
                    kind: toml_span::ErrorKind::Custom(err.to_string().into()),
                    span: lic.span,
                    line_info: None,
                })?,
            )
        } else {
            None
        };
        let start = tab.optional("start");
        let end = tab.optional("end");

        tab.finalize(None)?;

        Ok(Self {
            path,
            checksum,
            license,
            start,
            end,
        })
    }
}

pub struct Clarification {
    /// The full clarified license expression, as if it appeared as the `license`
    /// in the crate's Cargo.toml manifest
    pub license: Expression,
    /// Normally, if clarifying a file via git, the file in question is retrieved
    /// from the same commit the package was built with, which is retrieved via
    /// the `.cargo_vcs_info.json` file included in the package. However, this
    /// file may not be present, notably if the crate is published with the
    /// `--allow-dirty` flag due to file system modifications that aren't committed
    /// to source control. In this case, the revision must be specified manually
    /// and used instead. This option should absolutely only be used in such a
    /// case, as otherwise it is possible for a drift between the license as it
    /// was at the time of the actual publish of the crate, and the revision
    /// specified here.
    pub override_git_commit: Option<String>,
    /// 1 or more files that are used as the source of truth for the license
    /// expression
    pub files: Vec<ClarificationFile>,
    /// 1 or more files, retrieved from the source git repository for the same
    /// version that was published, used as the source of truth for the license
    /// expression
    pub git: Vec<ClarificationFile>,
}

impl<'de> Deserialize<'de> for Clarification {
    fn deserialize(value: &mut Value<'de>) -> Result<Self, toml_span::DeserError> {
        let mut tab = de::TableHelper::new(value)?;

        let license = parse_expr(&mut tab, "license")?;
        let override_git_commit = tab.optional("override-git-commit");
        let files = tab.optional("files").unwrap_or_default();
        let git = tab.optional("git").unwrap_or_default();

        tab.finalize(None)?;

        Ok(Self {
            license,
            override_git_commit,
            files,
            git,
        })
    }
}

pub struct KrateConfig {
    /// The list of additional accepted licenses for this crate, again in
    /// priority order
    pub accepted: Vec<spdx::Licensee>,
    /// Overrides the license expression for a crate as long as 1 or more file
    /// checksums match
    pub clarify: Option<Clarification>,
}

impl<'de> Deserialize<'de> for KrateConfig {
    fn deserialize(value: &mut Value<'de>) -> Result<Self, toml_span::DeserError> {
        let mut tab = de::TableHelper::new(value)?;

        let accepted = if let Some((_, mut lic)) = tab.take("accepted") {
            let mut l = Vec::new();

            match lic.take() {
                toml_span::value::ValueInner::Array(lic) => {
                    for mut v in lic {
                        match v.take_string(None) {
                            Ok(lstr) => {
                                // We need to allow deprecated identifiers since external dependencies
                                // can use them even though they shouldn't
                                match spdx::Licensee::parse_mode(&lstr, MODE) {
                                    Ok(licensee) => l.push(licensee),
                                    Err(error) => {
                                        tab.errors.push(toml_span::Error {
                                            kind: toml_span::ErrorKind::Custom(
                                                error.reason.to_string().into(),
                                            ),
                                            span: (v.span.start + error.span.start
                                                ..v.span.start + error.span.end)
                                                .into(),
                                            line_info: None,
                                        });
                                    }
                                }
                            }
                            Err(err) => {
                                tab.errors.push(err);
                            }
                        }
                    }
                }
                other => {
                    tab.errors.push(de::expected("an array", other, lic.span));
                }
            }

            l
        } else {
            Vec::new()
        };

        let clarify = tab.optional("clarify");

        tab.finalize(None)?;

        Ok(Self { accepted, clarify })
    }
}

/// Configures how private crates are handled and detected
#[derive(Default)]
pub struct Private {
    /// If enabled, ignores workspace crates that aren't published, or are
    /// only published to private registries
    pub ignore: bool,
    /// One or more private registries that you might publish crates to, if
    /// a crate is only published to private registries, and `ignore` is true,
    /// the crate will not have its license checked
    pub registries: Vec<String>,
}

impl<'de> Deserialize<'de> for Private {
    fn deserialize(value: &mut Value<'de>) -> Result<Self, toml_span::DeserError> {
        let mut tab = de::TableHelper::new(value)?;

        let ignore = tab.optional("ignore").unwrap_or_default();
        let registries = tab.optional("registries").unwrap_or_default();

        tab.finalize(None)?;

        Ok(Self { ignore, registries })
    }
}

#[derive(Default)]
pub struct Config {
    /// Only includes dependencies that match at least one of the specified
    /// targets
    pub targets: Vec<String>,
    /// Configures how private crates are handled and detected
    pub private: Private,
    /// Sets the maximum depth from the root of each crate that will be scanned
    /// for license files.
    pub max_depth: Option<u32>,
    /// Ignores any build dependencies in the graph
    pub ignore_build_dependencies: bool,
    /// Ignores any dev dependencies in the graph
    pub ignore_dev_dependencies: bool,
    /// Ignores any transitive dependencies in the graph, ie, only direct
    /// dependencies of crates in the workspace will be included
    pub ignore_transitive_dependencies: bool,
    /// The list of licenses we will use for all crates, in priority order
    pub accepted: Vec<spdx::Licensee>,
    /// Some crates have extremely complicated licensing which requires tedious
    /// configuration to actually correctly identify. Rather than require every
    /// user of cargo-about to redo that same configuration if they happen to
    /// use those problematic crates, they can apply workarounds instead.
    pub workarounds: Vec<String>,
    /// Crate specific configuration
    pub crates: BTreeMap<String, toml_span::Spanned<KrateConfig>>,
}

impl<'de> Deserialize<'de> for Config {
    fn deserialize(value: &mut Value<'de>) -> Result<Self, toml_span::DeserError> {
        let mut tab = de::TableHelper::new(value)?;

        let targets = tab.optional("targets").unwrap_or_default();
        let private = tab.optional("private").unwrap_or_default();
        if tab.take("no-clearly-defined").is_some() {
            log::warn!("`no-clearly-defined` has been removed");
        }
        if tab.take("clearly-defined-timeout-secs").is_some() {
            log::warn!("`clearly-defined-timeout-secs` has been removed");
        }
        if tab.take("filter-noassertion").is_some() {
            log::warn!("`filter-noassertion` has been removed");
        }
        let max_depth = tab.optional("max-depth");
        let ignore_build_dependencies = tab
            .optional("ignore-build-dependencies")
            .unwrap_or_default();
        let ignore_dev_dependencies = tab.optional("ignore-dev-dependencies").unwrap_or_default();
        let ignore_transitive_dependencies = tab
            .optional("ignore-transitive-dependencies")
            .unwrap_or_default();
        let accepted = if let Some((_, mut lic)) = tab.take("accepted") {
            let mut l = Vec::new();

            match lic.take() {
                toml_span::value::ValueInner::Array(lic) => {
                    for mut v in lic {
                        match v.take_string(None) {
                            Ok(lstr) => {
                                // We need to allow deprecated identifiers since external dependencies
                                // can use them even though they shouldn't
                                match spdx::Licensee::parse_mode(&lstr, MODE) {
                                    Ok(licensee) => l.push(licensee),
                                    Err(error) => {
                                        tab.errors.push(toml_span::Error {
                                            kind: toml_span::ErrorKind::Custom(
                                                error.reason.to_string().into(),
                                            ),
                                            span: (v.span.start + error.span.start
                                                ..v.span.start + error.span.end)
                                                .into(),
                                            line_info: None,
                                        });
                                    }
                                }
                            }
                            Err(err) => {
                                tab.errors.push(err);
                            }
                        }
                    }
                }
                other => {
                    tab.errors.push(de::expected("an array", other, lic.span));
                }
            }

            l
        } else {
            Vec::new()
        };
        let workarounds = tab.optional("workarounds").unwrap_or_default();

        let mut crates = BTreeMap::default();
        for (key, mut value) in tab.table {
            match KrateConfig::deserialize(&mut value) {
                Ok(kc) => {
                    crates.insert(
                        key.name.into_owned(),
                        toml_span::Spanned::with_span(kc, value.span),
                    );
                }
                Err(mut err) => {
                    tab.errors.append(&mut err.errors);
                }
            }
        }

        if !tab.errors.is_empty() {
            return Err(toml_span::DeserError { errors: tab.errors });
        }

        Ok(Self {
            targets,
            private,
            max_depth,
            ignore_build_dependencies,
            ignore_dev_dependencies,
            ignore_transitive_dependencies,
            accepted,
            workarounds,
            crates,
        })
    }
}
