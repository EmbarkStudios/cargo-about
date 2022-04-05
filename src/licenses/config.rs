use krates::Utf8PathBuf as PathBuf;
use serde::{de, ser, Deserialize, Serialize};
use spdx::Expression;
use std::{collections::BTreeMap, fmt};

mod spdx_expr {
    use super::*;

    #[inline]
    pub(crate) fn serialize<S>(expr: &Expression, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        serializer.serialize_str(expr.as_ref())
    }

    #[inline]
    pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<Expression, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        <&'de str>::deserialize(deserializer)
            .and_then(|value| Expression::parse(value).map_err(de::Error::custom))
    }
}
mod spdx_expr_opt {
    use super::*;

    #[inline]
    pub(crate) fn serialize<S>(expr: &Option<Expression>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        match expr {
            Some(expr) => serializer.serialize_str(expr.as_ref()),
            None => serializer.serialize_none(),
        }
    }

    #[inline]
    pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<Option<Expression>, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        match <Option<&'de str>>::deserialize(deserializer)? {
            Some(value) => Ok(Some(
                spdx::Expression::parse(value).map_err(de::Error::custom)?,
            )),
            None => Ok(None),
        }
    }
}

#[inline]
fn deserialize_licensee<'de, D>(
    deserializer: D,
) -> std::result::Result<Vec<spdx::Licensee>, D::Error>
where
    D: de::Deserializer<'de>,
{
    struct Visitor;

    impl<'de> de::Visitor<'de> for Visitor {
        type Value = Vec<spdx::Licensee>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("array of SPDX licensees")
        }

        fn visit_seq<S>(self, mut seq: S) -> Result<Self::Value, S::Error>
        where
            S: de::SeqAccess<'de>,
        {
            let mut vec = Vec::new();

            // Update the max while there are additional values.
            while let Some(v) = seq.next_element()? {
                let lic = spdx::Licensee::parse(v).map_err(|e| {
                    de::Error::custom(format!("'{}' is not a valid SPDX licensee: {}", v, e))
                })?;

                vec.push(lic);
            }

            Ok(vec)
        }
    }

    deserializer.deserialize_seq(Visitor)
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Additional {
    pub root: PathBuf,
    #[serde(with = "spdx_expr")]
    pub license: Expression,
    pub license_file: PathBuf,
    pub license_start: Option<usize>,
    pub license_end: Option<usize>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Ignore {
    #[serde(with = "spdx_expr")]
    pub license: Expression,
    pub license_file: PathBuf,
    pub license_start: Option<usize>,
    pub license_end: Option<usize>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct ClarificationFile {
    /// The crate relative path to the file
    pub path: PathBuf,
    /// The SHA-256 checksum of the file in hex
    pub checksum: String,
    /// The license applied to the file. Defaults to the license of the parent
    /// clarification if not specified.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "spdx_expr_opt"
    )]
    pub license: Option<Expression>,
    /// The beginning of the text to checksum
    pub start: Option<String>,
    /// The end of the text to checksum
    pub end: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Clarification {
    /// The full clarified license expression, as if it appeared as the `license`
    /// in the crate's Cargo.toml manifest
    #[serde(with = "spdx_expr")]
    pub license: Expression,
    /// Normally, if clarifying a file via git, the file in question is retrieved
    /// from the same commit the package was built with, which is retrieved via
    /// the `.cargo_vcs_info.json` file included in the package. However, this
    /// file may not be present, notably if the crate is published with the
    /// `--allow-dirty` flag due to file system modifications that aren't commited
    /// to source control. In this case, the revision must be specified manually
    /// and used instead. This option should absolutely only be used in such a
    /// case, as otherwise it is possible for a drift between the license as it
    /// was at the time of the actual publish of the crate, and the revision
    /// specified here.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub override_git_commit: Option<String>,
    /// 1 or more files that are used as the source of truth for the license
    /// expression
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<ClarificationFile>,
    /// 1 or more files, retrieved from the source git repository for the same
    /// version that was published, used as the source of truth for the license
    /// expression
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub git: Vec<ClarificationFile>,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct KrateConfig {
    /// The list of additional accepted licenses for this crate, again in
    /// priority order
    #[serde(default, deserialize_with = "deserialize_licensee")]
    pub accepted: Vec<spdx::Licensee>,
    /// Overrides the license expression for a crate as long as 1 or more file
    /// checksums match
    pub clarify: Option<Clarification>,
}

/// Configures how private crates are handled and detected
#[derive(Deserialize, Default, Debug)]
#[serde(deny_unknown_fields)]
pub struct Private {
    /// If enabled, ignores workspace crates that aren't published, or are
    /// only published to private registries
    #[serde(default)]
    pub ignore: bool,
    /// One or more private registries that you might publish crates to, if
    /// a crate is only published to private registries, and `ignore` is true,
    /// the crate will not have its license checked
    #[serde(default)]
    pub registries: Vec<String>,
}

#[derive(Deserialize, Debug, Default)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    /// Only includes dependencies that match at least one of the specified
    /// targets
    #[serde(default)]
    pub targets: Vec<String>,
    /// Configures how private crates are handled and detected
    #[serde(default)]
    pub private: Private,
    /// Disallows the use of clearlydefined.io to retrieve harvested license
    /// information and relies purely on local file scanning and clarifications
    #[serde(default)]
    pub no_clearly_defined: bool,
    /// Ignores any build dependencies in the graph
    #[serde(default)]
    pub ignore_build_dependencies: bool,
    /// Ignores any dev dependencies in the graph
    #[serde(default)]
    pub ignore_dev_dependencies: bool,
    /// Ignores any transitive dependencies in the graph, ie, only direct
    /// dependencies of crates in the workspace will be included
    #[serde(default)]
    pub ignore_transitive_dependencies: bool,
    /// The list of licenses we will use for all crates, in priority order
    #[serde(deserialize_with = "deserialize_licensee")]
    pub accepted: Vec<spdx::Licensee>,
    /// Some crates have extremely complicated licensing which requires tedious
    /// configuration to actually correctly identify. Rather than require every
    /// user of cargo-about to redo that same configuration if they happen to
    /// use those problematic crates, they can apply workarounds instead.
    #[serde(default)]
    pub workarounds: Vec<String>,
    /// Crate specific configuration
    #[serde(flatten)]
    pub crates: BTreeMap<String, KrateConfig>,
}
