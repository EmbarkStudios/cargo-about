#![warn(clippy::all)]
#![warn(rust_2018_idioms)]

use anyhow::{anyhow, bail, Context, Error};
use std::path::{Path, PathBuf};
use structopt::StructOpt;

mod generate;
mod init;

#[derive(StructOpt, Debug)]
enum Command {
    /// Outputs a listing of all licenses and the crates that use them
    #[structopt(name = "generate")]
    Generate(generate::Args),
    #[structopt(name = "init")]
    Init(init::Args),
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
        .level(log::LevelFilter::Warn)
        .level_for("cargo_about", level)
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

fn load_config(manifest_path: &Path) -> Result<cargo_about::licenses::config::Config, Error> {
    let mut parent = manifest_path.parent();

    // Move up directories until we find an about.toml, to handle
    // cases where eg in a workspace there is a top-level about.toml
    // but the user is only getting a listing for a particular crate from it
    while let Some(p) = parent {
        if !p.join("Cargo.toml").exists() {
            break;
        }

        let about_toml = p.join("about.toml");

        if about_toml.exists() {
            let contents = std::fs::read_to_string(&about_toml)?;
            let cfg = toml::from_str(&contents)?;

            log::info!("loaded config from {}", about_toml.display());
            return Ok(cfg);
        }

        parent = p.parent();
    }

    Ok(cargo_about::licenses::config::Config::default())
}

fn real_main() -> Result<(), Error> {
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

    let cfg = load_config(&manifest_path)?;

    let (all_crates, store) = rayon::join(
        || {
            log::info!("gathering crates for {}", manifest_path.display());
            cargo_about::get_all_crates(&manifest_path)
        },
        || {
            log::info!("loading license store");
            cargo_about::licenses::LicenseStore::from_cache()
        },
    );

    let all_crates = all_crates?;
    let store = store?;

    log::info!("gathered {} crates", all_crates.krates.len());

    match args.cmd {
        Command::Generate(gen) => generate::cmd(gen, cfg, all_crates, store),
        Command::Init(init) => init::cmd(init),
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
