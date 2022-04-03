#![doc = include_str!("../README.md")]
// BEGIN - Embark standard lints v5 for Rust 1.55+
// do not change or add/remove here, but one can add exceptions after this section
// for more info see: <https://github.com/EmbarkStudios/rust-ecosystem/issues/59>
#![deny(unsafe_code)]
#![warn(
    clippy::all,
    clippy::await_holding_lock,
    clippy::char_lit_as_u8,
    clippy::checked_conversions,
    clippy::dbg_macro,
    clippy::debug_assert_with_mut_call,
    clippy::doc_markdown,
    clippy::empty_enum,
    clippy::enum_glob_use,
    clippy::exit,
    clippy::expl_impl_clone_on_copy,
    clippy::explicit_deref_methods,
    clippy::explicit_into_iter_loop,
    clippy::fallible_impl_from,
    clippy::filter_map_next,
    clippy::flat_map_option,
    clippy::float_cmp_const,
    clippy::fn_params_excessive_bools,
    clippy::from_iter_instead_of_collect,
    clippy::if_let_mutex,
    clippy::implicit_clone,
    clippy::imprecise_flops,
    clippy::inefficient_to_string,
    clippy::invalid_upcast_comparisons,
    clippy::large_digit_groups,
    clippy::large_stack_arrays,
    clippy::large_types_passed_by_value,
    clippy::let_unit_value,
    clippy::linkedlist,
    clippy::lossy_float_literal,
    clippy::macro_use_imports,
    clippy::manual_ok_or,
    clippy::map_err_ignore,
    clippy::map_flatten,
    clippy::map_unwrap_or,
    clippy::match_on_vec_items,
    clippy::match_same_arms,
    clippy::match_wild_err_arm,
    clippy::match_wildcard_for_single_variants,
    clippy::mem_forget,
    clippy::mismatched_target_os,
    clippy::missing_enforced_import_renames,
    clippy::mut_mut,
    clippy::mutex_integer,
    clippy::needless_borrow,
    clippy::needless_continue,
    clippy::needless_for_each,
    clippy::option_option,
    clippy::path_buf_push_overwrite,
    clippy::ptr_as_ptr,
    clippy::rc_mutex,
    clippy::ref_option_ref,
    clippy::rest_pat_in_fully_bound_structs,
    clippy::same_functions_in_if_condition,
    clippy::semicolon_if_nothing_returned,
    clippy::single_match_else,
    clippy::string_add_assign,
    clippy::string_add,
    clippy::string_lit_as_bytes,
    clippy::string_to_string,
    clippy::todo,
    clippy::trait_duplication_in_bounds,
    clippy::unimplemented,
    clippy::unnested_or_patterns,
    clippy::unused_self,
    clippy::useless_transmute,
    clippy::verbose_file_reads,
    clippy::zero_sized_map_values,
    future_incompatible,
    nonstandard_style,
    rust_2018_idioms
)]
// END - Embark standard lints v0.5 for Rust 1.55+
#![allow(clippy::single_match_else)]

use krates::cm;
use std::{cmp, fmt};

pub mod licenses;

pub struct Krate(pub cm::Package);

impl Krate {
    fn get_license_expression(&self) -> licenses::LicenseInfo {
        match &self.0.license {
            Some(license_field) => {
                //. Reasons this can fail:
                // * Empty! The rust crate used to validate this field has a bug
                // https://github.com/rust-lang-nursery/license-exprs/issues/23
                // * It also just does basic lexing, so parens, duplicate operators,
                // unpaired exceptions etc can all fail validation

                match spdx::Expression::parse(license_field) {
                    Ok(validated) => licenses::LicenseInfo::Expr(validated),
                    Err(err) => {
                        log::error!("unable to parse license expression for '{}': {}", self, err);
                        licenses::LicenseInfo::Unknown
                    }
                }
            }
            None => {
                log::warn!("crate '{}' doesn't have a license field", self);
                licenses::LicenseInfo::Unknown
            }
        }
    }
}

impl Ord for Krate {
    #[inline]
    fn cmp(&self, o: &Self) -> cmp::Ordering {
        match self.0.name.cmp(&o.0.name) {
            cmp::Ordering::Equal => self.0.version.cmp(&o.0.version),
            o => o,
        }
    }
}

impl PartialOrd for Krate {
    #[inline]
    fn partial_cmp(&self, o: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(o))
    }
}

impl PartialEq for Krate {
    #[inline]
    fn eq(&self, o: &Self) -> bool {
        self.cmp(o) == cmp::Ordering::Equal
    }
}

impl Eq for Krate {}

impl From<cm::Package> for Krate {
    fn from(mut pkg: cm::Package) -> Self {
        // Fix the license field as cargo used to allow the
        // invalid / separator
        if let Some(ref mut lf) = pkg.license {
            *lf = lf.replace('/', " OR ");
        }

        Self(pkg)
    }
}

impl krates::KrateDetails for Krate {
    fn name(&self) -> &str {
        &self.0.name
    }

    fn version(&self) -> &krates::semver::Version {
        &self.0.version
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
    cargo_toml: &krates::Utf8Path,
    no_default_features: bool,
    all_features: bool,
    features: Vec<String>,
    workspace: bool,
    cfg: &licenses::config::Config,
) -> anyhow::Result<Krates> {
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

    if cfg.ignore_non_workspace_dependencies {
        builder.ignore_kind(krates::DepKind::Normal, krates::Scope::NonWorkspace);
        builder.ignore_kind(krates::DepKind::Dev, krates::Scope::NonWorkspace);
        builder.ignore_kind(krates::DepKind::Build, krates::Scope::NonWorkspace);
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

#[inline]
pub fn to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    const CHARS: &[u8] = b"0123456789abcdef";

    for &byte in bytes {
        s.push(CHARS[(byte >> 4) as usize] as char);
        s.push(CHARS[(byte & 0xf) as usize] as char);
    }

    s
}

pub fn validate_sha256(buffer: &str, expected: &str) -> anyhow::Result<()> {
    anyhow::ensure!(
        expected.len() == 64,
        "checksum '{}' length is {} instead of expected 64",
        expected,
        expected.len()
    );

    let mut ctx = ring::digest::Context::new(&ring::digest::SHA256);

    ctx.update(buffer.as_bytes());

    // Ignore faulty CRLF style newlines
    // for line in buffer.split('\r') {
    //     ctx.update(line.as_bytes());
    // }

    let content_digest = ctx.finish();
    let digest = content_digest.as_ref();

    for (ind, exp) in expected.as_bytes().chunks(2).enumerate() {
        let mut cur = match exp[0] {
            b'A'..=b'F' => exp[0] - b'A' + 10,
            b'a'..=b'f' => exp[0] - b'a' + 10,
            b'0'..=b'9' => exp[0] - b'0',
            c => {
                anyhow::bail!("invalid byte in checksum '{}' @ {}: {}", expected, ind, c);
            }
        };

        cur <<= 4;

        cur |= match exp[1] {
            b'A'..=b'F' => exp[1] - b'A' + 10,
            b'a'..=b'f' => exp[1] - b'a' + 10,
            b'0'..=b'9' => exp[1] - b'0',
            c => {
                anyhow::bail!("invalid byte in checksum '{}' @ {}: {}", expected, ind, c);
            }
        };

        if digest[ind] != cur {
            anyhow::bail!("checksum mismatch, expected {}", expected);
        }
    }

    Ok(())
}
