/// A parsed code symbol (function, class, method, etc.).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub byte_start: usize,
    pub byte_end: usize,
    pub line_start: usize,
    pub line_end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Function,
    Method,
    Class,
    Import,
}

/// Output of `read()`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoldResult {
    /// The rendered view the caller should hand to the model.
    pub content: String,
    /// Symbols extracted from the file, with original positions.
    pub symbols: Vec<Symbol>,
    /// Byte ranges in the original source that were elided.
    pub hidden_ranges: Vec<(usize, usize)>,
    /// Detected language (`"python"`, `"typescript"`, ...).
    pub language: String,
    /// Estimated token count for `content`. Uses cl100k_base (GPT-4 tokenizer)
    /// as a proxy for Anthropic/OpenAI models — accurate to within ~15%.
    pub tokens_est: usize,
}
