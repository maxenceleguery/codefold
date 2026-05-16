# Contributing

## Branch model

| Branch | Purpose | Direct pushes? |
|---|---|---|
| `main` | What's deployed / published to registries. Tagged releases are cut from here. | **No** — branch-protected. Only updated by merging from `develop` via PR. |
| `develop` | Integration branch. Where features / fixes accumulate between releases. | **No** — branch-protected. Updated by merging topic branches via PR. |
| `feature/<name>` | New feature work. | yes (your own branch) |
| `fix/<name>` | Bug fix. | yes (your own branch) |

### Workflow

```
   feature/foo  ─┐
                 ├──PR──►  develop  ──PR──►  main  ──tag v0.x.y──►  registries
   fix/bar     ──┘
```

1. Branch off `develop`: `git checkout -b feature/my-thing develop`.
2. Push. Open a PR against `develop`. CI (`fmt`, `clippy`, `test` matrix, `python`) runs automatically.
3. Once green, merge into `develop`.
4. When `develop` is ready for release, open a PR `develop` → `main`. CI runs again.
5. Merge into `main`.
6. Tag the merge commit on `main` with `vX.Y.Z`. This triggers `.github/workflows/release.yml`,
   which publishes to crates.io, PyPI, and npm. See [PUBLISHING.md](PUBLISHING.md).

### Versioning

Semver (`major.minor.patch`):

- patch (`0.8.0 → 0.8.1`): bug fixes only.
- minor (`0.8.0 → 0.9.0`): new features, backward-compatible.
- major (`0.x.0 → 1.0.0`, `1.x → 2.0.0`): breaking changes.

Versions must be in lockstep across:

- `Cargo.toml` (workspace.package.version)
- `crates/codefold-cli/Cargo.toml` (codefold-core path-dep version)
- `bindings/node/Cargo.toml` (codefold-core path-dep version)
- `bindings/node/package.json` + every `bindings/node/npm/<triple>/package.json`

### Local checks before PR

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

For the Python binding:

```sh
cd bindings/python
uv venv && uv pip install maturin pytest
uv run maturin develop --release && uv run pytest -v
```

For the Node binding:

```sh
cd bindings/node
npm install
npm run build && npm test
```

### Why this layout

Solo OSS but treated like a small team: every change merges via PR with green CI.
Mistakes that would otherwise reach published artifacts get caught in `develop` first.
`main` is the single source of truth for "what's actually shipping".
