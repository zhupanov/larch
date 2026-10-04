#!/usr/bin/env bash
# check-stale-plugin.sh — Detect version skew between the installed larch plugin
# cache and the working-tree checkout.
#
# Usage:
#   check-stale-plugin.sh [--installed-plugin-json <path>] [--working-tree-root <path>]
#
# Options:
#   --installed-plugin-json <path>  Path to the installed plugin's plugin.json.
#                                   Default: ${CLAUDE_PLUGIN_ROOT}/.claude-plugin/plugin.json
#   --working-tree-root <path>      Path to the working-tree repo root.
#                                   Default: $(git rev-parse --show-toplevel)
#
# Larch dev-clone detection: the working-tree root must contain
# skills/implement/SKILL.md. Without this marker the script emits
# STALE_PLUGIN_CHECK=not-a-dev-clone and exits 0.
#
# Output (stdout, KEY=value, exits 0 for detection outcomes):
#   STALE_PLUGIN_CHECK=skip               CLAUDE_PLUGIN_ROOT unset or plugin.json missing
#   STALE_PLUGIN_CHECK=not-a-dev-clone    Not in a larch dev clone
#   STALE_PLUGIN_CHECK=versions-match     installed == working-tree
#   STALE_PLUGIN_CHECK=working-tree-ahead working-tree > installed (warn)
#     STALE_PLUGIN_INSTALLED_VERSION=<X.Y.Z>
#     STALE_PLUGIN_WORKING_TREE_VERSION=<X.Y.Z>
#   STALE_PLUGIN_CHECK=installed-ahead    installed > working-tree (no warn)
#
# Invalid CLI usage (unknown flags or missing flag values) exits 1.

set -euo pipefail

is_larch_dev_clone() {
    local root=${1:-}
    [ -n "$root" ] || root=$(git rev-parse --show-toplevel 2>/dev/null || true)
    [ -n "$root" ] || return 1
    [ -f "$root/skills/implement/SKILL.md" ]
}

INSTALLED_PLUGIN_JSON=""
WORKING_TREE_ROOT=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --installed-plugin-json)
            [[ $# -ge 2 ]] || { printf 'check-stale-plugin.sh: --installed-plugin-json requires a value\n' >&2; exit 1; }
            INSTALLED_PLUGIN_JSON="$2"; shift 2 ;;
        --working-tree-root)
            [[ $# -ge 2 ]] || { printf 'check-stale-plugin.sh: --working-tree-root requires a value\n' >&2; exit 1; }
            WORKING_TREE_ROOT="$2";     shift 2 ;;
        *) printf 'check-stale-plugin.sh: unknown option: %s\n' "$1" >&2; exit 1 ;;
    esac
done

emit_kv() { printf '%s=%s\n' "$1" "$2"; }

# --- Resolve installed plugin.json ---
if [ -z "$INSTALLED_PLUGIN_JSON" ]; then
    if [ -z "${CLAUDE_PLUGIN_ROOT:-}" ]; then
        emit_kv STALE_PLUGIN_CHECK skip
        exit 0
    fi
    INSTALLED_PLUGIN_JSON="${CLAUDE_PLUGIN_ROOT}/.claude-plugin/plugin.json"
fi
if [ ! -f "$INSTALLED_PLUGIN_JSON" ]; then
    emit_kv STALE_PLUGIN_CHECK skip
    exit 0
fi

# --- Resolve working-tree root ---
if [ -z "$WORKING_TREE_ROOT" ]; then
    WORKING_TREE_ROOT=$(git rev-parse --show-toplevel 2>/dev/null || true)
fi
if [ -z "$WORKING_TREE_ROOT" ] || [ ! -d "$WORKING_TREE_ROOT" ]; then
    emit_kv STALE_PLUGIN_CHECK skip
    exit 0
fi

# --- Dev-clone detection ---
if ! is_larch_dev_clone "$WORKING_TREE_ROOT"; then
    emit_kv STALE_PLUGIN_CHECK not-a-dev-clone
    exit 0
fi

# --- Working-tree plugin.json ---
WT_PLUGIN_JSON="$WORKING_TREE_ROOT/.claude-plugin/plugin.json"
if [ ! -f "$WT_PLUGIN_JSON" ]; then
    emit_kv STALE_PLUGIN_CHECK skip
    exit 0
fi

# --- Extract versions (grep + sed; no jq dependency) ---
extract_version() {
    local line
    line=$(grep '"version"' "$1" 2>/dev/null | head -1 || true)
    if [ -z "$line" ]; then
        printf '\n'
        return 0
    fi
    line=${line#*\"version\"}
    line=${line#*:}
    line=${line#*\"}
    printf '%s\n' "${line%%\"*}"
}

INSTALLED_VERSION=$(extract_version "$INSTALLED_PLUGIN_JSON")
WT_VERSION=$(extract_version "$WT_PLUGIN_JSON")

if [ -z "$INSTALLED_VERSION" ] || [ -z "$WT_VERSION" ]; then
    emit_kv STALE_PLUGIN_CHECK skip
    exit 0
fi

# --- Numeric X.Y.Z comparison using awk (Bash 3.2 compatible) ---
version_cmp() {
    local a=$1 b=$2
    awk -v a="$a" -v b="$b" '
    BEGIN {
        n = split(a, av, ".")
        m = split(b, bv, ".")
        for (i = 1; i <= 3; i++) {
            ai = (i <= n) ? av[i]+0 : 0
            bi = (i <= m) ? bv[i]+0 : 0
            if (ai > bi) { print "gt"; exit }
            if (ai < bi) { print "lt"; exit }
        }
        print "eq"
    }'
}

CMP=$(version_cmp "$WT_VERSION" "$INSTALLED_VERSION")

case "$CMP" in
    gt)
        emit_kv STALE_PLUGIN_CHECK working-tree-ahead
        emit_kv STALE_PLUGIN_INSTALLED_VERSION "$INSTALLED_VERSION"
        emit_kv STALE_PLUGIN_WORKING_TREE_VERSION "$WT_VERSION"
        ;;
    lt)
        emit_kv STALE_PLUGIN_CHECK installed-ahead
        ;;
    *)
        emit_kv STALE_PLUGIN_CHECK versions-match
        ;;
esac

exit 0
