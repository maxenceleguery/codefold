use std::path::PathBuf;

use codefold_core::{read, Level};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn detects_markdown_language() {
    let r = read(&fixture("markdown/sample.md"), Level::Full).unwrap();
    assert_eq!(r.language, "markdown");
}

#[test]
fn full_returns_verbatim() {
    let r = read(&fixture("markdown/sample.md"), Level::Full).unwrap();
    assert!(r.content.contains("A short tagline."));
    assert!(r.content.contains("Body under a setext heading."));
}

#[test]
fn bodies_returns_verbatim() {
    let r = read(&fixture("markdown/sample.md"), Level::Bodies).unwrap();
    assert!(r.content.contains("A short tagline."));
    assert!(r.content.contains("Body under a setext heading."));
}

#[test]
fn signatures_keeps_only_headings() {
    let r = read(&fixture("markdown/sample.md"), Level::Signatures).unwrap();
    // Headings stay (verbatim, with their markers).
    assert!(r.content.contains("# Project Title"));
    assert!(r.content.contains("## Overview"));
    assert!(r.content.contains("### Basic"));
    assert!(r.content.contains("#### Tip"));
    // Setext heading
    assert!(r.content.contains("Setext-style heading"));
    // Body prose is gone.
    assert!(!r.content.contains("A short tagline."));
    assert!(!r.content.contains("Body under a setext heading."));
    // The code fence inside the body is gone.
    assert!(!r.content.contains("echo \"this fenced"));
}

#[test]
fn signatures_substantially_smaller_than_full() {
    let r_full = read(&fixture("markdown/sample.md"), Level::Full).unwrap();
    let r_sig = read(&fixture("markdown/sample.md"), Level::Signatures).unwrap();
    assert!(
        r_sig.content.len() < r_full.content.len() / 2,
        "expected signatures < 50% of full ({}); got {}",
        r_full.content.len(),
        r_sig.content.len(),
    );
}

#[test]
fn emits_one_symbol_per_heading() {
    let r = read(&fixture("markdown/sample.md"), Level::Signatures).unwrap();
    let names: Vec<&str> = r.symbols.iter().map(|s| s.name.as_str()).collect();
    assert!(names.iter().any(|n| n.contains("Project Title")));
    assert!(names.iter().any(|n| n.contains("Overview")));
    assert!(names.iter().any(|n| n.contains("Basic")));
    assert!(names.iter().any(|n| n.contains("Tip")));
    assert!(names.iter().any(|n| n.contains("Setext-style heading")));
    // Heading depth prefix
    assert!(names.iter().any(|n| n.starts_with("# ")));
    assert!(names.iter().any(|n| n.starts_with("## ")));
    assert!(names.iter().any(|n| n.starts_with("### ")));
    assert!(names.iter().any(|n| n.starts_with("#### ")));
}

#[test]
fn fenced_code_with_hash_is_not_treated_as_heading() {
    // The fixture has `# even though this line starts with #` inside a code fence.
    // pulldown_cmark correctly does NOT report that as a heading.
    let r = read(&fixture("markdown/sample.md"), Level::Signatures).unwrap();
    assert!(
        !r.content.contains("even though"),
        "code-fence content should not appear in the headings-only outline"
    );
    let any_code_hash_symbol = r
        .symbols
        .iter()
        .any(|s| s.name.contains("even though this line starts with"));
    assert!(!any_code_hash_symbol);
}

#[test]
fn dot_markdown_extension_works() {
    let tmp = std::env::temp_dir().join("codefold-test-md-ext.markdown");
    std::fs::write(&tmp, "# Hello\n\nWorld.\n").unwrap();
    let r = read(&tmp, Level::Signatures).unwrap();
    assert_eq!(r.language, "markdown");
    assert!(r.content.contains("# Hello"));
    assert!(!r.content.contains("World"));
    let _ = std::fs::remove_file(&tmp);
}
