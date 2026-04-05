#!/usr/bin/env bash
# Run validation checks relevant to modified files on the current branch.
# This script is private to the /relevant-checks skill.
set -uo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)" || { echo "ERROR: not inside a git repository"; exit 1; }
cd "$REPO_ROOT" || exit 1

# ---------------------------------------------------------------------------
# Determine changed files (union of branch diff + staged + unstaged + untracked)
# ---------------------------------------------------------------------------
# Only fall back to origin/main if local main is truly unavailable, not if the
# diff is just empty (which happens on a new branch with no commits yet — when
# there are no branch commits, main...HEAD returns empty, and we rely on the
# staged/unstaged/untracked diffs to capture working tree changes).
if git rev-parse --verify main >/dev/null 2>&1; then
    branch_diff="$(git diff --name-only main...HEAD 2>/dev/null || true)"
elif git rev-parse --verify origin/main >/dev/null 2>&1; then
    branch_diff="$(git diff --name-only origin/main...HEAD 2>/dev/null || true)"
else
    branch_diff=""
fi

# Staged changes (files added to index but not yet committed)
staged_diff="$(git diff --cached --name-only 2>/dev/null || true)"

# Unstaged changes (modified but not yet staged)
unstaged_diff="$(git diff --name-only 2>/dev/null || true)"

# Untracked files (newly created, not yet staged — e.g., files written by Claude)
untracked="$(git ls-files --others --exclude-standard 2>/dev/null || true)"

# Union and deduplicate
MODIFIED_FILES="$(printf '%s\n%s\n%s\n%s' "$branch_diff" "$staged_diff" "$unstaged_diff" "$untracked" | sort -u | grep -v '^$' || true)"

if [ -z "$MODIFIED_FILES" ]; then
    echo "No modified files detected — no checks to run."
    exit 0
fi

# Detect file types
HAS_SH=false
HAS_PY=false

while IFS= read -r file; do
    case "$file" in
        *.sh) HAS_SH=true ;;
        *.py) HAS_PY=true ;;
    esac
done <<< "$MODIFIED_FILES"

if [ "$HAS_SH" = false ] && [ "$HAS_PY" = false ]; then
    echo "No .sh or .py files modified — no checks to run."
    exit 0
fi

FAILED=0

# Shell checks
if [ "$HAS_SH" = true ]; then
    echo "=== Running shellcheck ==="
    if make shellcheck; then
        echo "✅ shellcheck passed"
    else
        echo "❌ shellcheck failed"
        FAILED=1
    fi
fi

# Python checks (lint → test → format, each target run individually for diagnostics)
if [ "$HAS_PY" = true ]; then
    echo "=== Running Python checks ==="
    for target in lint test ruff-format validate-dataclasses validate-no-logging-exception-calls; do
        if make -C python "$target"; then
            echo "✅ python $target passed"
        else
            echo "❌ python $target failed"
            FAILED=1
        fi
    done
fi

# Summary
echo ""
if [ "$FAILED" -eq 0 ]; then
    echo "✅ All relevant checks passed"
else
    echo "❌ Some checks failed — see output above for details"
fi

exit "$FAILED"
