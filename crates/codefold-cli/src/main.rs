use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, ValueEnum};
use codefold_core::{read_opts, Level, Options};

#[derive(Parser, Debug)]
#[command(
    name = "codefold",
    about = "Structural code reader for LLM agents — `Read`, with zoom levels.",
    version
)]
struct Cli {
    /// Source file to read.
    path: PathBuf,

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

#[derive(Clone, Copy, Debug, ValueEnum)]
enum LevelArg {
    Full,
    Signatures,
    Bodies,
}

impl From<LevelArg> for Level {
    fn from(a: LevelArg) -> Self {
        match a {
            LevelArg::Full => Level::Full,
            LevelArg::Signatures => Level::Signatures,
            LevelArg::Bodies => Level::Bodies,
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let opts = Options {
        level: cli.level.into(),
        focus: cli.focus,
    };

    match read_opts(&cli.path, opts) {
        Ok(r) => {
            print!("{}", r.content);
            if cli.stats {
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
