use std::path::PathBuf;

use codefold_core::{read, Level};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn keeps_top_level_function_bodies() {
    let r = read(&fixture("typescript/auth.ts"), Level::Bodies).unwrap();
    assert!(
        r.content.contains("users.find(matches)"),
        "login body should be present"
    );
    assert!(
        r.content.contains("return store.verify(token)"),
        "verifyToken body should be present"
    );
}

#[test]
fn keeps_class_method_bodies() {
    let r = read(&fixture("typescript/auth.ts"), Level::Bodies).unwrap();
    assert!(
        r.content.contains("randomBytes(32)"),
        "issue body should be present"
    );
    assert!(
        r.content.contains("this.tokens.get(token)"),
        "verify body should be present"
    );
}

#[test]
fn collapses_nested_function_bodies() {
    let r = read(&fixture("typescript/auth.ts"), Level::Bodies).unwrap();
    // matches() inside login should keep its signature
    assert!(
        r.content.contains("function matches"),
        "nested matches signature should be present"
    );
    // but its body content should be collapsed
    assert!(
        !r.content.contains("u.email === email"),
        "matches body should be collapsed"
    );
}

#[test]
fn keeps_module_imports_and_classes() {
    let r = read(&fixture("typescript/auth.ts"), Level::Bodies).unwrap();
    assert!(r.content.contains("import { createHash"));
    assert!(r.content.contains("class TokenStore"));
    assert!(r.content.contains("interface User"));
}
