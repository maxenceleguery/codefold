use std::path::PathBuf;

use codefold_core::{read, Level};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn keeps_uppercase_functions_and_hides_lowercase() {
    let r = read(&fixture("go/auth.go"), Level::Public).unwrap();
    assert!(r.content.contains("func Login("));
    assert!(r.content.contains("func VerifyToken("));
    // Private fns hidden
    assert!(
        !r.content.contains("func hashPassword("),
        "private hashPassword should be hidden"
    );
    assert!(
        !r.content.contains("func mintToken("),
        "private mintToken should be hidden"
    );
}

#[test]
fn keeps_uppercase_methods_and_hides_lowercase() {
    let r = read(&fixture("go/auth.go"), Level::Public).unwrap();
    assert!(r.content.contains("func (u *User) CheckPassword("));
    assert!(r.content.contains("func (s *TokenStore) Issue("));
    assert!(r.content.contains("func (s *TokenStore) Verify("));
    // The private method `rotate` should be hidden
    assert!(
        !r.content.contains("func (s *TokenStore) rotate("),
        "private rotate method should be hidden"
    );
}

#[test]
fn keeps_uppercase_constants_and_hides_lowercase() {
    let r = read(&fixture("go/auth.go"), Level::Public).unwrap();
    assert!(r.content.contains("SessionTTLSeconds"));
    assert!(
        !r.content.contains("pepper"),
        "private const pepper should be hidden"
    );
}

#[test]
fn keeps_exported_types() {
    let r = read(&fixture("go/auth.go"), Level::Public).unwrap();
    assert!(r.content.contains("type User struct"));
    assert!(r.content.contains("type TokenStore struct"));
}

#[test]
fn bodies_hidden_at_public_level() {
    let r = read(&fixture("go/auth.go"), Level::Public).unwrap();
    assert!(!r.content.contains("subtle.ConstantTimeCompare"));
    assert!(!r.content.contains("store.Verify(token)"));
}

#[test]
fn keeps_constructor_function() {
    let r = read(&fixture("go/auth.go"), Level::Public).unwrap();
    // NewTokenStore is uppercase → kept
    assert!(r.content.contains("func NewTokenStore("));
}
