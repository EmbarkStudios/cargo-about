use super::ClarificationFile;
use anyhow::Context as _;

pub fn get(krate: &crate::Krate) -> anyhow::Result<Option<super::Clarification>> {
    if krate.name != "chrono" {
        return Ok(None);
    }

    // chrono puts both the MIT and Apache-2.0 licenses in the same file so we
    // need to split them out
    Ok(Some(super::Clarification {
        license: spdx::Expression::parse("Apache-2.0 OR MIT")
            .context("failed to parse license expression")?,
        files: vec![
            ClarificationFile {
                path: "LICENSE.txt".into(),
                checksum: "332b974a713ff4e5536be4732fbffd1026694d4a1cbe8d832c969625d991f22c"
                    .to_owned(),
                license: Some(spdx::Expression::parse("MIT").context("failed to parse MIT")?),
                start: Some("The MIT License (MIT)".to_owned()),
                end: Some("THE SOFTWARE.".to_owned()),
            },
            ClarificationFile {
                path: "LICENSE.txt".into(),
                checksum: "769f80b5bcb42ed0af4e4d2fd74e1ac9bf843cb80c5a29219d1ef3544428a6bb"
                    .to_owned(),
                license: Some(
                    spdx::Expression::parse("Apache-2.0").context("failed to parse MIT")?,
                ),
                start: Some("                              Apache License".to_owned()),
                end: Some("limitations under the License.".to_owned()),
            },
        ],
        git: Vec::new(),
    }))
}
