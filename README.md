# codefold

> `Read`, with zoom levels.

A structural code reader for LLM agents. Hand it a file and a zoom level — it gives you back the slice of the file the model actually needs to see.

Supported today: **Python** (`.py`/`.pyi`), **TypeScript** (`.ts`/`.tsx`), **JSX JavaScript** (`.jsx`), **Rust** (`.rs`), **Go** (`.go`). Rust core, Python wheel, Node binding, optional CLI.

## Why

LLM agents waste enormous amounts of context reading entire files when they only need the public surface. `cat`-equivalent reads are ~3-5× larger than they need to be on real-world codebases.

Numbers from this repo's test fixtures:

| File | Full | Signatures | Bodies | Saving (Signatures) |
|---|---:|---:|---:|---:|
| `auth.py` (90 LOC) | 474 tok | 320 tok | 465 tok | **−32%** |
| `heavy.py` (110 LOC, body-heavy) | 809 tok | 212 tok | 809 tok | **−74%** |
| `auth.ts` (75 LOC) | 468 tok | 324 tok | 455 tok | **−31%** |

Compression scales with the body-to-signature ratio. On real-world service files (>500 lines), expect 70-90% reductions at `signatures`.

## Install

**CLI** (Rust toolchain required):

```sh
cargo install codefold-cli
```

**Rust library**:

```sh
cargo add codefold-core
```

**Python** (prebuilt wheels for Linux/macOS/Windows):

```sh
pip install codefold
# or with uv:
uv add codefold
```

**Node.js** (prebuilt binaries for Linux/macOS/Windows):

```sh
npm install @maxenceleguery/codefold
```

```js
import { read } from "@maxenceleguery/codefold";
const r = read("src/auth.py", "signatures");
```

The bare `codefold` name on npm was blocked as too similar to an existing
`code-fold`; the package lives under the maintainer's user scope.

## Use

```sh
codefold src/auth.py --level signatures
codefold src/auth.py --level bodies --focus login,verify_token
codefold src/handlers.ts --level signatures --stats

# Multiple files at once (each prefixed with === path ===)
codefold src/auth.py src/handlers.ts --level public

# JSON output for programmatic consumers
codefold src/auth.py --format json --level signatures
```

### Subcommands

```sh
codefold update                  # check + interactively upgrade via `cargo install codefold-cli --force`
codefold update --check          # check only; don't upgrade
codefold update --yes            # upgrade without prompting (for scripts / CI)
codefold setup                   # install integration into agent harnesses (project scope)
codefold setup --scope user      # install user-level (~/.claude/CLAUDE.md + skill)
codefold setup --dry-run         # show what would change without writing
codefold setup -H claude-code    # target a specific harness (claude-code|cursor|copilot)
```

`update` requires `cargo` on PATH (it self-upgrades via crates.io). If you installed via `pip` or `npm`, those need to be updated separately — the CLI will print the right command if cargo isn't found.

`codefold setup` writes a delimited `<!-- codefold:start --> ... <!-- codefold:end -->` block to:

| Harness        | Project scope                          | User scope                                                  |
|----------------|----------------------------------------|-------------------------------------------------------------|
| Claude Code    | `./CLAUDE.md`                          | `~/.claude/CLAUDE.md` + `~/.claude/skills/codefold/SKILL.md`|
| Cursor         | `.cursor/rules/codefold.mdc`           | (n/a)                                                       |
| Copilot        | `.github/copilot-instructions.md`      | (n/a)                                                       |

The block is idempotent — re-running `setup` updates in place. It explicitly tells the agent to **brief any subagents** it spawns about codefold, since subagents don't inherit conversation context.

As a Rust library:

```rust
use codefold_core::{read, read_opts, Level, Options};

// Quick read
let r = read("src/auth.py".as_ref(), Level::Signatures)?;
println!("{}", r.content);
println!("≈{} tokens, {} symbols", r.tokens_est, r.symbols.len());

// With focus: keep `login` and `verify_token` at full body, the rest as signatures.
let opts = Options::new(Level::Signatures).focus(["login", "verify_token"]);
let r = read_opts("src/auth.py".as_ref(), opts)?;
```

As a Python library:

```python
import codefold

r = codefold.read("src/auth.py", level="signatures")
print(r.content)
print(f"~{r.tokens_est} tokens, {len(r.symbols)} symbols, {r.language}")

# With focus
r = codefold.read("src/auth.py", level="signatures", focus=["login", "verify_token"])
```

## Levels

| Level | What you get |
|---|---|
| `full` | The file verbatim. For API symmetry. |
| `signatures` | Imports, top-level constants, function/class signatures, docstring summaries. Bodies replaced with `...`. |
| `public` | Like `signatures`, but additionally filters out non-public symbols (Python: names starting with `_`; TypeScript: declarations not wrapped in `export` and methods marked `private`/`protected`). |
| `bodies` | Top-level and class-method bodies in full. Functions defined *inside* those bodies have their bodies collapsed to `...`. |

`--focus name1,name2,...` elevates the named symbols to `bodies` regardless of base level. A class name in focus expands to "every method of that class".

## Positioning

The agent-tooling space is busy. codefold's niche:

- **vs [skim](https://github.com/dean0x/skim)** — skim is shell middleware: it rewrites your commands and compresses their output. codefold is a primitive: a stateless library you `import` from inside your agent framework. Different distribution shape, different integration point.
- **vs [codebase-memory-mcp](https://github.com/DeusData/codebase-memory-mcp)** — codebase-memory builds a persistent SQLite knowledge graph of a whole repo, queried over MCP. codefold answers "give me *this one file* at level X" with no indexing, no server, no state.

If you're building an agent framework or a code-aware tool and you need granular file reads, you want codefold. If you want a turnkey CLI for your shell or a whole-repo retrieval layer, look at skim or codebase-memory.

## Status

Early. v0.8.1. Python, TypeScript, Rust, Go. API is not yet stable.

### Changelog

- **0.8.3** — CI fix: switch publish job from `npm ci` to `npm install --omit=optional` since `npm ci` enforces lock-vs-package.json sync even for self-referential optional deps that aren't on the registry yet at release time.
- **0.8.2** — CI fix: `npm ci --omit=optional` so the publish job doesn't choke on out-of-sync lock entries for our own (not-yet-published) scoped sub-packages. (Insufficient — superseded by 0.8.3.)
- **0.8.1** — CI fixes: upgrade npm on the release runner to >=11.5 (needed for Trusted Publishing OIDC); make the setup test platform-agnostic on Windows.
- **0.8.0** — `codefold update` (self-upgrade via cargo), `codefold setup` (install integration into Claude Code / Cursor / Copilot — project or user scope, idempotent block-replace, propagates to subagents). npm publish migrated to Trusted Publishing (OIDC).
- **0.7.0** — npm publish pipeline. `codefold` on npm with prebuilt binaries for Linux x86_64/aarch64, macOS x86_64/arm64, Windows x64. Uses napi-rs's per-platform sub-package pattern with provenance.
- **0.6.0** — Publishing pipeline: `codefold-core` and `codefold-cli` to crates.io, `codefold` (Python wheel) to PyPI via Trusted Publishing on tag pushes. Node.js binding (`@codefold/node`) scaffolded with napi-rs; npm publishing pipeline arrives in v0.7.0. MSRV bumped to 1.77 (napi-rs requirement).
- **0.5.1** — Fix Windows CI: the Go newline regression test asserted on `\n` directly, which broke when Windows checked out the fixture as CRLF. Switched to `.lines()` and added `.gitattributes` forcing LF.
- **0.5.0** — Go language support (`.go`). Public = uppercase-first identifier. Fixed gap rendering for grammars (like Go) that expose statement terminators as anonymous siblings.
- **0.4.0** — Python bindings via PyO3 + maturin (`import codefold`). Pinned CI clippy to a known-good toolchain.
- **0.3.0** — Rust language support (`.rs`). `pub` filter at Public level; trait-impl methods kept regardless of `pub`. GitHub Actions CI on Linux/macOS/Windows.
- **0.2.0** — `Public` level (Python `_`-prefix filter, TypeScript `export`/`private` filter). `Level` enum marked `#[non_exhaustive]`.
- **0.1.0** — Initial release. Python and TypeScript, `Full` / `Signatures` / `Bodies` levels, `focus=[...]`, token estimation, CLI, criterion benchmarks.

## Contributing

Branch off `develop`, open a PR against `develop`. Releases are tagged on `main`. See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

[MIT](LICENSE)
