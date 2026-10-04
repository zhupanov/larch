#!/usr/bin/env bash
# Fail-open Stop shim for the Rust post-review boundary guard.
# set -e is omitted so a missing verified runtime stays silent.

set -uo pipefail

hook_emit() { printf '%s\n' "$1"; }

SCRIPT_DIR="$(CDPATH='' cd -- "${BASH_SOURCE[0]%/*}" 2>/dev/null && pwd -P 2>/dev/null)" || exit 0
PLUGIN_ROOT="$(CDPATH='' cd -- "$SCRIPT_DIR/../../.." 2>/dev/null && pwd -P 2>/dev/null)" || PLUGIN_ROOT=""
if [ ! -x "$PLUGIN_ROOT/scripts/larch.sh" ] && [ -n "${CLAUDE_PLUGIN_ROOT:-}" ]; then
    PLUGIN_ROOT="$CLAUDE_PLUGIN_ROOT"
fi
[ -n "$PLUGIN_ROOT" ] && [ -x "$PLUGIN_ROOT/scripts/larch.sh" ] || exit 0

OUTPUT="$(CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" LARCH_BOOTSTRAP_NO_INSTALL=1 \
    "$PLUGIN_ROOT/scripts/larch.sh" hook stop-fail-close 2>/dev/null)"
RC=$?
if [ "$RC" -eq 0 ] && [ -n "$OUTPUT" ]; then
    hook_emit "$OUTPUT"
fi
exit 0
