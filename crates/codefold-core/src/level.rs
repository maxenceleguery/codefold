/// Zoom level for a `read()` call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Level {
    /// Imports plus function/class signatures and docstring summaries. Bodies hidden.
    Signatures,
    /// Top-level bodies in full; nested defs collapsed to signatures.
    Bodies,
    /// File contents verbatim. Provided for API symmetry.
    Full,
}
