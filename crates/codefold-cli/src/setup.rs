//! `codefold setup` — install codefold integration into LLM agent harnesses
//! so that EVERY agent (including subagents) knows to use the tool.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Args, ValueEnum};

#[derive(Args, Debug)]
pub struct SetupArgs {
    /// Where to install:
    ///   - `project` writes to ./CLAUDE.md, .cursor/rules/codefold.mdc, .github/copilot-instructions.md
    ///   - `user` writes to ~/.claude/CLAUDE.md and ~/.claude/skills/codefold/SKILL.md
    #[arg(short, long, value_enum, default_value_t = SetupScope::Project)]
    scope: SetupScope,

    /// Which harnesses to target. Defaults to all known.
    #[arg(short = 'H', long, value_enum)]
    harness: Vec<HarnessArg>,

    /// Project directory (defaults to current working directory). Only used
    /// with --scope project.
    #[arg(long)]
    project_dir: Option<PathBuf>,

    /// Print what would change without writing anything.
    #[arg(long)]
    dry_run: bool,

    /// Show which managed files exist (per harness × scope) and whether they
    /// are up-to-date with the current codefold version. Exits without writing.
    #[arg(long, conflicts_with_all = ["uninstall", "dry_run"])]
    list: bool,

    /// Remove codefold integration. For files codefold appends a block to
    /// (CLAUDE.md, copilot-instructions.md), the block is stripped. For files
    /// codefold fully owns (cursor rule, SKILL.md), the file is deleted.
    #[arg(long, conflicts_with = "list")]
    uninstall: bool,
}

#[derive(Clone, Copy, Debug, ValueEnum, PartialEq, Eq)]
pub enum SetupScope {
    Project,
    User,
}

/// Diagnostic helper for `codefold doctor`. Returns `(path, label)` for every
/// target the user might have installed at the given scope, with labels:
/// `up-to-date`, `absent`, `drifted`, `unmanaged`. Silently skips harnesses
/// that don't apply to the scope (no leaked warnings).
pub fn doctor_status(scope: SetupScope, version: &str) -> Vec<(PathBuf, String)> {
    let args = SetupArgs {
        scope,
        harness: Vec::new(),
        project_dir: None,
        dry_run: false,
        list: false,
        uninstall: false,
    };
    let (targets, _warnings) = plan_targets_quiet(&args);
    targets
        .into_iter()
        .map(|t| {
            let s = check_status(&t, version);
            let label = match s {
                Status::Absent => "absent",
                Status::UpToDate => "up-to-date",
                Status::Drifted => "drifted",
                Status::Unmanaged => "unmanaged",
            };
            (t.path, label.to_string())
        })
        .collect()
}

#[derive(Clone, Copy, Debug, ValueEnum, PartialEq, Eq)]
pub enum HarnessArg {
    ClaudeCode,
    Cursor,
    Copilot,
}

const MARKER_START: &str = "<!-- codefold:start -->";
const MARKER_END: &str = "<!-- codefold:end -->";

pub fn run(args: &SetupArgs, version: &str) -> ExitCode {
    let targets = plan_targets(args);
    if args.list {
        return list_targets(&targets, version);
    }
    if args.uninstall {
        return uninstall_targets(&targets, args.dry_run);
    }
    install_targets(&targets, args.dry_run, version)
}

/// A planning unit: where to act and how. Content templates are looked up by
/// `(mode, file basename)` at apply time so the same `Target` set is reusable
/// across install / list / uninstall.
struct Target {
    path: PathBuf,
    mode: TargetMode,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TargetMode {
    /// File is co-owned: we manage a `<!-- codefold:start --> ... <!-- codefold:end -->` block.
    Block,
    /// File is fully owned: we write the entire file.
    Full,
}

fn plan_targets(args: &SetupArgs) -> Vec<Target> {
    let (targets, warnings) = plan_targets_quiet(args);
    for w in warnings {
        eprintln!("{w}");
    }
    targets
}

/// Same as `plan_targets` but collects warnings (e.g. "--scope user not
/// supported for Cursor") rather than printing them, so callers like `doctor`
/// can decide whether to surface them.
fn plan_targets_quiet(args: &SetupArgs) -> (Vec<Target>, Vec<String>) {
    let harnesses: Vec<HarnessArg> = if args.harness.is_empty() {
        vec![
            HarnessArg::ClaudeCode,
            HarnessArg::Cursor,
            HarnessArg::Copilot,
        ]
    } else {
        args.harness.clone()
    };

    let project_root = args
        .project_dir
        .clone()
        .unwrap_or_else(|| std::env::current_dir().expect("cwd"));
    let user_home = dirs::home_dir();

    let mut targets = Vec::new();
    let mut warnings = Vec::new();
    for h in &harnesses {
        match (h, args.scope) {
            (HarnessArg::ClaudeCode, SetupScope::Project) => {
                targets.push(Target {
                    path: project_root.join("CLAUDE.md"),
                    mode: TargetMode::Block,
                });
            }
            (HarnessArg::ClaudeCode, SetupScope::User) => {
                if let Some(home) = user_home.as_ref() {
                    targets.push(Target {
                        path: home.join(".claude").join("CLAUDE.md"),
                        mode: TargetMode::Block,
                    });
                    targets.push(Target {
                        path: home
                            .join(".claude")
                            .join("skills")
                            .join("codefold")
                            .join("SKILL.md"),
                        mode: TargetMode::Full,
                    });
                } else {
                    warnings.push(
                        "codefold: could not resolve home directory; skipping --scope user."
                            .to_string(),
                    );
                }
            }
            (HarnessArg::Cursor, SetupScope::Project) => {
                targets.push(Target {
                    path: project_root
                        .join(".cursor")
                        .join("rules")
                        .join("codefold.mdc"),
                    mode: TargetMode::Full,
                });
            }
            (HarnessArg::Copilot, SetupScope::Project) => {
                targets.push(Target {
                    path: project_root.join(".github").join("copilot-instructions.md"),
                    mode: TargetMode::Block,
                });
            }
            (HarnessArg::Cursor, SetupScope::User) | (HarnessArg::Copilot, SetupScope::User) => {
                warnings.push(format!(
                    "codefold: --scope user is not supported for {:?}; skipping.",
                    h
                ));
            }
        }
    }
    (targets, warnings)
}

fn content_for(target: &Target, version: &str) -> String {
    let name = target
        .path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    match (target.mode, name) {
        (TargetMode::Block, "CLAUDE.md") => claude_md_block(version),
        (TargetMode::Block, "copilot-instructions.md") => copilot_block(version),
        (TargetMode::Full, "codefold.mdc") => cursor_rule(version),
        (TargetMode::Full, "SKILL.md") => skill_md(version),
        _ => String::new(),
    }
}

fn install_targets(targets: &[Target], dry_run: bool, version: &str) -> ExitCode {
    let mut wrote = 0usize;
    let mut errors = 0usize;
    for t in targets {
        let content = content_for(t, version);
        if dry_run {
            println!("[dry-run] would write {}", t.path.display());
            continue;
        }
        let result = match t.mode {
            TargetMode::Block => apply_block(&t.path, &content),
            TargetMode::Full => apply_full(&t.path, &content),
        };
        match result {
            Ok(true) => {
                println!("wrote {}", t.path.display());
                wrote += 1;
            }
            Ok(false) => println!("up to date {}", t.path.display()),
            Err(e) => {
                eprintln!("error {}: {e}", t.path.display());
                errors += 1;
            }
        }
    }
    if errors > 0 {
        eprintln!("codefold setup: {errors} error(s).");
        return ExitCode::from(1);
    }
    if !dry_run {
        println!();
        println!("Done. Wrote {wrote} file(s). codefold is now installed in your harnesses.");
        println!("Subagents spawned by these harnesses will see the instructions.");
    }
    ExitCode::SUCCESS
}

#[derive(Debug, PartialEq, Eq)]
enum Status {
    Absent,
    UpToDate,
    Drifted,
    Unmanaged,
}

fn check_status(target: &Target, version: &str) -> Status {
    let existing = match fs::read_to_string(&target.path) {
        Ok(s) => s,
        Err(_) => return Status::Absent,
    };
    let expected = content_for(target, version);
    match target.mode {
        TargetMode::Full => {
            if existing == expected {
                Status::UpToDate
            } else {
                // We own the file; differing content means an older version installed it.
                Status::Drifted
            }
        }
        TargetMode::Block => {
            let block = extract_codefold_block(&existing);
            match block {
                None => Status::Unmanaged,
                Some(b) if b.trim_end() == expected.trim_end() => Status::UpToDate,
                Some(_) => Status::Drifted,
            }
        }
    }
}

fn extract_codefold_block(text: &str) -> Option<&str> {
    let start = text.find(MARKER_START)?;
    let end = text.find(MARKER_END)?;
    if end <= start {
        return None;
    }
    Some(&text[start..end + MARKER_END.len()])
}

fn list_targets(targets: &[Target], version: &str) -> ExitCode {
    if targets.is_empty() {
        println!("(no targets matched the requested scope × harness combination)");
        return ExitCode::SUCCESS;
    }
    println!("{:<11}  {:<6}  path", "status", "mode",);
    println!("{}", "-".repeat(60));
    for t in targets {
        let status = check_status(t, version);
        let mode = match t.mode {
            TargetMode::Block => "block",
            TargetMode::Full => "full",
        };
        let label = match status {
            Status::Absent => "absent",
            Status::UpToDate => "up-to-date",
            Status::Drifted => "DRIFTED",
            Status::Unmanaged => "unmanaged",
        };
        println!("{:<11}  {:<6}  {}", label, mode, t.path.display());
    }
    ExitCode::SUCCESS
}

fn uninstall_targets(targets: &[Target], dry_run: bool) -> ExitCode {
    let mut removed = 0usize;
    let mut errors = 0usize;
    for t in targets {
        if dry_run {
            println!("[dry-run] would uninstall {}", t.path.display());
            continue;
        }
        let result: std::io::Result<bool> = match t.mode {
            TargetMode::Block => strip_block_from_file(&t.path),
            TargetMode::Full => delete_file_if_exists(&t.path),
        };
        match result {
            Ok(true) => {
                println!("removed {}", t.path.display());
                removed += 1;
            }
            Ok(false) => println!("not present {}", t.path.display()),
            Err(e) => {
                eprintln!("error {}: {e}", t.path.display());
                errors += 1;
            }
        }
    }
    if errors > 0 {
        eprintln!("codefold setup --uninstall: {errors} error(s).");
        return ExitCode::from(1);
    }
    if !dry_run {
        println!();
        println!("Done. Removed {removed} file(s) / block(s).");
    }
    ExitCode::SUCCESS
}

/// Remove the codefold block from a Block-mode file. Returns Ok(true) if the
/// file changed, Ok(false) if no block was found.
fn strip_block_from_file(path: &Path) -> std::io::Result<bool> {
    let Ok(existing) = fs::read_to_string(path) else {
        return Ok(false);
    };
    let (Some(start), Some(end)) = (existing.find(MARKER_START), existing.find(MARKER_END)) else {
        return Ok(false);
    };
    let end_full = end + MARKER_END.len();
    if end_full <= start {
        return Ok(false);
    }
    let mut out = String::with_capacity(existing.len());
    // Drop the block and a single trailing newline if present, and trim the
    // blank line that typically precedes our block.
    let before = existing[..start].trim_end_matches('\n');
    let before = before.trim_end_matches('\n');
    out.push_str(before);
    let after = &existing[end_full..];
    let after = after.strip_prefix('\n').unwrap_or(after);
    if !after.trim().is_empty() {
        out.push_str("\n\n");
        out.push_str(after.trim_start_matches('\n'));
    } else {
        out.push('\n');
    }
    if out == existing {
        return Ok(false);
    }
    // If the file is now empty (or only whitespace) and we own the parent dir,
    // delete the file outright so we don't leave a stub.
    if out.trim().is_empty() {
        fs::remove_file(path)?;
        return Ok(true);
    }
    fs::write(path, out)?;
    Ok(true)
}

fn delete_file_if_exists(path: &Path) -> std::io::Result<bool> {
    if !path.exists() {
        return Ok(false);
    }
    fs::remove_file(path)?;
    Ok(true)
}

// ----- low-level file ops ----------------------------------------------------

fn apply_block(path: &Path, block: &str) -> std::io::Result<bool> {
    let existing = fs::read_to_string(path).ok();
    let mut new_content = match &existing {
        Some(text) => merge_block(text, block),
        None => format!("{block}\n"),
    };
    if !new_content.ends_with('\n') {
        new_content.push('\n');
    }
    if existing.as_deref() == Some(new_content.as_str()) {
        return Ok(false);
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut f = fs::File::create(path)?;
    f.write_all(new_content.as_bytes())?;
    Ok(true)
}

fn apply_full(path: &Path, content: &str) -> std::io::Result<bool> {
    let existing = fs::read_to_string(path).ok();
    if existing.as_deref() == Some(content) {
        return Ok(false);
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut f = fs::File::create(path)?;
    f.write_all(content.as_bytes())?;
    Ok(true)
}

/// Merge a codefold block into existing text:
///   - if a codefold:start..codefold:end block exists, replace it
///   - else append the block (with a separating blank line)
fn merge_block(existing: &str, new_block: &str) -> String {
    if let (Some(start), Some(end)) = (existing.find(MARKER_START), existing.find(MARKER_END)) {
        let end_full = end + MARKER_END.len();
        if end_full > start {
            let mut out = String::with_capacity(existing.len() + new_block.len());
            out.push_str(&existing[..start]);
            out.push_str(new_block);
            // Skip the rest of the old block's line if it has a trailing newline
            let rest = &existing[end_full..];
            let rest = rest.strip_prefix('\n').unwrap_or(rest);
            out.push_str(rest);
            return out;
        }
    }
    let trimmed = existing.trim_end();
    if trimmed.is_empty() {
        return format!("{new_block}\n");
    }
    format!("{trimmed}\n\n{new_block}\n")
}

// ----- content templates -----------------------------------------------------

fn claude_md_block(version: &str) -> String {
    const TEMPLATE: &str = r#"@MARKER_START@
## codefold

When reading source files (especially > 200 lines), prefer the `codefold` CLI
over a raw file-read. It returns a chosen slice — signatures, public surface,
top-level bodies, or full — instead of dumping the whole file into context.
Languages: Python, TypeScript, Rust, Go.

```sh
codefold src/auth.py --level signatures
codefold src/auth.py --level public
codefold src/auth.py --level bodies --focus login,verify_token
```

**Important — propagation to subagents:** if you spawn subagents (Explore,
Plan, code-reviewer, etc.), they do NOT inherit your conversation context.
Brief them explicitly that codefold is available and they should prefer it
over raw file reads. Mention the CLI name and the four levels.

Installed by `codefold setup` v@VERSION@.
@MARKER_END@"#;
    TEMPLATE
        .replace("@MARKER_START@", MARKER_START)
        .replace("@MARKER_END@", MARKER_END)
        .replace("@VERSION@", version)
}

fn skill_md(version: &str) -> String {
    const TEMPLATE: &str = r#"---
name: codefold
description: Use the codefold CLI to read source files at a chosen zoom level (signatures, public, bodies, full) instead of dumping the whole file. Saves 30-90% of read tokens on typical files. Languages: Python, TypeScript, Rust, Go.
---

# codefold

`codefold` is a structural code reader. Use it when you want to look at a source
file without paying full-file token cost.

## When to use

- Any source file > ~200 lines, especially when you only need an overview.
- Before deciding which symbol to focus on.
- When you spawn a subagent that will read files — pass them this instruction.

## CLI

```sh
codefold <path> --level <full|signatures|public|bodies> [--focus name1,name2,...] [--stats]
```

### Levels

| Level        | What you get |
|--------------|--------------|
| `full`       | The file verbatim. For API symmetry. |
| `signatures` | Imports, top-level constants, function/class signatures, docstring summaries. Bodies replaced with `...`. |
| `public`     | Like `signatures`, additionally hides non-public symbols (Python `_`-prefix, TypeScript non-`export`/`private`, Rust non-`pub`, Go lowercase-first). |
| `bodies`     | Top-level + class-method bodies kept in full; nested function bodies collapsed. |

### Focus

`--focus login,verify_token` elevates those symbols to full-body even at lower
levels. A class name in focus expands to "all methods of that class".

### Examples

```sh
codefold src/auth.py --level signatures
codefold src/auth.py --level public --stats
codefold src/handlers.ts --level bodies --focus login
codefold src/main.rs --level public
```

## In Python / Rust / Node

```python
import codefold
r = codefold.read("src/auth.py", level="signatures", focus=["login"])
print(r.content)
```

```rust
use codefold_core::{read_opts, Level, Options};
let opts = Options::new(Level::Signatures).focus(["login"]);
let r = read_opts("src/auth.py".as_ref(), opts)?;
```

```js
import { read } from "@maxenceleguery/codefold";
const r = read("src/auth.py", "signatures", ["login"]);
```

## Pass it down to subagents

Subagents do not see this skill automatically unless you brief them. When
spawning an Explore / Plan / Reviewer agent, include in the prompt:

> "Use `codefold <path> --level signatures` instead of raw Read for any file
> over ~200 lines. See the `codefold` skill for the four levels and `--focus`."

Installed by `codefold setup` v@VERSION@.
"#;
    TEMPLATE.replace("@VERSION@", version)
}

fn cursor_rule(version: &str) -> String {
    const TEMPLATE: &str = r#"---
description: Prefer the codefold CLI for structural code reads
alwaysApply: true
---

When reading source files (Python, TypeScript, Rust, Go) over ~200 lines,
prefer `codefold <path> --level signatures` (or `public` / `bodies`) over a
raw cat/Read. It returns a chosen slice of the file instead of the whole file
and saves 30-90% of tokens.

Levels: `full`, `signatures`, `public`, `bodies`. `--focus name1,name2`
elevates specific symbols to full body even at lower levels.

Installed by `codefold setup` v@VERSION@.
"#;
    TEMPLATE.replace("@VERSION@", version)
}

fn copilot_block(version: &str) -> String {
    const TEMPLATE: &str = r#"@MARKER_START@
## codefold (LLM-friendly file reader)

Prefer the `codefold` CLI for reading source files (Python, TypeScript, Rust,
Go) over a raw read. It returns the file at a chosen zoom level instead of
dumping the whole file.

```sh
codefold src/auth.py --level signatures
codefold src/auth.py --level public
codefold src/auth.py --level bodies --focus login,verify_token
```

Levels: `full`, `signatures`, `public`, `bodies`. `--focus name1,name2`
elevates specific symbols to full body. Saves 30-90% of read tokens on real
files.

Installed by `codefold setup` v@VERSION@.
@MARKER_END@"#;
    TEMPLATE
        .replace("@MARKER_START@", MARKER_START)
        .replace("@MARKER_END@", MARKER_END)
        .replace("@VERSION@", version)
}

// ----- tests -----------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn read(path: &Path) -> String {
        fs::read_to_string(path).unwrap()
    }

    #[test]
    fn project_scope_writes_claude_md() {
        let tmp = TempDir::new().unwrap();
        let args = SetupArgs {
            scope: SetupScope::Project,
            harness: vec![HarnessArg::ClaudeCode],
            project_dir: Some(tmp.path().to_path_buf()),
            dry_run: false,
            list: false,
            uninstall: false,
        };
        // ExitCode's Debug format is platform-specific (unix_exit_status vs
        // windows_exit_status); we just assert on the side-effect.
        let _ = run(&args, "test");

        let claude_md = tmp.path().join("CLAUDE.md");
        let content = read(&claude_md);
        assert!(content.contains(MARKER_START));
        assert!(content.contains(MARKER_END));
        assert!(content.contains("codefold"));
        assert!(content.contains("subagents"));
    }

    #[test]
    fn block_write_is_idempotent() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("CLAUDE.md");
        fs::write(&path, "# Existing content\n\nSome notes.\n").unwrap();

        let args = SetupArgs {
            scope: SetupScope::Project,
            harness: vec![HarnessArg::ClaudeCode],
            project_dir: Some(tmp.path().to_path_buf()),
            dry_run: false,
            list: false,
            uninstall: false,
        };
        run(&args, "test");
        let after_first = read(&path);
        assert!(after_first.starts_with("# Existing content"));
        assert!(after_first.contains(MARKER_START));

        // Running again should NOT duplicate the block.
        run(&args, "test");
        let after_second = read(&path);
        assert_eq!(after_first, after_second);
        // Only one block.
        assert_eq!(after_second.matches(MARKER_START).count(), 1);
        assert_eq!(after_second.matches(MARKER_END).count(), 1);
    }

    #[test]
    fn block_write_replaces_existing_block() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("CLAUDE.md");
        // Existing file with an OLDER codefold block.
        let initial =
            format!("# Header\n\n{MARKER_START}\nold codefold v0.1\n{MARKER_END}\n\n## After\n");
        fs::write(&path, &initial).unwrap();

        let args = SetupArgs {
            scope: SetupScope::Project,
            harness: vec![HarnessArg::ClaudeCode],
            project_dir: Some(tmp.path().to_path_buf()),
            dry_run: false,
            list: false,
            uninstall: false,
        };
        run(&args, "new-version");

        let content = read(&path);
        assert!(!content.contains("old codefold v0.1"));
        assert!(content.contains("new-version"));
        assert!(content.contains("## After"));
        assert_eq!(content.matches(MARKER_START).count(), 1);
    }

    #[test]
    fn cursor_rule_full_file_write() {
        let tmp = TempDir::new().unwrap();
        let args = SetupArgs {
            scope: SetupScope::Project,
            harness: vec![HarnessArg::Cursor],
            project_dir: Some(tmp.path().to_path_buf()),
            dry_run: false,
            list: false,
            uninstall: false,
        };
        run(&args, "test");
        let rule = tmp
            .path()
            .join(".cursor")
            .join("rules")
            .join("codefold.mdc");
        let content = read(&rule);
        assert!(content.starts_with("---"));
        assert!(content.contains("alwaysApply: true"));
        assert!(content.contains("codefold"));
    }

    #[test]
    fn copilot_writes_under_github_dir() {
        let tmp = TempDir::new().unwrap();
        let args = SetupArgs {
            scope: SetupScope::Project,
            harness: vec![HarnessArg::Copilot],
            project_dir: Some(tmp.path().to_path_buf()),
            dry_run: false,
            list: false,
            uninstall: false,
        };
        run(&args, "test");
        let path = tmp.path().join(".github").join("copilot-instructions.md");
        let content = read(&path);
        assert!(content.contains(MARKER_START));
        assert!(content.contains("codefold"));
    }

    #[test]
    fn dry_run_writes_nothing() {
        let tmp = TempDir::new().unwrap();
        let args = SetupArgs {
            scope: SetupScope::Project,
            harness: vec![
                HarnessArg::ClaudeCode,
                HarnessArg::Cursor,
                HarnessArg::Copilot,
            ],
            project_dir: Some(tmp.path().to_path_buf()),
            dry_run: true,
            list: false,
            uninstall: false,
        };
        run(&args, "test");
        assert!(!tmp.path().join("CLAUDE.md").exists());
        assert!(!tmp.path().join(".cursor").exists());
        assert!(!tmp.path().join(".github").exists());
    }

    fn project_args(tmp: &TempDir) -> SetupArgs {
        SetupArgs {
            scope: SetupScope::Project,
            harness: vec![
                HarnessArg::ClaudeCode,
                HarnessArg::Cursor,
                HarnessArg::Copilot,
            ],
            project_dir: Some(tmp.path().to_path_buf()),
            dry_run: false,
            list: false,
            uninstall: false,
        }
    }

    #[test]
    fn check_status_classifies_correctly() {
        let tmp = TempDir::new().unwrap();
        let target_claude = Target {
            path: tmp.path().join("CLAUDE.md"),
            mode: TargetMode::Block,
        };
        let target_cursor = Target {
            path: tmp
                .path()
                .join(".cursor")
                .join("rules")
                .join("codefold.mdc"),
            mode: TargetMode::Full,
        };

        // Absent
        assert_eq!(check_status(&target_claude, "v"), Status::Absent);
        assert_eq!(check_status(&target_cursor, "v"), Status::Absent);

        // Unmanaged: file exists with no codefold block (Block mode only)
        fs::write(&target_claude.path, "# hand-written\n").unwrap();
        assert_eq!(check_status(&target_claude, "v"), Status::Unmanaged);

        // Up-to-date after install
        let args = project_args(&tmp);
        run(&args, "v-current");
        assert_eq!(check_status(&target_claude, "v-current"), Status::UpToDate);
        assert_eq!(check_status(&target_cursor, "v-current"), Status::UpToDate);

        // Drifted when version differs (template embeds the version)
        assert_eq!(check_status(&target_claude, "v-other"), Status::Drifted);
        assert_eq!(check_status(&target_cursor, "v-other"), Status::Drifted);
    }

    #[test]
    fn uninstall_strips_block_and_leaves_user_content() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("CLAUDE.md");
        fs::write(&path, "# Existing content\n\nSome notes.\n").unwrap();

        run(&project_args(&tmp), "v");
        let post_install = read(&path);
        assert!(post_install.contains(MARKER_START));

        let mut args = project_args(&tmp);
        args.uninstall = true;
        run(&args, "v");
        // File should still exist with the user's original content but no block.
        let post_uninstall = read(&path);
        assert!(
            post_uninstall.starts_with("# Existing content"),
            "user content should be preserved, got: {post_uninstall:?}"
        );
        assert!(!post_uninstall.contains(MARKER_START));
        assert!(!post_uninstall.contains(MARKER_END));
    }

    #[test]
    fn uninstall_deletes_full_owned_files() {
        let tmp = TempDir::new().unwrap();
        run(&project_args(&tmp), "v");
        let cursor_rule = tmp
            .path()
            .join(".cursor")
            .join("rules")
            .join("codefold.mdc");
        assert!(cursor_rule.exists());

        let mut args = project_args(&tmp);
        args.uninstall = true;
        run(&args, "v");

        assert!(
            !cursor_rule.exists(),
            "fully-owned cursor rule should be deleted on uninstall"
        );
    }

    #[test]
    fn uninstall_on_empty_state_is_a_noop() {
        let tmp = TempDir::new().unwrap();
        let mut args = project_args(&tmp);
        args.uninstall = true;
        // Should not crash on absent files.
        run(&args, "v");
        assert!(!tmp.path().join("CLAUDE.md").exists());
    }

    #[test]
    fn list_runs_without_writing() {
        let tmp = TempDir::new().unwrap();
        let mut args = project_args(&tmp);
        args.list = true;
        run(&args, "v");
        // --list must not create files.
        assert!(!tmp.path().join("CLAUDE.md").exists());
        assert!(!tmp.path().join(".cursor").exists());
    }
}
