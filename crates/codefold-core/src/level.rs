/// Zoom level for a `read()` call.
///
/// Marked `#[non_exhaustive]` so adding new variants is not a breaking change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Level {
    /// Imports plus function/class signatures and docstring summaries. Bodies hidden.
    Signatures,
    /// Like `Signatures` but additionally filters out non-public symbols:
    /// Python names starting with `_` (except dunder), TypeScript declarations
    /// not wrapped in `export_statement` and methods marked `private`.
    Public,
    /// Top-level bodies in full; nested defs collapsed to signatures.
    Bodies,
    /// File contents verbatim. Provided for API symmetry.
    Full,
}
