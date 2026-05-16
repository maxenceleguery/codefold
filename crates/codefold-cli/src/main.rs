use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand, ValueEnum};
use codefold_core::{read_opts, Level, Options};

mod doctor;
mod setup;
mod update;

#[derive(Parser, Debug)]
#[command(
    name = "codefold",
    about = "Structural code reader for LLM agents — `Read`, with zoom levels.",
    version,
    args_conflicts_with_subcommands = true
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    // ----- backwards-compatible top-level path-as-default-arg -----
    /// Source file to read (when no subcommand given).
    path: Option<PathBuf>,

    /// Zoom level.
    #[arg(short, long, value_enum, default_value_t = LevelArg::Signatures)]
    level: LevelArg,

    /// Symbol names to render at full body, even at lower levels. Repeatable
    /// or comma-separated.
    #[arg(short, long, value_delimiter = ',')]
    focus: Vec<String>,

    /// Print a summary line (language, tokens, symbols, hidden ranges) to stderr.
    #[arg(long)]
    stats: bool,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Read a source file at a chosen zoom level (explicit form of the default).
    Read(ReadArgs),
    /// Update codefold-cli to the latest release.
    Update(update::UpdateArgs),
    /// Install codefold integration into LLM agent harnesses on this project or your user account.
    Setup(setup::SetupArgs),
    /// Diagnose the install: cargo availability, network, integration files.
    Doctor(doctor::DoctorArgs),
}

#[derive(Args, Debug)]
struct ReadArgs {
    /// Source file to read.
    path: PathBuf,

    #[arg(short, long, value_enum, default_value_t = LevelArg::Signatures)]
    level: LevelArg,

    #[arg(short, long, value_delimiter = ',')]
    focus: Vec<String>,

    #[arg(long)]
    stats: bool,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum LevelArg {
    Full,
    Signatures,
    Public,
    Bodies,
}

impl From<LevelArg> for Level {
    fn from(a: LevelArg) -> Self {
        match a {
            LevelArg::Full => Level::Full,
            LevelArg::Signatures => Level::Signatures,
            LevelArg::Public => Level::Public,
            LevelArg::Bodies => Level::Bodies,
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Some(Command::Read(args)) => run_read(&args.path, args.level, args.focus, args.stats),
        Some(Command::Update(args)) => update::run(&args, env!("CARGO_PKG_VERSION")),
        Some(Command::Setup(args)) => setup::run(&args, env!("CARGO_PKG_VERSION")),
        Some(Command::Doctor(args)) => doctor::run(&args, env!("CARGO_PKG_VERSION")),
        None => match cli.path {
            Some(path) => run_read(&path, cli.level, cli.focus, cli.stats),
            None => {
                use clap::CommandFactory;
                Cli::command().print_help().ok();
                eprintln!();
                ExitCode::from(2)
            }
        },
    }
}

fn run_read(path: &Path, level: LevelArg, focus: Vec<String>, stats: bool) -> ExitCode {
    let opts = Options {
        level: level.into(),
        focus,
    };
    match read_opts(path, opts) {
        Ok(r) => {
            print!("{}", r.content);
            if stats {
                let hidden_bytes: usize = r.hidden_ranges.iter().map(|(a, b)| b - a).sum();
                eprintln!(
                    "[codefold] language={} tokens={} symbols={} hidden_ranges={} hidden_bytes={}",
                    r.language,
                    r.tokens_est,
                    r.symbols.len(),
                    r.hidden_ranges.len(),
                    hidden_bytes,
                );
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("codefold: {e}");
            ExitCode::from(1)
        }
    }
}
