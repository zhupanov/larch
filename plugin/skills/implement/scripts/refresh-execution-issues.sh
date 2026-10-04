#!/usr/bin/env bash
# refresh-execution-issues.sh — refresh execution-issue metadata through Rust.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
PLUGIN_ROOT="${CLAUDE_PLUGIN_ROOT:-$(cd "$SCRIPT_DIR/../../.." && pwd -P)}"
exec env CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" "$PLUGIN_ROOT/scripts/larch.sh" execution-issues refresh "$@"
