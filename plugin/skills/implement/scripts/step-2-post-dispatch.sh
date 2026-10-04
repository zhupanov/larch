#!/usr/bin/env bash
# step-2-post-dispatch.sh — thin wrapper for /implement Step 2 post-dispatch.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
PLUGIN_ROOT="${CLAUDE_PLUGIN_ROOT:-$(cd "$SCRIPT_DIR/../../.." && pwd -P)}"
export CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT"
exec "$PLUGIN_ROOT/scripts/larch.sh" implement step-2-post-dispatch "$@"
