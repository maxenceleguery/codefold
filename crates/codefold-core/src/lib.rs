//! codefold — structural code reader for LLM agents.
//!
//! `Read`, with zoom levels.

pub mod error;
mod go;
mod language;
mod level;
mod markdown;
mod options;
mod parse;
mod python;
mod result;
mod rust;
mod tokens;
mod typescript;

pub use error::Error;
pub use language::Language;
pub use level::Level;
pub use options::Options;
pub use result::{FoldResult, Symbol, SymbolKind};

use std::fs;
use std::path::Path;

pub type Result<T> = std::result::Result<T, Error>;

/// Read `path` at the requested zoom `level`.
pub fn read(path: &Path, level: Level) -> Result<FoldResult> {
    read_opts(path, Options::new(level))
}

/// Read `path` with full options (level + focus).
pub fn read_opts(path: &Path, opts: Options) -> Result<FoldResult> {
    let language = Language::detect(path)?;
    let source = fs::read_to_string(path).map_err(|source| Error::Io {
        path: path.into(),
        source,
    })?;
    render_source(&source, language, opts)
}

/// Render an already-loaded source string. Useful when the source comes from
/// stdin, a network, or any non-file origin — the caller supplies the language
/// explicitly since extension-detection isn't available.
pub fn read_source(source: &str, language: Language, opts: Options) -> Result<FoldResult> {
    render_source(source, language, opts)
}

fn render_source(source: &str, language: Language, opts: Options) -> Result<FoldResult> {
    if opts.level == Level::Full {
        let tokens_est = tokens::estimate(source);
        return Ok(FoldResult {
            content: source.to_string(),
            symbols: Vec::new(),
            hidden_ranges: Vec::new(),
            language: language.name().to_string(),
            tokens_est,
        });
    }

    // Markdown doesn't go through tree-sitter (different grammar ecosystem);
    // dispatch directly to its renderer.
    if let Language::Markdown = language {
        let out = markdown::render(source, opts.level);
        let tokens_est = tokens::estimate(&out.content);
        return Ok(FoldResult {
            content: out.content,
            symbols: out.symbols,
            hidden_ranges: out.hidden_ranges,
            language: language.name().to_string(),
            tokens_est,
        });
    }

    let tree = parse::parse(language, source).map_err(|_| Error::Parse {
        path: Default::default(),
    })?;

    let (content, symbols, hidden_ranges) = match language {
        Language::Python => {
            let out = python::render(source, &tree, opts.level, &opts.focus);
            (out.content, out.symbols, out.hidden_ranges)
        }
        Language::TypeScript | Language::TypeScriptTsx => {
            let out = typescript::render(source, &tree, opts.level, &opts.focus);
            (out.content, out.symbols, out.hidden_ranges)
        }
        Language::Rust => {
            let out = rust::render(source, &tree, opts.level, &opts.focus);
            (out.content, out.symbols, out.hidden_ranges)
        }
        Language::Go => {
            let out = go::render(source, &tree, opts.level, &opts.focus);
            (out.content, out.symbols, out.hidden_ranges)
        }
        Language::Markdown => unreachable!("markdown is handled before parse::parse"),
    };

    let tokens_est = tokens::estimate(&content);
    Ok(FoldResult {
        content,
        symbols,
        hidden_ranges,
        language: language.name().to_string(),
        tokens_est,
    })
}
