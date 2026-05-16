# Publishing codefold

`v*` tags trigger `.github/workflows/release.yml`, which publishes to **crates.io** and **PyPI** via Trusted Publishing — no long-lived tokens stored in GitHub Secrets.

You need to do a one-time setup on each registry before the first tag push. After that, every `git push origin vX.Y.Z` triggers the release.

## One-time setup

### 1. crates.io Trusted Publisher

Status: GA on crates.io as of late 2025.

1. Log in to <https://crates.io>.
2. Visit <https://crates.io/me/trusted-publishers/new>.
3. Add a new GitHub trusted publisher with:
   - **Repository owner:** `maxenceleguery`
   - **Repository name:** `codefold`
   - **Workflow filename:** `release.yml`
   - **Environment:** (leave blank)
4. Repeat for both crate names:
   - `codefold-core`
   - `codefold-cli`

   You may need to publish each crate manually once with `cargo publish` to claim the name (Trusted Publisher binds to an existing crate). If they don't exist yet, see "First publish" below.

### 2. PyPI Trusted Publisher

1. Log in to <https://pypi.org/manage/account/publishing/>.
2. Under "Add a new pending publisher" (for a not-yet-published project) or on the project's "Publishing" tab (if it exists), add:
   - **PyPI project name:** `codefold`
   - **Owner:** `maxenceleguery`
   - **Repository:** `codefold`
   - **Workflow:** `release.yml`
   - **Environment:** (leave blank)

PyPI Trusted Publishing also works for first-time publishing — no manual `twine upload` needed first.

### 3. (Later) npm publishing — v0.7.0

The `@codefold/node` package isn't published yet. Setup will be documented here when the v0.7.0 release workflow lands.

## First publish

If crates.io won't accept Trusted Publishing until the crate exists, do this once locally:

```sh
# Login with your crates.io API token (one-off; needed for the first publish only)
cargo login

# Publish core
cargo publish -p codefold-core

# Wait ~60s for crates.io to surface the new version
sleep 60

# Publish cli
cargo publish -p codefold-cli
```

Then configure the Trusted Publisher (step 1 above) so subsequent versions go via OIDC.

For PyPI, Trusted Publishing works first-time — just configure (step 2) before tagging.

## Cutting a release

```sh
# Bump the version in:
#   - Cargo.toml (workspace.package.version)
#   - crates/codefold-cli/Cargo.toml (codefold-core path-dep version)
#   - bindings/node/Cargo.toml         (codefold-core path-dep version)
#   - bindings/node/package.json       (top-level version)
# These must all agree.

git add -A
git commit -m "Release v0.X.Y"
git tag -a v0.X.Y -m "v0.X.Y"
git push && git push origin v0.X.Y
```

The workflow then:

1. Publishes `codefold-core` to crates.io
2. Waits 45 s for propagation
3. Publishes `codefold-cli` to crates.io
4. Builds wheels for Linux x86_64/aarch64, macOS x86_64/arm64, Windows x64
5. Builds an sdist
6. Publishes all wheels + sdist to PyPI

## Workflow file

See [`.github/workflows/release.yml`](.github/workflows/release.yml). Edit there for any policy change (matrix, retries, etc.).
