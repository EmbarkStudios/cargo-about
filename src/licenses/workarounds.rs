use crate::licenses::{
    KrateLicense,
    config::{Clarification, ClarificationFile, Config},
    fetch::GitCache,
};

mod bitvec;
mod chrono;
mod clap;
mod cocoa;
mod gtk;
mod prost;
mod ring;
mod rustix;
mod rustls;
mod sentry;
mod tonic;
mod tract;
mod unicode_ident;
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

    for workaround in &cfg.workarounds {
        let Some(retrieve_workaround) = WORKAROUNDS
            .iter()
            .find_map(|(name, func)| (workaround == *name).then_some(func))
        else {
            log::warn!("no workaround registered for the '{workaround}' crate");
            continue;
        };

        for krate in krates.krates() {
            if let Err(i) = super::binary_search(licensed_krates, krate) {
                match retrieve_workaround(krate) {
                    Ok(Some(clarification)) => {
                        match crate::licenses::apply_clarification(gc, krate, &clarification) {
                            Ok(files) => {
                                log::debug!("applying workaround '{workaround}' to '{krate}'");

                                licensed_krates.insert(
                                    i,
                                    KrateLicense {
                                        krate,
                                        lic_info: super::LicenseInfo::Expr(clarification.license),
                                        license_files: files,
                                    },
                                );
                            }
                            Err(e) => {
                                log::debug!(
                                    "unable to apply workaround '{workaround}' to '{krate}': {e:#}"
                                );
                            }
                        }
                    }
                    Ok(None) => {}
                    Err(e) => {
                        log::debug!(
                            "unable to apply workaround '{workaround}' to '{krate}': {e:#}"
                        );
                    }
                }
            }
        }
    }
}

#[allow(clippy::type_complexity)]
const WORKAROUNDS: &[(
    &str,
    &dyn Fn(&crate::Krate) -> anyhow::Result<Option<Clarification>>,
)] = &[
    ("bitvec", &self::bitvec::get),
    ("chrono", &self::chrono::get),
    ("clap", &self::clap::get),
    ("cocoa", &self::cocoa::get),
    ("gtk", &self::gtk::get),
    ("prost", &self::prost::get),
    ("ring", &self::ring::get),
    ("rustls", &self::rustls::get),
    ("sentry", &self::sentry::get),
    ("tonic", &self::tonic::get),
    ("tract", &self::tract::get),
    ("unicode-ident", &self::unicode_ident::get),
    ("wasmtime", &self::wasmtime::get),
    ("rustix", &self::rustix::get),
];
