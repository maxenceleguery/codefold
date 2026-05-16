use std::path::PathBuf;

use assert_cmd::Command;
use predicates::prelude::*;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../codefold-core/tests/fixtures")
        .join(name)
}

#[test]
fn defaults_to_signatures_level() {
    Command::cargo_bin("codefold")
        .unwrap()
        .arg(fixture("python/auth.py"))
        .assert()
        .success()
        .stdout(predicate::str::contains("def login"))
        .stdout(predicate::str::contains("user = next(").not());
}

#[test]
fn full_level_returns_verbatim_content() {
    Command::cargo_bin("codefold")
        .unwrap()
        .args([
            fixture("python/auth.py").to_str().unwrap(),
            "--level",
            "full",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("user = next("));
}

#[test]
fn focus_keeps_named_symbol_body() {
    Command::cargo_bin("codefold")
        .unwrap()
        .args([
            fixture("python/auth.py").to_str().unwrap(),
            "--level",
            "signatures",
            "--focus",
            "login",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("user = next("));
}

#[test]
fn stats_flag_emits_summary_to_stderr() {
    Command::cargo_bin("codefold")
        .unwrap()
        .args([fixture("python/auth.py").to_str().unwrap(), "--stats"])
        .assert()
        .success()
        .stderr(predicate::str::contains("[codefold]"))
        .stderr(predicate::str::contains("tokens="));
}

#[test]
fn unsupported_extension_exits_with_error() {
    let tmp = std::env::temp_dir().join("codefold_cli_unknown.xyz");
    std::fs::write(&tmp, "hello").unwrap();

    Command::cargo_bin("codefold")
        .unwrap()
        .arg(&tmp)
        .assert()
        .failure()
        .stderr(predicate::str::contains("unsupported language"));

    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn typescript_file_works() {
    Command::cargo_bin("codefold")
        .unwrap()
        .args([
            fixture("typescript/auth.ts").to_str().unwrap(),
            "--level",
            "signatures",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("class TokenStore"));
}

#[test]
fn doctor_runs_and_prints_diagnostics() {
    let out = Command::cargo_bin("codefold")
        .unwrap()
        .args(["doctor", "--scope", "project"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("codefold doctor"));
    assert!(stdout.contains("cargo"));
    assert!(stdout.contains("Integration (project scope)"));
}

#[test]
fn setup_list_does_not_write() {
    let tmp = tempfile::TempDir::new().unwrap();
    Command::cargo_bin("codefold")
        .unwrap()
        .args([
            "setup",
            "--list",
            "--project-dir",
            tmp.path().to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("status").and(predicate::str::contains("path")));
    assert!(!tmp.path().join("CLAUDE.md").exists());
    assert!(!tmp.path().join(".cursor").exists());
}

#[test]
fn setup_uninstall_removes_block_via_cli() {
    let tmp = tempfile::TempDir::new().unwrap();
    Command::cargo_bin("codefold")
        .unwrap()
        .args(["setup", "--project-dir", tmp.path().to_str().unwrap()])
        .assert()
        .success();
    assert!(tmp.path().join("CLAUDE.md").exists());

    Command::cargo_bin("codefold")
        .unwrap()
        .args([
            "setup",
            "--uninstall",
            "--project-dir",
            tmp.path().to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("removed"));
    // CLAUDE.md was created solely by codefold (no prior content), so it
    // should now be gone after uninstall.
    assert!(!tmp.path().join("CLAUDE.md").exists());
}

#[test]
fn tsx_file_works() {
    Command::cargo_bin("codefold")
        .unwrap()
        .args([
            fixture("typescript/page.tsx").to_str().unwrap(),
            "--level",
            "signatures",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("export function Page"))
        .stdout(predicate::str::contains("<div").not());
}

#[test]
fn unsupported_error_lists_supported_extensions() {
    let tmp = std::env::temp_dir().join("codefold_cli_xyz.xyz");
    std::fs::write(&tmp, "anything").unwrap();
    Command::cargo_bin("codefold")
        .unwrap()
        .arg(&tmp)
        .assert()
        .failure()
        .stderr(predicate::str::contains("unsupported language"))
        .stderr(predicate::str::contains(".py"))
        .stderr(predicate::str::contains(".tsx"))
        .stderr(predicate::str::contains(".rs"))
        .stderr(predicate::str::contains(".go"));
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn json_format_single_file_emits_object() {
    let out = Command::cargo_bin("codefold")
        .unwrap()
        .args([
            fixture("python/auth.py").to_str().unwrap(),
            "--level",
            "signatures",
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(v.is_object(), "single-file json should be an object");
    assert_eq!(v["language"], "python");
    assert!(v["tokens_est"].as_u64().unwrap() > 0);
    assert!(v["symbols"].is_array());
    let names: Vec<String> = v["symbols"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap().to_string())
        .collect();
    assert!(names.contains(&"login".to_string()));
}

#[test]
fn multi_file_text_separates_with_headers() {
    let out = Command::cargo_bin("codefold")
        .unwrap()
        .args([
            fixture("python/auth.py").to_str().unwrap(),
            fixture("typescript/auth.ts").to_str().unwrap(),
            "--level",
            "signatures",
        ])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("=== ") && stdout.contains("auth.py"));
    assert!(stdout.contains("auth.ts"));
    assert!(stdout.contains("def login"));
    assert!(stdout.contains("class TokenStore"));
}

#[test]
fn multi_file_json_emits_array() {
    let out = Command::cargo_bin("codefold")
        .unwrap()
        .args([
            fixture("python/auth.py").to_str().unwrap(),
            fixture("typescript/auth.ts").to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let arr = v.as_array().expect("multi-file json should be an array");
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["language"], "python");
    assert_eq!(arr[1]["language"], "typescript");
}

#[test]
fn json_failure_for_unknown_extension_still_emits_structure() {
    let tmp = std::env::temp_dir().join("codefold_cli_json_bad.xyz");
    std::fs::write(&tmp, "x").unwrap();
    let out = Command::cargo_bin("codefold")
        .unwrap()
        .args([tmp.to_str().unwrap(), "--format", "json"])
        .output()
        .unwrap();
    assert!(!out.status.success(), "exit 1 expected on error");
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(v["error"].as_str().unwrap().contains("unsupported"));
    let _ = std::fs::remove_file(&tmp);
}
