#!/usr/bin/env bash
# git-stage.sh — Stage one or more files without committing.
#
# Wraps `git add -- <files>` so callers don't invoke `git` directly. Used by
# /implement's Conflict Resolution Procedure to stage resolved files before
# continuing the rebase. Distinct from scripts/git-commit.sh (which also
# commits) and scripts/git-amend-add.sh (which also amends).
#
# Usage:
#   git-stage.sh <file> [<file> ...]
#
# Exit codes: passthrough from `git add`.

set -euo pipefail

if [[ $# -eq 0 ]]; then
    echo "git-stage.sh: at least one file argument is required" >&2
    echo "usage: git-stage.sh <file> [<file> ...]" >&2
    exit 1
fi

exec git add -- "$@"
