#!/usr/bin/env bash
# git-show-stage.sh — Print the content of a specific index stage for a file.
#
# Wraps `git show :<N>:<file>` for use inside /implement's Conflict Resolution
# Procedure Phase 1:
#   - stage 1: common ancestor (base)
#   - stage 2: "ours" (upstream/main during rebase)
#   - stage 3: "theirs" (feature branch commit during rebase)
#
# Usage:
#   git-show-stage.sh --stage <1|2|3> --file <path>
#
# Output (stdout): file content at the requested stage.
#
# Exit codes:
#   0 — success
#   1 — usage error or stage missing (no output written)

set -euo pipefail

STAGE=""
FILE=""
while [[ $# -gt 0 ]]; do
    case "$1" in
        --stage) STAGE="${2:?--stage requires a value}"; shift 2 ;;
        --file) FILE="${2:?--file requires a value}"; shift 2 ;;
        *) echo "git-show-stage.sh: unknown argument: $1" >&2; exit 1 ;;
    esac
done

if [[ -z "$STAGE" || -z "$FILE" ]]; then
    echo "git-show-stage.sh: --stage and --file are required" >&2
    exit 1
fi

case "$STAGE" in
    1|2|3) ;;
    *) echo "git-show-stage.sh: --stage must be 1, 2, or 3 (got: $STAGE)" >&2; exit 1 ;;
esac

exec git show ":$STAGE:$FILE"
