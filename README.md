# codefold

> `Read`, with zoom levels.

A structural code reader for LLM agents. Hand it a file and a zoom level — it gives you back the slice of the file the model actually needs to see.

Supported today: **Python**, **TypeScript**, **Rust**. Rust core, optional CLI, Python and Node bindings on the roadmap.

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

CLI (Rust toolchain required):

```sh
cargo install --path crates/codefold-cli
```

Python (requires Rust toolchain at install time until binary wheels are on PyPI):

```sh
uv venv && source .venv/bin/activate
uv pip install maturin
uv run maturin develop --release --manifest-path bindings/python/Cargo.toml
```

Node binding: coming.

## Use

```sh
codefold src/auth.py --level signatures
codefold src/auth.py --level bodies --focus login,verify_token
codefold src/handlers.ts --level signatures --stats
```

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

Early. v0.4.0. Python, TypeScript, Rust. API is not yet stable.

### Changelog

- **0.4.0** — Python bindings via PyO3 + maturin (`import codefold`). Pinned CI clippy to a known-good toolchain.
- **0.3.0** — Rust language support (`.rs`). `pub` filter at Public level; trait-impl methods kept regardless of `pub`. GitHub Actions CI on Linux/macOS/Windows.
- **0.2.0** — `Public` level (Python `_`-prefix filter, TypeScript `export`/`private` filter). `Level` enum marked `#[non_exhaustive]`.
- **0.1.0** — Initial release. Python and TypeScript, `Full` / `Signatures` / `Bodies` levels, `focus=[...]`, token estimation, CLI, criterion benchmarks.

## License

[MIT](LICENSE)
