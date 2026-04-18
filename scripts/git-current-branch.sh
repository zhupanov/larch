#!/usr/bin/env bash
# git-current-branch.sh — Print the current branch name in KEY=VALUE form.
#
# Wraps `git symbolic-ref --short HEAD` so callers invoke a pre-approved
# script instead of a raw `git` command (avoids per-invocation permission
# prompts in Claude Code sessions).
#
# Usage:
#   git-current-branch.sh
#
# Output (stdout):
#   BRANCH=<name>          On a named branch.
#
# Exit codes:
#   0 — on a named branch (BRANCH emitted)
#   1 — detached HEAD or not in a git repo (nothing emitted, error on stderr)

set -euo pipefail

if BRANCH=$(git symbolic-ref --short HEAD 2>/dev/null); then
    echo "BRANCH=$BRANCH"
    exit 0
fi

echo "git-current-branch.sh: not on a named branch (detached HEAD or not a git repo)" >&2
exit 1
