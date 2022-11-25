#![doc = include_str!("../../README.md")]
use anyhow::Context as _;

mod clarify;
mod generate;
mod init;

#[global_allocator]
static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[derive(clap::Subcommand, Debug)]
enum Command {
    /// Outputs a listing of all licenses and the crates that use them
    Generate(generate::Args),
    /// Initializes an about.toml configuration
    Init(init::Args),
    /// Computes a clarification for a file
    Clarify(clarify::Args),
}

#[derive(clap::ValueEnum, Copy, Clone, Debug)]
pub enum Color {
    Auto,
    Always,
    Never,
}

impl std::str::FromStr for Color {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let lower = s.to_ascii_lowercase();

        Ok(match lower.as_str() {
            "auto" => Self::Auto,
            "always" => Self::Always,
            "never" => Self::Never,
            _ => anyhow::bail!("unknown color option '{s}' specified"),
        })
    }
}

fn parse_level(s: &str) -> anyhow::Result<log::LevelFilter> {
    s.parse::<log::LevelFilter>()
        .with_context(|| format!("failed to parse level '{s}'"))
}

#[derive(Debug, clap::Parser)]
#[clap(author, version, about, long_about = None)]
struct Opts {
    /// The log level for messages, only log messages at or above
    /// the level will be emitted.
    #[clap(
        short = 'L',
        default_value = "warn",
        value_parser = parse_level,
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
    #[clap(value_enum, short, long, ignore_case = true, default_value = "auto")]
    color: Color,
    #[clap(subcommand)]
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
                date = time::OffsetDateTime::now_utc(),
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
    use clap::Parser;

    let args = Opts::parse_from({
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
            log::error!("{e:#}");
            #[allow(clippy::exit)]
            std::process::exit(1);
        }
    }
}
