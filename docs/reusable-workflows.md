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
| `dependabot-automerge.yml` | merge allowed Dependabot updates after green CI; label/flag others |
| `sonarcloud.yml` | SonarCloud scan (+ Rust LCOV coverage); skipped without `SONAR_TOKEN` |

Secrets (declared in each workflow's `on.workflow_call.secrets`):

- `snyk.yml`, `pipeline.yml`: `SNYK_TOKEN` (optional).
- `release-plz.yml`: `RELEASE_PLZ_TOKEN` (required).
- `release.yml`: `MACOS_CERTIFICATE`, `MACOS_CERTIFICATE_PWD`, `APPSTORE_CONNECT_KEY`,
  `APPSTORE_CONNECT_KEY_ID`, `APPSTORE_CONNECT_ISSUER_ID`, `APPLE_ID`, `APPLE_TEAM_ID`,
  `APPLE_APP_PASSWORD`, `CARGO_REGISTRY_TOKEN`, `PUBLISH_TOKEN` (all optional).
- `sonarcloud.yml`: `SONAR_TOKEN` (optional; every step skips without it).
- `dependabot-automerge.yml`: none (uses `github.token`).

`permissions:` the calling job must grant:

- `ci-rust.yml`, `lint.yml`: `contents: read`, `actions: write`.
- `codeql.yml`, `semgrep.yml`, `snyk.yml`, `pipeline.yml`: `contents: read`,
  `security-events: write` (SARIF upload), `actions: write`.
- `release-plz.yml`: `contents: write`, `pull-requests: write`, `actions: write`.
- `release.yml`: `contents: write` (tag + Release), `checks: read`, `actions: write`.
- `dependabot-automerge.yml`: `contents: write`, `pull-requests: write`, `actions: read`.
- `sonarcloud.yml`: `contents: read`, `pull-requests: read`, `actions: write`.

`actions: write` is for cancel-on-failure: every job of every workflow above except
`dependabot-automerge.yml` ends with the `cancel-run` action under `if: failure()`, so the first
failing job (a test, clippy, fmt, CodeQL, Semgrep, Snyk, SonarCloud, a release step) cancels
the whole run at once: every other running or queued job, the caller's own jobs included
(`github.run_id` inside a reusable workflow is the caller's run). Matrices use
`fail-fast: true`. `pipeline.yml`'s `gate` runs with `always()`, so after such a cancel it
fails instead of being skipped (a skipped required check counts as passed). A calling job that
grants less than `actions: write` makes the run fail at startup (GitHub refuses a nested job
that asks for more than its caller grants), even with the input off. Pass
`cancel-run-on-failure: false` where a later job of the caller must still run after a failure,
e.g. a Dependabot flow whose notify job reports a failed CI. `dependabot-automerge.yml` has no
cancel step for the same reason: its `notify-failure` must run after `automerge` fails.

Composite actions (reference them as `listepo/infra/.github/actions/<name>@<sha>`):

| Action | Purpose |
| --- | --- |
| `gate` | fail unless every job in a `needs` JSON succeeded (`skip-ok` lists allowed skips) |
| `revert-on-failure` | revert a failed push, push the revert, open a draft re-apply PR |
| `macos-sign` | Developer ID codesign (or identity discovery for cargo-dist) + notarization |
| `cancel-run` | cancel the current workflow run (last step, `if: failure()`); `actions: write` |

Private repositories: no scans (CodeQL, Semgrep, Snyk, SonarCloud) by listepo policy.

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

## Concurrency (only the newest run executes)

Cancelling older runs is the caller's job: put a workflow-level `concurrency` in the thin
caller, keyed by workflow and ref, so a new push cancels the older queued or in-progress run of
the same workflow on the same branch or PR and the newest run runs to the end.

```yaml
concurrency:
  group: ${{ github.workflow }}-${{ github.event.pull_request.number || github.ref }}
  cancel-in-progress: ${{ github.event_name == 'pull_request' }}
```

- `cancel-in-progress: true` cancels older runs on every ref. The consumer callers here
  (docs/migration) cancel on pull requests only: on the default branch each push keeps its
  run, so `revert-on-failure` reverts the push that actually failed rather than the newest
  one. Use `true` where no job acts on a single push.
- The reusable workflows here set no `concurrency` of their own for CI (`ci.yml`,
  `pipeline.yml`, `ci-rust.yml`, scans, `sonarcloud.yml`): inside a called workflow the
  `github` context is the caller's, so a group built from `github.workflow` equals the
  caller's group and cancels or deadlocks the caller (docs/github-limits.md), and a
  cancel inside an older caller run fails its `gate` instead of cancelling the run cleanly.
- `lint.yml` cancels older runs only when it runs directly in this repository; called, it
  uses a group of its own run and never cancels.
- This repository's own triggered workflows (`action-pins.yml`, `lint.yml`, `self-test.yml`)
  use `group: ${{ github.workflow }}-${{ github.ref }}`, `cancel-in-progress: true`.
- Release and bump are never cancelled mid-run: they push a version commit or tag and
  dispatch publishing, and a cancel half-way can leave a pushed version with no release.
  `bump.yml` (`release-${{ github.repository }}`) and `release-plz.yml`
  (`release-plz-${{ github.repository }}`) set `cancel-in-progress: false`: a newer run waits,
  and GitHub keeps only the newest pending run per group. `release.yml` sets nothing; its
  caller should:

```yaml
# caller of release.yml / bump.yml (workflow_dispatch)
concurrency:
  group: ${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: false
```

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
| `package-args` | `--workspace` | packages for every cargo command (`-p x`; `""` = root only) |
| `feature-args` | `--all-features` | used by clippy, check, test, doctests, build, MSRV |
| `clippy-args` | `""` | extra args before `--` for clippy and check |
| `tools` | `""` | taiki-e/install-action tools (e.g. `nextest`) |
| `setup-command` | `""` | bash before clippy (system packages) |
| `test-command` | `cargo test $PACKAGE_ARGS --all-targets $FEATURE_ARGS` | native targets only |
| `doc-tests` | `true` | `cargo test $PACKAGE_ARGS --doc $FEATURE_ARGS`; no lib: skipped |
| `build-command` | `cargo build $PACKAGE_ARGS --all-targets $FEATURE_ARGS --target "$TARGET"` | |
| `msrv` | `""` | e.g. `1.85`; adds an `msrv` job |
| `msrv-command` | `cargo check $PACKAGE_ARGS --all-targets $FEATURE_ARGS` | |
| `fmt-runs-on`, `mise-install-args`, `cache-all-refs`, `timeout-minutes` | | |

## codeql.yml / semgrep.yml / snyk.yml

- codeql: `languages` (JSON, default `["actions"]`), `build-mode` (`none`), `build-command`
  (for `manual`), `queries` (`security-and-quality`), `config-file`, `runs-on`, `upload`.
  Keep the repository's CodeQL *default setup* off.
- semgrep: `config` (`p/default`), `extra-args`, `fail-on-findings` (`false`), `upload`.
- snyk: `args` (`--all-projects`), `monitor` (`true`), `upload`; secret `SNYK_TOKEN`.
  Snyk CLI does not test Cargo projects; it covers npm, pub, Go, Python, NuGet manifests.

## pipeline.yml

Inputs:

- `rust` (false) runs ci-rust.yml. Every ci-rust.yml input is passed through as `rust-<name>`
  with the same default: `rust-matrix`, `rust-rust-version`, `rust-fmt-runs-on`,
  `rust-working-directory`, `rust-mise-install-args`, `rust-clippy-args`, `rust-tools`,
  `rust-setup-command`, `rust-test-command`, `rust-doc-tests`, `rust-build-command`,
  `rust-package-args`, `rust-feature-args`, `rust-msrv`, `rust-msrv-command`,
  `rust-timeout-minutes`, `rust-cache-all-refs`.
- `codeql` (true), `codeql-languages`, `codeql-build-mode`, `codeql-build-command`,
  `codeql-queries`, `codeql-config-file`, `codeql-runs-on`.
- `semgrep` (true), `semgrep-config`, `semgrep-extra-args`, `semgrep-fail-on-findings`.
- `snyk` (true), `snyk-args`, `snyk-monitor`.
- `upload-sarif` (true).

Output: `result` (`success`). Draft PRs skip every job, including the gate, so a draft never
shows a green `gate`.

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
      actions: write
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
      actions: write
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

## dependabot-automerge.yml

Unifies rtok/ketch/cox. The caller runs its CI and passes the result. Only Dependabot's own
PRs from a branch of the repository are touched. Update types listed in
`allowed-update-types` are merged with `gh pr merge --<merge-method> --match-head-commit`
(never `--admin`, never `--auto`); others get `review-label`, and a major update is assigned
to `maintainer` with a review request. A failed CI or merge assigns and mentions `maintainer`.

| Input | Default |
| --- | --- |
| `ci-result` (required) | – (pass `needs.ci.result`) |
| `allowed-update-types` | `version-update:semver-patch` (space-separated) |
| `merge-method` | `squash` (`merge`, `rebase`) |
| `wait-workflow` | `""` (e.g. `pipeline.yml`: its latest PR run on the head commit must be green) |
| `wait-minutes` | `90` (max 90) |
| `review-label` | `needs-review` |
| `maintainer` | `listepo` (empty: nobody is assigned or mentioned) |

```yaml
name: Dependabot
on:
  pull_request:
    branches: [main]
    types: [opened, synchronize, reopened]
permissions: {}
concurrency:
  group: dependabot-pr-${{ github.event.pull_request.number }}
  cancel-in-progress: true
jobs:
  ci:
    if: github.actor == 'dependabot[bot]'
    uses: ./.github/workflows/ci.yml
    permissions:
      contents: read
  automerge:
    needs: ci
    if: ${{ !cancelled() && github.actor == 'dependabot[bot]' }}
    uses: listepo/infra/.github/workflows/dependabot-automerge.yml@<sha> # main
    permissions:
      contents: write
      pull-requests: write
      actions: read
    with:
      ci-result: ${{ needs.ci.result }}
      wait-workflow: pipeline.yml
```

The repository must allow squash merges (the default method) and, for the review label, the
token needs `pull-requests: write`.

## sonarcloud.yml

Unifies the sonarcloud.yml of rtok, ketch, cox, runa, crates-packages, slint_dart and stator.
Every step skips with a notice when `SONAR_TOKEN` is empty; coverage and scan are soft-fail
unless `soft-fail: false`.

| Input | Default |
| --- | --- |
| `organization`, `project-key` | `""` (use `sonar-project.properties`) |
| `args` | `""` extra scanner args |
| `project-base-dir` | `.` |
| `mise`, `mise-install-args` | `true`, `""` |
| `rust` | `false` (llvm-tools-preview, cargo-llvm-cov, rust-cache) |
| `setup-command` | `""` |
| `coverage-command` | `""` (Rust: `cargo llvm-cov --locked --lcov`, to `coverage/lcov.info`) |
| `soft-fail` | `true` |
| `timeout-minutes` | `60` |

```yaml
jobs:
  sonarcloud:
    uses: listepo/infra/.github/workflows/sonarcloud.yml@<sha> # main
    permissions:
      contents: read
      pull-requests: read
      actions: write
    with:
      rust: true
      organization: listepo
      project-key: listepo_ketch
    secrets:
      SONAR_TOKEN: ${{ secrets.SONAR_TOKEN }}
```

## revert-on-failure (composite action)

Unifies both generations found in six repositories: the older one (bindsmith, cox, runa,
slint_dart, stator: skip new branches, missing bases, bot commits, earlier auto-reverts and
pushes touching `.github/`) and rtok's (adds the draft re-apply PR).

Inputs: `before` (`github.event.before`), `after` (`github.sha`), `branch`
(`github.ref_name`), `reapply-pr` (`true`), `skip-workflow-changes` (`true`), `dry-run`
(`false`), `token` (`github.token`). Outputs: `reverted`, `pr-url`. The action checks out the
repository itself. Job permissions: `contents: write`, plus `pull-requests: write` for the
re-apply PR (and the repository setting "Allow GitHub Actions to create and approve pull
requests").

```yaml
  revert-on-failure:
    needs: [lint, test]
    if: >-
      always() && github.event_name == 'push' && github.ref == 'refs/heads/main'
      && contains(join(needs.*.result, ','), 'failure')
    runs-on: ubuntu-latest
    permissions:
      contents: write
      pull-requests: write
    steps:
      - uses: listepo/infra/.github/actions/revert-on-failure@<sha> # main
```

## macos-sign (composite action)

Extracted from rtok/dunnage/ketch `build-setup.yml` (identity discovery for cargo-dist) and
ketch `build-check.yml` (notarization). A no-op on non-macOS runners. Every credential is
optional: missing ones skip with a notice unless `require: "true"`.

| Input | Notes |
| --- | --- |
| `mode` | `sign` (default), `discover` (export `CODESIGN_IDENTITY` for dist), `notarize` |
| `paths` | newline/space-separated files |
| `certificate`, `certificate-password` | base64 `.p12` Developer ID Application + password |
| `notarize` | `true` (sign mode) |
| `api-key`, `api-key-id`, `api-issuer` | App Store Connect API key (base64 `.p8`) |
| `apple-id`, `team-id`, `app-password` | alternative Apple ID auth |
| `require` | `false` |

Outputs: `identity`, `signed`, `notarized`. `release.yml` uses it (`macos-sign` input).
In a cargo-dist `build-setup.yml`:

```yaml
- uses: listepo/infra/.github/actions/macos-sign@<sha> # main
  if: runner.os == 'macOS'
  with:
    mode: discover
    certificate: ${{ secrets.MACOS_CERTIFICATE }}
    certificate-password: ${{ secrets.MACOS_CERTIFICATE_PWD }}
    require: "true"
```

