use std::path::PathBuf;

use codefold_core::{read, Level};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn tsx_is_detected_as_tsx() {
    let r = read(&fixture("typescript/page.tsx"), Level::Full).unwrap();
    assert_eq!(r.language, "tsx");
}

#[test]
fn signatures_keeps_exports_and_hides_jsx() {
    let r = read(&fixture("typescript/page.tsx"), Level::Signatures).unwrap();
    assert!(r.content.contains("export function Page("));
    assert!(r.content.contains("export interface PageProps"));
    // JSX returned inside Page() lives in the body → hidden at Signatures.
    assert!(!r.content.contains("<div className"));
    assert!(!r.content.contains("Welcome,"));
}

#[test]
fn bodies_keeps_jsx_inside_function_bodies() {
    let r = read(&fixture("typescript/page.tsx"), Level::Bodies).unwrap();
    assert!(r.content.contains("<div className"));
    assert!(r.content.contains("setCount"));
}

#[test]
fn public_hides_non_exported_helpers() {
    let r = read(&fixture("typescript/page.tsx"), Level::Public).unwrap();
    assert!(r.content.contains("export function Page"));
    assert!(r.content.contains("export interface PageProps"));
    assert!(
        !r.content.contains("function internalHelper"),
        "non-exported helper should be hidden at Public level"
    );
}

#[test]
fn jsx_extension_also_detected_as_tsx() {
    // Use a temp file since we don't have a .jsx fixture; same fixture content
    // works either way because the tsx grammar parses JS+JSX too.
    let tmp = std::env::temp_dir().join("codefold-test.jsx");
    std::fs::write(&tmp, "export function App() { return <div>hello</div>; }\n").unwrap();

    let r = read(&tmp, Level::Signatures).unwrap();
    assert_eq!(r.language, "tsx");
    assert!(r.content.contains("export function App"));
    assert!(!r.content.contains("<div>hello</div>"));

    let _ = std::fs::remove_file(&tmp);
}
