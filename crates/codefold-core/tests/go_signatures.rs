use std::fs;
use std::path::PathBuf;

use codefold_core::{read, Level, SymbolKind};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn detects_go_language() {
    let r = read(&fixture("go/auth.go"), Level::Full).unwrap();
    assert_eq!(r.language, "go");
}

#[test]
fn keeps_package_clause_and_imports() {
    let r = read(&fixture("go/auth.go"), Level::Signatures).unwrap();
    assert!(r.content.contains("package auth"));
    assert!(r.content.contains("\"crypto/sha256\""));
    assert!(r.content.contains("\"errors\""));
}

#[test]
fn keeps_function_signatures_and_hides_bodies() {
    let r = read(&fixture("go/auth.go"), Level::Signatures).unwrap();
    assert!(r.content.contains("func Login("));
    assert!(r.content.contains("func VerifyToken("));
    // Bodies hidden
    assert!(!r.content.contains("errors.New(\"invalid credentials\")"));
    assert!(!r.content.contains("store.Verify(token)"));
}

#[test]
fn keeps_method_signatures_with_receivers() {
    let r = read(&fixture("go/auth.go"), Level::Signatures).unwrap();
    assert!(r.content.contains("func (u *User) CheckPassword("));
    assert!(r.content.contains("func (s *TokenStore) Issue("));
    assert!(r.content.contains("func (s *TokenStore) Verify("));
}

#[test]
fn hides_method_bodies() {
    let r = read(&fixture("go/auth.go"), Level::Signatures).unwrap();
    assert!(!r.content.contains("subtle.ConstantTimeCompare"));
    assert!(!r.content.contains("s.tokens[token] = userID"));
}

#[test]
fn keeps_structs_verbatim() {
    let r = read(&fixture("go/auth.go"), Level::Signatures).unwrap();
    assert!(r.content.contains("type User struct"));
    assert!(r.content.contains("ID           uint64"));
    assert!(r.content.contains("PasswordHash string"));
    assert!(r.content.contains("type TokenStore struct"));
}

#[test]
fn keeps_doc_comments() {
    let r = read(&fixture("go/auth.go"), Level::Signatures).unwrap();
    assert!(r.content.contains("// User is a registered user."));
    assert!(r.content.contains("// Login attempts to log a user in"));
}

#[test]
fn keeps_const_declarations_at_signatures_level() {
    let r = read(&fixture("go/auth.go"), Level::Signatures).unwrap();
    assert!(r.content.contains("SessionTTLSeconds"));
    // private at Signatures is still kept
    assert!(r.content.contains("pepper"));
}

#[test]
fn substantially_reduces_size() {
    let path = fixture("go/auth.go");
    let full_len = fs::read_to_string(&path).unwrap().len();
    let r = read(&path, Level::Signatures).unwrap();
    assert!(
        r.content.len() < full_len * 8 / 10,
        "expected signatures < 80% of full ({full_len}), got {}",
        r.content.len()
    );
}

#[test]
fn preserves_newlines_between_top_level_declarations() {
    // Regression: tree-sitter-go exposes `\n` as anonymous children of
    // source_file, which once caused adjacent decls to merge onto one line
    // (e.g. `package authimport (`).
    let r = read(&fixture("go/auth.go"), Level::Signatures).unwrap();
    assert!(
        !r.content.contains("authimport"),
        "newline between package and import was lost"
    );
    // `lines()` strips both \n and \r\n, so this is line-ending agnostic
    // (Windows CI checks out fixtures as CRLF by default).
    assert!(
        r.content.lines().any(|line| line.trim_end() == "package auth"),
        "package clause should appear as its own line"
    );
    // No top-level decl should immediately follow `}` without a newline.
    for line in r.content.lines() {
        let trimmed = line.trim_start();
        assert!(
            !line.contains("}package ")
                && !line.contains("}import ")
                && !line.contains("}type ")
                && !line.contains("}func ")
                && !line.contains("}const ")
                && !line.contains("}var "),
            "decls merged onto one line: {trimmed:?}"
        );
    }
}

#[test]
fn emits_symbols_with_kinds() {
    let r = read(&fixture("go/auth.go"), Level::Signatures).unwrap();
    let names: Vec<&str> = r.symbols.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"User"), "missing User");
    assert!(names.contains(&"TokenStore"), "missing TokenStore");
    assert!(names.contains(&"Login"), "missing Login");
    assert!(names.contains(&"VerifyToken"), "missing VerifyToken");
    assert!(names.contains(&"CheckPassword"));
    assert!(names.contains(&"Issue"));

    let user = r.symbols.iter().find(|s| s.name == "User").unwrap();
    assert_eq!(user.kind, SymbolKind::Class);

    let login = r.symbols.iter().find(|s| s.name == "Login").unwrap();
    assert_eq!(login.kind, SymbolKind::Function);

    let issue = r.symbols.iter().find(|s| s.name == "Issue").unwrap();
    assert_eq!(issue.kind, SymbolKind::Method);
}
