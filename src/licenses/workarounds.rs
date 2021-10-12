use crate::licenses::{
    config::{Clarification, ClarificationFile, Config},
    KrateLicense,
};
pub mod ring;

pub fn apply_workarounds<'krate>(
    krates: &'krate crate::Krates,
    cfg: &Config,
    licensed_krates: &mut Vec<KrateLicense<'krate>>,
) {
    if cfg.workarounds.is_empty() {
        return;
    }

    for workaround_cfg in &cfg.workarounds {
        let retrieve_workaround = match WORKAROUNDS
            .iter()
            .find_map(|(name, func)| (workaround_cfg.name == *name).then(|| func))
        {
            Some(func) => func,
            None => {
                log::warn!(
                    "no workaround registered for the '{}' crate",
                    workaround_cfg.name
                );
                continue;
            }
        };

        let version_req = workaround_cfg
            .version
            .clone()
            .unwrap_or(krates::semver::VersionReq::STAR);

        for (_, krate) in krates.search_matches(&workaround_cfg.name, version_req) {
            if let Err(i) = super::binary_search(licensed_krates, &krate.krate) {
                match retrieve_workaround(&krate.krate) {
                    Ok(clarification) => {
                        match crate::licenses::apply_clarification(&krate.krate, &clarification) {
                            Ok(files) => {
                                log::info!("applying workaround to '{}'", krate.krate);

                                licensed_krates.insert(
                                    i,
                                    KrateLicense {
                                        krate: &krate.krate,
                                        lic_info: super::LicenseInfo::Expr(clarification.license),
                                        license_files: files,
                                    },
                                );
                            }
                            Err(e) => {
                                log::debug!(
                                    "unable to apply workaround to '{}': {:#}",
                                    krate.krate,
                                    e
                                );
                            }
                        }
                    }
                    Err(e) => {
                        log::debug!("unable to apply workaround to '{}': {:#}", krate.krate, e);
                    }
                }
            }
        }
    }
}

const WORKAROUNDS: &[(
    &str,
    &dyn Fn(&crate::Krate) -> anyhow::Result<Clarification>,
)] = &[("ring", &self::ring::get)];
