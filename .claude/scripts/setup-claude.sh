#!/usr/bin/env bash
# setup-claude.sh — Bootstrap and maintain .claude/ structure in client repos.
#
# Creates symlinks from the client repo's .claude/ directory to the dev-tools
# submodule's .claude/ directory. Safe to re-run (idempotent).
#
# Usage: dev-tools/.claude/scripts/setup-claude.sh [--check] [--force]
#   --check  Dry-run: report what would be created/updated/deleted
#   --force  Overwrite non-symlink files with symlinks

set -euo pipefail

# --- Resolve paths ---
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
DEV_TOOLS_CLAUDE="$(cd "$SCRIPT_DIR/.." && pwd)"
DEV_TOOLS_ROOT="$(cd "$DEV_TOOLS_CLAUDE/.." && pwd)"
REPO_ROOT="$(cd "$DEV_TOOLS_ROOT/.." && pwd)"
CLAUDE_DIR="$REPO_ROOT/.claude"

# --- Parse flags ---
CHECK_MODE=false
FORCE_MODE=false
for arg in "$@"; do
  case "$arg" in
    --check) CHECK_MODE=true ;;
    --force) FORCE_MODE=true ;;
    *) echo "Unknown argument: $arg" >&2; exit 1 ;;
  esac
done

# --- Counters ---
CREATED=0
UPDATED=0
REMOVED=0
SKIPPED=0

# --- Prerequisites ---
if [ ! -d "$DEV_TOOLS_CLAUDE" ] || [ -z "$(ls -A "$DEV_TOOLS_CLAUDE" 2>/dev/null)" ]; then
  echo "ERROR: dev-tools/.claude/ does not exist or is empty." >&2
  echo "Run: git submodule update --init dev-tools" >&2
  exit 1
fi

if [ ! -f "$DEV_TOOLS_CLAUDE/settings.json" ]; then
  echo "ERROR: dev-tools/.claude/settings.json not found." >&2
  echo "Run: git submodule update --init dev-tools" >&2
  exit 1
fi

# --- Helper: create a symlink ---
# Args: $1=symlink_path $2=relative_target
create_symlink() {
  local link_path="$1"
  local target="$2"
  local link_dir
  link_dir="$(dirname "$link_path")"

  # If the target already exists and is not a symlink
  if [ -e "$link_path" ] && [ ! -L "$link_path" ]; then
    if [ "$FORCE_MODE" = true ]; then
      if [ "$CHECK_MODE" = true ]; then
        echo "  [would replace] $link_path → $target (--force)"
        UPDATED=$((UPDATED + 1))
        return
      fi
      rm -rf "$link_path"
    else
      echo "  [skipped] $link_path exists and is not a symlink (use --force to overwrite)"
      SKIPPED=$((SKIPPED + 1))
      return
    fi
  fi

  # If a symlink already exists, check if it points to the right target
  if [ -L "$link_path" ]; then
    local current_target
    current_target="$(readlink "$link_path")"
    if [ "$current_target" = "$target" ]; then
      return  # Already correct, no action needed
    fi
    if [ "$CHECK_MODE" = true ]; then
      echo "  [would update] $link_path → $target (was → $current_target)"
      UPDATED=$((UPDATED + 1))
      return
    fi
    rm "$link_path"
    UPDATED=$((UPDATED + 1))
  else
    if [ "$CHECK_MODE" = true ]; then
      echo "  [would create] $link_path → $target"
      CREATED=$((CREATED + 1))
      return
    fi
    CREATED=$((CREATED + 1))
  fi

  mkdir -p "$link_dir"
  ln -s "$target" "$link_path"
  echo "  [created] $link_path → $target"
}

# --- 1. settings.json symlink ---
echo "=== settings.json ==="
create_symlink "$CLAUDE_DIR/settings.json" "../dev-tools/.claude/settings.json"

# --- 2. agents/ directory symlink ---
echo "=== agents/ ==="
create_symlink "$CLAUDE_DIR/agents" "../dev-tools/.claude/agents"

# --- 3. scripts/generic/ directory symlink ---
echo "=== scripts/generic/ ==="
mkdir -p "$CLAUDE_DIR/scripts"
if [ -d "$DEV_TOOLS_CLAUDE/scripts/generic" ]; then
  create_symlink "$CLAUDE_DIR/scripts/generic" "../../dev-tools/.claude/scripts/generic"
else
  echo "  [warning] dev-tools/.claude/scripts/generic not found — run: git submodule update --init dev-tools"
fi

# --- 4. Generic skill directory symlinks ---
GENERIC_SKILLS=(design implement research review shazam loop-review skill-creator)
echo "=== skills (generic) ==="
mkdir -p "$CLAUDE_DIR/skills"
for skill in "${GENERIC_SKILLS[@]}"; do
  if [ -d "$DEV_TOOLS_CLAUDE/skills/$skill" ]; then
    create_symlink "$CLAUDE_DIR/skills/$skill" "../../dev-tools/.claude/skills/$skill"
  else
    echo "  [warning] dev-tools/.claude/skills/$skill not found, skipping"
  fi
done

# --- 5. Shared doc file symlinks (only generic ones) ---
SHARED_GENERIC_DOCS=(external-reviewers.md reviewer-templates.md voting-protocol.md)
echo "=== skills/shared (generic docs) ==="
mkdir -p "$CLAUDE_DIR/skills/shared"
for doc in "${SHARED_GENERIC_DOCS[@]}"; do
  if [ -f "$DEV_TOOLS_CLAUDE/skills/shared/$doc" ]; then
    create_symlink "$CLAUDE_DIR/skills/shared/$doc" "../../../dev-tools/.claude/skills/shared/$doc"
  else
    echo "  [warning] dev-tools/.claude/skills/shared/$doc not found, skipping"
  fi
done

# --- 6. repo-config.json (repo-specific, NOT symlinked) ---
# This file is intentionally repo-local. Each repo defines its own slackChannel.
# Do NOT add this to the symlink list — it must remain per-repo.
echo "=== repo-config.json ==="
if [ ! -e "$CLAUDE_DIR/repo-config.json" ]; then
  if [ "$CHECK_MODE" = true ]; then
    echo "  [would create] $CLAUDE_DIR/repo-config.json (template — will need slackChannel configured)"
    CREATED=$((CREATED + 1))
  else
    echo '{"slackChannel": ""}' > "$CLAUDE_DIR/repo-config.json"
    echo "  [created] $CLAUDE_DIR/repo-config.json (template)"
    echo "  ⚠ Set slackChannel in .claude/repo-config.json to your team's Slack channel."
    CREATED=$((CREATED + 1))
  fi
else
  echo "  repo-config.json already exists, skipping"
fi

# --- 7. Dead symlink cleanup ---
echo "=== Dead symlink cleanup ==="
while IFS= read -r -d '' symlink; do
  target="$(readlink "$symlink")"
  # Only clean up symlinks pointing to dev-tools
  if [[ "$target" == *"dev-tools/"* ]]; then
    # Resolve relative to the symlink's directory
    link_dir="$(dirname "$symlink")"
    if [ ! -e "$link_dir/$target" ]; then
      if [ "$CHECK_MODE" = true ]; then
        echo "  [would remove] dead symlink: $symlink → $target"
      else
        rm "$symlink"
        echo "  [removed] dead symlink: $symlink → $target"
      fi
      REMOVED=$((REMOVED + 1))
    fi
  fi
done < <(find "$CLAUDE_DIR" -type l -print0 2>/dev/null)

# --- Summary ---
echo ""
echo "=== Summary ==="
if [ "$CHECK_MODE" = true ]; then
  echo "  Dry run — no changes made"
fi
echo "  Created: $CREATED"
echo "  Updated: $UPDATED"
echo "  Removed: $REMOVED"
echo "  Skipped: $SKIPPED"
