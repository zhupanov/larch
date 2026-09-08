#!/usr/bin/env bash
# step-7a.sh — thin wrapper delegating to the Rust bgjob adapter.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
PLUGIN_ROOT="${CLAUDE_PLUGIN_ROOT:-$(cd "$SCRIPT_DIR/../../.." && pwd -P)}"
exec "$PLUGIN_ROOT/scripts/larch.sh" implement step-7a "$@"
