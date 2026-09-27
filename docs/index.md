# Using listepo/infra

## ci-rust.yml

Runs `cargo fmt --all --check`, `cargo clippy --all-targets --all-features -- -D warnings`
and `cargo check --all-targets` on one runner.

```yaml
# .github/workflows/ci.yml in the calling repository
name: ci

on:
  pull_request:
  workflow_dispatch:

permissions:
  contents: read

jobs:
  rust:
    uses: listepo/infra/.github/workflows/ci-rust.yml@main
    with:
      runs-on: ubuntu-latest
      working-directory: .
      # rust-version: "1.97.1"          # optional; default is the version in mise.toml
      # mise-install-args: rust just    # optional; tools mise installs (default: rust)
      # clippy-args: --workspace        # optional; extra clippy args before `--`
```

### Inputs

| Input | Default | Description |
| --- | --- | --- |
| `rust-version` | `""` | Exact toolchain via rustup. Empty installs Rust from the caller's `mise.toml` with `jdx/mise-action` (listepo convention: `mise.toml` is the single source of the Rust version). |
| `runs-on` | `ubuntu-latest` | Runner label. |
| `working-directory` | `.` | Cargo workspace directory. |
| `mise-install-args` | `rust` | Tools mise installs when `rust-version` is empty. |
| `clippy-args` | `""` | Extra clippy arguments placed before `--`. |

The caller's `mise.toml` should pin Rust, for example:

```toml
[tools]
rust = { version = "1.97.1", components = "rustfmt,clippy" }
```

## Pinning

`@main` is fine while the workflows settle. Once tags exist, pin to a tag or a full commit
SHA (`@<sha> # vX.Y.Z`) so changes here do not reach callers unannounced.

## Secrets

`ci-rust.yml` needs no secrets. A reusable workflow does not see the caller's secrets unless
they are passed explicitly or with `secrets: inherit`:

```yaml
jobs:
  rust:
    uses: listepo/infra/.github/workflows/ci-rust.yml@main
    secrets: inherit
```

Only add `secrets: inherit` for workflows that actually need secrets.

## Permissions

The workflows declare `permissions: contents: read`. A caller can only keep or reduce the
token permissions; the called workflow cannot raise them.

## Actions policy of calling repositories

This repository is public, so any repository can call its workflows. The calling
repository's Actions policy still applies to every action used inside the called workflow.
With "only actions owned by listepo", `listepo/infra` itself is allowed but the third-party
actions it uses (`actions/checkout`, `jdx/mise-action`, `Swatinem/rust-cache`) are blocked.
Such a repository needs "Allow actions created by GitHub" and these patterns allowed:

```text
jdx/mise-action@*
Swatinem/rust-cache@*
```
