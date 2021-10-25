use crate::licenses::{
    config::{Clarification, ClarificationFile, Config},
    fetch::GitCache,
    KrateLicense,
};

mod bitvec;
mod chrono;
mod clap;
mod cocoa;
mod gtk;
mod ring;
mod rustls;
mod tonic;
mod tract;
mod wasmtime;

pub(crate) fn apply_workarounds<'krate>(
    krates: &'krate crate::Krates,
    cfg: &Config,
    gc: &GitCache,
    licensed_krates: &mut Vec<KrateLicense<'krate>>,
) {
    if cfg.workarounds.is_empty() {
        return;
    }

    for workaround_cfg in &cfg.workarounds {
        let (retrieve_workaround, is_exact) =
            match WORKAROUNDS.iter().find_map(|(name, is_exact, func)| {
                (workaround_cfg.name == *name).then(|| (func, is_exact))
            }) {
                Some(func) => func,
                None => {
                    log::warn!(
                        "no workaround registered for the '{}' crate",
                        workaround_cfg.name
                    );
                    continue;
                }
            };

        if *is_exact {
            let version_req = workaround_cfg
                .version
                .clone()
                .unwrap_or(krates::semver::VersionReq::STAR);

            for (_, krate) in krates.search_matches(&workaround_cfg.name, version_req) {
                if let Err(i) = super::binary_search(licensed_krates, &krate.krate) {
                    match retrieve_workaround(&krate.krate) {
                        Ok(Some(clarification)) => {
                            match crate::licenses::apply_clarification(
                                gc,
                                &krate.krate,
                                &clarification,
                            ) {
                                Ok(files) => {
                                    log::debug!("applying workaround to '{}'", krate.krate);

                                    licensed_krates.insert(
                                        i,
                                        KrateLicense {
                                            krate: &krate.krate,
                                            lic_info: super::LicenseInfo::Expr(
                                                clarification.license,
                                            ),
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
                        Ok(None) => {}
                        Err(e) => {
                            log::debug!("unable to apply workaround to '{}': {:#}", krate.krate, e);
                        }
                    }
                }
            }
        } else {
            for krate in krates.krates().map(|kn| &kn.krate) {
                if let Err(i) = super::binary_search(licensed_krates, krate) {
                    match retrieve_workaround(krate) {
                        Ok(Some(clarification)) => {
                            match crate::licenses::apply_clarification(gc, krate, &clarification) {
                                Ok(files) => {
                                    log::debug!(
                                        "applying workaround '{}' to '{}'",
                                        workaround_cfg.name,
                                        krate
                                    );

                                    licensed_krates.insert(
                                        i,
                                        KrateLicense {
                                            krate,
                                            lic_info: super::LicenseInfo::Expr(
                                                clarification.license,
                                            ),
                                            license_files: files,
                                        },
                                    );
                                }
                                Err(e) => {
                                    log::debug!(
                                        "unable to apply workaround '{}' to '{}': {:#}",
                                        workaround_cfg.name,
                                        krate,
                                        e
                                    );
                                }
                            }
                        }
                        Ok(None) => {}
                        Err(e) => {
                            log::debug!(
                                "unable to apply workaround '{}' to '{}': {:#}",
                                workaround_cfg.name,
                                krate,
                                e
                            );
                        }
                    }
                }
            }
        }
    }
}

#[allow(clippy::type_complexity)]
const WORKAROUNDS: &[(
    &str,
    bool,
    &dyn Fn(&crate::Krate) -> anyhow::Result<Option<Clarification>>,
)] = &[
    ("bitvec", false, &self::bitvec::get),
    ("chrono", true, &self::chrono::get),
    ("clap", false, &self::clap::get),
    ("cocoa", false, &self::cocoa::get),
    ("gtk", false, &self::gtk::get),
    ("ring", true, &self::ring::get),
    ("rustls", true, &self::rustls::get),
    ("tonic", false, &self::tonic::get),
    ("tract", false, &self::tract::get),
    ("wasmtime", false, &self::wasmtime::get),
];
