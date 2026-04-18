#!/usr/bin/env bash
# git-push.sh — Push the current branch to origin (fast-forward, no force).
#
# Wraps a plain `git push` for non-force updates (e.g., when /implement's
# Step 10 / Step 12c adds a CI fix commit on top of the existing remote tip).
# For force-with-lease updates after a rebase, use `git-force-push.sh` instead.
#
# Usage:
#   git-push.sh
#
# Output (stdout): BRANCH=<name>
#
# Exit codes:
#   0 — push succeeded (or branch was already up-to-date)
#   1 — not on a named branch (detached HEAD / not a git repo)
#   >0 — passthrough from `git push`

set -euo pipefail

if ! BRANCH=$(git symbolic-ref --short HEAD 2>/dev/null); then
    echo "git-push.sh: not on a named branch" >&2
    exit 1
fi
echo "BRANCH=$BRANCH"

exec git push
