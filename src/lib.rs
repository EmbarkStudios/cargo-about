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

pub fn get_all_crates<P: AsRef<Path>>(cargo_toml: P) -> Result<Krates, Error> {
    use rayon::prelude::*;

    let metadata = cargo_metadata::MetadataCommand::new()
        .manifest_path(cargo_toml)
        .features(cargo_metadata::CargoOpt::AllFeatures)
        .exec()
        .context("failed to fetch metadata")?;

    let krates = metadata.packages;

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
