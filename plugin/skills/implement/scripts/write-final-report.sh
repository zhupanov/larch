#!/usr/bin/env bash
# write-final-report.sh — thin wrapper around the Rust final-report writer.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
PLUGIN_ROOT="${CLAUDE_PLUGIN_ROOT:-$(cd "$SCRIPT_DIR/../../.." && pwd -P)}"

exec "$PLUGIN_ROOT/scripts/larch.sh" final-report write "$@"
