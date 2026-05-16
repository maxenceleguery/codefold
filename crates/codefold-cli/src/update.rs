//! `codefold update` — check GitHub releases and (optionally) apply the upgrade.

use std::io::{IsTerminal, Write};
use std::process::{Command, ExitCode};
use std::time::Duration;

use clap::Args;

const RELEASES_URL: &str = "https://api.github.com/repos/maxenceleguery/codefold/releases/latest";

#[derive(Args, Debug)]
pub struct UpdateArgs {
    /// Only check for a newer release; do not run the upgrade.
    #[arg(long)]
    pub check: bool,

    /// Apply the upgrade without prompting (non-interactive). Implies running
    /// `cargo install codefold-cli --force`.
    #[arg(short, long)]
    pub yes: bool,
}

/// Exit code emitted by `codefold update --check` when a newer release is
/// available, so scripts can distinguish "outdated" from "error" (exit 1) and
/// "up to date" (exit 0). Picked from the freedesktop convention of "non-fatal
/// but actionable" codes.
pub const EXIT_UPDATE_AVAILABLE: u8 = 10;

pub fn run(args: &UpdateArgs, current: &str) -> ExitCode {
    let release = match latest_release() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("codefold: update check failed: {e}");
            return ExitCode::from(1);
        }
    };

    let latest_clean = release.tag.trim_start_matches('v');
    if !is_newer(latest_clean, current) {
        println!("codefold {current} is up to date.");
        return ExitCode::SUCCESS;
    }

    println!("codefold {current} installed.");
    println!("→ {} is available.", release.tag);
    print_release_notes(&release.body);

    if args.check {
        println!("Run `codefold update` (without --check) to upgrade.");
        return ExitCode::from(EXIT_UPDATE_AVAILABLE);
    }

    let should_apply = args.yes || prompt_yes_no(true);
    if !should_apply {
        println!("Skipping upgrade. To upgrade later, run: codefold update");
        return ExitCode::SUCCESS;
    }

    apply_via_cargo()
}

fn print_release_notes(body: &str) {
    let body = body.trim();
    if body.is_empty() {
        return;
    }
    const MAX_LINES: usize = 15;
    let lines: Vec<&str> = body.lines().collect();
    println!();
    println!("--- release notes ---");
    for line in lines.iter().take(MAX_LINES) {
        println!("{line}");
    }
    if lines.len() > MAX_LINES {
        println!("(... {} more lines on GitHub)", lines.len() - MAX_LINES);
    }
    println!();
}

fn prompt_yes_no(default_yes: bool) -> bool {
    if !std::io::stdin().is_terminal() {
        // Non-interactive (script, CI): never auto-apply without --yes.
        eprintln!("(non-interactive; pass --yes to apply, --check to only check)");
        return false;
    }
    let suffix = if default_yes { "[Y/n]" } else { "[y/N]" };
    print!("Upgrade now? {suffix} ");
    let _ = std::io::stdout().flush();
    let mut buf = String::new();
    if std::io::stdin().read_line(&mut buf).is_err() {
        return false;
    }
    let answer = buf.trim().to_lowercase();
    if answer.is_empty() {
        return default_yes;
    }
    matches!(answer.as_str(), "y" | "yes")
}

fn apply_via_cargo() -> ExitCode {
    // Locate cargo on PATH. If absent, the user installed via pip/npm/system
    // package — we can't safely auto-upgrade that, so print manual commands.
    if which_cargo().is_none() {
        println!();
        println!("`cargo` not found on PATH. codefold may have been installed via a different");
        println!("package manager. Pick the matching command:");
        println!("  cargo install codefold-cli --force");
        println!("  pip install --upgrade codefold");
        println!("  npm i -g @maxenceleguery/codefold@latest");
        return ExitCode::from(1);
    }

    println!();
    println!("$ cargo install codefold-cli --force");
    let status = Command::new("cargo")
        .args(["install", "codefold-cli", "--force"])
        .status();
    match status {
        Ok(s) if s.success() => {
            println!();
            println!("codefold upgraded successfully.");
            ExitCode::SUCCESS
        }
        Ok(s) => {
            eprintln!("codefold: `cargo install` exited with status {s}");
            ExitCode::from(1)
        }
        Err(e) => {
            eprintln!("codefold: failed to spawn cargo: {e}");
            ExitCode::from(1)
        }
    }
}

fn which_cargo() -> Option<()> {
    // Minimal PATH walk: avoid pulling in the `which` crate for one binary.
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let cargo = if cfg!(windows) {
            dir.join("cargo.exe")
        } else {
            dir.join("cargo")
        };
        if cargo.is_file() {
            return Some(());
        }
    }
    None
}

struct Release {
    tag: String,
    body: String,
}

fn latest_release() -> Result<Release, String> {
    let response = ureq::get(RELEASES_URL)
        .set(
            "User-Agent",
            &format!("codefold/{}", env!("CARGO_PKG_VERSION")),
        )
        .set("Accept", "application/vnd.github+json")
        .timeout(Duration::from_secs(5))
        .call()
        .map_err(|e| e.to_string())?;

    let json: serde_json::Value = response.into_json().map_err(|e| e.to_string())?;
    let tag = json
        .get("tag_name")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "missing `tag_name` in release response".to_string())?;
    let body = json
        .get("body")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    Ok(Release { tag, body })
}

/// Strict semver-ish comparison: returns true if `a` is strictly greater than `b`.
/// Both inputs must be `MAJOR.MINOR.PATCH` (no pre-release / build metadata).
fn is_newer(a: &str, b: &str) -> bool {
    fn parts(v: &str) -> Option<(u64, u64, u64)> {
        let mut iter = v.split('.');
        let major = iter.next()?.parse().ok()?;
        let minor = iter.next()?.parse().ok()?;
        let patch = iter.next()?.parse().ok()?;
        Some((major, minor, patch))
    }
    match (parts(a), parts(b)) {
        (Some(pa), Some(pb)) => pa > pb,
        _ => a > b,
    }
}

#[cfg(test)]
mod tests {
    use super::is_newer;

    #[test]
    fn newer_minor_bump() {
        assert!(is_newer("0.8.0", "0.7.0"));
    }

    #[test]
    fn newer_patch_bump() {
        assert!(is_newer("0.7.1", "0.7.0"));
    }

    #[test]
    fn same_is_not_newer() {
        assert!(!is_newer("0.7.0", "0.7.0"));
    }

    #[test]
    fn older_is_not_newer() {
        assert!(!is_newer("0.7.0", "0.7.1"));
    }

    #[test]
    fn major_dominates_minor() {
        assert!(is_newer("1.0.0", "0.99.99"));
    }
}
