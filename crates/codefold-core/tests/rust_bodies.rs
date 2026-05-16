use std::path::PathBuf;

use codefold_core::{read, Level};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn keeps_top_level_fn_bodies() {
    let r = read(&fixture("rust/auth.rs"), Level::Bodies).unwrap();
    assert!(
        r.content.contains("users.iter().find"),
        "login body should be present"
    );
    assert!(
        r.content.contains("store.verify(token)"),
        "verify_token body should be present"
    );
}

#[test]
fn keeps_impl_method_bodies() {
    let r = read(&fixture("rust/auth.rs"), Level::Bodies).unwrap();
    assert!(
        r.content.contains("constant_time_eq"),
        "check_password body should be present"
    );
    assert!(
        r.content.contains("self.tokens.insert"),
        "issue body should be present"
    );
}

#[test]
fn collapses_nested_fn_bodies() {
    let r = read(&fixture("rust/auth.rs"), Level::Bodies).unwrap();
    // matches() is nested inside login. Its signature should appear, body hidden.
    assert!(
        r.content.contains("fn matches("),
        "nested matches signature should be present"
    );
    assert!(
        !r.content.contains("u.check_password(password)"),
        "matches body should be collapsed"
    );
}

#[test]
fn keeps_use_struct_impl_headers() {
    let r = read(&fixture("rust/auth.rs"), Level::Bodies).unwrap();
    assert!(r.content.contains("use std::collections::HashMap"));
    assert!(r.content.contains("pub struct User"));
    assert!(r.content.contains("impl User"));
}
