#![warn(clippy::all)]
#![warn(rust_2018_idioms)]

use anyhow::{Context, Error};
use std::{collections::HashMap, path::Path};

pub type Pid = cargo_metadata::PackageId;
pub type Krate = cargo_metadata::Package;

pub mod licenses;

pub struct Krates {
    pub krates: Vec<Krate>,
    pub krate_map: HashMap<Pid, usize>,
    pub resolved: cargo_metadata::Resolve,
    //pub lock_file: PathBuf,
}

pub fn get_all_crates<P: AsRef<Path>>(
    cargo_toml: P,
    no_default_features: bool,
    all_features: bool,
    features: Option<&str>,
) -> Result<Krates, Error> {
    use rayon::prelude::*;

    let metadata = {
        let mut mdc = cargo_metadata::MetadataCommand::new();
        mdc.manifest_path(cargo_toml);

        // The metadata command builder is weird and only allows you to specify
        // one of these, but really you might need to do multiple of them
        if no_default_features {
            mdc.other_options(["--no-default-features".to_owned()]);
        }

        if all_features {
            mdc.other_options(["--all-features".to_owned()]);
        }

        if let Some(fts) = features {
            mdc.other_options(["--features".to_owned(), fts.to_owned()]);
        }

        mdc.exec().context("failed to fetch metadata")?
    };

    let mut krates = metadata.packages;

    for krate in &mut krates {
        if let Some(ref mut lf) = krate.license {
            *lf = lf.replace("/", " OR ");
        }
    }

    krates.par_sort_by(|a, b| a.id.cmp(&b.id));

    let map = krates
        .iter()
        .enumerate()
        .map(|(i, ci)| (ci.id.clone(), i))
        .collect();

    let mut resolved = metadata.resolve.unwrap();

    resolved.nodes.par_sort_by(|a, b| a.id.cmp(&b.id));
    resolved
        .nodes
        .par_iter_mut()
        .for_each(|nodes| nodes.dependencies.par_sort());

    Ok(Krates {
        krates,
        krate_map: map,
        resolved,
        //lock_file: root.as_ref().join("Cargo.lock"),
    })
}
