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
}

#[derive(Clone, Copy, Debug, ValueEnum, PartialEq, Eq)]
pub enum SetupScope {
    Project,
    User,
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

    let mut changes: Vec<Change> = Vec::new();

    for h in &harnesses {
        match (h, args.scope) {
            (HarnessArg::ClaudeCode, SetupScope::Project) => {
                changes.push(plan_block_write(
                    project_root.join("CLAUDE.md"),
                    claude_md_block(version),
                ));
            }
            (HarnessArg::ClaudeCode, SetupScope::User) => match user_home.as_ref() {
                Some(home) => {
                    changes.push(plan_block_write(
                        home.join(".claude").join("CLAUDE.md"),
                        claude_md_block(version),
                    ));
                    changes.push(plan_full_write(
                        home.join(".claude")
                            .join("skills")
                            .join("codefold")
                            .join("SKILL.md"),
                        skill_md(version),
                    ));
                }
                None => {
                    eprintln!("codefold: could not resolve home directory; skipping --scope user.");
                }
            },
            (HarnessArg::Cursor, SetupScope::Project) => {
                changes.push(plan_full_write(
                    project_root
                        .join(".cursor")
                        .join("rules")
                        .join("codefold.mdc"),
                    cursor_rule(version),
                ));
            }
            (HarnessArg::Copilot, SetupScope::Project) => {
                changes.push(plan_block_write(
                    project_root.join(".github").join("copilot-instructions.md"),
                    copilot_block(version),
                ));
            }
            // Cursor / Copilot don't have a user-level config in common practice.
            (HarnessArg::Cursor, SetupScope::User) | (HarnessArg::Copilot, SetupScope::User) => {
                eprintln!(
                    "codefold: --scope user is not supported for {:?}; skipping.",
                    h
                );
            }
        }
    }

    let mut wrote = 0usize;
    let mut errors = 0usize;
    for change in &changes {
        if args.dry_run {
            println!("[dry-run] would write {}", change.path.display());
            continue;
        }
        match change.apply() {
            Ok(true) => {
                println!("wrote {}", change.path.display());
                wrote += 1;
            }
            Ok(false) => {
                println!("up to date {}", change.path.display());
            }
            Err(e) => {
                eprintln!("error {}: {e}", change.path.display());
                errors += 1;
            }
        }
    }

    if errors > 0 {
        eprintln!("codefold setup: {errors} error(s).");
        ExitCode::from(1)
    } else {
        if !args.dry_run {
            println!();
            println!("Done. Wrote {wrote} file(s). codefold is now installed in your harnesses.");
            println!("Subagents spawned by these harnesses will see the instructions.");
        }
        ExitCode::SUCCESS
    }
}

// ----- change planning -------------------------------------------------------

/// A pending file change. Either a delimited block-replace (idempotent merge
/// into an existing file) or a full-file write.
struct Change {
    path: PathBuf,
    mode: ChangeMode,
}

enum ChangeMode {
    /// Insert or replace a `<!-- codefold:start --> ... <!-- codefold:end -->`
    /// block in the file. Existing surrounding content is preserved.
    Block(String),
    /// Overwrite the file with the given content.
    Full(String),
}

fn plan_block_write(path: PathBuf, block: String) -> Change {
    Change {
        path,
        mode: ChangeMode::Block(block),
    }
}

fn plan_full_write(path: PathBuf, content: String) -> Change {
    Change {
        path,
        mode: ChangeMode::Full(content),
    }
}

impl Change {
    /// Apply the change. Returns Ok(true) if the file changed, Ok(false) if
    /// no change was needed.
    fn apply(&self) -> std::io::Result<bool> {
        match &self.mode {
            ChangeMode::Block(block) => apply_block(&self.path, block),
            ChangeMode::Full(content) => apply_full(&self.path, content),
        }
    }
}

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
        };
        run(&args, "test");
        assert!(!tmp.path().join("CLAUDE.md").exists());
        assert!(!tmp.path().join(".cursor").exists());
        assert!(!tmp.path().join(".github").exists());
    }
}
