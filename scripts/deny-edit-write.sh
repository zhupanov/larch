#!/usr/bin/env bash
# Fail-closed PreToolUse shim for the Rust token-scoped write guard.
# set -e is omitted so every delegation failure can emit the deny fallback.

set -uo pipefail

readonly FALLBACK_DENY='{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"The active skill is read-only-repo -- Edit/Write/NotebookEdit outside /tmp or the larch session cache is not permitted."}}'
# scripts/larch.sh exits 97 under LARCH_BOOTSTRAP_NO_INSTALL=1 when the plugin
# root has no executable for its version. Still deny, but name the repair.
readonly LARCH_NO_INSTALL_EXIT=97

hook_emit() { printf '%s\n' "$1"; }

no_install_deny() {
    local root="$PLUGIN_ROOT"
    case "$root" in
        *[!A-Za-z0-9/._@+\ -]*) root='<CLAUDE_PLUGIN_ROOT>' ;;
    esac
    printf '{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"read-only-repo guard: the larch executable is not installed for this plugin version (exit 97). Repair from a terminal outside Claude Code: CLAUDE_PLUGIN_ROOT=%s CLAUDE_PLUGIN_DATA=<absolute-dir> %s/scripts/larch.sh --version"}}\n' "$root" "$root"
}

SCRIPT_DIR="$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]}")" 2>/dev/null && pwd -P)" || {
    hook_emit "$FALLBACK_DENY"
    exit 0
}
PLUGIN_ROOT="$(CDPATH='' cd -- "$SCRIPT_DIR/.." 2>/dev/null && pwd -P)" || PLUGIN_ROOT=""
if [ ! -x "$PLUGIN_ROOT/scripts/larch.sh" ] && [ -n "${CLAUDE_PLUGIN_ROOT:-}" ]; then
    PLUGIN_ROOT="$CLAUDE_PLUGIN_ROOT"
fi
if [ -z "$PLUGIN_ROOT" ] || [ ! -x "$PLUGIN_ROOT/scripts/larch.sh" ]; then
    hook_emit "$FALLBACK_DENY"
    exit 0
fi

OUTPUT="$(CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" LARCH_BOOTSTRAP_NO_INSTALL=1 \
    "$PLUGIN_ROOT/scripts/larch.sh" hook deny-edit-write "${1:-}" 2>/dev/null)"
RC=$?
if [ "$RC" -eq "$LARCH_NO_INSTALL_EXIT" ]; then
    hook_emit "$(no_install_deny)"
elif [ "$RC" -ne 0 ]; then
    hook_emit "$FALLBACK_DENY"
elif [ -n "$OUTPUT" ]; then
    hook_emit "$OUTPUT"
fi
exit 0
