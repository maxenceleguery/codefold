use std::path::PathBuf;

use codefold_core::{read, Level};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn keeps_public_top_level_functions() {
    let r = read(&fixture("python/auth.py"), Level::Public).unwrap();
    assert!(r.content.contains("def login"), "missing public login");
    assert!(
        r.content.contains("def verify_token"),
        "missing public verify_token"
    );
}

#[test]
fn hides_underscore_prefixed_top_level_functions() {
    let r = read(&fixture("python/auth.py"), Level::Public).unwrap();
    assert!(
        !r.content.contains("def _hash_password"),
        "_hash_password should be hidden"
    );
}

#[test]
fn keeps_public_classes_with_public_methods_only() {
    let r = read(&fixture("python/auth.py"), Level::Public).unwrap();
    assert!(r.content.contains("class User"));
    assert!(r.content.contains("class TokenStore"));
    assert!(r.content.contains("def check_password"));
    assert!(r.content.contains("def issue"));
    assert!(r.content.contains("def verify"));
}

#[test]
fn hides_underscore_prefixed_methods() {
    let r = read(&fixture("python/auth.py"), Level::Public).unwrap();
    assert!(
        !r.content.contains("def _rotate"),
        "_rotate should be hidden"
    );
}

#[test]
fn keeps_dunder_methods() {
    let r = read(&fixture("python/auth.py"), Level::Public).unwrap();
    assert!(
        r.content.contains("def __init__"),
        "__init__ should be kept (dunder is public-by-convention)"
    );
}

#[test]
fn keeps_imports_and_module_docstring() {
    let r = read(&fixture("python/auth.py"), Level::Public).unwrap();
    assert!(r.content.contains("import hashlib"));
    assert!(r.content.contains("Authentication helpers"));
}

#[test]
fn keeps_public_top_level_constants() {
    let r = read(&fixture("python/auth.py"), Level::Public).unwrap();
    assert!(r.content.contains("SESSION_TTL_SECONDS"));
}

#[test]
fn hides_underscore_prefixed_top_level_constants() {
    let r = read(&fixture("python/auth.py"), Level::Public).unwrap();
    assert!(
        !r.content.contains("_PEPPER"),
        "_PEPPER private constant should be hidden"
    );
}

#[test]
fn bodies_are_hidden_at_public_level() {
    let r = read(&fixture("python/auth.py"), Level::Public).unwrap();
    // Public level renders signatures only — no bodies.
    assert!(!r.content.contains("user = next("));
    assert!(!r.content.contains("secrets.compare_digest"));
}
