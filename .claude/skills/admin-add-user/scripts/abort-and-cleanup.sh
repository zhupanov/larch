#!/usr/bin/env bash
# abort-and-cleanup.sh — Revert JSON edits and clean up on admin-add-user failure.
#
# Used when data validation tests fail. Reverts the JSON identity map
# files, switches to main, deletes the feature branch, and removes
# the session temp directory.
#
# Usage:
#   abort-and-cleanup.sh --branch <name> --tmpdir <path>
#
# Arguments:
#   --branch — Feature branch to delete
#   --tmpdir — Session temp directory to remove
#
# Outputs (key=value to stdout):
#   CLEANUP_DONE=true
#
# Exit codes:
#   0 — cleanup completed (best-effort, individual steps may warn)

# Not using set -e — cleanup steps are best-effort
set -uo pipefail

usage() { echo "Usage: abort-and-cleanup.sh --branch <name> --tmpdir <path>" >&2; }

BRANCH=""
TMPDIR_PATH=""
while [[ $# -gt 0 ]]; do
    case "$1" in
        --branch) BRANCH="${2:?--branch requires a value}"; shift 2 ;;
        --tmpdir) TMPDIR_PATH="${2:?--tmpdir requires a value}"; shift 2 ;;
        --help) usage; exit 0 ;;
        *) echo "Unknown option: $1" >&2; usage; exit 1 ;;
    esac
done

if [[ -z "$BRANCH" ]] || [[ -z "$TMPDIR_PATH" ]]; then
    echo "ERROR: --branch and --tmpdir are required" >&2
    usage; exit 1
fi

# Revert JSON identity map edits
git checkout -- data/email_to_slack_userid_map.json data/github_username_to_email_map.json 2>/dev/null || true

# Switch to main
git checkout main 2>/dev/null || true

# Delete the feature branch
git branch -D "$BRANCH" 2>/dev/null || true

# Remove temp directory (validate /tmp/ prefix)
if [[ -n "$TMPDIR_PATH" && ("$TMPDIR_PATH" == /tmp/* || "$TMPDIR_PATH" == /private/tmp/*) ]]; then
    rm -rf "$TMPDIR_PATH"
fi

echo "CLEANUP_DONE=true"
