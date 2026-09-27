# listepo/infra

Shared reusable GitHub workflows and build configs for listepo repositories
(rtok, ketch, stator, bindsmith, runa, cox).

## Reusable workflows

| Workflow | Purpose |
| --- | --- |
| [`ci-rust.yml`](.github/workflows/ci-rust.yml) | Rust CI: fmt, clippy, check, tests on a shared five-target matrix, optional MSRV |
| [`lint.yml`](.github/workflows/lint.yml) | actionlint (+ shellcheck) on the caller's workflows |
| [`codeql.yml`](.github/workflows/codeql.yml) | CodeQL code scanning per language |
| [`semgrep.yml`](.github/workflows/semgrep.yml) | Semgrep OSS scan, SARIF to code scanning |
| [`snyk.yml`](.github/workflows/snyk.yml) | Snyk Open Source scan, skipped without `SNYK_TOKEN` |
| [`pipeline.yml`](.github/workflows/pipeline.yml) | ci-rust + CodeQL + Semgrep + Snyk in parallel behind a `gate` |
| [`release-plz.yml`](.github/workflows/release-plz.yml) | release PR, then verify + dispatch of the release workflow |
| [`release.yml`](.github/workflows/release.yml) | manual release: checks, verify, build, sign, smoke, Release, publish |

Composite action [`gate`](.github/actions/gate/action.yml) fails unless every needed job
succeeded. `self-test.yml` runs ci-rust.yml and pipeline.yml against a fixture crate.

## How to call

```yaml
jobs:
  pipeline:
    uses: listepo/infra/.github/workflows/pipeline.yml@<full commit sha>
    permissions:
      contents: read
      security-events: write
      actions: read
    with:
      rust: true
      codeql-languages: '["actions", "rust"]'
    secrets:
      SNYK_TOKEN: ${{ secrets.SNYK_TOKEN }}
```

See [docs/reusable-workflows.md](docs/reusable-workflows.md) for every workflow's inputs,
secrets, required permissions and caller examples, [docs/index.md](docs/index.md) for
ci-rust.yml details, and [docs/centralization-candidates.md](docs/centralization-candidates.md)
for what else could move here.

## License

[GPL-3.0](LICENSE)
