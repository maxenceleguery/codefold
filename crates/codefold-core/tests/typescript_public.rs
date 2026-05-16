use std::path::PathBuf;

use codefold_core::{read, Level};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn keeps_exported_top_level_functions() {
    let r = read(&fixture("typescript/auth.ts"), Level::Public).unwrap();
    assert!(
        r.content.contains("export function login"),
        "missing exported login"
    );
    assert!(
        r.content.contains("export function verifyToken"),
        "missing exported verifyToken"
    );
}

#[test]
fn hides_non_exported_top_level_functions() {
    let r = read(&fixture("typescript/auth.ts"), Level::Public).unwrap();
    assert!(
        !r.content.contains("function hashPassword"),
        "non-exported hashPassword should be hidden"
    );
}

#[test]
fn keeps_exported_classes_and_interfaces() {
    let r = read(&fixture("typescript/auth.ts"), Level::Public).unwrap();
    assert!(r.content.contains("export class TokenStore"));
    assert!(r.content.contains("export interface User"));
}

#[test]
fn hides_private_class_methods() {
    let r = read(&fixture("typescript/auth.ts"), Level::Public).unwrap();
    assert!(
        !r.content.contains("_rotate"),
        "private _rotate should be hidden"
    );
}

#[test]
fn keeps_imports() {
    let r = read(&fixture("typescript/auth.ts"), Level::Public).unwrap();
    assert!(r.content.contains("import { createHash"));
}

#[test]
fn bodies_are_hidden_at_public_level() {
    let r = read(&fixture("typescript/auth.ts"), Level::Public).unwrap();
    assert!(!r.content.contains("users.find(matches)"));
    assert!(!r.content.contains("randomBytes(32)"));
}

#[test]
fn keeps_exported_constants() {
    let r = read(&fixture("typescript/auth.ts"), Level::Public).unwrap();
    assert!(r.content.contains("SESSION_TTL_SECONDS"));
}

#[test]
fn hides_non_exported_constants() {
    let r = read(&fixture("typescript/auth.ts"), Level::Public).unwrap();
    assert!(
        !r.content.contains("_PEPPER"),
        "non-exported _PEPPER should be hidden"
    );
}
