#!/usr/bin/env bash
# design-step3-review.sh — thin wrapper delegating to Rust plan-review step3-review.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
PLUGIN_ROOT="${CLAUDE_PLUGIN_ROOT:-$(cd "$SCRIPT_DIR/../../.." && pwd -P)}"
exec "$PLUGIN_ROOT/scripts/larch.sh" plan-review step3-review "$@"
