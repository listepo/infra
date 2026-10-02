# Contributor License Agreement (CLA)

The pyrlyn projects are published under `GPL-3.0-only` and also offered under a royalty-free
license and a paid commercial license (`LICENSE`, `LICENSE-ROYALTY-FREE.md`, `PRICING.md` in each
project). Offering contributed code under the second and third requires a license from every
contributor, so outside contributions need a signed CLA.

- [`CLA.md`](../CLA.md): the agreement (English, canonical).
- [`CLA.ru.md`](../CLA.ru.md): Russian translation; the English text prevails.
- [`.github/workflows/cla.yml`](../.github/workflows/cla.yml): the check, reusable from every
  repository through a thin caller.

> These are drafts, not legal advice. Have a lawyer review `CLA.md` before it is used.

## What the agreement says

The contributor keeps the copyright and grants Ivan Tugay a perpetual, worldwide,
non-exclusive, royalty-free, irrevocable copyright license, with the right to sublicense and to
license the contribution under any terms (GPL, the royalty-free and commercial licenses,
proprietary products), plus an Apache-style patent license. In return, a contribution in a
published GPL version of a project stays available in that project under `GPL-3.0-only`. It also
covers: representations (original work, employer permission), no obligation to use, no warranty,
a moral rights waiver where the law allows it (a consent to the uses otherwise), transfer to a
successor, and signing by pull request comment. One signature covers all pyrlyn repositories.

It is modelled on the Apache Individual CLA v2.2 (copyright and patent grants, representations),
the Harmony Agreements HA-CLA-I with the "any license" outbound option (relicensing plus a
commitment to keep the project license), and the SAP CLA that CLA Assistant uses by default
(signing by GitHub identity). The text is new; none of it is copied.

## How signing works

1. A contributor opens a pull request. The `cla` check lists every committer that is neither in
   the allowlist nor in the signatures file, posts a comment with a link to `CLA.md` and fails.
2. Each of them comments on the pull request, signed in to GitHub, exactly:
   `I have read the CLA Document and I hereby sign the CLA`
3. The comment triggers the check again (`issue_comment`). It appends the signer's GitHub login,
   numeric user ID, comment ID, time, repository ID and pull request number to the signatures
   file, then re-runs the failed `pull_request_target` run so the pull request's status turns
   green. Commenting `recheck` runs the check again without signing.
4. After the merge, the pull request conversation is locked so signature comments cannot be
   edited or deleted (`lock-after-merge`, default `true`).

Allowlisted (never asked to sign): `dependabot[bot]`, `github-actions[bot]`, `renovate[bot]`
and `listepo` (the copyright holder). Dependabot pull requests skip the job entirely: Dependabot
runs get no Actions secrets, and a skipped required check counts as passed. Commits whose email
is not linked to a GitHub account cannot sign; the comment tells the author to link the email.

## Why CLA Assistant Lite (the GitHub Action)

| | CLA Assistant Lite (`contributor-assistant/github-action`) | cla-assistant.io (GitHub App) |
| --- | --- | --- |
| Runs | in our own Actions, pinned by SHA | SAP-hosted service |
| Signatures | JSON file in a repository we own | the service's database |
| Install | a workflow file + one secret | an org owner installs the app and grants OAuth access |
| Third party with org access | none | yes (reads members, writes statuses) |
| Signing | PR comment with a fixed phrase | "Sign in with GitHub" web page |

Both record the GitHub identity of the signer. The Action was chosen because it needs no
third-party app with access to the organization, the signature records stay in our own
repository (versioned, exportable, readable by a lawyer), and it fits how infra already works:
one reusable workflow, thin callers pinned by SHA.

The alternative, for reference: Ivan signs in at <https://cla-assistant.io> with GitHub,
authorizes the OAuth app for the `pyrlyn` organization (Settings > Third-party access, or the
app's install prompt), links each repository to a gist that holds the CLA text, and enables the
required status check `license/cla`. No workflow or secret is needed, but the signatures live in
SAP's service and the app keeps organization access.

### Maintenance risk

`contributor-assistant/github-action` was archived (read-only) on 2026-03-23. v2.6.1, the last
release, keeps working according to its maintainer, but it receives no fixes. Its `action.yml`
declares `node20`; since 2026-09-23 GitHub runs every JavaScript action on Node 24, so the action
runs on a runtime it was not released for. If it breaks, fork it into the organization (for
example `pyrlyn/cla-action`, Apache-2.0), set `runs.using: node24`, rebuild `dist/` and repin
`cla.yml` to the fork's commit. Community forks exist (e.g. `badideasforsale/cla-github-action`
v3), but none is established enough to trust with a write token today.

## Where signatures are stored

Recommended: a dedicated **private repository `pyrlyn/cla-signatures`**, branch `main`, file
`signatures/v1/cla.json` (the defaults of `cla.yml`).

- The token only needs write access to that one repository. Storing signatures on a branch of
  `pyrlyn/infra` (e.g. `cla-signatures`) would need a token that can push to infra, the repository
  every workflow pin points into; a leaked token there is far more dangerous.
- Signature commits stay out of the infra history and its pins, Dependabot and `sync-docs`.
- One file for the whole organization: a contributor signs once for every repository.
- Private keeps the list of signers from being scraped. The evidence itself (the signing comment
  on the pull request) stays public either way.

`main` of `pyrlyn/cla-signatures` must **not** be protected (the action commits to it directly),
and the repository must not be empty (create it with a README). Do not create `cla.json` by hand:
the action creates it on the first run. Back it up like any legal record.

## Turning the check on and off

The check is off by default: the `cla` job runs only when the variable `CLA_ENABLED` is `true`
(organization or repository variable, Settings > Secrets and variables > Actions > Variables).
While it is unset the job is skipped, and a skipped check passes, so it never blocks a pull
request. Set `CLA_ENABLED=true` only after the setup below is done.

## Setup (manual, by an organization owner)

Nothing below has been done yet.

### 1. Create the signatures repository

```sh
gh repo create pyrlyn/cla-signatures --private --add-readme \
  --description "CLA signatures for pyrlyn repositories (written by the cla workflow)"
```

### 2. Create the token and the secret `CLA_SIGNATURES_TOKEN`

The check uses two tokens:

- `github.token` of the repository the pull request is in, for comments, the status, the re-run
  and the lock. The caller job grants it `actions: write`, `contents: read`,
  `pull-requests: write`, `statuses: write`. Nothing to create.
- `CLA_SIGNATURES_TOKEN`, for reading and committing the signatures file in the other repository.
  Create a **fine-grained personal access token** (GitHub > Settings > Developer settings >
  Fine-grained tokens):
  - Resource owner: `pyrlyn` (the organization must allow fine-grained tokens: Organization
    settings > Personal access tokens; approve the request if approval is required).
  - Repository access: only `pyrlyn/cla-signatures`.
  - Repository permissions: **Contents: Read and write** (Metadata: Read is added automatically).
    No other permission; it does not need pull request access, which `github.token` covers.
  - Expiration: up to a year; put a reminder to rotate it.

Store it as an organization secret, visible to the repositories that run the check:

```sh
gh secret set CLA_SIGNATURES_TOKEN --org pyrlyn --visibility selected \
  --repos cox,rtok,ketch,runa,crates-packages,infra
```

(`gh` prompts for the value.) On the Free plan organization secrets reach public repositories
only; all of these are public. Until the secret exists, every `cla` run fails with "Please add a
personal access token".

A GitHub App instead of a PAT (no personal expiry, a bot identity): create an organization app
with Repository permissions **Contents: Read and write**, install it on `pyrlyn/cla-signatures`
only, store its ID and private key as `CLA_APP_ID` / `CLA_APP_PRIVATE_KEY`, and add an
`actions/create-github-app-token` step (owner `pyrlyn`, repositories `cla-signatures`) before the
CLA step in `cla.yml`, passing its token as `PERSONAL_ACCESS_TOKEN`. That is a change to
`cla.yml`; the PAT is the documented default of the action.

### 3. Add the caller to each repository

`.github/workflows/cla.yml` in the calling repository (keep `name: cla` unique there):

```yaml
name: cla

on:
  pull_request_target:
    types: [opened, synchronize, reopened, closed]
  issue_comment:
    types: [created]

permissions:
  contents: read

jobs:
  cla:
    uses: pyrlyn/infra/.github/workflows/cla.yml@<full commit sha>
    permissions:
      actions: write
      contents: read
      pull-requests: write
      statuses: write
    secrets:
      CLA_SIGNATURES_TOKEN: ${{ secrets.CLA_SIGNATURES_TOKEN }}
```

Inputs (all optional): `document-url`, `signatures-organization`, `signatures-repository`,
`signatures-branch`, `signatures-path`, `allowlist` (comma-separated, `*` wildcard),
`lock-after-merge`. `pyrlyn/infra` itself runs `cla.yml` directly on its own pull requests.

`pull_request_target` and `issue_comment` always run the workflow file of the **default
branch**, so the check starts working only after the caller is merged, and changes to a caller in
a pull request do not affect that pull request. The workflow never checks out pull request code;
keep it that way, since `pull_request_target` gives fork pull requests access to the secret.

Actions must be enabled in the repository (crates-packages has them disabled today).

### 4. Make the check required

Do this after steps 1-3, or every pull request is blocked.

Organization rulesets need GitHub Team (the `pyrlyn` organization is on Free; the API answers
403), so add the check to each repository's existing `protect-main` ruleset, which already
requires `gate`:

1. Repository > Settings > Rules > Rulesets > `protect-main`.
2. Require status checks to pass > Add checks > `cla / cla` (the caller job `cla` and the
   reusable job `cla`; it appears in the list after the check has run once in the repository).
   In `pyrlyn/infra` itself the check is named `cla`.
3. Save. Keep "Do not require status checks on creation" as it is.

With `gh` (replace the ruleset ID; `gh api repos/pyrlyn/<repo>/rulesets` lists them), fetch the
ruleset, add `{"context": "cla / cla"}` to the `required_status_checks` rule's
`parameters.required_status_checks`, and send the whole ruleset back:

```sh
gh api repos/pyrlyn/<repo>/rulesets/<id> > ruleset.json
# edit ruleset.json, then:
jq '{name, target, enforcement, conditions, rules, bypass_actors}' ruleset.json \
  | gh api -X PUT repos/pyrlyn/<repo>/rulesets/<id> --input -
```

Pull requests opened by `bump.yml`, `release-plz.yml` and `revert-on-failure` are opened with
Ivan's token and committed as `github-actions[bot]` or `listepo`, both allowlisted, so the
required check passes for them. If a bot account
that is not allowlisted starts committing, add it to `allowlist`.

## Troubleshooting

- **"Please add a personal access token"**: `CLA_SIGNATURES_TOKEN` is missing or not visible to
  the repository.
- **"Could not retrieve repository contents" / 403 / 404**: the token cannot read or write
  `pyrlyn/cla-signatures`, or the repository is empty or `main` is protected.
- **Signed, but the status is still red**: comment `recheck`, or re-run the failed `cla` run. Two
  signatures committed at the same moment can conflict; `recheck` retries.
- **"Unable to locate this workflow's ID"**: another workflow in the repository is also named
  `cla`, or the caller was renamed.
