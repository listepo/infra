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
| [`dependabot-automerge.yml`](.github/workflows/dependabot-automerge.yml) | merge allowed Dependabot updates after green CI |
| [`sonarcloud.yml`](.github/workflows/sonarcloud.yml) | SonarCloud scan, skipped without `SONAR_TOKEN` |
| [`ci.yml`](.github/workflows/ci.yml) | **single entrypoint**: config-driven (`.github/infra.yml`) Rust CI, scans, SonarCloud, lint and repo-specific jobs behind a `gate` |
| [`sync-docs.yml`](.github/workflows/sync-docs.yml) | publish docs/ to listepo/landing |
| [`pages.yml`](.github/workflows/pages.yml) | build a static site and deploy it to GitHub Pages |
| [`bump.yml`](.github/workflows/bump.yml) | verify, then run the release script (bump + dispatch dist) |
| [`revert-on-failure.yml`](.github/workflows/revert-on-failure.yml) | revert a failed push to the default branch |

Composite actions: [`gate`](.github/actions/gate/action.yml) (fail unless every needed job
succeeded), [`revert-on-failure`](.github/actions/revert-on-failure/action.yml),
[`macos-sign`](.github/actions/macos-sign/action.yml) and
[`cancel-run`](.github/actions/cancel-run/action.yml) (cancel the whole run when a job fails).
`self-test.yml` runs ci-rust.yml and pipeline.yml against a fixture crate.

See [docs/config.md](docs/config.md) for the config file and [docs/migration/](docs/migration/)
for each consumer's thin callers.

## How to call

```yaml
jobs:
  pipeline:
    uses: listepo/infra/.github/workflows/pipeline.yml@<full commit sha>
    permissions:
      contents: read
      security-events: write
      actions: write
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
