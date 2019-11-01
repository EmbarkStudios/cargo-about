#![warn(clippy::all)]
#![warn(rust_2018_idioms)]

use anyhow::{anyhow, bail, Context, Error};
use std::path::PathBuf;
use structopt::StructOpt;

mod generate;

#[derive(StructOpt, Debug)]
enum Command {
    /// Outputs a listing of all licenses and the crates that use them
    #[structopt(name = "generate")]
    Generate(generate::Args),
}

fn parse_level(s: &str) -> Result<log::LevelFilter, Error> {
    s.parse::<log::LevelFilter>()
        .map_err(|e| anyhow!("failed to parse level '{}': {}", s, e))
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
    /// The path of the Cargo.toml to use
    #[structopt(short, long = "manifest-path", parse(from_os_str))]
    manifest_path: Option<PathBuf>,
    #[structopt(subcommand)]
    cmd: Command,
}

fn setup_logger(level: log::LevelFilter) -> Result<(), fern::InitError> {
    use ansi_term::Color::*;
    use log::Level::*;

    fern::Dispatch::new()
        .level(level)
        .format(move |out, message, record| {
            out.finish(format_args!(
                "{date} [{level}] {message}\x1B[0m",
                date = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"),
                level = match record.level() {
                    Error => Red.paint("ERROR"),
                    Warn => Yellow.paint("WARN"),
                    Info => Green.paint("INFO"),
                    Debug => Blue.paint("DEBUG"),
                    Trace => Purple.paint("TRACE"),
                },
                message = message,
            ));
        })
        .chain(std::io::stderr())
        .apply()?;
    Ok(())
}

fn real_main() -> Result<(), Error> {
    let args = Opts::from_iter({
        std::env::args().enumerate().filter_map(|(i, a)| {
            if i == 1 && a == "about-me" {
                None
            } else {
                Some(a)
            }
        })
    });

    setup_logger(args.log_level)?;

    let manifest_path = args
        .manifest_path
        .clone()
        .or_else(|| {
            std::env::current_dir()
                .and_then(|cd| Ok(cd.join("Cargo.toml")))
                .ok()
        })
        .context("unable to determine manifest path")?;

    if !manifest_path.exists() {
        bail!(
            "cargo manifest path '{}' does not exist",
            manifest_path.display()
        );
    }

    let (all_crates, store) = rayon::join(
        || {
            log::info!("gathering crates for {}", manifest_path.display());
            about_me::get_all_crates(&manifest_path)
        },
        || {
            log::info!("loading license store");
            about_me::licenses::LicenseStore::from_cache()
        },
    );

    let all_crates = all_crates?;
    let store = store?;

    log::info!("gathered {} crates", all_crates.krates.len());

    match args.cmd {
        Command::Generate(gen) => generate::cmd(gen, all_crates, store),
    }
}

fn main() {
    match real_main() {
        Ok(_) => {}
        Err(e) => {
            log::error!("{}", e);
            std::process::exit(1);
        }
    }
}
