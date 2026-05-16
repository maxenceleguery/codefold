//! codefold — structural code reader for LLM agents.
//!
//! `Read`, with zoom levels.

#![doc(html_no_source)]

pub mod error;

pub use error::Error;

/// Result type used across the crate.
pub type Result<T> = std::result::Result<T, Error>;
