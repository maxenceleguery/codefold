//! Token estimation backed by tiktoken's cl100k_base BPE.
//!
//! cl100k_base is what GPT-4 uses. It's not identical to the tokenizer Claude
//! uses, but it's within 10-15% on typical code/English content and is good
//! enough for context budgeting. Initialization is amortized across calls
//! via a one-time load.

use once_cell::sync::Lazy;
use tiktoken_rs::CoreBPE;

static BPE: Lazy<CoreBPE> =
    Lazy::new(|| tiktoken_rs::cl100k_base().expect("cl100k_base tokenizer should load"));

pub fn estimate(text: &str) -> usize {
    if text.is_empty() {
        return 0;
    }
    BPE.encode_with_special_tokens(text).len()
}
