#!/usr/bin/env bash
set -euo pipefail

if [ -z "${CLAUDE_PLUGIN_ROOT:-}" ]; then
  _script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
  CLAUDE_PLUGIN_ROOT="$(cd "$_script_dir/../../.." && pwd -P)"
fi
export CLAUDE_PLUGIN_ROOT

exec "$CLAUDE_PLUGIN_ROOT/scripts/larch.sh" design step5b-prepare "$@"
