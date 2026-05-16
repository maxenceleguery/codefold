use std::path::PathBuf;

use codefold_core::{read, Level};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn keeps_top_level_function_bodies() {
    let r = read(&fixture("go/auth.go"), Level::Bodies).unwrap();
    assert!(r.content.contains("errors.New(\"invalid credentials\")"));
    assert!(r.content.contains("store.Verify(token)"));
}

#[test]
fn keeps_method_bodies() {
    let r = read(&fixture("go/auth.go"), Level::Bodies).unwrap();
    assert!(r.content.contains("subtle.ConstantTimeCompare"));
    assert!(r.content.contains("s.tokens[token] = userID"));
}

#[test]
fn keeps_imports_types_methods() {
    let r = read(&fixture("go/auth.go"), Level::Bodies).unwrap();
    assert!(r.content.contains("package auth"));
    assert!(r.content.contains("type User struct"));
    assert!(r.content.contains("func (s *TokenStore) Issue("));
}
