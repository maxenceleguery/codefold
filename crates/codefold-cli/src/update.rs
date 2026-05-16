//! `codefold update` — check GitHub releases for a newer version.

use std::process::ExitCode;
use std::time::Duration;

const RELEASES_URL: &str = "https://api.github.com/repos/maxenceleguery/codefold/releases/latest";

pub fn run(current: &str) -> ExitCode {
    match latest_release_tag() {
        Ok(latest) => {
            let latest_clean = latest.trim_start_matches('v');
            if is_newer(latest_clean, current) {
                println!("codefold {current} installed.");
                println!("→ {latest} is available.");
                println!();
                println!("Update with one of:");
                println!("  cargo install codefold-cli --force");
                println!("  pip install --upgrade codefold");
                println!("  npm i -g @maxenceleguery/codefold@latest");
                ExitCode::SUCCESS
            } else {
                println!("codefold {current} is up to date.");
                ExitCode::SUCCESS
            }
        }
        Err(e) => {
            eprintln!("codefold: update check failed: {e}");
            ExitCode::from(1)
        }
    }
}

fn latest_release_tag() -> Result<String, String> {
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
    json.get("tag_name")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "missing `tag_name` in release response".to_string())
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
        // Fall back to string compare so unknown shapes don't claim "newer".
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
