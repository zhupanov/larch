#!/usr/bin/env bash
# Fail-open SessionStart shim for the Rust statusline hook.

set -uo pipefail

SCRIPT_DIR="$(CDPATH='' cd -- "${BASH_SOURCE[0]%/*}" 2>/dev/null && pwd -P 2>/dev/null)" || exit 0
PLUGIN_ROOT="$(CDPATH='' cd -- "$SCRIPT_DIR/.." 2>/dev/null && pwd -P 2>/dev/null)" || PLUGIN_ROOT=""
if [ ! -x "$PLUGIN_ROOT/scripts/larch.sh" ] && [ -n "${CLAUDE_PLUGIN_ROOT:-}" ]; then
    PLUGIN_ROOT="$CLAUDE_PLUGIN_ROOT"
fi
[ -n "$PLUGIN_ROOT" ] && [ -x "$PLUGIN_ROOT/scripts/larch.sh" ] || exit 0

CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" LARCH_BOOTSTRAP_NO_INSTALL=1 \
    "$PLUGIN_ROOT/scripts/larch.sh" hook sessionstart-statusline >/dev/null 2>&1 || true
exit 0
