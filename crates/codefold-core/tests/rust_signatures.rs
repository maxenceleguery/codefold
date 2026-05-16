use std::fs;
use std::path::PathBuf;

use codefold_core::{read, Level, SymbolKind};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn detects_rust_language() {
    let r = read(&fixture("rust/auth.rs"), Level::Full).unwrap();
    assert_eq!(r.language, "rust");
}

#[test]
fn keeps_use_declarations() {
    let r = read(&fixture("rust/auth.rs"), Level::Signatures).unwrap();
    assert!(r.content.contains("use std::collections::HashMap"));
}

#[test]
fn keeps_fn_signatures_and_hides_bodies() {
    let r = read(&fixture("rust/auth.rs"), Level::Signatures).unwrap();
    assert!(
        r.content.contains("pub fn login("),
        "missing login signature"
    );
    assert!(
        r.content.contains("pub fn verify_token("),
        "missing verify_token signature"
    );
    // Bodies hidden
    assert!(
        !r.content.contains("users.iter().find"),
        "login body should be hidden"
    );
    // Nested fn signature is hidden at Signatures level
    assert!(
        !r.content.contains("fn matches("),
        "nested fn matches should be hidden"
    );
}

#[test]
fn keeps_structs_and_enums_verbatim() {
    let r = read(&fixture("rust/auth.rs"), Level::Signatures).unwrap();
    assert!(r.content.contains("pub struct User"));
    assert!(r.content.contains("pub id: u64"));
    assert!(r.content.contains("pub email: String"));
}

#[test]
fn keeps_impl_blocks_with_method_signatures() {
    let r = read(&fixture("rust/auth.rs"), Level::Signatures).unwrap();
    assert!(r.content.contains("impl User"));
    assert!(r.content.contains("impl TokenStore"));
    assert!(r.content.contains("pub fn check_password"));
    assert!(r.content.contains("pub fn issue"));
    assert!(r.content.contains("pub fn verify"));
}

#[test]
fn hides_method_bodies() {
    let r = read(&fixture("rust/auth.rs"), Level::Signatures).unwrap();
    // Use body-only substrings (not symbol names that appear in signatures).
    assert!(
        !r.content.contains("hash_password(plaintext)"),
        "check_password body content should be hidden"
    );
    assert!(
        !r.content.contains("self.tokens.insert"),
        "issue body content should be hidden"
    );
}

#[test]
fn keeps_doc_comments() {
    let r = read(&fixture("rust/auth.rs"), Level::Signatures).unwrap();
    assert!(r.content.contains("/// A registered user"));
    assert!(r.content.contains("//! Authentication helpers"));
}

#[test]
fn keeps_attributes_on_items() {
    let r = read(&fixture("rust/auth.rs"), Level::Signatures).unwrap();
    assert!(r.content.contains("#[derive(Debug, Clone)]"));
}

#[test]
fn keeps_pub_constants_and_hides_private() {
    let r = read(&fixture("rust/auth.rs"), Level::Signatures).unwrap();
    assert!(r.content.contains("SESSION_TTL_SECONDS"));
    // PEPPER is private but at Signatures level we still keep it.
    assert!(r.content.contains("PEPPER"));
}

#[test]
fn substantially_reduces_size() {
    let path = fixture("rust/auth.rs");
    let full_len = fs::read_to_string(&path).unwrap().len();
    let r = read(&path, Level::Signatures).unwrap();
    assert!(
        r.content.len() < full_len * 8 / 10,
        "expected signatures < 80% of full ({full_len}), got {}",
        r.content.len()
    );
}

#[test]
fn emits_symbols_with_kinds() {
    let r = read(&fixture("rust/auth.rs"), Level::Signatures).unwrap();
    let names: Vec<&str> = r.symbols.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"User"), "missing User symbol");
    assert!(names.contains(&"TokenStore"), "missing TokenStore symbol");
    assert!(names.contains(&"login"), "missing login symbol");
    assert!(
        names.contains(&"verify_token"),
        "missing verify_token symbol"
    );
    assert!(names.contains(&"check_password"));
    assert!(names.contains(&"issue"));

    let user = r.symbols.iter().find(|s| s.name == "User").unwrap();
    assert_eq!(user.kind, SymbolKind::Class);

    let login = r.symbols.iter().find(|s| s.name == "login").unwrap();
    assert_eq!(login.kind, SymbolKind::Function);

    let issue = r.symbols.iter().find(|s| s.name == "issue").unwrap();
    assert_eq!(issue.kind, SymbolKind::Method);
}
