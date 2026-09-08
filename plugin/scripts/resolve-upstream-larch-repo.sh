#!/usr/bin/env bash
# resolve-upstream-larch-repo.sh — resolve the plugin's canonical upstream GitHub repo.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/.." && pwd -P)"
export CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT"
exec "$PLUGIN_ROOT/scripts/larch.sh" plugin resolve-repository
