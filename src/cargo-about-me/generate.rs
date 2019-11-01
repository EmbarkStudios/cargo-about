use about_me::{licenses, Krate, Krates};
use anyhow::Error;
use handlebars::Handlebars;
use serde::Serialize;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub struct Args {
    /// The confidence threshold required for license files
    /// to be positively identified: 0.0 - 1.0
    #[structopt(short, long, default_value = "0.8")]
    threshold: f32,
    #[structopt(short, long, parse(from_os_str))]
    templates: PathBuf,
}

pub fn cmd(args: Args, krates: Krates, store: licenses::LicenseStore) -> Result<(), Error> {
    let registry = {
        let mut reg = Handlebars::new();
        reg.register_templates_directory("hbs", args.templates)?;
        reg
    };

    let summary = licenses::Gatherer::with_store(std::sync::Arc::new(store))
        .with_confidence_threshold(args.threshold)
        .gather(&krates.krates);

    licenses::sanity_check(&summary)?;

    unimplemented!()
}

#[derive(Serialize)]
struct License<'a> {
    /// The full name of the license
    name: String,
    /// The SPDX short identifier for the license
    id: String,
    /// The full license text
    text: String,
    /// The list of crates this license was applied to
    crates: Vec<&'a Krate>,
}

#[derive(Serialize)]
struct Input<'a> {
    licenses: Vec<License<'a>>,
}
