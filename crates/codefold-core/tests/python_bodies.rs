use std::fs;
use std::path::PathBuf;

use codefold_core::{read, Level};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn keeps_top_level_function_bodies() {
    let r = read(&fixture("python/auth.py"), Level::Bodies).unwrap();
    // login's body content should appear in the output.
    assert!(
        r.content.contains("user = next("),
        "login body content missing"
    );
    assert!(
        r.content.contains("return store.issue(user.id)"),
        "login body content missing"
    );
    // verify_token's body
    assert!(
        r.content.contains("return store.verify(token)"),
        "verify_token body missing"
    );
}

#[test]
fn keeps_class_method_bodies() {
    let r = read(&fixture("python/auth.py"), Level::Bodies).unwrap();
    assert!(
        r.content.contains("secrets.compare_digest"),
        "check_password body should be kept"
    );
    assert!(
        r.content.contains("secrets.token_urlsafe"),
        "issue body should be kept"
    );
    assert!(
        r.content.contains("self._tokens.get(token)"),
        "verify body should be kept"
    );
}

#[test]
fn collapses_nested_function_bodies() {
    let r = read(&fixture("python/auth.py"), Level::Bodies).unwrap();
    // `def matches(u)` is nested inside login. Its signature should appear,
    // but the body content `u.check_password(password)` should not.
    assert!(
        r.content.contains("def matches"),
        "nested def matches signature missing"
    );
    assert!(
        !r.content.contains("u.email == email"),
        "matches body should be collapsed"
    );
}

#[test]
fn keeps_module_docstring_and_imports() {
    let r = read(&fixture("python/auth.py"), Level::Bodies).unwrap();
    assert!(r.content.contains("Authentication helpers"));
    assert!(r.content.contains("import hashlib"));
    assert!(r.content.contains("from dataclasses import dataclass"));
}

#[test]
fn keeps_decorators_on_classes() {
    let r = read(&fixture("python/auth.py"), Level::Bodies).unwrap();
    assert!(r.content.contains("@dataclass"));
}

#[test]
fn bodies_output_at_most_as_long_as_full() {
    let path = fixture("python/auth.py");
    let full_len = fs::read_to_string(&path).unwrap().len();
    let r = read(&path, Level::Bodies).unwrap();
    assert!(
        r.content.len() <= full_len,
        "bodies length {} exceeded full length {}",
        r.content.len(),
        full_len
    );
}

#[test]
fn bodies_emits_symbols() {
    let r = read(&fixture("python/auth.py"), Level::Bodies).unwrap();
    let names: Vec<&str> = r.symbols.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"login"));
    assert!(names.contains(&"User"));
    assert!(names.contains(&"check_password"));
}
