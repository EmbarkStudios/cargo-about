#![warn(clippy::all)]
#![warn(rust_2018_idioms)]

use anyhow::{Context, Error};
use krates::cm;
use std::{fmt, path::Path};

pub mod licenses;

pub struct Krate(cm::Package);

impl From<cm::Package> for Krate {
    fn from(mut pkg: cm::Package) -> Self {
        // Fix the license field as cargo used to allow the
        // invalid / separator
        if let Some(ref mut lf) = pkg.license {
            *lf = lf.replace("/", " OR ");
        }

        Self(pkg)
    }
}

impl fmt::Display for Krate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.0.name, self.0.version)
    }
}

impl std::ops::Deref for Krate {
    type Target = cm::Package;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub type Krates = krates::Krates<Krate>;

pub fn get_all_crates(
    cargo_toml: std::path::PathBuf,
    no_default_features: bool,
    all_features: bool,
    features: Vec<String>,
    cfg: &licenses::config::Config,
) -> Result<Krates, Error> {
    //let mut mdc = cm::MetadataCommand::new();

    // We take a manifest-path, but we use current directory, otherwise
    // any .cargo/config the user's project might have won't be taken into
    // account
    // mdc.current_dir(cargo_toml.as_ref().parent().unwrap());
    // mdc.manifest_path("Cargo.toml");

    let mut mdc = krates::Cmd::new();
    mdc.manifest_path(cargo_toml.clone());

    // The metadata command builder is weird and only allows you to specify
    // one of these, but really you might need to do multiple of them
    if no_default_features {
        mdc.no_default_features();
    }

    if all_features {
        mdc.all_features();
    }

    mdc.features(features);

    let mut builder = krates::Builder::new();

    if cfg.ignore_build_dependencies {
        builder.ignore_kind(krates::DepKind::Build, krates::Scope::All);
    }

    if cfg.ignore_dev_dependencies {
        builder.ignore_kind(krates::DepKind::Dev, krates::Scope::All);
    }

    builder.include_targets(cfg.targets.iter().map(|triple| (triple.clone(), vec![])));

    let graph = builder.build(
        mdc,
        Some(|filtered: cm::Package| match filtered.source {
            Some(src) => {
                if src.is_crates_io() {
                    log::debug!("filtered {} {}", filtered.name, filtered.version);
                } else {
                    log::debug!("filtered {} {} {}", filtered.name, filtered.version, src);
                }
            }
            None => log::debug!("filtered crate {} {}", filtered.name, filtered.version),
        }),
    )?;

    use krates::petgraph::dot::{Config, Dot};

    println!("{}", Dot::with_config(&graph.graph(), &[]));

    Ok(graph)
}
