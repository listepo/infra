# Reusable workflows

Shared GitHub Actions workflows for listepo repositories. Each file under
`.github/workflows/` with `on: workflow_call` is called from a thin workflow in the consuming
repository. All third-party actions are pinned to full commit SHAs.

| Workflow | Purpose |
| --- | --- |
| `ci-rust.yml` | fmt, clippy, check, tests on a shared OS/target matrix, optional MSRV |
| `lint.yml` | actionlint (+ shellcheck) on the caller's workflows |
| `codeql.yml` | CodeQL per language, SARIF to code scanning |
| `semgrep.yml` | Semgrep OSS (`p/default`), SARIF to code scanning |
| `snyk.yml` | Snyk Open Source; skipped without a token |
| `pipeline.yml` | ci-rust + CodeQL + Semgrep + Snyk in parallel behind a `gate` |
| `release-plz.yml` | release PR; on its merge verify, then dispatch the release workflow |
| `release.yml` | manual release: checks, verify, build, sign/notarize, smoke, Release, publish |

Secrets (declared in each workflow's `on.workflow_call.secrets`):

- `snyk.yml`, `pipeline.yml`: `SNYK_TOKEN` (optional).
- `release-plz.yml`: `RELEASE_PLZ_TOKEN` (required).
- `release.yml`: `MACOS_CERTIFICATE`, `MACOS_CERTIFICATE_PWD`, `APPLE_ID`, `APPLE_TEAM_ID`,
  `APPLE_APP_PASSWORD`, `CARGO_REGISTRY_TOKEN`, `PUBLISH_TOKEN` (all optional).

`permissions:` the calling job must grant:

- `ci-rust.yml`, `lint.yml`: `contents: read`.
- `codeql.yml`, `semgrep.yml`, `snyk.yml`, `pipeline.yml`: `contents: read`,
  `security-events: write` (SARIF upload), `actions: read`.
- `release-plz.yml`: `contents: write`, `pull-requests: write`, `actions: write`.
- `release.yml`: `contents: write` (tag + Release), `checks: read`, `actions: read`.

Composite action: `.github/actions/gate` fails unless every job in a `needs` JSON succeeded
(`skipped` counts as failure unless listed in `skip-ok`). Use it for a local gate that also
covers repository-specific jobs.

## Referencing and pinning

```yaml
uses: listepo/infra/.github/workflows/pipeline.yml@<full-sha> # main 2026-09-27
```

- Pin to a full commit SHA (optionally with a `# vX.Y.Z` comment once tags exist). Dependabot
  (`package-ecosystem: github-actions`) updates SHA-pinned reusable workflow refs like action
  refs, so the pin moves by pull request.
- Inside this repository, workflows call each other with `$/.github/workflows/<file>` (GitHub's
  self-repository syntax, July 2026): the nested call resolves to listepo/infra at the commit
  the caller pinned, never to the caller's repository and never to `main`.
- GitHub limits (github.com): 10 levels of nesting, 50 unique reusable workflows per run.
  The deepest chain here is caller -> pipeline.yml -> ci-rust.yml (3 levels).
- Secrets never flow implicitly: declare each one in `secrets:` of the calling job (preferred)
  or use `secrets: inherit`. Every workflow here declares its secrets explicitly.
- Permissions only go down: a called workflow gets at most what the calling job grants. Grant
  the table's permissions on the calling job.
- The calling repository's Actions policy also applies to the actions used here. With
  "Allow listepo, and select non-listepo, actions" allow: GitHub-owned actions,
  `jdx/mise-action@*`, `Swatinem/rust-cache@*`, `taiki-e/install-action@*`,
  `snyk/actions/*`, `release-plz/action@*`.

## Free plan and private repositories

listepo/infra is public, so any repository (public or private) can call it. Code scanning
(uploading SARIF from CodeQL, Semgrep or Snyk) is free for public repositories only; private
repositories need GitHub Code Security (formerly Advanced Security), which a personal Free
plan does not have. For private callers pass `upload: false` (or `upload-sarif: false` to
`pipeline.yml`): the scans still run, the SARIF is kept as a workflow artifact, and the job
does not need to upload. Rulesets/branch protection are also unavailable for private repos on
Free, so `gate` is advisory there unless something `needs:` it.

## ci-rust.yml

Rust comes from the caller's `mise.toml` (listepo convention) unless `rust-version` is set;
every job fails if `rustc --version` is not the pinned version.

| Input | Default | Notes |
| --- | --- | --- |
| `matrix` | `""` (shared five-target matrix) | JSON `[{"os", "target", "test"?}]` |
| `rust-version` | `""` | exact toolchain via rustup instead of mise |
| `working-directory` | `.` | Cargo workspace |
| `feature-args` | `--all-features` | used by clippy, check, test, build, MSRV |
| `clippy-args` | `""` | extra args before `--` for clippy and check |
| `tools` | `""` | taiki-e/install-action tools (e.g. `nextest`) |
| `setup-command` | `""` | bash before clippy (system packages) |
| `test-command` | `cargo test --all-targets $FEATURE_ARGS` | native targets only |
| `build-command` | `cargo build --all-targets $FEATURE_ARGS --target "$TARGET"` | cross targets |
| `msrv` | `""` | e.g. `1.85`; adds an `msrv` job |
| `msrv-command` | `cargo check --workspace --all-targets $FEATURE_ARGS` | |
| `fmt-runs-on`, `mise-install-args`, `cache-all-refs`, `timeout-minutes` | | |

## codeql.yml / semgrep.yml / snyk.yml

- codeql: `languages` (JSON, default `["actions"]`), `build-mode` (`none`), `build-command`
  (for `manual`), `queries` (`security-and-quality`), `config-file`, `runs-on`, `upload`.
  Keep the repository's CodeQL *default setup* off.
- semgrep: `config` (`p/default`), `extra-args`, `fail-on-findings` (`false`), `upload`.
- snyk: `args` (`--all-projects`), `monitor` (`true`), `upload`; secret `SNYK_TOKEN`.
  Snyk CLI does not test Cargo projects; it covers npm, pub, Go, Python, NuGet manifests.

## pipeline.yml

Inputs: `rust` (false), `rust-working-directory`, `rust-matrix`, `rust-setup-command`,
`rust-test-command`, `rust-feature-args`, `rust-msrv`, `codeql` (true), `codeql-languages`,
`codeql-build-mode`, `codeql-queries`, `semgrep` (true), `semgrep-config`, `snyk` (true),
`upload-sarif` (true). Output: `result` (`success`). Draft PRs skip every job, including the
gate, so a draft never shows a green `gate`.

Copy-paste caller (`.github/workflows/pipeline.yml`):

```yaml
name: pipeline

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]
    types: [opened, synchronize, reopened, ready_for_review]
  schedule:
    - cron: "17 4 * * 1"
  workflow_dispatch:

concurrency:
  group: pipeline-${{ github.event.pull_request.number || github.ref }}
  cancel-in-progress: ${{ github.event_name == 'pull_request' }}

permissions:
  contents: read

jobs:
  pipeline:
    uses: listepo/infra/.github/workflows/pipeline.yml@<sha> # main
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

The required check is `pipeline / gate`. A repository with its own CI (e.g. `just`
recipes) sets `rust: false`, keeps its local `ci.yml`, and adds a local gate:

```yaml
  ci:
    uses: ./.github/workflows/ci.yml
  gate:
    needs: [ci, pipeline]
    if: >-
      !cancelled()
      && (github.event_name != 'pull_request' || !github.event.pull_request.draft)
    runs-on: ubuntu-latest
    steps:
      - uses: listepo/infra/.github/actions/gate@<sha> # main
        with:
          needs: ${{ toJSON(needs) }}
```

## release-plz.yml

On push to the default branch. Inputs: `tag-prefix` (`v`), `package` (`""` = first workspace
package), `release-workflow` (`release.yml`, dispatched with `-f tag=<prefix><version>`),
`verify-command`, `verify-os` (`["ubuntu-latest", "macos-latest"]`), `mise` (true).
Secret `RELEASE_PLZ_TOKEN` is required (fine-grained PAT, contents + pull requests write): a
PR opened with `GITHUB_TOKEN` would run no CI. The merge is recognised by a commit line equal
to `release: vX.Y.Z` (optionally ` (#123)`).

```yaml
on:
  push:
    branches: [main]
jobs:
  release-plz:
    uses: listepo/infra/.github/workflows/release-plz.yml@<sha> # main
    permissions:
      contents: write
      pull-requests: write
      actions: write
    with:
      verify-command: just check
    secrets:
      RELEASE_PLZ_TOKEN: ${{ secrets.RELEASE_PLZ_TOKEN }}
```

## release.yml

Manual release modeled on rtok (verify on the release commit, then build and publish). The
caller owns `workflow_dispatch`:

```yaml
name: release
on:
  workflow_dispatch:
    inputs:
      tag:
        description: Release tag (vX.Y.Z)
        required: true
        type: string
      dry-run:
        type: boolean
        default: false
permissions:
  contents: read
jobs:
  release:
    uses: listepo/infra/.github/workflows/release.yml@<sha> # main
    permissions:
      contents: write
      checks: read
      actions: read
    with:
      tag: ${{ inputs.tag }}
      dry-run: ${{ inputs.dry-run }}
      required-checks: |
        gate
      verify-command: just check
      bins: mytool
      smoke-command: '"$BIN_DIR/mytool" --version'
      macos-sign: true
    secrets:
      MACOS_CERTIFICATE: ${{ secrets.MACOS_CERTIFICATE }}
      MACOS_CERTIFICATE_PWD: ${{ secrets.MACOS_CERTIFICATE_PWD }}
      APPLE_ID: ${{ secrets.APPLE_ID }}
      APPLE_TEAM_ID: ${{ secrets.APPLE_TEAM_ID }}
      APPLE_APP_PASSWORD: ${{ secrets.APPLE_APP_PASSWORD }}
```

Stages: `checks` (tag valid and unused, `required-checks` concluded `success` on the commit,
publish secrets present when asked for) -> `verify` (`verify-command` on `verify-os`) ->
`build` per `build-matrix` entry (`setup-command`, `build-command`, collect `bins` from
`bin-dir`, codesign + notarize on macOS when `macos-sign` and the secrets exist, otherwise a
notice unless `require-macos-sign`, `smoke-command` on native targets, `.tar.gz`/`.zip` +
`.sha256`) -> `release` (`gh release create` at the tested commit: creates the tag; notes from
`notes-command` or GitHub's generated notes; prerelease when the tag has a `-` suffix; `draft`)
-> `publish` (`publish-crates` with `CARGO_REGISTRY_TOKEN`, and/or `publish-command` with
`PUBLISH_TOKEN`, archives in `./dist`). `dry-run: true` stops after `build`.

Repositories built with cargo-dist (rtok, ketch, dunnage, runa, cox) keep dist's generated
`release.yml`: dist regenerates it and fails `dist plan` on a hand-edited copy. They use
`release-plz.yml` (verify + dispatch of the dist workflow) from here and keep signing in dist's
`build-setup.yml`.

What stays in each repository: the thin callers above, `.github/dependabot.yml` (GitHub reads
it only from the repository itself), repository-specific jobs (e.g. rtok's webui/wasm checks,
plugin-version checks, revert-on-failure), cargo-dist's `release.yml` and `build-setup.yml`.
