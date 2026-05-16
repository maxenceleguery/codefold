//! Python bindings for codefold-core.

use std::path::PathBuf;

use codefold_core::{read_opts, Error, Level, Options};
use pyo3::exceptions::{PyFileNotFoundError, PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyList;

/// Python-side mirror of `codefold_core::Symbol`.
#[pyclass(name = "Symbol", get_all, frozen, module = "codefold")]
#[derive(Clone)]
struct PySymbol {
    name: String,
    kind: String,
    byte_start: usize,
    byte_end: usize,
    line_start: usize,
    line_end: usize,
}

#[pymethods]
impl PySymbol {
    fn __repr__(&self) -> String {
        format!(
            "Symbol(name={:?}, kind={:?}, line_start={}, line_end={})",
            self.name, self.kind, self.line_start, self.line_end
        )
    }
}

/// Python-side mirror of `codefold_core::FoldResult`.
#[pyclass(name = "FoldResult", get_all, frozen, module = "codefold")]
struct PyFoldResult {
    content: String,
    symbols: Vec<PySymbol>,
    hidden_ranges: Vec<(usize, usize)>,
    language: String,
    tokens_est: usize,
}

#[pymethods]
impl PyFoldResult {
    fn __repr__(&self) -> String {
        format!(
            "FoldResult(language={:?}, tokens_est={}, symbols={}, hidden_ranges={})",
            self.language,
            self.tokens_est,
            self.symbols.len(),
            self.hidden_ranges.len(),
        )
    }
}

fn parse_level(s: &str) -> PyResult<Level> {
    match s.to_ascii_lowercase().as_str() {
        "full" => Ok(Level::Full),
        "signatures" | "sig" => Ok(Level::Signatures),
        "public" | "pub" => Ok(Level::Public),
        "bodies" | "body" => Ok(Level::Bodies),
        other => Err(PyValueError::new_err(format!(
            "unknown level {other:?}; expected one of full/signatures/public/bodies"
        ))),
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

/// Read `path` at the requested zoom `level`.
///
/// Args:
///     path: Source file to read.
///     level: One of "full", "signatures", "public", "bodies". Default "signatures".
///     focus: Optional iterable of symbol names to render at full body
///            regardless of base level.
///
/// Returns:
///     A `FoldResult` with `content`, `symbols`, `hidden_ranges`, `language`,
///     `tokens_est`.
#[pyfunction]
#[pyo3(signature = (path, level="signatures", focus=None))]
fn read(path: PathBuf, level: &str, focus: Option<&Bound<'_, PyList>>) -> PyResult<PyFoldResult> {
    let level = parse_level(level)?;
    let mut opts = Options::new(level);
    if let Some(focus_list) = focus {
        let mut names = Vec::with_capacity(focus_list.len());
        for item in focus_list.iter() {
            let s: String = item.extract()?;
            names.push(s);
        }
        opts.focus = names;
    }

    let result = read_opts(&path, opts).map_err(convert_error)?;

    let symbols: Vec<PySymbol> = result
        .symbols
        .into_iter()
        .map(|s| PySymbol {
            name: s.name,
            kind: symbol_kind_name(s.kind).to_string(),
            byte_start: s.byte_start,
            byte_end: s.byte_end,
            line_start: s.line_start,
            line_end: s.line_end,
        })
        .collect();

    Ok(PyFoldResult {
        content: result.content,
        symbols,
        hidden_ranges: result.hidden_ranges,
        language: result.language,
        tokens_est: result.tokens_est,
    })
}

fn convert_error(e: Error) -> PyErr {
    match e {
        Error::Io { path, source } => {
            PyFileNotFoundError::new_err(format!("{}: {}", path.display(), source))
        }
        Error::UnsupportedLanguage(ext) => {
            PyValueError::new_err(format!("unsupported language for extension {ext:?}"))
        }
        Error::Parse { path } => {
            PyRuntimeError::new_err(format!("parse failed for {}", path.display()))
        }
    }
}

/// codefold — `Read`, with zoom levels.
#[pymodule]
fn codefold(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add_function(wrap_pyfunction!(read, m)?)?;
    m.add_class::<PyFoldResult>()?;
    m.add_class::<PySymbol>()?;
    Ok(())
}
