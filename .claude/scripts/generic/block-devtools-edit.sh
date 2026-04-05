#!/usr/bin/env bash
# PreToolUse hook: Block edits to dev-tools/ directory.
# Policy source: CLAUDE-generic.md "dev-tools Submodule" section.
#
# Stdin: JSON with tool_input.file_path (absolute path)
# Exit 0: allow the operation
# Exit 2: block the operation (stdout is the reason shown to Claude)
#
# Fails CLOSED: if JSON parsing fails, blocks the edit (exit 2).

set -uo pipefail

INPUT=$(cat) || {
  echo "dev-tools edit guard: failed to read stdin — blocking as precaution"
  exit 2
}

# Extract file_path from JSON. If jq fails or field is missing, treat as suspicious → block.
FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty' 2>/dev/null) || {
  echo "dev-tools edit guard: failed to parse tool input — blocking as precaution"
  exit 2
}

# No file_path means this isn't a file operation we care about → allow.
if [ -z "$FILE_PATH" ]; then
  exit 0
fi

# If the file being edited is inside the same git repo as the working directory,
# allow — we're editing our own files, not a submodule. When dev-tools is used
# as a submodule, the file's repo root will differ from the CWD repo root.
REPO_ROOT=$(git rev-parse --show-toplevel 2>/dev/null || echo "")
FILE_REPO_ROOT=$(git -C "$(dirname "$FILE_PATH")" rev-parse --show-toplevel 2>/dev/null || echo "")
if [[ -n "$REPO_ROOT" ]] && [[ -n "$FILE_REPO_ROOT" ]] && [[ "$FILE_REPO_ROOT" == "$REPO_ROOT" ]]; then
  exit 0
fi

# Fallback: if .git is a real directory (not a submodule gitlink), we are in
# the canonical repo. Allow edits to files under this directory, excluding
# any path containing dev-tools/ (which could be a nested submodule).
# Use REPO_ROOT when available (more precise than $PWD), fall back to $PWD.
# Note: this assumes $PWD is the project root (set by Claude Code).
ALLOW_ROOT="${REPO_ROOT:-$PWD}"
if [[ -d "$ALLOW_ROOT/.git" ]] && [[ "$FILE_PATH" == "$ALLOW_ROOT"/* ]]; then
  RELPATH="${FILE_PATH#"$ALLOW_ROOT"/}"
  if [[ "$RELPATH" != dev-tools/* ]] && [[ "$RELPATH" != */dev-tools/* ]]; then
    # Also check resolved path to catch symlinks pointing into dev-tools
    FALLBACK_RESOLVED=""
    if [[ -e "$FILE_PATH" ]]; then
      FALLBACK_RESOLVED=$(realpath "$FILE_PATH" 2>/dev/null || true)
    else
      FALLBACK_PARENT=$(dirname "$FILE_PATH")
      if [[ -d "$FALLBACK_PARENT" ]]; then
        FALLBACK_RESOLVED="$(realpath "$FALLBACK_PARENT" 2>/dev/null || true)/$(basename "$FILE_PATH")"
      fi
    fi
    if [[ -z "$FALLBACK_RESOLVED" ]] || [[ "$FALLBACK_RESOLVED" != */dev-tools/* ]]; then
      exit 0
    fi
  fi
fi

# Check if the path contains /dev-tools/ anywhere (works with absolute paths).
if [[ "$FILE_PATH" == */dev-tools/* || "$FILE_PATH" == dev-tools/* ]]; then
  echo "dev-tools is a git submodule — never edit directly. File PRs in the dev-tools repo instead."
  exit 2
fi

# Also check the resolved (real) path — catches edits through symlinks pointing into dev-tools.
# For non-existent files (Write to new file), resolve the parent directory and append basename.
RESOLVED_PATH=""
if [[ -e "$FILE_PATH" ]]; then
  RESOLVED_PATH=$(realpath "$FILE_PATH" 2>/dev/null || true)
else
  PARENT_DIR=$(dirname "$FILE_PATH")
  BASE_NAME=$(basename "$FILE_PATH")
  if [[ -d "$PARENT_DIR" ]]; then
    RESOLVED_PARENT=$(realpath "$PARENT_DIR" 2>/dev/null || true)
    if [[ -n "$RESOLVED_PARENT" ]]; then
      RESOLVED_PATH="${RESOLVED_PARENT}/${BASE_NAME}"
    fi
  fi
fi

if [[ -n "$RESOLVED_PATH" ]] && [[ "$RESOLVED_PATH" == */dev-tools/* ]]; then
  echo "dev-tools is a git submodule — never edit directly. File PRs in the dev-tools repo instead."
  exit 2
fi

exit 0
