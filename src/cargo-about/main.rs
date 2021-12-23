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
    clippy::disallowed_method,
    clippy::disallowed_type,
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
#![allow(clippy::exit, clippy::single_match_else)]

use anyhow::Context as _;
use structopt::StructOpt;

mod clarify;
mod generate;
mod init;

#[global_allocator]
static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[derive(StructOpt, Debug)]
enum Command {
    /// Outputs a listing of all licenses and the crates that use them
    Generate(generate::Args),
    /// Initializes an about.toml configuration
    Init(init::Args),
    /// Computes a clarification for a file
    Clarify(clarify::Args),
}

#[derive(StructOpt, Copy, Clone, Debug)]
pub enum Color {
    Auto,
    Always,
    Never,
}

impl Color {
    fn variants() -> &'static [&'static str] {
        &["auto", "always", "never"]
    }
}

impl std::str::FromStr for Color {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let lower = s.to_ascii_lowercase();

        Ok(match lower.as_str() {
            "auto" => Self::Auto,
            "always" => Self::Always,
            "never" => Self::Never,
            _ => anyhow::bail!("unknown color option '{}' specified", s),
        })
    }
}

fn parse_level(s: &str) -> anyhow::Result<log::LevelFilter> {
    s.parse::<log::LevelFilter>()
        .with_context(|| format!("failed to parse level '{}'", s))
}

#[derive(Debug, StructOpt)]
struct Opts {
    /// The log level for messages, only log messages at or above
    /// the level will be emitted.
    #[structopt(
        short = "L",
        long = "log-level",
        default_value = "warn",
        parse(try_from_str = parse_level),
        long_help = "The log level for messages, only log messages at or above the level will be emitted.

Possible values:
* off
* error
* warn
* info
* debug
* trace"
    )]
    log_level: log::LevelFilter,
    #[structopt(short, long, default_value = "auto", possible_values = Color::variants())]
    color: Color,
    #[structopt(subcommand)]
    cmd: Command,
}

fn setup_logger(level: log::LevelFilter) -> Result<(), fern::InitError> {
    use ansi_term::Color;
    use log::Level as Lvl;

    fern::Dispatch::new()
        .level(log::LevelFilter::Warn)
        .level_for("cargo_about", level)
        .format(move |out, message, record| {
            out.finish(format_args!(
                "{date} [{level}] {message}\x1B[0m",
                date = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"),
                level = match record.level() {
                    Lvl::Error => Color::Red.paint("ERROR"),
                    Lvl::Warn => Color::Yellow.paint("WARN"),
                    Lvl::Info => Color::Green.paint("INFO"),
                    Lvl::Debug => Color::Blue.paint("DEBUG"),
                    Lvl::Trace => Color::Purple.paint("TRACE"),
                },
                message = message,
            ));
        })
        .chain(std::io::stderr())
        .apply()?;
    Ok(())
}

fn real_main() -> anyhow::Result<()> {
    let args = Opts::from_iter({
        std::env::args().enumerate().filter_map(|(i, a)| {
            if i == 1 && a == "about" {
                None
            } else {
                Some(a)
            }
        })
    });

    setup_logger(args.log_level)?;

    match args.cmd {
        Command::Generate(gen) => generate::cmd(gen, args.color),
        Command::Init(init) => init::cmd(init),
        Command::Clarify(clarify) => clarify::cmd(clarify),
    }
}

fn main() {
    match real_main() {
        Ok(_) => {}
        Err(e) => {
            log::error!("{:#}", e);
            std::process::exit(1);
        }
    }
}
