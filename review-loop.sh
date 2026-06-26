#!/usr/bin/env bash
# Usage: ./review-loop.sh
# Smoke:
#   printf '%s\n' '{"verdict":"APPROVE","blocking_comments":[],"non_blocking_comments":[],"missing_tests":[],"follow_up_patch_plan":[]}' | ./review-loop.sh --dry-run-verdict
#   printf '%s\n' '{"verdict":"BLOCK MERGE","blocking_comments":[{"file":"review-loop.sh","location":"x","issue":"x","why_it_matters":"x","suggested_fix":"x"}],"non_blocking_comments":[],"missing_tests":[],"follow_up_patch_plan":[]}' | ./review-loop.sh --dry-run-verdict
#   printf '%s\n' 'not json' | ./review-loop.sh --dry-run-verdict
#   ./review-loop.sh --dry-run-verdict-schema-test
#   ./review-loop.sh --dry-run-missing-test-allowlist-test
#   ./review-loop.sh --dry-run-blocking-comment-allowlist-test
#   ./review-loop.sh --dry-run-sandbox-gate
#   ./review-loop.sh --dry-run-blocking-fix-sandbox-gate-test
# Run from the project root, on a PR branch checked out via `gh pr checkout <n>`.
set -euo pipefail

validate_review_output_schema() {
  local source="${1:-}"
  local jq_filter='
    def string_array: type == "array" and all(.[]; type == "string");
    type == "object"
    and (keys | sort == ["blocking_comments", "follow_up_patch_plan", "missing_tests", "non_blocking_comments", "verdict"])
    and (.verdict | type == "string" and IN("APPROVE", "APPROVE WITH COMMENTS", "REQUEST CHANGES", "BLOCK MERGE"))
    and (.blocking_comments | type == "array")
    and all(.blocking_comments[]; type == "object"
      and (keys | sort == ["file", "issue", "location", "suggested_fix", "why_it_matters"])
      and (.file | type == "string")
      and (.location | type == "string")
      and (.issue | type == "string")
      and (.why_it_matters | type == "string")
      and (.suggested_fix | type == "string"))
    and (.non_blocking_comments | string_array)
    and (.missing_tests | string_array)
    and (.follow_up_patch_plan | string_array)
  '

  if [[ -n "$source" ]]; then
    jq -e "$jq_filter" "$source" >/dev/null
  else
    jq -e "$jq_filter" >/dev/null
  fi
}

require_disposable_sandbox_for_pr_checks() {
  if [[ "${I_AM_IN_A_DISPOSABLE_SANDBOX:-}" != "1" ]]; then
    echo "Refusing to run PR-controlled package scripts outside a disposable sandbox."
    echo "Set I_AM_IN_A_DISPOSABLE_SANDBOX=1 only inside an isolated disposable environment."
    return 1
  fi
}

require_disposable_sandbox_for_fix_pass() {
  if [[ "${I_AM_IN_A_DISPOSABLE_SANDBOX:-}" != "1" ]]; then
    echo "Refusing to run an automated fix pass outside a disposable sandbox."
    echo "Set I_AM_IN_A_DISPOSABLE_SANDBOX=1 only inside an isolated disposable environment."
    return 1
  fi
}

extract_missing_test_paths() {
  jq -r '
    .missing_tests[]
    | scan("[A-Za-z0-9._/@+-]+/(?:[A-Za-z0-9._@+-]+/)*[A-Za-z0-9._@+-]*(?:test|spec)[A-Za-z0-9._@+-]*\\.[A-Za-z0-9._@+-]+")
    | sub("[).,;:]+$"; "")
  ' | sort -u
}

is_mergeable_verdict() {
  [[ "$1" == "APPROVE" || "$1" == "APPROVE WITH COMMENTS" ]]
}

validate_allowed_fix_paths() {
  local path

  for path in "$@"; do
    if [[ -z "$path" || "$path" == /* || "$path" == *'..'* || "$path" =~ [[:cntrl:]] || ! "$path" =~ ^[A-Za-z0-9._/@+-]+$ ]]; then
      echo "Blocking comment has an unsafe file path: $path"
      exit 1
    fi
  done
}

validate_blocking_comment_file_paths() {
  jq -e '
    all(.[]; .file
      | type == "string"
      and (explode | all(. > 31 and . != 127))
      and test("^[A-Za-z0-9._/@+-]+$")
      and (startswith("/") | not)
      and (contains("..") | not))
  ' >/dev/null
}

ensure_only_allowed_files_changed() {
  local changed allowed path
  local -a changed_files=()
  local -a allowed_files=("$@")

  mapfile -t changed_files < <(
    {
      git diff --name-only
      git diff --cached --name-only
      git ls-files --others --exclude-standard
    } | sort -u
  )

  if [ "${#changed_files[@]}" -eq 0 ]; then
    return 0
  fi

  echo "-- Changed files after fix --"
  printf '%s\n' "${changed_files[@]}"

  for changed in "${changed_files[@]}"; do
    allowed=0
    for path in "${allowed_files[@]}"; do
      if [[ "$changed" == "$path" ]]; then
        allowed=1
        break
      fi
    done

    if [ "$allowed" -ne 1 ]; then
      echo "Refusing to commit changes outside blocking comment file allowlist: $changed"
      echo "Allowed files:"
      printf '%s\n' "${allowed_files[@]}"
      exit 1
    fi
  done
}

assert_dry_run_verdict_rejects() {
  local input="$1"

  if printf '%s\n' "$input" | "$0" --dry-run-verdict >/dev/null 2>&1; then
    echo "Expected --dry-run-verdict to reject: $input"
    exit 1
  fi
}

if [[ "${1:-}" == "--dry-run-verdict-schema-test" ]]; then
  assert_dry_run_verdict_rejects '{}'
  assert_dry_run_verdict_rejects '{"verdict":"APPROVE"}'
  assert_dry_run_verdict_rejects '{"verdict":"BLOCK MERGE","blocking_comments":[],"non_blocking_comments":[],"missing_tests":["Add regression coverage in clinic/src/lib/review-loop.test.ts"],"follow_up_patch_plan":[]}'
  assert_dry_run_verdict_rejects '{"verdict":"REQUEST CHANGES","blocking_comments":[],"non_blocking_comments":[],"missing_tests":[],"follow_up_patch_plan":[]}'
  echo "Dry-run verdict schema gate rejects invalid review JSON."
  exit 0
fi

if [[ "${1:-}" == "--dry-run-missing-test-allowlist-test" ]]; then
  command -v git >/dev/null 2>&1 || { echo "Missing dependency: git"; exit 1; }
  command -v jq >/dev/null 2>&1 || { echo "Missing dependency: jq"; exit 1; }
  TEST_REPO=$(mktemp -d -t review-loop-allowlist.XXXXXX)
  (
    cd "$TEST_REPO"
    git init --quiet
    REVIEW_OUTPUT='{"verdict":"BLOCK MERGE","blocking_comments":[{"file":"src/app.ts","location":"x","issue":"x","why_it_matters":"x","suggested_fix":"x"}],"non_blocking_comments":[],"missing_tests":["Add the regression test in tests/app.test.ts."],"follow_up_patch_plan":[]}'
    BLOCKING_JSON=$(printf '%s\n' "$REVIEW_OUTPUT" | jq '.blocking_comments')
    mapfile -t ALLOWED_FIX_PATHS < <(echo "$BLOCKING_JSON" | jq -r '.[].file')
    mapfile -t MISSING_TEST_PATHS < <(printf '%s\n' "$REVIEW_OUTPUT" | extract_missing_test_paths)
    ALLOWED_FIX_PATHS+=("${MISSING_TEST_PATHS[@]}")
    validate_allowed_fix_paths "${ALLOWED_FIX_PATHS[@]}"
    mkdir -p tests
    touch tests/app.test.ts
    ensure_only_allowed_files_changed "${ALLOWED_FIX_PATHS[@]}"
  )
  echo "Missing-test paths are accepted by the fix allowlist."
  exit 0
fi

if [[ "${1:-}" == "--dry-run-blocking-comment-allowlist-test" ]]; then
  command -v jq >/dev/null 2>&1 || { echo "Missing dependency: jq"; exit 1; }
  REVIEW_OUTPUT=$'{"verdict":"BLOCK MERGE","blocking_comments":[{"file":"src/app.ts\\npackage.json","location":"x","issue":"x","why_it_matters":"x","suggested_fix":"x"}],"non_blocking_comments":[],"missing_tests":[],"follow_up_patch_plan":[]}'
  BLOCKING_JSON=$(printf '%s\n' "$REVIEW_OUTPUT" | jq '.blocking_comments')
  if echo "$BLOCKING_JSON" | validate_blocking_comment_file_paths; then
    echo "Blocking comment path allowlist accepted an embedded newline."
    exit 1
  fi
  echo "Blocking comment path allowlist rejects embedded newlines."
  exit 0
fi

if [[ "${1:-}" == "--dry-run-verdict" ]]; then
  command -v jq >/dev/null 2>&1 || { echo "Missing dependency: jq"; exit 1; }
  if ! REVIEW_OUTPUT=$(jq -c . 2>/dev/null); then
    echo "Malformed review JSON."
    exit 2
  fi
  if ! printf '%s\n' "$REVIEW_OUTPUT" | validate_review_output_schema; then
    echo "Review JSON does not match the required schema."
    exit 2
  fi
  VERDICT=$(echo "$REVIEW_OUTPUT" | jq -r '.verdict')
  BLOCKING_COUNT=$(echo "$REVIEW_OUTPUT" | jq '.blocking_comments | length')
  echo "Verdict: $VERDICT | Blocking comments: $BLOCKING_COUNT"
  if [ "$BLOCKING_COUNT" -eq 0 ]; then
    if is_mergeable_verdict "$VERDICT"; then
      echo "No blocking comments. Review loop complete."
      exit 0
    fi
    echo "Non-mergeable verdict has no actionable blocking comments."
    exit 2
  fi
  require_disposable_sandbox_for_fix_pass
  echo "-- Blocking issues present. Starting scoped fix pass. --"
  exit 20
fi

if [[ "${1:-}" == "--dry-run-sandbox-gate" ]]; then
  if (unset I_AM_IN_A_DISPOSABLE_SANDBOX; require_disposable_sandbox_for_pr_checks >/dev/null 2>&1); then
    echo "Sandbox gate failed open."
    exit 1
  fi
  echo "Sandbox gate fails closed before PR-controlled checks."
  exit 0
fi

if [[ "${1:-}" == "--dry-run-blocking-fix-sandbox-gate-test" ]]; then
  BLOCKING_REVIEW='{"verdict":"BLOCK MERGE","blocking_comments":[{"file":"review-loop.sh","location":"x","issue":"x","why_it_matters":"x","suggested_fix":"x"}],"non_blocking_comments":[],"missing_tests":[],"follow_up_patch_plan":[]}'
  OUTPUT=$(unset I_AM_IN_A_DISPOSABLE_SANDBOX; printf '%s\n' "$BLOCKING_REVIEW" | "$0" --dry-run-verdict 2>&1) && {
    echo "Blocking fix dry-run did not fail closed outside a disposable sandbox."
    exit 1
  }
  if grep -q "Starting scoped fix pass" <<< "$OUTPUT"; then
    echo "Blocking fix dry-run reached the fix pass before enforcing the sandbox gate."
    exit 1
  fi
  echo "Blocking fix pass fails closed before the fix command can run."
  exit 0
fi

for cmd in gh codex git jq mktemp npm; do
  command -v "$cmd" >/dev/null 2>&1 || { echo "Missing dependency: $cmd"; exit 1; }
done

if [[ "${I_AM_IN_A_DISPOSABLE_SANDBOX:-}" == "1" ]]; then
  CODEX_EXEC_SANDBOX_ARGS=(--dangerously-bypass-approvals-and-sandbox)
else
  CODEX_EXEC_SANDBOX_ARGS=(--sandbox workspace-write)
fi

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

git fetch origin "+refs/heads/$BASE_BRANCH:refs/remotes/origin/$BASE_BRANCH" --quiet || { echo "git fetch failed for $BASE_BRANCH"; exit 1; }
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
  require_disposable_sandbox_for_pr_checks > "$1" 2>&1 || return $?
  command -v cargo >/dev/null 2>&1 || { echo "Missing dependency: cargo" >> "$1"; return 1; }

  if [[ "${ALLOW_NPM_INSTALL_SCRIPTS:-}" == "1" ]]; then
    NPM_CI_ARGS=(ci)
  else
    NPM_CI_ARGS=(ci --ignore-scripts)
  fi

  export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"

  { (cd clinic && npm "${NPM_CI_ARGS[@]}" && npm run build && npm exec -- vitest run) \
      && (cd clinic/src-tauri && cargo check) \
      && (cd clinic/src-tauri && cargo test) ; } >> "$1" 2>&1
}

for i in $(seq 1 "$MAX_ITERS"); do
  echo "== Review pass $i/$MAX_ITERS (fresh session, no memory of prior fixes) =="

  REVIEW_JSON="$LOG_DIR/review-$i.json"
  REVIEW_STDOUT="$LOG_DIR/review-$i-stdout.txt"

  codex exec "${CODEX_EXEC_SANDBOX_ARGS[@]}" --ephemeral \
    --output-schema "$SCHEMA_FILE" \
    --output-last-message "$REVIEW_JSON" \
    "$REVIEW_PROMPT" > "$REVIEW_STDOUT" \
    || { echo "Codex review call failed."; exit 1; }

  if ! jq -e . "$REVIEW_JSON" >/dev/null 2>&1; then
    echo "Got non-JSON final message from codex review call on pass $i."
    echo "Final message saved to: $REVIEW_JSON"
    echo "CLI stdout saved to: $REVIEW_STDOUT"
    exit 1
  fi
  if ! validate_review_output_schema "$REVIEW_JSON"; then
    echo "Codex review JSON did not match the required schema on pass $i."
    echo "Final message saved to: $REVIEW_JSON"
    echo "CLI stdout saved to: $REVIEW_STDOUT"
    exit 1
  fi

  VERDICT=$(jq -r '.verdict' "$REVIEW_JSON")
  BLOCKING_COUNT=$(jq '.blocking_comments | length' "$REVIEW_JSON")

  echo "Verdict: $VERDICT | Blocking comments: $BLOCKING_COUNT"

  if [ "$BLOCKING_COUNT" -eq 0 ]; then
    if ! is_mergeable_verdict "$VERDICT"; then
      echo "Non-mergeable verdict has no actionable blocking comments on pass $i. Do not merge. Logs in $LOG_DIR"
      exit 1
    fi
    echo "-- Running real checks before declaring PR mergeable --"
    CHECK_LOG="$LOG_DIR/checks-$i.log"
    if run_checks "$CHECK_LOG"; then
      CHECK_STATUS=0
    else
      CHECK_STATUS=$?
    fi
    if [ "$CHECK_STATUS" -ne 0 ]; then
      echo "Verification failed. Check log: $CHECK_LOG"
      if [ -f "$CHECK_LOG" ]; then
        echo "Last 200 lines:"
        tail -n 200 "$CHECK_LOG" || true
      else
        echo "Check log was not created."
      fi
      exit "$CHECK_STATUS"
    fi
    if [[ "$VERDICT" == "APPROVE" ]]; then
      echo "Clean approval on pass $i. Safe to merge PR #${PR_NUMBER:-?}."
    else
      echo "No blocking comments on pass $i. Verdict: $VERDICT. Safe to merge PR #${PR_NUMBER:-?} with comments."
    fi
    exit 0
  fi

  if [ "$i" -eq "$MAX_ITERS" ]; then
    echo "Hit $MAX_ITERS iterations without clean approval. Do not merge. Logs in $LOG_DIR"
    exit 1
  fi

  require_disposable_sandbox_for_fix_pass
  echo "-- Blocking issues present. Starting scoped fix pass. --"
  BLOCKING_JSON=$(jq '.blocking_comments' "$REVIEW_JSON")
  echo "$BLOCKING_JSON" | validate_blocking_comment_file_paths
  mapfile -t ALLOWED_FIX_PATHS < <(echo "$BLOCKING_JSON" | jq -r '.[].file' | sort -u)
  mapfile -t MISSING_TEST_PATHS < <(jq '.' "$REVIEW_JSON" | extract_missing_test_paths)
  ALLOWED_FIX_PATHS+=("${MISSING_TEST_PATHS[@]}")
  validate_allowed_fix_paths "${ALLOWED_FIX_PATHS[@]}"

  FIX_PROMPT="Fix ONLY the issues in this JSON array of blocking review comments (piped in above). Do not refactor outside the named files/functions. Stop once done -- do not re-review or judge your own fix."

  if [[ "$VERDICT" == "BLOCK MERGE" ]]; then
    MISSING_TESTS=$(jq '.missing_tests' "$REVIEW_JSON")
    FIX_PROMPT="$FIX_PROMPT

Additionally, here are tests flagged as missing: $MISSING_TESTS
Add only the ones that directly cover the blocking issues above. Ignore the rest."
  fi

  echo "$BLOCKING_JSON" | codex exec "${CODEX_EXEC_SANDBOX_ARGS[@]}" "$FIX_PROMPT" \
    || { echo "Codex fix pass failed."; exit 1; }

  echo "-- Running real checks after fix --"
  CHECK_LOG="$LOG_DIR/checks-$i.log"
  if run_checks "$CHECK_LOG"; then
    CHECK_STATUS=0
  else
    CHECK_STATUS=$?
  fi

  if [ "$CHECK_STATUS" -ne 0 ]; then
    echo "Fix broke verification. Check log: $CHECK_LOG"
    echo "Last 200 lines:"
    tail -n 200 "$CHECK_LOG" || true
    exit "$CHECK_STATUS"
  fi

  echo "-- Committing fix before re-review --"
  ensure_only_allowed_files_changed "${ALLOWED_FIX_PATHS[@]}"
  mapfile -t CHANGED_FIX_PATHS < <(
    {
      git diff --name-only
      git diff --cached --name-only
      git ls-files --others --exclude-standard
    } | sort -u
  )
  if [ "${#CHANGED_FIX_PATHS[@]}" -gt 0 ]; then
    git add -- "${CHANGED_FIX_PATHS[@]}"
  fi
  if git diff --cached --quiet; then
    echo "Nothing to commit."
  else
    git commit -m "fix: address blocking review comments (pass $i)" --quiet
  fi

  echo "-- Re-reviewing from scratch next pass --"
done
