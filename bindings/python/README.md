# codefold (Python)

[![PyPI](https://img.shields.io/pypi/v/codefold)](https://pypi.org/project/codefold/)

Python bindings for [codefold](https://github.com/maxenceleguery/codefold), a
structural code reader for LLM agents: `Read`, with zoom levels. Hand it a file
and a level, get back only the slice of the file the model needs.

Languages: Python, TypeScript (incl. TSX/JSX), Rust, Go, Markdown.

## Install

```sh
pip install codefold   # or: uv add codefold
```

Prebuilt abi3 wheels (Python 3.8+) for Linux, macOS and Windows.

## Use

```python
import codefold

r = codefold.read("src/auth.py", level="signatures")
print(r.content)
print(f"~{r.tokens_est} tokens, {len(r.symbols)} symbols, {r.language}")

# Keep `login` at full body, everything else as signatures.
r = codefold.read("src/auth.py", level="signatures", focus=["login"])

# Fold code you already have in memory (a buffer, a diff, stdin).
r = codefold.read_source(source, "typescript", level="public")
```

## Levels

| Level | What you get |
|---|---|
| `signatures` (default) | imports, constants, function/class signatures, docstring summaries; bodies elided |
| `public` | `signatures`, filtered to the public surface (`export`, `pub`, uppercase Go, no `_` prefix) |
| `bodies` | top-level and method bodies in full, nested bodies collapsed |
| `full` | the file verbatim |

## API

- `read(path, level="signatures", focus=None) -> FoldResult`
- `read_source(source, language, level="signatures", focus=None) -> FoldResult`;
  `language` is a name or extension (`"python"`/`"py"`, `"ts"`, `"tsx"`, `"rust"`, `"go"`, `"md"`)
- `FoldResult`: `content`, `symbols`, `hidden_ranges`, `language`, `tokens_est`
- `Symbol`: `name`, `kind` (`function`, `method`, `class`, `import`, `section`),
  `byte_start`, `byte_end`, `line_start`, `line_end`

Errors: `FileNotFoundError` for unreadable paths, `ValueError` for an unknown
level or language.

MIT licensed. Docs and CLI: <https://codefold.maxenceleguery.net>.
