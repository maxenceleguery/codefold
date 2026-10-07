//! Python bindings for codefold-core.

use std::path::PathBuf;

use codefold_core::{read_opts, Error, Level, Options};
use pyo3::exceptions::{PyFileNotFoundError, PyRuntimeError, PyValueError};
use pyo3::prelude::*;

/// Python-side mirror of `codefold_core::Symbol`.
#[pyclass(
    name = "Symbol",
    get_all,
    frozen,
    skip_from_py_object,
    module = "codefold"
)]
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

/// Read `path` at the requested zoom `level`.
///
/// Args:
///     path: Source file to read.
///     level: One of "full", "signatures", "public", "bodies". Default "signatures".
///     focus: Optional list or tuple of symbol names to render at full body
///            regardless of base level.
///
/// Returns:
///     A `FoldResult` with `content`, `symbols`, `hidden_ranges`, `language`,
///     `tokens_est`.
#[pyfunction]
#[pyo3(signature = (path, level="signatures", focus=None))]
fn read(path: PathBuf, level: &str, focus: Option<Vec<String>>) -> PyResult<PyFoldResult> {
    let opts = options(level, focus)?;
    read_opts(&path, opts)
        .map(PyFoldResult::from)
        .map_err(convert_error)
}

/// Fold an in-memory `source` string (stdin, a diff, an editor buffer, ...).
///
/// Args:
///     source: The code to fold.
///     language: Language name or extension: "python"/"py", "typescript"/"ts",
///               "tsx"/"jsx", "rust"/"rs", "go", "markdown"/"md".
///     level: Same as `read`. Default "signatures".
///     focus: Same as `read`.
#[pyfunction]
#[pyo3(signature = (source, language, level="signatures", focus=None))]
fn read_source(
    source: &str,
    language: &str,
    level: &str,
    focus: Option<Vec<String>>,
) -> PyResult<PyFoldResult> {
    let language = language.parse().map_err(convert_error)?;
    let opts = options(level, focus)?;
    codefold_core::read_source(source, language, opts)
        .map(PyFoldResult::from)
        .map_err(convert_error)
}

fn options(level: &str, focus: Option<Vec<String>>) -> PyResult<Options> {
    let mut opts = Options::new(parse_level(level)?);
    opts.focus = focus.unwrap_or_default();
    Ok(opts)
}

impl From<codefold_core::FoldResult> for PyFoldResult {
    fn from(r: codefold_core::FoldResult) -> Self {
        PyFoldResult {
            content: r.content,
            symbols: r
                .symbols
                .into_iter()
                .map(|s| PySymbol {
                    name: s.name,
                    kind: s.kind.as_str().to_string(),
                    byte_start: s.byte_start,
                    byte_end: s.byte_end,
                    line_start: s.line_start,
                    line_end: s.line_end,
                })
                .collect(),
            hidden_ranges: r.hidden_ranges,
            language: r.language,
            tokens_est: r.tokens_est,
        }
    }
}

fn convert_error(e: Error) -> PyErr {
    match e {
        Error::Io { path, source } => {
            PyFileNotFoundError::new_err(format!("{}: {}", path.display(), source))
        }
        Error::UnsupportedLanguage(ext) => {
            PyValueError::new_err(format!("unsupported language {ext:?}"))
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
    m.add_function(wrap_pyfunction!(read_source, m)?)?;
    m.add_class::<PyFoldResult>()?;
    m.add_class::<PySymbol>()?;
    Ok(())
}
