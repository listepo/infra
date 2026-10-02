# Reusable workflows

Shared GitHub Actions workflows for pyrlyn repositories. Each file under
`.github/workflows/` with `on: workflow_call` is called from a thin workflow in the consuming
repository. All third-party actions are pinned to full commit SHAs.

| Workflow | Purpose |
| --- | --- |
| `ci-rust.yml` | fmt, clippy, check, tests on a shared OS/target matrix, optional MSRV |
| `ci-dotnet.yml` | `dotnet test` for every solution; a passing no-op until a .NET project exists |
| `changes.yml` | classify changed files by ecosystem (Rust, Swift, .NET) for suite selection |
| `lint.yml` | actionlint (+ shellcheck) on the caller's workflows |
| `codeql.yml` | CodeQL per language, SARIF to code scanning |
| `semgrep.yml` | Semgrep OSS (`p/default`), SARIF to code scanning |
| `snyk.yml` | Snyk Open Source; skipped without a token |
| `pipeline.yml` | ci-rust + CodeQL + Semgrep + Snyk in parallel behind a `gate` |
| `bump.yml` | the only release path: version commit, PR, required checks, rebase merge, tag + Release, release build |
| `release-plz.yml` | release PR only; never tags, releases or dispatches (bump does) |
| `release.yml` | release build on bump's tag: checks, verify, build, sign/notarize, smoke, upload, publish |
| `notify-release-failure.yml` | open or update a `release-failure` issue for a failed release |
| `dependabot-automerge.yml` | merge allowed Dependabot updates after green CI; label/flag others |
| `sonarcloud.yml` | SonarCloud scan (+ Rust LCOV coverage); skipped without `SONAR_TOKEN` |

Secrets (declared in each workflow's `on.workflow_call.secrets`):

- `snyk.yml`, `pipeline.yml`: `SNYK_TOKEN` (optional).
- `bump.yml`: `BUMP_TOKEN` (pass `RELEASE_PLZ_TOKEN`; opens the PR so its CI runs),
  `CARGO_REGISTRY_TOKEN` (optional, `publish-command`).
- `release-plz.yml`: `RELEASE_PLZ_TOKEN` (required).
- `release.yml`: `MACOS_CERTIFICATE`, `MACOS_CERTIFICATE_PWD`, `APPSTORE_CONNECT_KEY`,
  `APPSTORE_CONNECT_KEY_ID`, `APPSTORE_CONNECT_ISSUER_ID`, `APPLE_ID`, `APPLE_TEAM_ID`,
  `APPLE_APP_PASSWORD`, `CARGO_REGISTRY_TOKEN`, `PUBLISH_TOKEN` (all optional).
- `sonarcloud.yml`: `SONAR_TOKEN` (optional; every step skips without it).
- `dependabot-automerge.yml`: none (uses `github.token`).

`permissions:` the calling job must grant:

- `ci-rust.yml`, `ci-dotnet.yml`, `lint.yml`: `contents: read`, `actions: write`.
- `changes.yml`: `contents: read`.
- `codeql.yml`, `semgrep.yml`, `snyk.yml`, `pipeline.yml`: `contents: read`,
  `security-events: write` (SARIF upload), `actions: write`.
- `release-plz.yml`: `contents: write`, `pull-requests: write`, `actions: write`,
  `issues: write`.
- `release.yml`: `contents: write` (Release assets), `checks: read`, `actions: write`,
  `issues: write`.
- `bump.yml`: `contents: write`, `pull-requests: write`, `actions: write`, `checks: read`,
  `statuses: read`, `issues: write`.
- `notify-release-failure.yml`: `actions: read`, `issues: write`.
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

Composite actions (reference them as `pyrlyn/infra/.github/actions/<name>@<sha>`):

| Action | Purpose |
| --- | --- |
| `gate` | fail unless every job in a `needs` JSON succeeded (`skip-ok` lists allowed skips) |
| `revert-on-failure` | revert a failed push, push the revert, open a draft re-apply PR |
| `macos-sign` | Developer ID codesign (or identity discovery for cargo-dist) + notarization |
| `cancel-run` | cancel the current workflow run (last step, `if: failure()`); `actions: write` |
| `notify-release-failure` | `release-failure` issue (mention + assign) for a failed release run |
| `changes` | changed files by ecosystem: `rust`/`swift`/`dotnet`, `*_deps`, `*_full`, `*_present` |

Private repositories: no scans (CodeQL, Semgrep, Snyk, SonarCloud) by pyrlyn policy.

## Referencing and pinning

```yaml
uses: pyrlyn/infra/.github/workflows/pipeline.yml@<full-sha> # main 2026-09-27
```

- Pin to a full commit SHA (optionally with a `# vX.Y.Z` comment once tags exist). Dependabot
  (`package-ecosystem: github-actions`) updates SHA-pinned reusable workflow refs like action
  refs, so the pin moves by pull request.
- Inside this repository, workflows call each other with `$/.github/workflows/<file>` (GitHub's
  self-repository syntax, July 2026): the nested call resolves to pyrlyn/infra at the commit
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
- Release and bump are never cancelled mid-run: bump merges a version commit, then tags it
  and dispatches publishing, and a cancel half-way can leave a merged version with no tag.
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

pyrlyn/infra is public, so any repository (public or private) can call it. Code scanning
(uploading SARIF from CodeQL, Semgrep or Snyk) is free for public repositories only; private
repositories need GitHub Code Security (formerly Advanced Security), which a personal Free
plan does not have. For private callers pass `upload: false` (or `upload-sarif: false` to
`pipeline.yml`): the scans still run, the SARIF is kept as a workflow artifact, and the job
does not need to upload. Rulesets/branch protection are also unavailable for private repos on
Free, so `gate` is advisory there unless something `needs:` it.

## ci-rust.yml

Rust comes from the caller's `mise.toml` (pyrlyn convention) unless `rust-version` is set;
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
| `changed-only` | `false` | no work (jobs still pass under their names) when no Rust file changed |
| `full-package-args` | `--workspace` | replaces `package-args` when a Cargo.toml/Cargo.lock changed |
| `fmt-runs-on`, `mise-install-args`, `cache-all-refs`, `timeout-minutes` | | |

The `plan` job runs the `changes` action. A Cargo.toml or Cargo.lock change (or a run without
a diff: schedule, workflow_dispatch, a `.github/` or `mise.toml` change) always runs the full
suite: `full-package-args` instead of `package-args`, and `changed-only` never skips it.
`changed-only` is off by default because tests often read non-Rust files (docs, fixtures);
when on, the matrix still expands and every step is a no-op, so required checks named after
the targets report success instead of waiting.

## Dependency-driven suite selection (`changes`)

`changes.yml` (reusable, one `changes` job) and the `changes` composite action classify the
files a pull request, merge-group or push changed (GitHub compare API, three-dot, so exactly
the pull request's diff; `contents: read`, no checkout):

| Ecosystem | `<eco>_deps` (dependency manifests and locks) | `<eco>` also counts |
| --- | --- | --- |
| Rust | `Cargo.toml`, `Cargo.lock` (any directory) | `*.rs`, `.cargo/`, `rust-toolchain*`, `clippy/rustfmt/deny.toml`, `.config/nextest.toml` |
| Swift | `Package.swift`, `Package@swift-*.swift`, `Package.resolved`, XcodeGen `project.yml`, `*.xcodeproj/project.pbxproj` | `*.swift`, `*.m`, `*.mm`, `*.metal`, `*.xcconfig`, `*.entitlements`, `*.xcstrings`, `*.xcodeproj/`, `*.xcworkspace/`, `*.xcassets/`, `.swiftlint.yml`, `.swift-format` |
| .NET | `*.csproj`, `*.fsproj`, `*.vbproj`, `Directory.Packages.props`, `packages.lock.json`, `global.json`, `NuGet.config` (any case) | `*.cs`, `*.fs`, `*.vb`, `*.razor`, `*.xaml`, `*.props`, `*.targets`, `*.sln`, `*.slnx`, ... |

Outputs (strings `true`/`false`): `<eco>` (anything of that ecosystem changed),
`<eco>_deps`, `<eco>_full` (= `<eco>_deps` or `forced`: run the whole suite, never a narrowed
one), `<eco>_present` (the tree has such a project; `changes.yml` exports it for .NET only),
`forced` and `reason`. Inputs `rust-paths`, `swift-paths`, `dotnet-paths` add
repository-specific regexes (one per line) that count as that ecosystem, e.g. a script that
builds the app.

It fails open: an event without a diff, a `.github/`, `mise.toml` or `.tool-versions` change,
300 or more changed files, or any API error sets `forced` and every `<eco>`/`<eco>_full` to
`true`. So a broken classification runs more, never less.

Required checks: gate a job on the outputs with `!cancelled() && (needs.changes.result !=
'success' || needs.changes.outputs.swift == 'true')` so a failed `changes` job runs the suite
instead of skipping it. A non-matrix job skipped by its `if:` reports `skipped`, which branch
protection treats as passed, under its usual name. A matrix job must not be skipped at job
level (the check would be named after the raw `${{ matrix.* }}` expression and a required
target check would wait forever): gate its steps, as `ci-rust.yml` does. A local `gate` job
that `needs:` a job skipped this way passes it in `skip-ok`.

Caller example (the Swift suite of an app that links a Rust library):

```yaml
jobs:
  changes:
    uses: pyrlyn/infra/.github/workflows/changes.yml@<sha> # main
    permissions:
      contents: read
    with:
      swift-paths: |
        ^desktop/
  swift:
    needs: changes
    if: >-
      !cancelled() && (needs.changes.result != 'success'
      || needs.changes.outputs.swift == 'true' || needs.changes.outputs.rust == 'true')
    runs-on: macos-26
    steps:
      - run: swift test   # always the whole package; swift_full is true on a pin change
```

## ci-dotnet.yml

One job, `dotnet`, that always runs and reports under that name. Without a `*.csproj`,
`*.fsproj`, `*.vbproj`, `*.sln` or `*.slnx` in the repository it only classifies and passes
with a notice (no .NET project exists in any pyrlyn repository yet). Once one exists it
restores (`--locked-mode` when a `packages.lock.json` is tracked) and runs `dotnet test` for
every solution, or every project when there is none: the full suite, also for any .NET
dependency change.

| Input | Default | Notes |
| --- | --- | --- |
| `dotnet-version` | `""` | actions/setup-dotnet version; empty = `global.json`, else the runner's SDK |
| `working-directory` | `.` | |
| `test-command` | `""` | bash replacing restore + `dotnet test` |
| `changed-only` | `false` | no-op when no .NET file changed; a dependency change still runs it |
| `runs-on`, `timeout-minutes`, `cancel-run-on-failure` | | |

`ci.yml` runs it as the `dotnet` check (`enabled: true` by default, see docs/config.md).

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
  `rust-timeout-minutes`, `rust-cache-all-refs`, `rust-changed-only`, `rust-full-package-args`.
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
    uses: pyrlyn/infra/.github/workflows/pipeline.yml@<sha> # main
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
      - uses: pyrlyn/infra/.github/actions/gate@<sha> # main
        with:
          needs: ${{ toJSON(needs) }}
```

## Release failure notifications

GitHub cannot filter Actions notifications per workflow, so the maintainer's personal Actions
notifications stay off and only release workflows notify, by issue. `release.yml`,
`release-plz.yml` and `bump.yml` end with a `notify-failure` job (`if: always() &&
contains(needs.*.result, 'failure')`; `always()` because `cancel-run` has cancelled the rest
of the run by then) that runs the `notify-release-failure` action: it opens
`Release failed: <workflow> <ref>` labeled `release-failure` (created when missing), mentions
and assigns `notify-maintainer` (default `listepo`; empty turns it off), and lists the run link
and the failed jobs. An open `release-failure` issue for the same ref (a hidden
`<!-- release-failure ref=... -->` marker) gets a comment instead. The ref is the tag where
one is known (`release.yml` `tag`), else the branch.
`release.yml` skips it on `dry-run`. Callers of these three must grant `issues: write`: GitHub
rejects a nested job that asks for more than the caller grants, even with the input empty.

Release workflows that live in the repository (cargo-dist's `release.yml`, a desktop release)
call `notify-release-failure.yml` from a last job:

```yaml
  notify-failure:
    needs: [plan, build-local-artifacts, build-global-artifacts, host, announce]
    if: >-
      always() && github.event_name != 'pull_request'
      && contains(needs.*.result, 'failure')
    permissions:
      actions: read
      issues: write
    uses: pyrlyn/infra/.github/workflows/notify-release-failure.yml@<sha> # main
    with:
      ref: ${{ inputs.tag || github.ref_name }}
      needs: ${{ toJSON(needs) }}
```

A dist `release.yml` without `allow-dirty = ["ci"]` must not be edited (`dist plan` fails); a
separate `workflow_run` watcher calls it with `run-id`, `run-attempt`, `workflow`, `ref` and
`sha` from `github.event.workflow_run` when `conclusion == 'failure'`. Ordinary CI never calls
it.

## bump.yml

The one way a version is released, for every repository. Nothing else creates a release tag:
not release-plz (`git_tag_enable = false`, `git_release_enable = false`, no `release` command),
not cargo-dist (`dispatch-releases = true` + `create-release = false`: it only fills bump's
draft Release), not a script (release scripts only make the local version commit).

1. `<release-script> <level> --local` (or `release-plz update` with `release-plz-update: true`)
   makes one version commit on top of the default branch.
2. It is pushed to `release/bump-<tag>` and a PR into the default branch is opened. bump
   waits until every required status check of the default branch's rules (read from
   `GET /repos/{repo}/rules/branches/{branch}`, or `required-checks`) has concluded
   `success`/`skipped`/`neutral` on the branch head.
3. `gh pr merge --rebase --match-head-commit <head>` with `GITHUB_TOKEN` (no bypass actor, so
   the rules decide). Rebase rewrites the SHA: the landed commit is read back from the PR's
   `mergeCommit`, and must have the tested tree and the tested base as its only parent.
4. Only then: the tag on that commit (git refs API), the GitHub Release (a draft unless
   `release-draft: false`; notes from the version's CHANGELOG.md section), and every
   `release-workflows` file dispatched with `--ref <tag> -f tag=<tag>` (a tag or Release made
   with `GITHUB_TOKEN` triggers no `push: tags` / `release` workflow). Then `publish-command`.

A failure, a timeout or a closed PR before the merge closes the PR, deletes the branch and
fails the run (and opens a `release-failure` issue): no tag, no Release. If the default branch
moves during the checks the branch is rebuilt on the new head (`max-attempts`, 3).
`dry-run: true` opens the PR, waits for the checks and closes it. `release-untagged-head: true`
releases an untagged version already on the default branch (nothing to commit) after its
required checks are green; off by default, so such a version is never released by accident.

GitHub limits it designs around:

- `pull_request` CI never starts for a PR opened or pushed with `GITHUB_TOKEN`, and pyrlyn has
  "Allow GitHub Actions to create and approve pull requests" off. So `BUMP_TOKEN` (the existing
  `RELEASE_PLZ_TOKEN`) pushes the branch and opens the PR; the PR's own CI reports the checks.
  It never merges (its owner may be a bypass actor). Without it, `GITHUB_TOKEN` opens the PR
  and `ci-workflows` lists the workflows to dispatch on the branch (each needs
  `workflow_dispatch`; check run names, e.g. `pipeline / gate` for a reusable call, are the
  same for a dispatched run).
- The repository must allow rebase merging (`allow_rebase_merge`), and the ruleset's
  `pull_request` rule must list `rebase` in `allowed_merge_methods`.

```yaml
name: Bump and release
on:
  workflow_dispatch:
    inputs:
      level:
        type: choice
        options: [patch, minor, major]
        default: patch
      dry-run:
        type: boolean
        default: false
permissions:
  contents: read
jobs:
  bump:
    uses: pyrlyn/infra/.github/workflows/bump.yml@<sha> # main
    permissions:
      contents: write
      pull-requests: write
      actions: write
      checks: read
      statuses: read
      issues: write # notify-failure
    with:
      level: ${{ inputs.level }}
      dry-run: ${{ inputs.dry-run }}
      release-script: tools/release.sh
      ci-workflows: |
        pipeline.yml
    secrets:
      BUMP_TOKEN: ${{ secrets.RELEASE_PLZ_TOKEN }}
```

## release-plz.yml

On push to the default branch: open or refresh the release PR (`release-plz release-pr`),
held back while the current version is untagged. It never releases: merging the PR tags
nothing and dispatches nothing (the old `detect`/`verify`/`dispatch` jobs released without
bump and are gone). Prefer bump.yml alone; a merged release PR leaves an untagged version that
bump releases only with `release-untagged-head`. Inputs: `tag-prefix` (`v`), `package` (`""` =
first workspace package), `mise` (true). Secret `RELEASE_PLZ_TOKEN` is required (fine-grained
PAT, contents + pull requests write): a PR opened with `GITHUB_TOKEN` would run no CI.

## release.yml

The release build for repositories not built with cargo-dist. bump.yml tags the merged commit,
creates the draft Release and dispatches the caller's workflow with `--ref <tag> -f tag=<tag>`;
this workflow never tags. The caller owns `workflow_dispatch`:

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
    uses: pyrlyn/infra/.github/workflows/release.yml@<sha> # main
    permissions:
      contents: write
      checks: read
      actions: write
      issues: write # notify-failure
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

Stages: `checks` (the tag exists and names the commit the run is on, `required-checks`
concluded `success` on it,
publish secrets present when asked for) -> `verify` (`verify-command` on `verify-os`) ->
`build` per `build-matrix` entry (`setup-command`, `build-command`, collect `bins` from
`bin-dir`, codesign + notarize on macOS when `macos-sign` and the secrets exist, otherwise a
notice unless `require-macos-sign`, `smoke-command` on native targets, `.tar.gz`/`.zip` +
`.sha256`) -> `release` (uploads to bump's Release and publishes it; `notes-command` replaces
bump's notes; prerelease when the tag has a `-` suffix; `draft` keeps it a draft)
-> `publish` (`publish-crates` with `CARGO_REGISTRY_TOKEN`, and/or `publish-command` with
`PUBLISH_TOKEN`, archives in `./dist`). `dry-run: true` stops after `build`.

Repositories built with cargo-dist (rtok, ketch, dunnage, runa) keep dist's generated
`release.yml`: dist regenerates it and fails `dist plan` on a hand-edited copy. Their
dist-workspace.toml sets `dispatch-releases = true` and `create-release = false`, so the
workflow bump dispatches uploads to bump's draft Release and undrafts it, and never tags.

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
    uses: pyrlyn/infra/.github/workflows/dependabot-automerge.yml@<sha> # main
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
    uses: pyrlyn/infra/.github/workflows/sonarcloud.yml@<sha> # main
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
      - uses: pyrlyn/infra/.github/actions/revert-on-failure@<sha> # main
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
- uses: pyrlyn/infra/.github/actions/macos-sign@<sha> # main
  if: runner.os == 'macOS'
  with:
    mode: discover
    certificate: ${{ secrets.MACOS_CERTIFICATE }}
    certificate-password: ${{ secrets.MACOS_CERTIFICATE_PWD }}
    require: "true"
```

