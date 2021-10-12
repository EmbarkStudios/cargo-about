use krates::Utf8PathBuf as PathBuf;
use serde::{de, Deserialize};
use std::{collections::BTreeMap, fmt};

#[inline]
fn deserialize_spdx_expr<'de, D>(deserializer: D) -> std::result::Result<spdx::Expression, D::Error>
where
    D: de::Deserializer<'de>,
{
    <&'de str>::deserialize(deserializer)
        .and_then(|value| spdx::Expression::parse(value).map_err(de::Error::custom))
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
    #[serde(deserialize_with = "deserialize_spdx_expr")]
    pub license: spdx::Expression,
    pub license_file: PathBuf,
    pub license_start: Option<usize>,
    pub license_end: Option<usize>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Ignore {
    #[serde(deserialize_with = "deserialize_spdx_expr")]
    pub license: spdx::Expression,
    pub license_file: PathBuf,
    pub license_start: Option<usize>,
    pub license_end: Option<usize>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct KrateConfig {
    #[serde(default)]
    pub additional: Vec<Additional>,
    /// A list of files that are ignored for the purposes of license retrieval,
    /// eg. due to them being present in the source, but not actually used for
    /// the target(s) you are building for, such as test code, or platform
    /// specific code
    #[serde(default)]
    pub ignore: Vec<Ignore>,
    /// The list of additional accepted licenses for this crate, again in
    /// priority order
    #[serde(default, deserialize_with = "deserialize_licensee")]
    pub accepted: Vec<spdx::Licensee>,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Workaround {
    /// The name of the crate
    pub name: String,
    /// The version range the workaround applies to, defaults to all versions
    pub version: Option<krates::semver::VersionReq>,
}

#[derive(Deserialize, Debug, Default)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    /// Only includes dependencies that match at least one of the specified
    /// targets
    #[serde(default)]
    pub targets: Vec<String>,
    /// Disallows the use of clearlydefined.io to retrieve harvested license
    /// information and relies purely on local file scanning
    #[serde(default)]
    pub disallow_clearly_defined: bool,
    /// Ignores any build dependencies in the graph
    #[serde(default)]
    pub ignore_build_dependencies: bool,
    /// Ignores any dev dependencies in the graph
    #[serde(default)]
    pub ignore_dev_dependencies: bool,
    /// The list of licenses we will use for all crates, in priority order
    #[serde(deserialize_with = "deserialize_licensee")]
    pub accepted: Vec<spdx::Licensee>,
    /// Some crates have extremely complicated licensing which requires tedious
    /// configuration to actually correctly identify. Rather than require every
    /// user of cargo-about to redo that same configuration if they happen to
    /// use those problematic crates, they can apply workarounds instead.
    #[serde(default)]
    pub workarounds: Vec<Workaround>,
    /// Crate specific configuration
    #[serde(flatten)]
    pub crates: BTreeMap<String, KrateConfig>,
}
