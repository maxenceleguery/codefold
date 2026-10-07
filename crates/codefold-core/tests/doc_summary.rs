//! At `signatures`/`public`, doc comments keep only their summary paragraph,
//! like Python docstrings. `bodies` and focused symbols keep them in full.

use codefold_core::{read_source, FoldResult, Language, Level, Options};

fn fold(src: &str, lang: Language, level: Level, focus: &[&str]) -> FoldResult {
    let mut opts = Options::new(level);
    opts.focus = focus.iter().map(|s| s.to_string()).collect();
    read_source(src, lang, opts).unwrap()
}

const RUST: &str = "\
/// Adds one.
///
/// # Examples
///
/// ```
/// assert_eq!(add(1), 2);
/// ```
pub fn add(x: i32) -> i32 {
    x + 1
}

impl Foo {
    /// Builds a foo.
    ///
    /// Long explanation.
    #[must_use]
    pub fn new() -> Self {
        Foo
    }
}
";

#[test]
fn rust_signatures_keep_doc_summary_only() {
    let r = fold(RUST, Language::Rust, Level::Signatures, &[]);
    assert!(
        r.content
            .contains("/// Adds one.\npub fn add(x: i32) -> i32"),
        "{}",
        r.content
    );
    assert!(!r.content.contains("# Examples"), "{}", r.content);
    assert!(
        r.content
            .contains("    /// Builds a foo.\n    #[must_use]\n    pub fn new() -> Self"),
        "{}",
        r.content
    );
    assert!(!r.content.contains("Long explanation"), "{}", r.content);
}

#[test]
fn rust_public_level_summarizes_too() {
    let r = fold(RUST, Language::Rust, Level::Public, &[]);
    assert!(r.content.contains("/// Adds one."));
    assert!(!r.content.contains("# Examples"));
}

#[test]
fn elided_doc_is_reported_hidden() {
    let r = fold(RUST, Language::Rust, Level::Signatures, &[]);
    for needle in ["# Examples", "Long explanation"] {
        let at = RUST.find(needle).unwrap();
        assert!(
            r.hidden_ranges.iter().any(|&(s, e)| s <= at && at < e),
            "{needle} not in hidden_ranges {:?}",
            r.hidden_ranges
        );
    }
}

#[test]
fn bodies_and_focus_keep_full_docs() {
    let r = fold(RUST, Language::Rust, Level::Bodies, &[]);
    assert!(r.content.contains("# Examples"));
    assert!(r.content.contains("Long explanation"));

    let r = fold(RUST, Language::Rust, Level::Signatures, &["add"]);
    assert!(r.content.contains("# Examples"), "{}", r.content);
    assert!(!r.content.contains("Long explanation"));
}

#[test]
fn single_paragraph_docs_are_untouched() {
    let src = "/// One line.\n/// Two line.\nfn f() {}\n";
    let r = fold(src, Language::Rust, Level::Signatures, &[]);
    assert!(r.content.contains("/// One line.\n/// Two line.\nfn f()"));
}

#[test]
fn ts_jsdoc_keeps_summary_and_closing() {
    let src = "\
/**
 * Logs a user in.
 *
 * Longer notes.
 * @param name the user
 */
export function login(name: string): boolean {
  return true;
}
";
    let r = fold(src, Language::TypeScript, Level::Signatures, &[]);
    assert!(
        r.content
            .contains("/**\n * Logs a user in.\n */\nexport function login"),
        "{}",
        r.content
    );
    assert!(!r.content.contains("@param"));
}

#[test]
fn ts_jsdoc_tags_end_the_summary() {
    let src = "/**\n * Adds.\n * @param x n\n */\nfunction add(x: number) {}\n";
    let r = fold(src, Language::TypeScript, Level::Signatures, &[]);
    assert!(
        r.content.contains("/**\n * Adds.\n */\nfunction add"),
        "{}",
        r.content
    );
}

#[test]
fn go_doc_keeps_first_paragraph() {
    let src = "\
package auth

// Login authenticates a user.
//
// It checks the password hash.
func Login(name string) bool {
\treturn true
}
";
    let r = fold(src, Language::Go, Level::Signatures, &[]);
    assert!(
        r.content
            .contains("// Login authenticates a user.\nfunc Login(name string) bool"),
        "{}",
        r.content
    );
    assert!(!r.content.contains("password hash"));
}

#[test]
fn crlf_docs_are_summarized() {
    let src = RUST.replace('\n', "\r\n");
    let r = fold(&src, Language::Rust, Level::Signatures, &[]);
    assert!(
        r.content.contains("/// Adds one.\r\npub fn add"),
        "{}",
        r.content
    );
    assert!(!r.content.contains("# Examples"));
}
