# Config-driven CI (`ci.yml` + `.github/infra.yml`)

Goal: a consumer switches checks on/off and tunes them by editing YAML, never a workflow file.
The consumer's only CI workflow is a thin caller of `ci.yml` (see docs/migration/).

## Layout (chosen)

Three layers, later wins key by key (maps deep-merge, lists are replaced):

1. `.github/actions/config/defaults.yml` in infra: every key with its default.
2. `config/repos/<repository name>.yml` in infra (optional): central per-repo overrides,
   read at the commit the consumer pinned.
3. `.github/infra.yml` in the consumer (optional; path = `config-file` input): local overrides.

A `config` job (composite action `.github/actions/config`) sparse-checks-out only the config
file, merges with `yq` (preinstalled on GitHub-hosted runners), normalizes with a stdlib
Python script and emits `json`, `jobs`, `skip-ok`, `draft-skip`. Every job in ci.yml has
`needs: config` and `if: fromJSON(needs.config.outputs.json).<check>.run`; inputs come from
the same JSON.

## Options compared

| Option | + | − |
| --- | --- | --- |
| A. Local `.github/infra.yml` only | change ships with the code it tests (same PR); no infra re-pin; reviewers see it | one more file per repo |
| B. Central `config/repos/<repo>.yml` only | all settings in one repo; consumers truly hold only the wrapper | every tweak = infra PR + re-pin in the consumer (the pin also freezes the config); PR in a consumer cannot change its own CI |
| C. Inputs on the caller (`with:`) | no parsing | the caller file grows back into logic; that is what we are removing |
| D. Repository variables (`vars.*`) | no file | not versioned, not reviewable, strings only |
| **Chosen: defaults + optional central + local** | A for day-to-day; B available for fleet-wide policy (e.g. force `snyk: {enabled: false}` everywhere) | two places to look; the step summary prints what ran and why |

## GitHub constraints that shape the design

- `uses:` of a job cannot be an expression, so ci.yml lists every possible reusable call and
  turns each off with `if:`; new check types need an infra change.
- `if:` may read `needs.<job>.outputs`; `with:` may too; matrices take `fromJSON(...)`
  (repository-specific jobs are one matrix job, max 256 entries).
- A skipped required check counts as passed, so `gate` runs with `always()` and fails on any
  non-success except jobs listed in `skip-ok` (disabled by config or event filter). A failed
  `config` job fails the gate. On a draft PR with `skip-drafts`, the gate itself is skipped
  (same as pipeline.yml).
- Permissions are validated for every nested job up front, even disabled ones: the caller
  grants the union (`contents: read`, `security-events: write`, `pull-requests: read`,
  `actions: write`) regardless of what the config enables.
- Steps cannot be generated, so custom jobs have a fixed step shape (checkout, env, mise,
  rustc pin check, rust-cache, taiki-e tools, setup, run). Anything else goes into a script in
  the consumer repository (content, not workflow logic).
- Secrets cannot come from config; ci.yml declares SNYK_TOKEN and SONAR_TOKEN.

## Schema (version 1)

```yaml
version: 1                      # required
cancel-run-on-failure: true     # cancel the run when a job fails (input "false" overrides)
skip-drafts: true               # draft PRs run nothing
upload-sarif: auto              # true | false | auto (= public repositories)

# Each check: enabled (true | false | auto = public repos only), events (list of
# github.event_name; [] = all) + the inputs of the matching reusable workflow.
rust:       {enabled: false, events: [], matrix: [], rust-version: "", fmt-runs-on: ubuntu-latest,
             working-directory: ".", mise-install-args: rust, clippy-args: "", tools: "",
             setup-command: "", test-command: "...", doc-tests: true, build-command: "...",
             package-args: --workspace, feature-args: --all-features, msrv: "",
             msrv-command: "...", timeout-minutes: 60, cache-all-refs: false}
codeql:     {enabled: auto, languages: [actions], build-mode: none, build-command: "",
             queries: security-and-quality, config-file: "", runs-on: ubuntu-latest}
semgrep:    {enabled: auto, config: p/default, extra-args: "", fail-on-findings: false}
snyk:       {enabled: auto, args: --all-projects, monitor: true}
sonarcloud: {enabled: false, organization: "", project-key: "", args: "", project-base-dir: ".",
             mise: true, mise-install-args: "", rust: false, setup-command: "",
             coverage-command: "", soft-fail: true, timeout-minutes: 60}
lint:       {enabled: true, actionlint-version: 1.7.12, args: "", extra-command: ""}

jobs:                           # repository-specific jobs -> `ci / <name>`
  - name: footprint             # required
    run: bash scripts/footprint.sh --check   # required (bash, `set -euo pipefail`)
    enabled: true
    events: []                  # e.g. [pull_request]
    runs-on: ubuntu-latest      # or a list: one job per runner
    matrix: []                  # list of maps; keys exported UPPER_CASE; `runs-on` key picks the runner
    mise: true                  # true = all of mise.toml, false = none, "rust node" = install args
    rust-pin: false             # fail unless rustc == mise.toml pin (needs mise)
    rust-cache: false
    tools: ""                   # taiki-e/install-action list, e.g. "nextest,cargo-deny"
    env: {}                     # exported before setup/run
    setup: ""                   # bash before run
    working-directory: "."
    shell: bash                 # bash | pwsh
    timeout-minutes: 60
    fetch-depth: 1
    allow-failure: false
```

Unknown keys fail the `config` job (typos never silently disable a check).
Example: tests/fixtures/infra.yml (self-test), docs/migration/*/infra.yml.
