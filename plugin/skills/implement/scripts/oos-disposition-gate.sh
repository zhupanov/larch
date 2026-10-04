#!/usr/bin/env bash
# oos-disposition-gate.sh — thin wrapper delegating to larch oos disposition-gate.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
PLUGIN_ROOT="${CLAUDE_PLUGIN_ROOT:-$(cd "$SCRIPT_DIR/../../.." && pwd -P)}"
exec env CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" "$PLUGIN_ROOT/scripts/larch.sh" oos disposition-gate "$@"
