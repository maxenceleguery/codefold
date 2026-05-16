use std::path::PathBuf;

use codefold_core::{read, Level};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn tokens_est_positive_for_non_empty_content() {
    let r = read(&fixture("python/auth.py"), Level::Full).unwrap();
    assert!(
        r.tokens_est > 0,
        "tokens_est should be > 0 for non-empty file"
    );
}

#[test]
fn tokens_est_zero_for_empty_content() {
    let dir = std::env::temp_dir();
    let path = dir.join("codefold_empty_test.py");
    std::fs::write(&path, "").unwrap();

    let r = read(&path, Level::Full).unwrap();
    assert_eq!(r.tokens_est, 0);

    let _ = std::fs::remove_file(&path);
}

#[test]
fn tokens_est_decreases_with_signatures_compression() {
    let path = fixture("python/heavy.py");
    let full = read(&path, Level::Full).unwrap();
    let sig = read(&path, Level::Signatures).unwrap();
    assert!(
        sig.tokens_est < full.tokens_est,
        "signatures tokens {} should be < full tokens {}",
        sig.tokens_est,
        full.tokens_est
    );
}

#[test]
fn tokens_est_within_reasonable_ratio_to_content_length() {
    let r = read(&fixture("python/auth.py"), Level::Full).unwrap();
    let chars = r.content.len();
    // Code averages ~3-5 chars per token. Bound loosely on both sides.
    assert!(
        r.tokens_est >= chars / 8,
        "tokens_est {} too low for content of {chars} chars",
        r.tokens_est
    );
    assert!(
        r.tokens_est <= chars,
        "tokens_est {} > chars {}; nonsensical",
        r.tokens_est,
        chars
    );
}
