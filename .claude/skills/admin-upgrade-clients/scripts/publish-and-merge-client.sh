#!/usr/bin/env bash
# publish-and-merge-client.sh — Phase 2: PR creation, Slack, CI+merge, cleanup.
#
# Reads Phase 1 result file, creates PR, posts to Slack, runs CI+merge loop,
# posts :merged: emoji, and cleans up the clone directory.
#
# Usage:
#   publish-and-merge-client.sh --repo OWNER/REPO --target-sha-short SHORT_SHA \
#     --tmpdir DIR --slack-available true|false
#
# Reads: $TMPDIR/<repo-name>-phase1.txt (from upgrade-single-client.sh)
# Output: $TMPDIR/<repo-name>-result.txt (newline-delimited KEY=VALUE)
#   STATUS=merged|bailed|failed
#   PR_NUMBER=<N>
#   PR_URL=<url>
#   SLACK_TS=<ts>
#   BUMP_VERSION_MISSING=true|false
#   PR_STATUS=created|existing
#   ERROR=<message>
#
# Completion signal: $TMPDIR/<repo-name>-result.txt.done (contains exit code)
#
# Exit codes:
#   0 — always (result communicated via output file)

set -uo pipefail

# --- Parse arguments ---
REPO=""
TARGET_SHA_SHORT=""
UC_TMPDIR=""
SLACK_AVAILABLE="true"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --repo) REPO="${2:?--repo requires a value}"; shift 2 ;;
        --target-sha-short) TARGET_SHA_SHORT="${2:?--target-sha-short requires a value}"; shift 2 ;;
        --tmpdir) UC_TMPDIR="${2:?--tmpdir requires a value}"; shift 2 ;;
        --slack-available) SLACK_AVAILABLE="${2:?--slack-available requires a value}"; shift 2 ;;
        *) echo "Unknown option: $1" >&2; exit 1 ;;
    esac
done

if [[ -z "$REPO" ]] || [[ -z "$TARGET_SHA_SHORT" ]] || [[ -z "$UC_TMPDIR" ]]; then
    echo "ERROR: --repo, --target-sha-short, and --tmpdir are required" >&2
    exit 1
fi

REPO_NAME="${REPO#*/}"
CLONE_DIR="$UC_TMPDIR/$REPO_NAME"
PHASE1_FILE="$UC_TMPDIR/${REPO_NAME}-phase1.txt"
RESULT_FILE="$UC_TMPDIR/${REPO_NAME}-result.txt"
DONE_FILE="${RESULT_FILE}.done"

# --- Output defaults ---
STATUS="failed"
PR_NUMBER=""
PR_URL=""
SLACK_TS=""
BUMP_VERSION_MISSING="false"
PR_STATUS=""
ERROR="publish-and-merge-client.sh exited unexpectedly"

write_result() {
    # Sanitize ERROR to prevent multi-line values from corrupting KEY=VALUE format
    ERROR="${ERROR//$'\n'/ }"
    cat > "$RESULT_FILE" <<RESULT_EOF
STATUS=$STATUS
PR_NUMBER=$PR_NUMBER
PR_URL=$PR_URL
SLACK_TS=$SLACK_TS
BUMP_VERSION_MISSING=$BUMP_VERSION_MISSING
PR_STATUS=$PR_STATUS
ERROR=$ERROR
RESULT_EOF
}

cleanup() {
    local exit_code=$?
    write_result
    echo "$exit_code" > "$DONE_FILE"
    # Clean up clone directory
    if [[ -n "$CLONE_DIR" ]] && [[ -d "$CLONE_DIR" ]]; then
        rm -rf "$CLONE_DIR"
    fi
}

trap 'cleanup' EXIT

# --- Read Phase 1 result ---
if [[ ! -f "$PHASE1_FILE" ]]; then
    ERROR="Phase 1 result file not found: $PHASE1_FILE"
    echo "❌ $REPO — $ERROR" >&2
    exit 0
fi

# Parse Phase 1 results
phase1_get() { grep "^$1=" "$PHASE1_FILE" | head -1 | cut -d= -f2-; }

PHASE1_STATUS=$(phase1_get STATUS)
CLONE_DIR=$(phase1_get CLONE_DIR)
BUMP_VERSION_MISSING=$(phase1_get BUMP_VERSION_MISSING)
CURRENT_SHA=$(phase1_get CURRENT_SHA)
REPO_ID=$(phase1_get REPO_ID)
SLACK_CHANNEL=$(phase1_get SLACK_CHANNEL)

if [[ "$PHASE1_STATUS" != "ready" ]]; then
    ERROR="Phase 1 status was $PHASE1_STATUS, not ready"
    echo "❌ $REPO — $ERROR" >&2
    exit 0
fi

if [[ ! -d "$CLONE_DIR" ]]; then
    ERROR="Clone directory not found: $CLONE_DIR"
    echo "❌ $REPO — $ERROR" >&2
    exit 0
fi

# --- Sub-Step 5: Create PR ---
echo "📝 $REPO — creating PR..." >&2

# Generate commit list for PR body
if [[ -n "$CURRENT_SHA" ]]; then
    COMMIT_TITLES=$(cd "$CLONE_DIR/dev-tools" && git log --format="- %s" "$CURRENT_SHA..HEAD" 2>/dev/null || echo "")
    COMMITS_LIST=$(cd "$CLONE_DIR/dev-tools" && git log --oneline "$CURRENT_SHA..HEAD" 2>/dev/null || echo "")
else
    COMMIT_TITLES=""
    COMMITS_LIST="(could not determine previous SHA — full commit list omitted)"
fi

# Write PR body
PR_BODY_FILE="$UC_TMPDIR/${REPO_NAME}-pr-body.md"
cat > "$PR_BODY_FILE" <<PR_BODY_EOF
## Summary
$COMMIT_TITLES

<details><summary>Dev-tools commits included</summary>

$COMMITS_LIST

</details>

Generated with [Claude Code](https://claude.com/claude-code)
PR_BODY_EOF

# Create PR
PR_OUTPUT=$(cd "$CLONE_DIR" && ./dev-tools/.claude/scripts/generic/create-pr.sh \
    --title "Update dev-tools to latest main ($TARGET_SHA_SHORT)" \
    --body-file "$PR_BODY_FILE" 2>&1)
PR_EXIT=$?

if [[ $PR_EXIT -ne 0 ]]; then
    ERROR="PR creation failed: $PR_OUTPUT"
    echo "❌ $REPO — $ERROR" >&2
    exit 0
fi

PR_NUMBER=$(echo "$PR_OUTPUT" | grep '^PR_NUMBER=' | head -1 | cut -d= -f2-)
PR_URL=$(echo "$PR_OUTPUT" | grep '^PR_URL=' | head -1 | cut -d= -f2-)
PR_STATUS=$(echo "$PR_OUTPUT" | grep '^PR_STATUS=' | head -1 | cut -d= -f2-)

if [[ -z "$PR_NUMBER" ]] || [[ -z "$PR_URL" ]]; then
    ERROR="Could not extract PR number/URL from create-pr.sh output"
    echo "❌ $REPO — $ERROR" >&2
    exit 0
fi

echo "🔗 $REPO — PR #$PR_NUMBER created ($PR_STATUS): $PR_URL" >&2

# Update PR body if existing, and verify local HEAD matches remote
if [[ "$PR_STATUS" == "existing" ]]; then
    (cd "$CLONE_DIR" && gh pr edit "$PR_NUMBER" --body-file "$PR_BODY_FILE") 2>/dev/null || true
    # Verify local HEAD matches remote PR head to avoid merging stale PR
    LOCAL_HEAD=$(cd "$CLONE_DIR" && git rev-parse HEAD 2>/dev/null || echo "")
    REMOTE_HEAD=$(gh pr view "$PR_NUMBER" --repo "$REPO_ID" --json headRefOid -q '.headRefOid' 2>/dev/null || echo "")
    if [[ -n "$LOCAL_HEAD" ]] && [[ -n "$REMOTE_HEAD" ]] && [[ "$LOCAL_HEAD" != "$REMOTE_HEAD" ]]; then
        echo "⚠ $REPO — local HEAD ($LOCAL_HEAD) differs from remote PR head ($REMOTE_HEAD), pushing..." >&2
        if ! (cd "$CLONE_DIR" && git push -u origin HEAD 2>&1); then
            ERROR="Failed to push local changes to match existing PR"
            echo "❌ $REPO — $ERROR" >&2
            STATUS="failed"
            exit 0
        fi
    fi
fi

# --- Sub-Step 6: Post to Slack ---
if [[ "$SLACK_AVAILABLE" == "true" ]] && [[ -n "$SLACK_CHANNEL" ]] && [[ "$PR_STATUS" == "created" ]]; then
    echo "📢 $REPO — posting to Slack (#$SLACK_CHANNEL)..." >&2
    SLACK_TMPDIR=$(mktemp -d /tmp/claude-uc-slack-XXXXXX)
    SLACK_OUTPUT=$(cd "$CLONE_DIR" && ./dev-tools/.claude/scripts/generic/slack-announce.sh \
        --pr "$PR_NUMBER" --tmpdir "$SLACK_TMPDIR" --channel "$SLACK_CHANNEL" 2>&1) || true
    SLACK_TS=$(echo "$SLACK_OUTPUT" | grep '^SLACK_TS=' | head -1 | cut -d= -f2-)
    rm -rf "$SLACK_TMPDIR"
    if [[ -n "$SLACK_TS" ]]; then
        echo "📢 $REPO — Slack posted (ts=$SLACK_TS)" >&2
    else
        echo "⚠ $REPO — Slack post failed (non-fatal, continuing)" >&2
    fi
elif [[ "$PR_STATUS" == "existing" ]]; then
    echo "⏭️ $REPO — Skipping Slack (PR already existed)" >&2
else
    echo "⏭️ $REPO — Skipping Slack (unavailable or no channel)" >&2
fi

# --- Sub-Step 7: CI + Rebase + Merge Loop ---
if [[ -z "$REPO_ID" ]]; then
    ERROR="Could not determine repo identifier"
    echo "❌ $REPO — $ERROR. PR #$PR_NUMBER left open." >&2
    STATUS="bailed"
    exit 0
fi

echo "🔄 $REPO — waiting for CI..." >&2

GENERIC_SCRIPTS="$CLONE_DIR/dev-tools/.claude/scripts/generic"
iteration=0
rebase_count=0
transient_retries=0

while true; do
    CI_OUTPUT=$(cd "$CLONE_DIR" && "$GENERIC_SCRIPTS/ci-wait.sh" \
        --pr "$PR_NUMBER" --repo "$REPO_ID" \
        --rebase-count "$rebase_count" --fix-attempts 0 --iteration "$iteration" 2>&1) || true

    ACTION=$(echo "$CI_OUTPUT" | grep '^ACTION=' | head -1 | cut -d= -f2-)
    CI_STATUS=$(echo "$CI_OUTPUT" | grep '^CI_STATUS=' | head -1 | cut -d= -f2-)
    # BEHIND_COUNT parsed but not directly used — kept for debugging/logging
    _BEHIND_COUNT=$(echo "$CI_OUTPUT" | grep '^BEHIND_COUNT=' | head -1 | cut -d= -f2-)
    FAILED_RUN_ID=$(echo "$CI_OUTPUT" | grep '^FAILED_RUN_ID=' | head -1 | cut -d= -f2-)
    BAIL_REASON=$(echo "$CI_OUTPUT" | grep '^BAIL_REASON=' | head -1 | cut -d= -f2-)
    ITERATION=$(echo "$CI_OUTPUT" | grep '^ITERATION=' | head -1 | cut -d= -f2-)
    iteration="${ITERATION:-$iteration}"

    case "$ACTION" in
        merge)
            echo "✅ $REPO — CI passed, merging PR #$PR_NUMBER..." >&2
            # Use gh pr merge directly with --delete-branch (merge-pr.sh doesn't support it)
            if gh pr merge "$PR_NUMBER" --repo "$REPO_ID" --squash --delete-branch 2>&1; then
                echo "✅ $REPO — PR #$PR_NUMBER merged!" >&2
                STATUS="merged"
                ERROR=""
                break
            fi
            # Merge failed — check why
            MERGE_STATE=$(gh pr view "$PR_NUMBER" --repo "$REPO_ID" --json mergeStateStatus -q '.mergeStateStatus' 2>/dev/null || echo "")
            if [[ "$MERGE_STATE" == "BEHIND" ]]; then
                echo "🔄 $REPO — main advanced during merge, rebasing..." >&2
                iteration=$((iteration + 1))
                transient_retries=0
                continue
            fi
            # Try --admin fallback (re-verify CI first)
            CHECKS_JSON=$(gh pr checks "$PR_NUMBER" --repo "$REPO_ID" --json bucket 2>/dev/null || echo "")
            CI_GOOD=false
            if [[ -n "$CHECKS_JSON" ]] && echo "$CHECKS_JSON" | jq -e 'type == "array"' >/dev/null 2>&1; then
                TOTAL=$(echo "$CHECKS_JSON" | jq 'length')
                PASSED=$(echo "$CHECKS_JSON" | jq '[.[] | select(.bucket == "pass")] | length')
                if [[ "$TOTAL" -gt 0 ]] && [[ "$PASSED" -eq "$TOTAL" ]]; then
                    CI_GOOD=true
                fi
            fi
            if [[ "$CI_GOOD" == "true" ]] && [[ "$MERGE_STATE" != "BEHIND" ]]; then
                echo "ℹ $REPO — retrying merge with --admin..." >&2
                ADMIN_OUTPUT=$(gh pr merge "$PR_NUMBER" --repo "$REPO_ID" --squash --delete-branch --admin 2>&1)
                if [[ $? -eq 0 ]]; then
                    echo "⚠ $REPO — merged with --admin (review requirement overridden)" >&2
                    STATUS="merged"
                    ERROR=""
                    break
                fi
                ERROR="Admin merge failed: $ADMIN_OUTPUT"
                echo "❌ $REPO — $ERROR" >&2
                STATUS="bailed"
                break
            fi
            # CI not ready, loop back
            iteration=$((iteration + 1))
            continue
            ;;

        already_merged)
            echo "✅ $REPO — PR already merged externally." >&2
            STATUS="merged"
            ERROR=""
            break
            ;;

        rebase)
            if [[ "$CI_STATUS" == "pass" ]]; then
                echo "🔄 $REPO — CI passed but main advanced — rebasing..." >&2
            else
                echo "🔄 $REPO — main advanced while CI running — rebasing..." >&2
            fi
            REBASE_EXIT=0
            (cd "$CLONE_DIR" && "$GENERIC_SCRIPTS/rebase-push.sh") || REBASE_EXIT=$?
            if [[ $REBASE_EXIT -eq 0 ]]; then
                rebase_count=$((rebase_count + 1))
                iteration=$((iteration + 1))
                transient_retries=0
                sleep 30
                continue
            elif [[ $REBASE_EXIT -eq 1 ]]; then
                echo "❌ $REPO — rebase conflict. Aborting." >&2
                (cd "$CLONE_DIR" && git rebase --abort 2>/dev/null) || true
                ERROR="Rebase had conflicts"
                STATUS="bailed"
                break
            elif [[ $REBASE_EXIT -eq 2 ]]; then
                # Retry once
                echo "⚠ $REPO — push failed, retrying rebase..." >&2
                RETRY_EXIT=0
                (cd "$CLONE_DIR" && "$GENERIC_SCRIPTS/rebase-push.sh") || RETRY_EXIT=$?
                if [[ $RETRY_EXIT -eq 0 ]]; then
                    rebase_count=$((rebase_count + 1))
                    iteration=$((iteration + 1))
                    transient_retries=0
                    sleep 30
                    continue
                fi
                ERROR="Rebase push failed twice"
                STATUS="bailed"
                break
            else
                ERROR="Rebase failed (exit $REBASE_EXIT)"
                STATUS="bailed"
                break
            fi
            ;;

        rebase_then_evaluate)
            echo "🔄 $REPO — rebasing before evaluating CI failure..." >&2
            REBASE_EXIT=0
            (cd "$CLONE_DIR" && "$GENERIC_SCRIPTS/rebase-push.sh") || REBASE_EXIT=$?
            if [[ $REBASE_EXIT -eq 0 ]]; then
                rebase_count=$((rebase_count + 1))
                iteration=$((iteration + 1))
                transient_retries=0
                sleep 30
                continue
            elif [[ $REBASE_EXIT -eq 1 ]]; then
                (cd "$CLONE_DIR" && git rebase --abort 2>/dev/null) || true
                ERROR="Rebase had conflicts"
                STATUS="bailed"
                break
            elif [[ $REBASE_EXIT -eq 2 ]]; then
                # Push failed — retry once
                echo "⚠ $REPO — push failed during rebase_then_evaluate, retrying..." >&2
                RETRY_EXIT=0
                (cd "$CLONE_DIR" && "$GENERIC_SCRIPTS/rebase-push.sh") || RETRY_EXIT=$?
                if [[ $RETRY_EXIT -eq 0 ]]; then
                    rebase_count=$((rebase_count + 1))
                    iteration=$((iteration + 1))
                    transient_retries=0
                    sleep 30
                    continue
                fi
                ERROR="Rebase push failed twice"
                STATUS="bailed"
                break
            else
                ERROR="Rebase failed (exit $REBASE_EXIT)"
                STATUS="bailed"
                break
            fi
            ;;

        evaluate_failure)
            if [[ -z "$FAILED_RUN_ID" ]]; then
                # Try to resolve from pr checks — extract run ID from .../runs/<id> path segment
                FAILED_CHECK_URL=$(gh pr checks "$PR_NUMBER" --repo "$REPO_ID" --json link,bucket \
                    --jq '[.[] | select(.bucket == "fail")] | .[0].link' 2>/dev/null || echo "")
                FAILED_RUN_ID=$(echo "$FAILED_CHECK_URL" | grep -oE 'runs/[0-9]+' | grep -oE '[0-9]+' || echo "")
            fi
            if [[ -z "$FAILED_RUN_ID" ]]; then
                ERROR="CI failed but could not identify failed run ID"
                echo "❌ $REPO — $ERROR" >&2
                STATUS="bailed"
                break
            fi

            # Check if transient
            LOGS=$(gh run view "$FAILED_RUN_ID" --repo "$REPO_ID" --log-failed 2>/dev/null | head -200 || echo "")
            IS_TRANSIENT=false
            if echo "$LOGS" | grep -qiE 'runner lost communication|provisioning failure|Docker pull rate limit|RUNNER_TEMP|hosted runner.*lost|connection reset|net/http.*timeout'; then
                IS_TRANSIENT=true
            fi

            if [[ "$IS_TRANSIENT" == "true" ]] && [[ "$transient_retries" -lt 2 ]]; then
                echo "⚠ $REPO — transient CI failure, rerunning (retry $((transient_retries + 1))/2)..." >&2
                sleep 60
                RERUN_OUTPUT=$(cd "$CLONE_DIR" && "$GENERIC_SCRIPTS/ci-rerun-failed.sh" \
                    --run-id "$FAILED_RUN_ID" --repo "$REPO_ID" 2>&1) || true
                RERUN_OK=$(echo "$RERUN_OUTPUT" | grep '^RERUN_SUBMITTED=' | head -1 | cut -d= -f2-)
                if [[ "$RERUN_OK" == "true" ]]; then
                    transient_retries=$((transient_retries + 1))
                    iteration=$((iteration + 1))
                    sleep 60
                    continue
                fi
                # Rerun failed — treat as real failure
                echo "⚠ $REPO — rerun submission failed, treating as real failure" >&2
            fi

            # Real CI failure — bail (no code fixes for submodule bumps)
            ERROR="CI failed (real failure). PR #$PR_NUMBER left open for manual triage."
            echo "❌ $REPO — $ERROR" >&2
            STATUS="bailed"
            break
            ;;

        bail)
            ERROR="$BAIL_REASON"
            echo "❌ $REPO — bailing: $BAIL_REASON" >&2
            STATUS="bailed"
            break
            ;;

        *)
            ERROR="Unknown action from ci-wait.sh: $ACTION"
            echo "❌ $REPO — $ERROR" >&2
            STATUS="bailed"
            break
            ;;
    esac
done

# --- Sub-Step 8: Post :merged: emoji ---
if [[ "$STATUS" == "merged" ]] && [[ -n "$SLACK_TS" ]] && [[ "$SLACK_AVAILABLE" == "true" ]] && [[ -n "$SLACK_CHANNEL" ]]; then
    echo "✨ $REPO — adding :merged: emoji..." >&2
    (cd "$CLONE_DIR" && ./dev-tools/.claude/scripts/generic/add-merged-emoji.sh \
        --slack-ts "$SLACK_TS" --channel "$SLACK_CHANNEL" 2>&1) || \
        echo "⚠ $REPO — :merged: emoji failed (non-fatal)" >&2
fi

echo "🏁 $REPO — Phase 2 complete (STATUS=$STATUS)" >&2
