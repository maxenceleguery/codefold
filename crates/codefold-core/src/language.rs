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
        Self::from_ext(path.extension().and_then(|e| e.to_str()).unwrap_or(""))
    }

    fn from_ext(ext: &str) -> Result<Self, Error> {
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

/// Parses a language name (`"python"`, `"tsx"`, ...) or a file extension
/// (`"py"`, `"md"`, ...), case-insensitively.
impl std::str::FromStr for Language {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Error> {
        match s.to_ascii_lowercase().as_str() {
            "python" => Ok(Language::Python),
            "typescript" => Ok(Language::TypeScript),
            "rust" => Ok(Language::Rust),
            other => Self::from_ext(other),
        }
    }
}
