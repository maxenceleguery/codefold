use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand, ValueEnum};
use codefold_core::{read_opts, FoldResult, Level, Options, SymbolKind};

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

    // ----- backwards-compatible top-level paths-as-default-args -----
    /// Source files to read (when no subcommand given). One or many.
    paths: Vec<PathBuf>,

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

    /// Output format. `text` (default): rendered content to stdout, optional
    /// stats to stderr. `json`: a structured object (or array, with multiple
    /// files) emitted to stdout.
    #[arg(short = 'F', long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Read source files at a chosen zoom level (explicit form of the default).
    Read(ReadArgs),
    /// Update codefold-cli to the latest release.
    Update(update::UpdateArgs),
    /// Install codefold integration into LLM agent harnesses on this project or your user account.
    Setup(setup::SetupArgs),
}

#[derive(Args, Debug)]
struct ReadArgs {
    /// Source files to read. One or many.
    #[arg(required = true)]
    paths: Vec<PathBuf>,

    #[arg(short, long, value_enum, default_value_t = LevelArg::Signatures)]
    level: LevelArg,

    #[arg(short, long, value_delimiter = ',')]
    focus: Vec<String>,

    #[arg(long)]
    stats: bool,

    #[arg(short = 'F', long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
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

#[derive(Clone, Copy, Debug, ValueEnum, PartialEq, Eq)]
enum OutputFormat {
    Text,
    Json,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Some(Command::Read(args)) => {
            run_read(&args.paths, args.level, args.focus, args.stats, args.format)
        }
        Some(Command::Update(args)) => update::run(&args, env!("CARGO_PKG_VERSION")),
        Some(Command::Setup(args)) => setup::run(&args, env!("CARGO_PKG_VERSION")),
        None => {
            if !cli.paths.is_empty() {
                run_read(&cli.paths, cli.level, cli.focus, cli.stats, cli.format)
            } else {
                use clap::CommandFactory;
                Cli::command().print_help().ok();
                eprintln!();
                ExitCode::from(2)
            }
        }
    }
}

fn run_read(
    paths: &[PathBuf],
    level: LevelArg,
    focus: Vec<String>,
    stats: bool,
    format: OutputFormat,
) -> ExitCode {
    let mut results: Vec<(PathBuf, Result<FoldResult, codefold_core::Error>)> =
        Vec::with_capacity(paths.len());
    for p in paths {
        let opts = Options {
            level: level.into(),
            focus: focus.clone(),
        };
        results.push((p.clone(), read_opts(p, opts)));
    }

    match format {
        OutputFormat::Text => emit_text(&results, paths.len() > 1, stats),
        OutputFormat::Json => emit_json(&results, paths.len() == 1),
    }
}

fn emit_text(
    results: &[(PathBuf, Result<FoldResult, codefold_core::Error>)],
    multi: bool,
    stats: bool,
) -> ExitCode {
    let mut any_err = false;
    for (path, res) in results {
        match res {
            Ok(r) => {
                if multi {
                    println!("=== {} ===", path.display());
                }
                print!("{}", r.content);
                if multi && !r.content.ends_with('\n') {
                    println!();
                }
                if stats {
                    let hidden_bytes: usize = r.hidden_ranges.iter().map(|(a, b)| b - a).sum();
                    eprintln!(
                        "[codefold] {}: language={} tokens={} symbols={} hidden_ranges={} hidden_bytes={}",
                        path.display(),
                        r.language,
                        r.tokens_est,
                        r.symbols.len(),
                        r.hidden_ranges.len(),
                        hidden_bytes,
                    );
                }
            }
            Err(e) => {
                any_err = true;
                eprintln!("codefold: {}: {e}", path.display());
            }
        }
    }
    if any_err {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn emit_json(
    results: &[(PathBuf, Result<FoldResult, codefold_core::Error>)],
    single: bool,
) -> ExitCode {
    use serde_json::json;
    let mut any_err = false;
    let arr: Vec<serde_json::Value> = results
        .iter()
        .map(|(p, res)| match res {
            Ok(r) => json!({
                "path": p.display().to_string(),
                "language": r.language,
                "tokens_est": r.tokens_est,
                "content": r.content,
                "symbols": r.symbols.iter().map(|s| json!({
                    "name": s.name,
                    "kind": symbol_kind_str(s.kind),
                    "byte_start": s.byte_start,
                    "byte_end": s.byte_end,
                    "line_start": s.line_start,
                    "line_end": s.line_end,
                })).collect::<Vec<_>>(),
                "hidden_ranges": r.hidden_ranges.iter()
                    .map(|(s, e)| json!({"start": s, "end": e}))
                    .collect::<Vec<_>>(),
            }),
            Err(e) => {
                any_err = true;
                json!({
                    "path": p.display().to_string(),
                    "error": e.to_string(),
                })
            }
        })
        .collect();

    let value = if single {
        arr.into_iter().next().unwrap_or(serde_json::Value::Null)
    } else {
        serde_json::Value::Array(arr)
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&value).unwrap_or_default()
    );

    if any_err {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn symbol_kind_str(k: SymbolKind) -> &'static str {
    match k {
        SymbolKind::Function => "function",
        SymbolKind::Method => "method",
        SymbolKind::Class => "class",
        SymbolKind::Import => "import",
    }
}
