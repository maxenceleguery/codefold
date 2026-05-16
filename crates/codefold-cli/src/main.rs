use std::io::Read;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand, ValueEnum};
use codefold_core::{read_opts, read_source, FoldResult, Language, Level, Options, SymbolKind};

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

    /// Override language detection. Required when reading from stdin (path `-`);
    /// optional otherwise (overrides the extension).
    #[arg(long, value_enum)]
    lang: Option<LangArg>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Read source files at a chosen zoom level (explicit form of the default).
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

    #[arg(long, value_enum)]
    lang: Option<LangArg>,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum LangArg {
    Python,
    Typescript,
    Tsx,
    Rust,
    Go,
    Markdown,
}

impl From<LangArg> for Language {
    fn from(a: LangArg) -> Self {
        match a {
            LangArg::Python => Language::Python,
            LangArg::Typescript => Language::TypeScript,
            LangArg::Tsx => Language::TypeScriptTsx,
            LangArg::Rust => Language::Rust,
            LangArg::Go => Language::Go,
            LangArg::Markdown => Language::Markdown,
        }
    }
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
        Some(Command::Read(args)) => run_read(
            &args.paths,
            args.level,
            args.focus,
            args.stats,
            args.format,
            args.lang,
        ),
        Some(Command::Update(args)) => update::run(&args, env!("CARGO_PKG_VERSION")),
        Some(Command::Setup(args)) => setup::run(&args, env!("CARGO_PKG_VERSION")),
        Some(Command::Doctor(args)) => doctor::run(&args, env!("CARGO_PKG_VERSION")),
        None => {
            if !cli.paths.is_empty() {
                run_read(
                    &cli.paths, cli.level, cli.focus, cli.stats, cli.format, cli.lang,
                )
            } else {
                use clap::CommandFactory;
                Cli::command().print_help().ok();
                eprintln!();
                ExitCode::from(2)
            }
        }
    }
}

/// (path, source, result) bundle: we keep the original source so the JSON
/// emitter can compute line numbers for `hidden_ranges`.
struct ReadOutcome {
    path: PathBuf,
    source: Option<String>,
    result: Result<FoldResult, codefold_core::Error>,
}

fn run_read(
    paths: &[PathBuf],
    level: LevelArg,
    focus: Vec<String>,
    stats: bool,
    format: OutputFormat,
    lang: Option<LangArg>,
) -> ExitCode {
    let mut stdin_consumed = false;
    let mut outcomes: Vec<ReadOutcome> = Vec::with_capacity(paths.len());

    for p in paths {
        let is_stdin = p.as_os_str() == "-";
        let opts = Options {
            level: level.into(),
            focus: focus.clone(),
        };

        if is_stdin {
            if stdin_consumed {
                outcomes.push(ReadOutcome {
                    path: p.clone(),
                    source: None,
                    result: Err(codefold_core::Error::Parse { path: p.clone() }),
                });
                eprintln!("codefold: only one '-' (stdin) allowed per invocation");
                continue;
            }
            stdin_consumed = true;
            let language = match lang.map(Language::from) {
                Some(l) => l,
                None => {
                    eprintln!("codefold: reading from stdin requires --lang");
                    return ExitCode::from(2);
                }
            };
            let mut buf = String::new();
            if let Err(e) = std::io::stdin().read_to_string(&mut buf) {
                eprintln!("codefold: failed to read stdin: {e}");
                return ExitCode::from(1);
            }
            let result = read_source(&buf, language, opts);
            outcomes.push(ReadOutcome {
                path: p.clone(),
                source: Some(buf),
                result,
            });
        } else if let Some(l) = lang {
            // --lang override on a real file: read the file ourselves, pass to
            // read_source with the forced language.
            match std::fs::read_to_string(p) {
                Ok(buf) => {
                    let result = read_source(&buf, l.into(), opts);
                    outcomes.push(ReadOutcome {
                        path: p.clone(),
                        source: Some(buf),
                        result,
                    });
                }
                Err(e) => outcomes.push(ReadOutcome {
                    path: p.clone(),
                    source: None,
                    result: Err(codefold_core::Error::Io {
                        path: p.clone(),
                        source: e,
                    }),
                }),
            }
        } else {
            // Stash the source if we'll need it for JSON line numbers; else
            // skip the second read.
            let source = if format == OutputFormat::Json {
                std::fs::read_to_string(p).ok()
            } else {
                None
            };
            let result = read_opts(p, opts);
            outcomes.push(ReadOutcome {
                path: p.clone(),
                source,
                result,
            });
        }
    }

    match format {
        OutputFormat::Text => emit_text(&outcomes, paths.len() > 1, stats),
        OutputFormat::Json => emit_json(&outcomes, paths.len() == 1),
    }
}

fn emit_text(outcomes: &[ReadOutcome], multi: bool, stats: bool) -> ExitCode {
    let mut any_err = false;
    for o in outcomes {
        match &o.result {
            Ok(r) => {
                if multi {
                    println!("=== {} ===", o.path.display());
                }
                print!("{}", r.content);
                if multi && !r.content.ends_with('\n') {
                    println!();
                }
                if stats {
                    let hidden_bytes: usize = r.hidden_ranges.iter().map(|(a, b)| b - a).sum();
                    eprintln!(
                        "[codefold] {}: language={} tokens={} symbols={} hidden_ranges={} hidden_bytes={}",
                        o.path.display(),
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
                eprintln!("codefold: {}: {e}", o.path.display());
            }
        }
    }
    if any_err {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn emit_json(outcomes: &[ReadOutcome], single: bool) -> ExitCode {
    use serde_json::json;
    let mut any_err = false;
    let arr: Vec<serde_json::Value> = outcomes
        .iter()
        .map(|o| match &o.result {
            Ok(r) => {
                let line_starts = o.source.as_deref().map(build_line_starts);
                let hidden_ranges: Vec<serde_json::Value> = r
                    .hidden_ranges
                    .iter()
                    .map(|(s, e)| {
                        let mut obj = serde_json::Map::new();
                        obj.insert("start".into(), json!(s));
                        obj.insert("end".into(), json!(e));
                        if let Some(starts) = line_starts.as_ref() {
                            obj.insert("line_start".into(), json!(byte_to_line(starts, *s)));
                            // end-byte is exclusive; clamp to the previous byte to get
                            // the line of the last character.
                            obj.insert(
                                "line_end".into(),
                                json!(byte_to_line(starts, e.saturating_sub(1))),
                            );
                        }
                        serde_json::Value::Object(obj)
                    })
                    .collect();
                json!({
                    "path": o.path.display().to_string(),
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
                    "hidden_ranges": hidden_ranges,
                })
            }
            Err(e) => {
                any_err = true;
                json!({
                    "path": o.path.display().to_string(),
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

fn build_line_starts(source: &str) -> Vec<usize> {
    let mut starts = vec![0usize];
    for (i, b) in source.bytes().enumerate() {
        if b == b'\n' && i + 1 < source.len() {
            starts.push(i + 1);
        }
    }
    starts
}

fn byte_to_line(line_starts: &[usize], byte: usize) -> usize {
    // 1-based line numbers. Binary search the largest start ≤ byte.
    match line_starts.binary_search(&byte) {
        Ok(i) => i + 1,
        Err(i) => i,
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
