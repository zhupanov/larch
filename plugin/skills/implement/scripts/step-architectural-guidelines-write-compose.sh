#!/usr/bin/env bash
# step-architectural-guidelines-write-compose.sh — thin wrapper for compose-time guideline assessment writes.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
PLUGIN_ROOT="${CLAUDE_PLUGIN_ROOT:-$(cd "$SCRIPT_DIR/../../.." && pwd -P)}"
IMPLEMENT_TMPDIR="${IMPLEMENT_TMPDIR:?IMPLEMENT_TMPDIR required}"
ASSESSMENT_ARG="${1:?assessment file path required}"
OUTCOME="${2:-}"

exec env CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" "$PLUGIN_ROOT/scripts/larch.sh" architectural-guidelines write-compose-assessment \
  --implement-tmpdir "$IMPLEMENT_TMPDIR" \
  --assessment-file "$ASSESSMENT_ARG" \
  --outcome "$OUTCOME"
