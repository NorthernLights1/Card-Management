#!/usr/bin/env bash
# Usage: ./review-loop.sh
# Run from the project root, on a PR branch checked out via `gh pr checkout <n>`.
set -euo pipefail

for cmd in gh codex git flutter dart jq mktemp; do
  command -v "$cmd" >/dev/null 2>&1 || { echo "Missing dependency: $cmd"; exit 1; }
done

if [ -n "$(git status --porcelain)" ]; then
  echo "Working tree has uncommitted changes. Commit or stash them before running this loop,"
  echo "so its auto-commits only ever contain its own fixes."
  exit 1
fi

PR_NUMBER=$(gh pr view --json number -q .number 2>/dev/null) || true
PR_TITLE=$(gh pr view --json title -q .title 2>/dev/null) || true
BASE_BRANCH=$(gh pr view --json baseRefName -q .baseRefName 2>/dev/null) || true

if [ -z "$BASE_BRANCH" ]; then
  echo "Couldn't detect base branch via 'gh pr view'."
  echo "Check out the PR first: gh pr checkout <number>"
  exit 1
fi

echo "Reviewing PR #${PR_NUMBER:-?}: ${PR_TITLE:-<unknown>}  -->  base: $BASE_BRANCH"

git fetch origin "$BASE_BRANCH" --quiet || { echo "git fetch failed for $BASE_BRANCH"; exit 1; }
COMPARE_REF="origin/${BASE_BRANCH}"

MAX_ITERS=5
LOG_DIR="$(mktemp -d -t codex-review-loop.XXXXXX)"
echo "Logs: $LOG_DIR"

SCHEMA_FILE="$LOG_DIR/review-schema.json"
cat > "$SCHEMA_FILE" << 'JSON_EOF'
{
  "type": "object",
  "additionalProperties": false,
  "properties": {
    "verdict": {
      "type": "string",
      "enum": ["APPROVE", "APPROVE WITH COMMENTS", "REQUEST CHANGES", "BLOCK MERGE"]
    },
    "blocking_comments": {
      "type": "array",
      "items": {
        "type": "object",
        "additionalProperties": false,
        "properties": {
          "file": {"type": "string"},
          "location": {"type": "string"},
          "issue": {"type": "string"},
          "why_it_matters": {"type": "string"},
          "suggested_fix": {"type": "string"}
        },
        "required": ["file", "location", "issue", "why_it_matters", "suggested_fix"]
      }
    },
    "non_blocking_comments": {"type": "array", "items": {"type": "string"}},
    "missing_tests": {"type": "array", "items": {"type": "string"}},
    "follow_up_patch_plan": {"type": "array", "items": {"type": "string"}}
  },
  "required": ["verdict", "blocking_comments", "non_blocking_comments", "missing_tests", "follow_up_patch_plan"]
}
JSON_EOF

REVIEW_PROMPT="Act as a strict PR reviewer.

Use the architecture review context if available.

Review the current branch against ${COMPARE_REF}.

Run:
- git status
- git diff --stat ${COMPARE_REF}
- git diff ${COMPARE_REF}
- git log --oneline -n 10

Then inspect any relevant surrounding files needed to understand the change.

Return only actionable review comments, populated according to the provided output schema.

Do not edit files unless explicitly asked."

run_checks() {
  # $1 = output file
  { flutter pub run build_runner build --delete-conflicting-outputs \
      && flutter analyze \
      && flutter test ; } > "$1" 2>&1
}

for i in $(seq 1 "$MAX_ITERS"); do
  echo "== Review pass $i/$MAX_ITERS (fresh session, no memory of prior fixes) =="

  REVIEW_OUTPUT=$(codex exec --dangerously-bypass-approvals-and-sandbox --ephemeral \
    --output-schema "$SCHEMA_FILE" "$REVIEW_PROMPT") \
    || { echo "Codex review call failed."; exit 1; }

  if ! echo "$REVIEW_OUTPUT" | jq -e . >/dev/null 2>&1; then
    echo "$REVIEW_OUTPUT" > "$LOG_DIR/review-$i-raw.txt"
    echo "Got non-JSON output from --output-schema on pass $i."
    echo "My assumption about --output-schema's output format may not match your installed CLI version."
    echo "Raw output saved to: $LOG_DIR/review-$i-raw.txt -- inspect it before re-running."
    exit 1
  fi
  echo "$REVIEW_OUTPUT" > "$LOG_DIR/review-$i.json"

  VERDICT=$(echo "$REVIEW_OUTPUT" | jq -r '.verdict')
  BLOCKING_COUNT=$(echo "$REVIEW_OUTPUT" | jq '.blocking_comments | length')

  echo "Verdict: $VERDICT | Blocking comments: $BLOCKING_COUNT"

  if [[ "$VERDICT" == "APPROVE" && "$BLOCKING_COUNT" -eq 0 ]]; then
    echo "Clean approval on pass $i. Safe to merge PR #${PR_NUMBER:-?}."
    exit 0
  fi

  if [ "$i" -eq "$MAX_ITERS" ]; then
    echo "Hit $MAX_ITERS iterations without clean approval. Do not merge. Logs in $LOG_DIR"
    exit 1
  fi

  echo "-- Blocking issues present. Starting scoped fix pass. --"
  BLOCKING_JSON=$(echo "$REVIEW_OUTPUT" | jq '.blocking_comments')
  FIX_PROMPT="Fix ONLY the issues in this JSON array of blocking review comments (piped in above). Do not refactor outside the named files/functions. Stop once done -- do not re-review or judge your own fix."

  if [[ "$VERDICT" == "BLOCK MERGE" ]]; then
    MISSING_TESTS=$(echo "$REVIEW_OUTPUT" | jq '.missing_tests')
    FIX_PROMPT="$FIX_PROMPT

Additionally, here are tests flagged as missing: $MISSING_TESTS
Add only the ones that directly cover the blocking issues above. Ignore the rest."
  fi

  echo "$BLOCKING_JSON" | codex exec --dangerously-bypass-approvals-and-sandbox "$FIX_PROMPT" \
    || { echo "Codex fix pass failed."; exit 1; }

  echo "-- Running real checks after fix --"
  CHECK_LOG="$LOG_DIR/checks-$i.log"
  if run_checks "$CHECK_LOG"; then
    CHECK_STATUS=0
  else
    CHECK_STATUS=$?
  fi

  if [ "$CHECK_STATUS" -ne 0 ]; then
    echo "Fix broke verification. Feeding last 200 lines of real output back to the same fix session."
    tail -n 200 "$CHECK_LOG" | codex exec resume --last --dangerously-bypass-approvals-and-sandbox \
      "Your fix broke verification. Here is the real output (last 200 lines) above. Fix the root cause without reintroducing the blocking issue you just fixed." \
      || { echo "Codex resume fix pass failed."; exit 1; }
  fi

  echo "-- Committing fix before re-review --"
  git add -A
  git commit -m "fix: address blocking review comments (pass $i)" --quiet || echo "Nothing to commit."

  echo "-- Re-reviewing from scratch next pass --"
done
