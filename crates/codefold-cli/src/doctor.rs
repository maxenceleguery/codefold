//! `codefold doctor` — environment + integration diagnostics.
//!
//! Verifies the things that silently break otherwise: is `cargo` on PATH for
//! self-upgrade; is the GitHub release API reachable; are the agent-harness
//! integration files present and up-to-date.

use std::path::PathBuf;
use std::process::{Command, ExitCode};
use std::time::Duration;

use clap::Args;

use crate::setup::doctor_status;

#[derive(Args, Debug)]
pub struct DoctorArgs {
    /// Where to check integration files: `project` (cwd), `user` (~/.claude), or
    /// `all` (default).
    #[arg(short, long, value_enum, default_value_t = DoctorScope::All)]
    scope: DoctorScope,
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub enum DoctorScope {
    Project,
    User,
    All,
}

pub fn run(args: &DoctorArgs, current_version: &str) -> ExitCode {
    println!("codefold doctor — diagnostics");
    println!();

    let mut any_problem = false;

    // ----- cargo ------------------------------------------------------------
    match which_cargo() {
        Some(path) => {
            let version = cargo_version().unwrap_or_else(|| "(version probe failed)".to_string());
            println!("[OK]    cargo  : {}  ({version})", path.display());
        }
        None => {
            println!("[WARN]  cargo  : not on PATH");
            println!("        → `codefold update` cannot self-upgrade without it.");
            any_problem = true;
        }
    }

    // ----- network: GitHub release API -------------------------------------
    match latest_release_tag() {
        Ok(tag) => {
            let latest = tag.trim_start_matches('v');
            if latest == current_version {
                println!("[OK]    update : on latest ({tag})");
            } else if crate::update::is_newer(latest, current_version) {
                println!("[INFO]  update : installed {current_version}, latest {tag} available");
                println!("        → run `codefold update`.");
            } else {
                // current > latest_release: user is ahead of the published
                // release (e.g., dev build, or release tag hasn't been published yet).
                println!(
                    "[OK]    update : installed {current_version}, latest GitHub release {tag} (you're ahead)"
                );
            }
        }
        Err(e) => {
            println!("[WARN]  update : could not reach GitHub release API ({e})");
        }
    }

    println!();

    // ----- integration files -----------------------------------------------
    let scopes: Vec<crate::setup::SetupScope> = match args.scope {
        DoctorScope::Project => vec![crate::setup::SetupScope::Project],
        DoctorScope::User => vec![crate::setup::SetupScope::User],
        DoctorScope::All => vec![
            crate::setup::SetupScope::Project,
            crate::setup::SetupScope::User,
        ],
    };

    for scope in scopes {
        let header = match scope {
            crate::setup::SetupScope::Project => "Integration (project scope):",
            crate::setup::SetupScope::User => "Integration (user scope):",
        };
        println!("{header}");

        let rows = doctor_status(scope, current_version);
        if rows.is_empty() {
            println!("  (no targets for this scope)");
        }
        for (path, label) in &rows {
            let tag = match label.as_str() {
                "up-to-date" => "[OK]   ",
                "absent" => "[ ]    ",
                "drifted" | "DRIFTED" => "[WARN] ",
                "unmanaged" => "[INFO] ",
                _ => "[?]    ",
            };
            println!("  {tag} {label:<11}  {}", path.display());
            if label == "drifted" || label == "DRIFTED" {
                any_problem = true;
            }
        }
        println!();
    }

    println!("Hints:");
    println!("  • absent / unmanaged in project scope → `codefold setup`");
    println!("  • absent / unmanaged in user scope    → `codefold setup --scope user`");
    println!("  • drifted                              → re-run `codefold setup` to refresh");

    if any_problem {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

// ----- helpers --------------------------------------------------------------

fn which_cargo() -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let cargo = if cfg!(windows) {
            dir.join("cargo.exe")
        } else {
            dir.join("cargo")
        };
        if cargo.is_file() {
            return Some(cargo);
        }
    }
    None
}

fn cargo_version() -> Option<String> {
    let out = Command::new("cargo").arg("--version").output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn latest_release_tag() -> Result<String, String> {
    let response =
        ureq::get("https://api.github.com/repos/maxenceleguery/codefold/releases/latest")
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
