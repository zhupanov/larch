#!/usr/bin/env bash
# post-tracking-issue.sh — compatibility delegate for tracking post-issue.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
PLUGIN_ROOT="${CLAUDE_PLUGIN_ROOT:-$(cd "$SCRIPT_DIR/../../.." && pwd -P)}"
exec "$PLUGIN_ROOT/scripts/larch.sh" tracking post-issue "$@"
