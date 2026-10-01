#!/usr/bin/env bash
# Post a notification to a labeled tracking issue in $GH_REPO: comment on the open issue that
# carries $MARKER (the dedup key, hidden in its body), or open one, then assign $WHO. Shared by
# the notify-release-failure action and .github/scripts/failure-digest.sh.
#
# Env: GH_REPO, GH_TOKEN (issues: write), TITLE, MARKER, LABEL, LABEL_COLOR,
# LABEL_DESCRIPTION, WHO, BODY_FILE (the notification, mentioning $WHO),
# ISSUE_BODY_FILE (optional: body of a new issue, which then gets BODY_FILE as its first
# comment; empty opens the issue with BODY_FILE as its body).
# Prints the issue number on stdout.
set -euo pipefail
: "${GH_REPO:?}" "${TITLE:?}" "${MARKER:?}" "${LABEL:?}" "${WHO:?}" "${BODY_FILE:?}"

gh label create "$LABEL" --color "${LABEL_COLOR:-B60205}" \
  --description "${LABEL_DESCRIPTION:-}" >/dev/null 2>&1 || true
issue="$(gh issue list --state open --label "$LABEL" --limit 100 --json number,body \
  | jq -r --arg m "$MARKER" '[.[] | select(.body | contains($m))][0].number // empty')"

new_issue() { # <body file>
  local body
  body="$(mktemp)"
  { cat "$1"; printf '\n%s\n' "$MARKER"; } >"$body"
  local url
  url="$(gh issue create --title "$TITLE" --label "$LABEL" --body-file "$body")"
  rm -f "$body"
  echo "::notice title=$LABEL::opened $url" >&2
  issue="${url##*/}"
}

if [ -n "$issue" ]; then
  gh issue comment "$issue" --body-file "$BODY_FILE" >/dev/null
  echo "::notice title=$LABEL::commented on #$issue" >&2
elif [ -n "${ISSUE_BODY_FILE:-}" ]; then
  new_issue "$ISSUE_BODY_FILE"
  gh issue comment "$issue" --body-file "$BODY_FILE" >/dev/null
  echo "::notice title=$LABEL::commented on #$issue" >&2
else
  new_issue "$BODY_FILE"
fi
# Assigning can fail (not a collaborator); the mention still notifies.
gh issue edit "$issue" --add-assignee "$WHO" >/dev/null \
  || echo "::warning::could not assign $WHO to #$issue" >&2
echo "$issue"
