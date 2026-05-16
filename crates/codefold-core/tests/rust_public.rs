use std::path::PathBuf;

use codefold_core::{read, Level};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn keeps_pub_functions_and_hides_private() {
    let r = read(&fixture("rust/auth.rs"), Level::Public).unwrap();
    assert!(r.content.contains("pub fn login("));
    assert!(r.content.contains("pub fn verify_token("));
    // Private functions hidden
    assert!(
        !r.content.contains("fn hash_password("),
        "private hash_password should be hidden"
    );
    assert!(
        !r.content.contains("fn mint_token("),
        "private mint_token should be hidden"
    );
    assert!(
        !r.content.contains("fn constant_time_eq("),
        "private constant_time_eq should be hidden"
    );
}

#[test]
fn keeps_pub_structs_and_methods() {
    let r = read(&fixture("rust/auth.rs"), Level::Public).unwrap();
    assert!(r.content.contains("pub struct User"));
    assert!(r.content.contains("pub struct TokenStore"));
    assert!(r.content.contains("pub fn check_password"));
    assert!(r.content.contains("pub fn issue"));
    assert!(r.content.contains("pub fn verify"));
}

#[test]
fn hides_private_methods_inside_impls() {
    let r = read(&fixture("rust/auth.rs"), Level::Public).unwrap();
    // `fn rotate` inside `impl TokenStore` has no `pub`, should be hidden.
    assert!(
        !r.content.contains("fn rotate"),
        "private rotate method should be hidden"
    );
}

#[test]
fn keeps_trait_impl_methods_even_without_pub() {
    let r = read(&fixture("rust/auth.rs"), Level::Public).unwrap();
    // `fn default` inside `impl Default for TokenStore` has no `pub` — but it's
    // public-via-the-trait, so it must remain visible at Public level.
    assert!(
        r.content.contains("fn default"),
        "trait-impl method `default` should be kept at Public level"
    );
}

#[test]
fn keeps_pub_constants_and_hides_private() {
    let r = read(&fixture("rust/auth.rs"), Level::Public).unwrap();
    assert!(r.content.contains("SESSION_TTL_SECONDS"));
    assert!(
        !r.content.contains("PEPPER"),
        "private PEPPER constant should be hidden"
    );
}

#[test]
fn bodies_are_hidden_at_public_level() {
    let r = read(&fixture("rust/auth.rs"), Level::Public).unwrap();
    assert!(!r.content.contains("users.iter().find"));
    assert!(!r.content.contains("self.tokens.insert"));
}
