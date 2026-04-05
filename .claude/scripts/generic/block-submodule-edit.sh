#!/usr/bin/env bash
# PreToolUse hook: Block edits to files inside any git submodule.
#
# Stdin: JSON with tool_input.file_path (absolute path)
# Exit 0: allow the operation
# Exit 2: block the operation (stdout is the reason shown to Claude)
#
# Fails CLOSED: if JSON parsing fails, blocks the edit (exit 2).

set -uo pipefail

INPUT=$(cat) || {
  echo "submodule edit guard: failed to read stdin — blocking as precaution"
  exit 2
}

# Extract file_path from JSON. If jq fails or field is missing, treat as suspicious → block.
FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty' 2>/dev/null) || {
  echo "submodule edit guard: failed to parse tool input — blocking as precaution"
  exit 2
}

# No file_path means this isn't a file operation we care about → allow.
if [ -z "$FILE_PATH" ]; then
  exit 0
fi

# Core check: compare the git repo root of the file being edited against the
# working directory's repo root. If they differ, the file lives in a submodule.
REPO_ROOT=$(git rev-parse --show-toplevel 2>/dev/null || echo "")

# Resolve the file's repo root. For non-existent files, use the parent directory.
FILE_DIR="$(dirname "$FILE_PATH")"
if [[ ! -d "$FILE_DIR" ]]; then
  # Parent directory doesn't exist yet — can't be in a submodule, allow.
  exit 0
fi

FILE_REPO_ROOT=$(git -C "$FILE_DIR" rev-parse --show-toplevel 2>/dev/null || echo "")

# If we can't determine either repo root, allow (don't block normal non-git edits).
if [[ -z "$REPO_ROOT" ]] || [[ -z "$FILE_REPO_ROOT" ]]; then
  exit 0
fi

# Same repo root → not a submodule, allow.
if [[ "$FILE_REPO_ROOT" == "$REPO_ROOT" ]]; then
  exit 0
fi

# Different repo root → file is in a submodule. Derive the submodule name.
SUBMODULE_NAME="${FILE_REPO_ROOT#"$REPO_ROOT"/}"
echo "This file is inside the '$SUBMODULE_NAME' submodule — never edit submodules directly. File PRs in the submodule's own repo instead."
exit 2
