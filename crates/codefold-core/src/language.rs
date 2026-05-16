use std::path::Path;

use crate::Error;

/// A supported source language.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Python,
    TypeScript,
    /// TypeScript with JSX (.tsx) or JavaScript with JSX (.jsx). Uses the
    /// `language_tsx` grammar which is a near-superset of plain TypeScript.
    TypeScriptTsx,
    Rust,
    Go,
    Markdown,
}

impl Language {
    pub fn name(self) -> &'static str {
        match self {
            Language::Python => "python",
            Language::TypeScript => "typescript",
            Language::TypeScriptTsx => "tsx",
            Language::Rust => "rust",
            Language::Go => "go",
            Language::Markdown => "markdown",
        }
    }

    /// Extensions recognized by `detect`, kept as a single source of truth so
    /// error messages stay in sync with the matcher.
    pub const SUPPORTED_EXTENSIONS: &'static [&'static str] = &[
        "py", "pyi", "ts", "tsx", "jsx", "rs", "go", "md", "markdown",
    ];

    pub fn detect(path: &Path) -> Result<Self, Error> {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        match ext {
            "py" | "pyi" => Ok(Language::Python),
            "ts" => Ok(Language::TypeScript),
            "tsx" | "jsx" => Ok(Language::TypeScriptTsx),
            "rs" => Ok(Language::Rust),
            "go" => Ok(Language::Go),
            "md" | "markdown" => Ok(Language::Markdown),
            other => Err(Error::UnsupportedLanguage(other.to_string())),
        }
    }
}
