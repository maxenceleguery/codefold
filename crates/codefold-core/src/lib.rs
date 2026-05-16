//! codefold — structural code reader for LLM agents.
//!
//! `Read`, with zoom levels.

pub mod error;
mod language;
mod level;
mod options;
mod parse;
mod python;
mod result;
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

    if opts.level == Level::Full {
        let tokens_est = tokens::estimate(&source);
        return Ok(FoldResult {
            content: source,
            symbols: Vec::new(),
            hidden_ranges: Vec::new(),
            language: language.name().to_string(),
            tokens_est,
        });
    }

    let tree = parse::parse(language, &source).map_err(|_| Error::Parse { path: path.into() })?;

    let (content, symbols, hidden_ranges) = match language {
        Language::Python => {
            let out = python::render(&source, &tree, opts.level, &opts.focus);
            (out.content, out.symbols, out.hidden_ranges)
        }
        Language::TypeScript => {
            let out = typescript::render(&source, &tree, opts.level, &opts.focus);
            (out.content, out.symbols, out.hidden_ranges)
        }
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
