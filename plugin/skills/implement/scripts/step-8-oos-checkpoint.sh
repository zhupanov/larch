#!/usr/bin/env bash
# step-8-oos-checkpoint.sh — thin relay for the Step 8 OOS checkpoint router.

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
PLUGIN_ROOT="${CLAUDE_PLUGIN_ROOT:-$(cd "$SCRIPT_DIR/../../.." && pwd -P)}"
IMPLEMENT_TMPDIR="${IMPLEMENT_TMPDIR:?IMPLEMENT_TMPDIR required}"
export IMPLEMENT_TMPDIR
if [ -z "${CLAUDE_PLUGIN_ROOT:-}" ] && [ -f "$IMPLEMENT_TMPDIR/plugin-root.env" ]; then
  # shellcheck source=/dev/null
  . "$IMPLEMENT_TMPDIR/plugin-root.env"
fi
[ -n "${CLAUDE_PLUGIN_ROOT:-}" ] || CLAUDE_PLUGIN_ROOT=$PLUGIN_ROOT
export CLAUDE_PLUGIN_ROOT

exec "$CLAUDE_PLUGIN_ROOT/scripts/larch.sh" implement step-8-oos-checkpoint "$@"
