#!/usr/bin/env bash
# Claude Code PostToolUse hook wrapper for repeated Read polling.
# set -e intentionally omitted: hooks must never block tool use.

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" 2>/dev/null && pwd -P)" || exit 0
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/.." 2>/dev/null && pwd -P)" || PLUGIN_ROOT=""
if [ ! -x "$PLUGIN_ROOT/scripts/larch.sh" ] && [ -n "${CLAUDE_PLUGIN_ROOT:-}" ]; then
    PLUGIN_ROOT="$CLAUDE_PLUGIN_ROOT"
fi

if [ -z "$PLUGIN_ROOT" ] || [ ! -x "$PLUGIN_ROOT/scripts/larch.sh" ]; then
    exit 0
fi

LARCH_BOOTSTRAP_NO_INSTALL=1 \
    "$PLUGIN_ROOT/scripts/larch.sh" hook anti-read-poll 2>/dev/null || true
exit 0
