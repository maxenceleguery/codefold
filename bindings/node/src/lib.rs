//! Node.js bindings for codefold-core.

#![deny(clippy::all)]

use std::path::PathBuf;

use codefold_core::{read_opts, Error as CoreError, Level, Options};
use napi::bindgen_prelude::*;
use napi_derive::napi;

/// A parsed code symbol (function, class, method, import, section).
///
/// Not named `Symbol`: napi-rs would type it as the JS primitive `symbol`.
#[napi(object)]
pub struct CodeSymbol {
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
    pub symbols: Vec<CodeSymbol>,
    /// Byte ranges in the original source that were elided.
    pub hidden_ranges: Vec<HiddenRange>,
    /// Language name (`"python"`, `"typescript"`, `"tsx"`, `"rust"`, `"go"`, `"markdown"`).
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


fn convert_error(e: CoreError) -> Error {
    match e {
        CoreError::Io { path, source } => Error::new(
            Status::GenericFailure,
            format!("{}: {}", path.display(), source),
        ),
        CoreError::UnsupportedLanguage(ext) => Error::new(
            Status::InvalidArg,
            format!("unsupported language {ext:?}"),
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
    let opts = options(level, focus)?;
    read_opts(&PathBuf::from(path), opts)
        .map(FoldResult::from)
        .map_err(convert_error)
}

/// Fold an in-memory source string (an editor buffer, a diff, stdin).
///
/// @param source   The code to fold.
/// @param language Language name or extension: `"python" | "py" | "typescript" | "ts" | "tsx" | "jsx" | "rust" | "rs" | "go" | "markdown" | "md"`.
/// @param level    Same as `read`. Default `"signatures"`.
/// @param focus    Same as `read`.
#[napi]
pub fn read_source(
    source: String,
    language: String,
    level: Option<String>,
    focus: Option<Vec<String>>,
) -> Result<FoldResult> {
    let language = language.parse().map_err(convert_error)?;
    let opts = options(level, focus)?;
    codefold_core::read_source(&source, language, opts)
        .map(FoldResult::from)
        .map_err(convert_error)
}

fn options(level: Option<String>, focus: Option<Vec<String>>) -> Result<Options> {
    Ok(Options {
        level: parse_level(level.as_deref().unwrap_or("signatures"))?,
        focus: focus.unwrap_or_default(),
    })
}

impl From<codefold_core::FoldResult> for FoldResult {
    fn from(r: codefold_core::FoldResult) -> Self {
        FoldResult {
            content: r.content,
            symbols: r
                .symbols
                .into_iter()
                .map(|s| CodeSymbol {
                    name: s.name,
                    kind: s.kind.as_str().to_string(),
                    byte_start: s.byte_start as u32,
                    byte_end: s.byte_end as u32,
                    line_start: s.line_start as u32,
                    line_end: s.line_end as u32,
                })
                .collect(),
            hidden_ranges: r
                .hidden_ranges
                .into_iter()
                .map(|(start, end)| HiddenRange {
                    start: start as u32,
                    end: end as u32,
                })
                .collect(),
            language: r.language,
            tokens_est: r.tokens_est as u32,
        }
    }
}
