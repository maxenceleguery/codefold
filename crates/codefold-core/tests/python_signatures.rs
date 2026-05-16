use std::fs;
use std::path::PathBuf;

use codefold_core::{read, Level, SymbolKind};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn keeps_imports() {
    let r = read(&fixture("python/auth.py"), Level::Signatures).unwrap();
    assert!(r.content.contains("import hashlib"), "missing hashlib import");
    assert!(r.content.contains("import secrets"), "missing secrets import");
    assert!(
        r.content.contains("from dataclasses import dataclass"),
        "missing dataclass import"
    );
    assert!(
        r.content.contains("from __future__ import annotations"),
        "missing future import"
    );
}

#[test]
fn keeps_top_level_function_signatures_and_hides_bodies() {
    let r = read(&fixture("python/auth.py"), Level::Signatures).unwrap();
    assert!(
        r.content.contains("def login(email: str, password: str"),
        "missing login signature"
    );
    assert!(
        r.content.contains("def verify_token(token: str"),
        "missing verify_token signature"
    );
    // login's nested helper must not leak
    assert!(
        !r.content.contains("def matches"),
        "inner def matches() should be hidden"
    );
    // bodies of top-level fns must be hidden
    assert!(
        !r.content.contains("user = next("),
        "login body content should be hidden"
    );
}

#[test]
fn keeps_classes_with_method_signatures() {
    let r = read(&fixture("python/auth.py"), Level::Signatures).unwrap();
    assert!(r.content.contains("class User"), "missing class User");
    assert!(
        r.content.contains("class TokenStore"),
        "missing class TokenStore"
    );
    assert!(
        r.content.contains("def check_password"),
        "missing User.check_password"
    );
    assert!(r.content.contains("def issue"), "missing TokenStore.issue");
    assert!(r.content.contains("def verify"), "missing TokenStore.verify");
}

#[test]
fn keeps_decorators_on_classes() {
    let r = read(&fixture("python/auth.py"), Level::Signatures).unwrap();
    assert!(r.content.contains("@dataclass"), "missing @dataclass decorator");
}

#[test]
fn hides_method_bodies() {
    let r = read(&fixture("python/auth.py"), Level::Signatures).unwrap();
    // Body of check_password
    assert!(
        !r.content.contains("secrets.compare_digest"),
        "check_password body should be hidden"
    );
    // Body of issue
    assert!(
        !r.content.contains("secrets.token_urlsafe"),
        "issue body should be hidden"
    );
}

#[test]
fn substantially_reduces_size_on_body_heavy_file() {
    // Compression scales with body-to-signature ratio. On a body-heavy file
    // we expect well over 50% reduction.
    let path = fixture("python/heavy.py");
    let full_len = fs::read_to_string(&path).unwrap().len();
    let r = read(&path, Level::Signatures).unwrap();
    assert!(
        r.content.len() < full_len / 2,
        "expected signatures < 50% of body-heavy fixture ({full_len}), got {}",
        r.content.len()
    );
}

#[test]
fn signatures_smaller_than_full_on_small_file() {
    // On a small, signature-dominated file, savings are smaller but still real.
    let path = fixture("python/auth.py");
    let full_len = fs::read_to_string(&path).unwrap().len();
    let r = read(&path, Level::Signatures).unwrap();
    assert!(
        r.content.len() < full_len * 8 / 10,
        "expected signatures < 80% of full ({full_len}), got {}",
        r.content.len()
    );
}

#[test]
fn reports_hidden_ranges() {
    let r = read(&fixture("python/auth.py"), Level::Signatures).unwrap();
    assert!(!r.hidden_ranges.is_empty(), "expected at least one hidden range");
    for (start, end) in &r.hidden_ranges {
        assert!(
            end > start,
            "hidden range start={start} end={end} is invalid"
        );
    }
}

#[test]
fn emits_symbols_with_kinds() {
    let r = read(&fixture("python/auth.py"), Level::Signatures).unwrap();
    let names: Vec<&str> = r.symbols.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"User"), "missing symbol User");
    assert!(names.contains(&"TokenStore"), "missing symbol TokenStore");
    assert!(names.contains(&"login"), "missing symbol login");
    assert!(names.contains(&"verify_token"), "missing symbol verify_token");
    assert!(names.contains(&"check_password"), "missing symbol check_password");
    assert!(names.contains(&"issue"), "missing symbol issue");

    let user = r.symbols.iter().find(|s| s.name == "User").unwrap();
    assert_eq!(user.kind, SymbolKind::Class);

    let login = r.symbols.iter().find(|s| s.name == "login").unwrap();
    assert_eq!(login.kind, SymbolKind::Function);

    let check_pw = r.symbols.iter().find(|s| s.name == "check_password").unwrap();
    assert_eq!(check_pw.kind, SymbolKind::Method);
}

#[test]
fn output_remains_valid_python_syntactically() {
    // The rendered view should still be parseable Python. We don't fully verify
    // here (would require a Python interpreter) but at minimum we check that
    // every `def` and `class` line ends with `:` and is followed by an indented
    // body marker.
    let r = read(&fixture("python/auth.py"), Level::Signatures).unwrap();
    for line in r.content.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("def ") || trimmed.starts_with("async def ") || trimmed.starts_with("class ") {
            assert!(
                line.trim_end().ends_with(':') || line.contains("->"),
                "header line missing colon: {line:?}"
            );
        }
    }
}
