use std::fs;
use std::path::PathBuf;

use codefold_core::{read, Level};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn full_level_returns_file_contents_verbatim() {
    let path = fixture("python/auth.py");
    let expected = fs::read_to_string(&path).unwrap();

    let result = read(&path, Level::Full).unwrap();

    assert_eq!(result.content, expected);
    assert_eq!(result.language, "python");
    assert!(
        result.hidden_ranges.is_empty(),
        "Full level should hide nothing"
    );
}

#[test]
fn full_level_reports_unsupported_extension() {
    let path = fixture("unknown.xyz");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, "anything").unwrap();

    let err = read(&path, Level::Full).unwrap_err();
    assert!(
        format!("{err}").contains("unsupported language"),
        "expected UnsupportedLanguage, got: {err}"
    );

    let _ = fs::remove_file(&path);
}
