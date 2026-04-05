#!/usr/bin/env bash
# upgrade-single-client.sh — Phase 1: Pre-PR per-client upgrade lifecycle.
#
# Handles: remote preflight check, clone, local up-to-date check,
# upgrade-dev-tools.sh, re-bootstrap .claude/ symlinks, check for
# /bump-version skill, read Slack channel.
#
# Stops BEFORE PR creation — the version bump (LLM-driven) and
# PR creation happen in later phases.
#
# Usage:
#   upgrade-single-client.sh --repo OWNER/REPO --target-sha SHA \
#     --target-sha-short SHORT_SHA --tmpdir DIR
#
# Output: $TMPDIR/<repo-name>-phase1.txt (newline-delimited KEY=VALUE)
#   STATUS=ready|skipped|failed
#   CLONE_DIR=<path>
#   HAS_BUMP=true|false
#   BUMP_VERSION_MISSING=true|false
#   SLACK_CHANNEL=<channel>
#   CURRENT_SHA=<sha>
#   BRANCH=<branch-name>
#   REPO_ID=<owner/repo>
#   ERROR=<message>
#
# Completion signal: $TMPDIR/<repo-name>-phase1.txt.done (contains exit code)
#
# Exit codes:
#   0 — always (result communicated via output file)

set -uo pipefail

# --- Parse arguments ---
REPO=""
TARGET_SHA=""
TARGET_SHA_SHORT=""
UC_TMPDIR=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --repo) REPO="${2:?--repo requires a value}"; shift 2 ;;
        --target-sha) TARGET_SHA="${2:?--target-sha requires a value}"; shift 2 ;;
        --target-sha-short) TARGET_SHA_SHORT="${2:?--target-sha-short requires a value}"; shift 2 ;;
        --tmpdir) UC_TMPDIR="${2:?--tmpdir requires a value}"; shift 2 ;;
        *) echo "Unknown option: $1" >&2; exit 1 ;;
    esac
done

if [[ -z "$REPO" ]] || [[ -z "$TARGET_SHA" ]] || [[ -z "$TARGET_SHA_SHORT" ]] || [[ -z "$UC_TMPDIR" ]]; then
    echo "ERROR: --repo, --target-sha, --target-sha-short, and --tmpdir are required" >&2
    exit 1
fi

REPO_NAME="${REPO#*/}"
CLONE_DIR="$UC_TMPDIR/$REPO_NAME"
RESULT_FILE="$UC_TMPDIR/${REPO_NAME}-phase1.txt"
DONE_FILE="${RESULT_FILE}.done"

# --- Output defaults ---
STATUS="failed"
HAS_BUMP="false"
BUMP_VERSION_MISSING="false"
SLACK_CHANNEL=""
CURRENT_SHA=""
BRANCH=""
REPO_ID=""
COMMITS_BEFORE=""
ERROR="upgrade-single-client.sh exited unexpectedly"

write_result() {
    # Sanitize ERROR to prevent multi-line values from corrupting KEY=VALUE format
    ERROR="${ERROR//$'\n'/ }"
    cat > "$RESULT_FILE" <<RESULT_EOF
STATUS=$STATUS
CLONE_DIR=$CLONE_DIR
HAS_BUMP=$HAS_BUMP
BUMP_VERSION_MISSING=$BUMP_VERSION_MISSING
SLACK_CHANNEL=$SLACK_CHANNEL
CURRENT_SHA=$CURRENT_SHA
BRANCH=$BRANCH
REPO_ID=$REPO_ID
COMMITS_BEFORE=$COMMITS_BEFORE
ERROR=$ERROR
RESULT_EOF
}

# Write .done sentinel on every exit path — capture exit code before write_result mutates $?
# shellcheck disable=SC2154  # exit_code is assigned inside the trap string
trap 'exit_code=$?; write_result; echo "$exit_code" > "$DONE_FILE"' EXIT

# --- Sub-Step 1: Remote Preflight Check ---
echo "🔍 $REPO — checking submodule SHA via API..." >&2
CURRENT_SHA=$(gh api "repos/$REPO/contents/dev-tools?ref=main" --jq '.sha' 2>/dev/null || echo "")

# Validate SHA format (must be 40-char hex)
if [[ -n "$CURRENT_SHA" ]] && ! [[ "$CURRENT_SHA" =~ ^[0-9a-f]{40}$ ]]; then
    echo "⚠ $REPO — API returned invalid SHA: $CURRENT_SHA. Falling back to clone." >&2
    CURRENT_SHA=""
fi

if [[ "$CURRENT_SHA" == "$TARGET_SHA" ]]; then
    echo "✅ $REPO — already up-to-date at $TARGET_SHA_SHORT (checked via API, no clone needed)" >&2
    STATUS="skipped"
    ERROR=""
    exit 0
fi

# --- Sub-Step 1b: Clone ---
echo "📦 $REPO — cloning..." >&2
if ! git clone --filter=blob:none "git@github.com:$REPO.git" "$CLONE_DIR" 2>&1; then
    echo "❌ $REPO — clone failed" >&2
    ERROR="clone failed"
    exit 0
fi

if ! (cd "$CLONE_DIR" && git submodule update --init dev-tools 2>&1); then
    echo "❌ $REPO — submodule init failed" >&2
    ERROR="submodule init failed"
    exit 0
fi

# --- Sub-Step 2: Check Already Up-to-Date (local fallback) ---
if [[ -z "$CURRENT_SHA" ]]; then
    CURRENT_SHA=$(cd "$CLONE_DIR" && git submodule status dev-tools | awk '{print $1}' | sed 's/^[-+?]//')
    if [[ "$CURRENT_SHA" == "$TARGET_SHA" ]]; then
        echo "✅ $REPO — already up-to-date at $TARGET_SHA_SHORT" >&2
        STATUS="skipped"
        ERROR=""
        exit 0
    fi
fi

# --- Sub-Step 3: Run Upgrade Script ---
echo "⬆️ $REPO — running upgrade-dev-tools.sh..." >&2
if ! (cd "$CLONE_DIR" && ./dev-tools/scripts/upgrade-dev-tools.sh 2>&1); then
    echo "❌ $REPO — upgrade script failed" >&2
    ERROR="upgrade script failed"
    exit 0
fi

BRANCH=$(cd "$CLONE_DIR" && git symbolic-ref --short HEAD)

# --- Sub-Step 3b: Re-bootstrap .claude/ symlinks ---
echo "🔧 $REPO — re-bootstrapping .claude/ symlinks..." >&2
if ! (cd "$CLONE_DIR" && ./dev-tools/.claude/scripts/setup-claude.sh --force 2>&1); then
    echo "⚠ $REPO — setup-claude.sh --force failed (non-fatal, continuing)" >&2
fi

# Check if any files changed in .claude/
CLAUDE_CHANGES=$(cd "$CLONE_DIR" && git status --porcelain .claude/)
if [[ -n "$CLAUDE_CHANGES" ]]; then
    echo "🔧 $REPO — committing .claude/ symlink changes..." >&2
    if ! (cd "$CLONE_DIR" && ./dev-tools/.claude/scripts/generic/git-commit.sh \
        -m "Re-bootstrap .claude/ symlinks after dev-tools upgrade" .claude/); then
        echo "❌ $REPO — failed to commit .claude/ changes" >&2
        ERROR="git-commit.sh failed for .claude/ re-bootstrap"
        exit 0
    fi
    echo "🔧 $REPO — re-bootstrapped .claude/ symlinks (commit added)" >&2
else
    echo "🔧 $REPO — .claude/ symlinks already up-to-date" >&2
fi

# --- Sub-Step 4: Check for /bump-version skill ---
BUMP_OUTPUT=$(cd "$CLONE_DIR" && ./dev-tools/.claude/scripts/generic/check-bump-version.sh --mode pre 2>/dev/null || echo "HAS_BUMP=false")
HAS_BUMP=$(echo "$BUMP_OUTPUT" | grep '^HAS_BUMP=' | cut -d= -f2-)
HAS_BUMP="${HAS_BUMP:-false}"
COMMITS_BEFORE=$(echo "$BUMP_OUTPUT" | grep '^COMMITS_BEFORE=' | cut -d= -f2-)
COMMITS_BEFORE="${COMMITS_BEFORE:-0}"
if [[ "$HAS_BUMP" == "false" ]]; then
    BUMP_VERSION_MISSING="true"
fi

# --- Sub-Step 5: Read Slack channel ---
CHANNEL_OUTPUT=$(cd "$CLONE_DIR" && ./dev-tools/.claude/scripts/generic/read-slack-channel.sh 2>/dev/null || echo "SLACK_CHANNEL=")
SLACK_CHANNEL=$(echo "$CHANNEL_OUTPUT" | grep '^SLACK_CHANNEL=' | cut -d= -f2-)

# --- Derive repo ID ---
REPO_ID=$(cd "$CLONE_DIR" && gh repo view --json nameWithOwner -q '.nameWithOwner' 2>/dev/null || echo "")
if [[ -z "$REPO_ID" ]]; then
    REPO_ID="$REPO"
fi

# --- Done ---
STATUS="ready"
ERROR=""
echo "✅ $REPO — Phase 1 complete (branch: $BRANCH, HAS_BUMP=$HAS_BUMP)" >&2
