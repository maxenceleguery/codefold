use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("io error reading {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("unsupported language for extension {0:?}")]
    UnsupportedLanguage(String),

    #[error("parse failed for {path}")]
    Parse { path: PathBuf },
}
