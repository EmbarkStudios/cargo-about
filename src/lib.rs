// BEGIN - Embark standard lints v0.3
// do not change or add/remove here, but one can add exceptions after this section
// for more info see: <https://github.com/EmbarkStudios/rust-ecosystem/issues/59>
#![deny(unsafe_code)]
#![warn(
    clippy::all,
    clippy::await_holding_lock,
    clippy::dbg_macro,
    clippy::debug_assert_with_mut_call,
    clippy::doc_markdown,
    clippy::empty_enum,
    clippy::enum_glob_use,
    clippy::exit,
    clippy::explicit_into_iter_loop,
    clippy::filter_map_next,
    clippy::fn_params_excessive_bools,
    clippy::if_let_mutex,
    clippy::imprecise_flops,
    clippy::inefficient_to_string,
    clippy::large_types_passed_by_value,
    clippy::let_unit_value,
    clippy::linkedlist,
    clippy::lossy_float_literal,
    clippy::macro_use_imports,
    clippy::map_err_ignore,
    clippy::map_flatten,
    clippy::map_unwrap_or,
    clippy::match_on_vec_items,
    clippy::match_same_arms,
    clippy::match_wildcard_for_single_variants,
    clippy::mem_forget,
    clippy::mismatched_target_os,
    clippy::needless_borrow,
    clippy::needless_continue,
    clippy::option_option,
    clippy::pub_enum_variant_names,
    clippy::ref_option_ref,
    clippy::rest_pat_in_fully_bound_structs,
    clippy::string_add_assign,
    clippy::string_add,
    clippy::string_to_string,
    clippy::suboptimal_flops,
    clippy::todo,
    clippy::unimplemented,
    clippy::unnested_or_patterns,
    clippy::unused_self,
    clippy::verbose_file_reads,
    future_incompatible,
    nonstandard_style,
    rust_2018_idioms
)]
// END - Embark standard lints v0.3

use anyhow::Error;
use krates::cm;
use std::fmt;

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
    workspace: bool,
    cfg: &licenses::config::Config,
) -> Result<Krates, Error> {
    let mut mdc = krates::Cmd::new();
    mdc.manifest_path(cargo_toml);

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

    if workspace {
        builder.workspace(true);
    }

    if cfg.ignore_build_dependencies {
        builder.ignore_kind(krates::DepKind::Build, krates::Scope::All);
    }

    if cfg.ignore_dev_dependencies {
        builder.ignore_kind(krates::DepKind::Dev, krates::Scope::All);
    }

    builder.include_targets(cfg.targets.iter().map(|triple| (triple.as_str(), vec![])));

    let graph = builder.build(mdc, |filtered: cm::Package| match filtered.source {
        Some(src) => {
            if src.is_crates_io() {
                log::debug!("filtered {} {}", filtered.name, filtered.version);
            } else {
                log::debug!("filtered {} {} {}", filtered.name, filtered.version, src);
            }
        }
        None => log::debug!("filtered crate {} {}", filtered.name, filtered.version),
    })?;

    Ok(graph)
}
