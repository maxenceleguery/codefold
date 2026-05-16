//! codefold — structural code reader for LLM agents.
//!
//! `Read`, with zoom levels.

pub mod error;
mod language;
mod level;
mod result;

pub use error::Error;
pub use language::Language;
pub use level::Level;
pub use result::{FoldResult, Symbol, SymbolKind};

use std::fs;
use std::path::Path;

pub type Result<T> = std::result::Result<T, Error>;

/// Read `path` at the requested zoom `level`.
pub fn read(path: &Path, level: Level) -> Result<FoldResult> {
    let language = Language::detect(path)?;
    let source = fs::read_to_string(path).map_err(|source| Error::Io {
        path: path.into(),
        source,
    })?;

    match level {
        Level::Full => Ok(FoldResult {
            content: source,
            symbols: Vec::new(),
            hidden_ranges: Vec::new(),
            language: language.name().to_string(),
        }),
        Level::Signatures | Level::Bodies => {
            // Implemented in upcoming TDD steps.
            unimplemented!("level {level:?} not yet implemented")
        }
    }
}
