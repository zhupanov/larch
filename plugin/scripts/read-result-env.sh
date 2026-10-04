#!/usr/bin/env bash
# read-result-env.sh — safely convert result-env KVs into a sourceable allowlisted env.
# Delegates allowlist filtering, symlink refusal, CR/LF rejection, fallback-input
# logic, WARN/ERROR stdout replay, and single-quote encoding to the Rust
# design lifecycle owner via larch.sh (#8580 migrated this verb off Python).

set -euo pipefail

_RRE_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
_REPO_ROOT="$(cd "$_RRE_SCRIPT_DIR/.." && pwd -P)"
CLAUDE_PLUGIN_ROOT="$_REPO_ROOT"
export CLAUDE_PLUGIN_ROOT

exec "$CLAUDE_PLUGIN_ROOT/scripts/larch.sh" design read-result-env "$@"
