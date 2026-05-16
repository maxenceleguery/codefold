use std::path::Path;

use crate::Error;

/// A supported source language.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Python,
    TypeScript,
    Rust,
    Go,
}

impl Language {
    pub fn name(self) -> &'static str {
        match self {
            Language::Python => "python",
            Language::TypeScript => "typescript",
            Language::Rust => "rust",
            Language::Go => "go",
        }
    }

    pub fn detect(path: &Path) -> Result<Self, Error> {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        match ext {
            "py" | "pyi" => Ok(Language::Python),
            "ts" => Ok(Language::TypeScript),
            "rs" => Ok(Language::Rust),
            "go" => Ok(Language::Go),
            other => Err(Error::UnsupportedLanguage(other.to_string())),
        }
    }
}
