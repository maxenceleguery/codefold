use std::path::PathBuf;

use codefold_core::{read, read_opts, Level, Options};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn focus_keeps_named_function_body_at_signatures_level() {
    let opts = Options::new(Level::Signatures).focus(["login"]);
    let r = read_opts(&fixture("python/auth.py"), opts).unwrap();

    assert!(
        r.content.contains("user = next("),
        "focused login body should be visible"
    );
    assert!(
        !r.content.contains("return store.verify(token)"),
        "non-focused verify_token body should be hidden"
    );
}

#[test]
fn focus_with_method_name_keeps_method_body() {
    let opts = Options::new(Level::Signatures).focus(["check_password"]);
    let r = read_opts(&fixture("python/auth.py"), opts).unwrap();

    assert!(
        r.content.contains("secrets.compare_digest"),
        "focused check_password body should be visible"
    );
    assert!(
        !r.content.contains("secrets.token_urlsafe"),
        "non-focused issue body should be hidden"
    );
}

#[test]
fn focus_with_class_name_keeps_all_methods_in_class() {
    let opts = Options::new(Level::Signatures).focus(["User"]);
    let r = read_opts(&fixture("python/auth.py"), opts).unwrap();

    // User.check_password body should be visible (User is in focus)
    assert!(
        r.content.contains("secrets.compare_digest"),
        "User.check_password body should be visible when User is focused"
    );
    // TokenStore methods should still be hidden
    assert!(
        !r.content.contains("secrets.token_urlsafe"),
        "TokenStore.issue body should remain hidden"
    );
}

#[test]
fn focus_with_multiple_names() {
    let opts = Options::new(Level::Signatures).focus(["login", "verify_token"]);
    let r = read_opts(&fixture("python/auth.py"), opts).unwrap();

    assert!(r.content.contains("user = next("));
    assert!(r.content.contains("return store.verify(token)"));
    assert!(
        !r.content.contains("secrets.compare_digest"),
        "non-focused check_password body should stay hidden"
    );
}

#[test]
fn empty_focus_equals_plain_read() {
    let opts = Options::new(Level::Signatures);
    let r1 = read_opts(&fixture("python/auth.py"), opts).unwrap();
    let r2 = read(&fixture("python/auth.py"), Level::Signatures).unwrap();
    assert_eq!(r1.content, r2.content);
}

#[test]
fn unknown_focus_name_is_a_no_op() {
    let opts = Options::new(Level::Signatures).focus(["does_not_exist"]);
    let r1 = read_opts(&fixture("python/auth.py"), opts).unwrap();
    let r2 = read(&fixture("python/auth.py"), Level::Signatures).unwrap();
    assert_eq!(r1.content, r2.content);
}
