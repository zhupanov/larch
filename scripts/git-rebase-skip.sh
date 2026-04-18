#!/usr/bin/env bash
# git-rebase-skip.sh — Skip the current rebase step.
#
# Wraps `git rebase --skip` for use inside /implement's Rebase + Re-bump
# Sub-procedure Phase 4 Exit 3 path (when a rebased commit has nothing new
# to apply against the new base).
#
# Usage:
#   git-rebase-skip.sh
#
# Exit codes: passthrough from `git rebase --skip`.

set -euo pipefail
exec git rebase --skip
