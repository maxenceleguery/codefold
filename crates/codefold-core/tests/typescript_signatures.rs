use std::fs;
use std::path::PathBuf;

use codefold_core::{read, Level, SymbolKind};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn detects_typescript() {
    let r = read(&fixture("typescript/auth.ts"), Level::Full).unwrap();
    assert_eq!(r.language, "typescript");
}

#[test]
fn keeps_imports() {
    let r = read(&fixture("typescript/auth.ts"), Level::Signatures).unwrap();
    assert!(
        r.content.contains("import { createHash"),
        "missing createHash import"
    );
    assert!(
        r.content.contains("from \"node:crypto\""),
        "missing crypto module path"
    );
}

#[test]
fn keeps_top_level_function_signatures_and_hides_bodies() {
    let r = read(&fixture("typescript/auth.ts"), Level::Signatures).unwrap();
    assert!(
        r.content.contains("function login("),
        "missing login signature"
    );
    assert!(
        r.content.contains("function verifyToken("),
        "missing verifyToken signature"
    );
    // login body should be hidden
    assert!(
        !r.content.contains("users.find(matches)"),
        "login body should be hidden"
    );
    // Nested helper should not appear at Signatures level
    assert!(
        !r.content.contains("function matches"),
        "nested function matches() should be hidden"
    );
}

#[test]
fn keeps_classes_with_method_signatures() {
    let r = read(&fixture("typescript/auth.ts"), Level::Signatures).unwrap();
    assert!(r.content.contains("class TokenStore"));
    assert!(r.content.contains("issue("));
    assert!(r.content.contains("verify("));
}

#[test]
fn hides_method_bodies() {
    let r = read(&fixture("typescript/auth.ts"), Level::Signatures).unwrap();
    assert!(
        !r.content.contains("randomBytes(32)"),
        "issue body should be hidden"
    );
    assert!(
        !r.content.contains("this.tokens.get"),
        "verify body should be hidden"
    );
}

#[test]
fn keeps_interfaces_and_type_aliases_verbatim() {
    let r = read(&fixture("typescript/auth.ts"), Level::Signatures).unwrap();
    assert!(r.content.contains("interface User"));
    assert!(r.content.contains("id: number"));
    assert!(r.content.contains("email: string"));
}

#[test]
fn keeps_top_level_const_assignments() {
    let r = read(&fixture("typescript/auth.ts"), Level::Signatures).unwrap();
    assert!(r.content.contains("SESSION_TTL_SECONDS"));
}

#[test]
fn emits_symbols_with_kinds() {
    let r = read(&fixture("typescript/auth.ts"), Level::Signatures).unwrap();
    let names: Vec<&str> = r.symbols.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"TokenStore"), "missing TokenStore symbol");
    assert!(names.contains(&"login"), "missing login symbol");
    assert!(names.contains(&"verifyToken"), "missing verifyToken symbol");
    assert!(names.contains(&"issue"), "missing issue method");

    let token_store = r.symbols.iter().find(|s| s.name == "TokenStore").unwrap();
    assert_eq!(token_store.kind, SymbolKind::Class);

    let login = r.symbols.iter().find(|s| s.name == "login").unwrap();
    assert_eq!(login.kind, SymbolKind::Function);

    let issue = r.symbols.iter().find(|s| s.name == "issue").unwrap();
    assert_eq!(issue.kind, SymbolKind::Method);
}

#[test]
fn substantially_reduces_size() {
    let path = fixture("typescript/auth.ts");
    let full_len = fs::read_to_string(&path).unwrap().len();
    let r = read(&path, Level::Signatures).unwrap();
    assert!(
        r.content.len() < full_len * 8 / 10,
        "expected signatures < 80% of full ({full_len}), got {}",
        r.content.len()
    );
}
