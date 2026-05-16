//! Node.js bindings for codefold-core.

#![deny(clippy::all)]

use std::path::PathBuf;

use codefold_core::{read_opts, Error as CoreError, Level, Options};
use napi::bindgen_prelude::*;
use napi_derive::napi;

/// A parsed code symbol (function, class, method, import).
#[napi(object)]
pub struct Symbol {
    pub name: String,
    pub kind: String,
    pub byte_start: u32,
    pub byte_end: u32,
    pub line_start: u32,
    pub line_end: u32,
}

/// A byte range in the original source that was elided from the rendered view.
#[napi(object)]
pub struct HiddenRange {
    pub start: u32,
    pub end: u32,
}

/// Output of `read()`.
#[napi(object)]
pub struct FoldResult {
    /// Rendered view the agent should consume.
    pub content: String,
    /// Symbols parsed from the file, with original positions.
    pub symbols: Vec<Symbol>,
    /// Byte ranges in the original source that were elided.
    pub hidden_ranges: Vec<HiddenRange>,
    /// Detected language (`"python"`, `"typescript"`, `"rust"`, `"go"`).
    pub language: String,
    /// Estimated token count for `content` (cl100k_base proxy).
    pub tokens_est: u32,
}

fn parse_level(s: &str) -> Result<Level> {
    match s.to_ascii_lowercase().as_str() {
        "full" => Ok(Level::Full),
        "signatures" | "sig" => Ok(Level::Signatures),
        "public" | "pub" => Ok(Level::Public),
        "bodies" | "body" => Ok(Level::Bodies),
        other => Err(Error::new(
            Status::InvalidArg,
            format!("unknown level {other:?}; expected full/signatures/public/bodies"),
        )),
    }
}

fn symbol_kind_name(k: codefold_core::SymbolKind) -> &'static str {
    match k {
        codefold_core::SymbolKind::Function => "function",
        codefold_core::SymbolKind::Method => "method",
        codefold_core::SymbolKind::Class => "class",
        codefold_core::SymbolKind::Import => "import",
    }
}

fn convert_error(e: CoreError) -> Error {
    match e {
        CoreError::Io { path, source } => Error::new(
            Status::GenericFailure,
            format!("{}: {}", path.display(), source),
        ),
        CoreError::UnsupportedLanguage(ext) => Error::new(
            Status::InvalidArg,
            format!("unsupported language for extension {ext:?}"),
        ),
        CoreError::Parse { path } => Error::new(
            Status::GenericFailure,
            format!("parse failed for {}", path.display()),
        ),
    }
}

/// Read `path` at a chosen zoom level.
///
/// @param path  Source file to read.
/// @param level One of `"full" | "signatures" | "public" | "bodies"`. Default `"signatures"`.
/// @param focus Optional list of symbol names to keep at full body regardless of base level.
#[napi]
pub fn read(path: String, level: Option<String>, focus: Option<Vec<String>>) -> Result<FoldResult> {
    let level = parse_level(level.as_deref().unwrap_or("signatures"))?;
    let opts = Options {
        level,
        focus: focus.unwrap_or_default(),
    };

    let result = read_opts(&PathBuf::from(path), opts).map_err(convert_error)?;

    let symbols = result
        .symbols
        .into_iter()
        .map(|s| Symbol {
            name: s.name,
            kind: symbol_kind_name(s.kind).to_string(),
            byte_start: s.byte_start as u32,
            byte_end: s.byte_end as u32,
            line_start: s.line_start as u32,
            line_end: s.line_end as u32,
        })
        .collect();

    let hidden_ranges = result
        .hidden_ranges
        .into_iter()
        .map(|(start, end)| HiddenRange {
            start: start as u32,
            end: end as u32,
        })
        .collect();

    Ok(FoldResult {
        content: result.content,
        symbols,
        hidden_ranges,
        language: result.language,
        tokens_est: result.tokens_est as u32,
    })
}
