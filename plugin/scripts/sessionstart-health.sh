#!/usr/bin/env bash
# Fail-open SessionStart shim for the Rust health advisory.
# set -e is omitted so every delegation failure reaches the stripped-PATH fallback.

set -uo pipefail

readonly JQ_ONLY_FALLBACK='{"hookSpecificOutput":{"hookEventName":"SessionStart","additionalContext":"larch hook preflight: jq not on PATH (install jq for advisory hook output)."}}'
readonly JQ_GIT_FALLBACK='{"hookSpecificOutput":{"hookEventName":"SessionStart","additionalContext":"larch hook preflight: jq not on PATH and git not on PATH; install jq and git for advisory hook output."}}'

hook_emit() { printf '%s\n' "$1"; }

SCRIPT_DIR="$(CDPATH='' cd -- "${BASH_SOURCE[0]%/*}" 2>/dev/null && pwd -P 2>/dev/null)" || SCRIPT_DIR=""
PLUGIN_ROOT="$(CDPATH='' cd -- "$SCRIPT_DIR/.." 2>/dev/null && pwd -P 2>/dev/null)" || PLUGIN_ROOT=""
if [ ! -x "$PLUGIN_ROOT/scripts/larch.sh" ] && [ -n "${CLAUDE_PLUGIN_ROOT:-}" ]; then
    PLUGIN_ROOT="$CLAUDE_PLUGIN_ROOT"
fi
if [ -n "$PLUGIN_ROOT" ] && [ -x "$PLUGIN_ROOT/scripts/larch.sh" ]; then
    OUTPUT="$(CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" LARCH_BOOTSTRAP_NO_INSTALL=1 \
        "$PLUGIN_ROOT/scripts/larch.sh" hook sessionstart-health 2>/dev/null)"
    RC=$?
    if [ "$RC" -eq 0 ]; then
        [ -z "$OUTPUT" ] || hook_emit "$OUTPUT"
        exit 0
    fi
fi

# Direct stdout is the only safe fallback when jq is absent and the verified
# runtime cannot start under the stripped hook PATH.
if ! command -v jq >/dev/null 2>&1; then
    if command -v git >/dev/null 2>&1; then
        hook_emit "$JQ_ONLY_FALLBACK"
    else
        hook_emit "$JQ_GIT_FALLBACK"
    fi
fi
exit 0
