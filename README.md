# listepo/infra

Shared reusable GitHub workflows and build configs for listepo repositories
(rtok, ketch, stator, bindsmith, runa, cox).

## Reusable workflows

| Workflow | Purpose |
| --- | --- |
| [`ci-rust.yml`](.github/workflows/ci-rust.yml) | Rust CI: `cargo fmt --check`, `cargo clippy -D warnings`, `cargo check`, with a check that `rustc` is the pinned version. Rust comes from the caller's `mise.toml` unless `rust-version` is set. |

`lint.yml` and `self-test.yml` are internal: actionlint on this repository's own workflows,
and `ci-rust.yml` run against a fixture crate.

## How to call

```yaml
jobs:
  rust:
    uses: listepo/infra/.github/workflows/ci-rust.yml@<full commit sha>
```

See [docs/index.md](docs/index.md) for inputs, pinning and secrets.

## License

[GPL-3.0](LICENSE)
