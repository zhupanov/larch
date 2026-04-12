#!/usr/bin/env bash
# check-reviewers.sh — Check external reviewer binary availability and optional health probe.
#
# Checks if codex and cursor binaries are installed. With --probe, also sends a
# trivial prompt to each available tool with a 60-second timeout to verify it is
# actually responding (catches auth failures, network issues, outages).
#
# Usage:
#   check-reviewers.sh [--probe]
#
# Outputs (key=value to stdout):
#   CODEX_AVAILABLE=true|false    — binary exists on PATH
#   CURSOR_AVAILABLE=true|false   — binary exists on PATH
#   CODEX_HEALTHY=true|false      — (only with --probe) responded to trivial prompt within timeout
#   CURSOR_HEALTHY=true|false     — (only with --probe) responded to trivial prompt within timeout
#
# Exit codes:
#   0 — always (availability/health are informational, not errors)

# No -e: exit codes from probe subprocesses are informational, not errors.
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

PROBE=false
if [[ "${1:-}" == "--probe" ]]; then
    PROBE=true
fi

CODEX_AVAILABLE="false"
CURSOR_AVAILABLE="false"

if command -v codex >/dev/null 2>&1; then
    CODEX_AVAILABLE="true"
fi

if command -v cursor >/dev/null 2>&1; then
    CURSOR_AVAILABLE="true"
fi

echo "CODEX_AVAILABLE=$CODEX_AVAILABLE"
echo "CURSOR_AVAILABLE=$CURSOR_AVAILABLE"

if [[ "$PROBE" == "true" ]]; then
    CODEX_HEALTHY="false"
    CURSOR_HEALTHY="false"

    PROBE_DIR=$(mktemp -d /tmp/larch-probe-XXXXXX)
    # Clean up probe tmpdir on exit
    trap 'rm -rf "$PROBE_DIR"' EXIT

    # Launch probes in parallel for available tools
    if [[ "$CODEX_AVAILABLE" == "true" ]]; then
        "$SCRIPT_DIR/run-external-reviewer.sh" \
            --tool codex \
            --output "$PROBE_DIR/codex-probe.txt" \
            --timeout 60 \
            -- codex exec --full-auto -C "$PWD" \
            --output-last-message "$PROBE_DIR/codex-probe.txt" \
            "Respond with OK" \
            >"$PROBE_DIR/codex-wrapper.log" 2>&1 &
    fi

    if [[ "$CURSOR_AVAILABLE" == "true" ]]; then
        "$SCRIPT_DIR/run-external-reviewer.sh" \
            --tool cursor \
            --output "$PROBE_DIR/cursor-probe.txt" \
            --timeout 60 \
            --capture-stdout \
            -- cursor agent -p --force --trust --model gpt-5.4-medium --workspace "$PWD" \
            "Respond with OK" \
            >"$PROBE_DIR/cursor-wrapper.log" 2>&1 &
    fi

    # Build sentinel list for wait-for-reviewers.sh
    SENTINELS=()
    if [[ "$CODEX_AVAILABLE" == "true" ]]; then
        SENTINELS+=("$PROBE_DIR/codex-probe.txt.done")
    fi
    if [[ "$CURSOR_AVAILABLE" == "true" ]]; then
        SENTINELS+=("$PROBE_DIR/cursor-probe.txt.done")
    fi

    if [[ ${#SENTINELS[@]} -gt 0 ]]; then
        # Wait for probes (120s = 60s timeout + 60s grace)
        "$SCRIPT_DIR/wait-for-reviewers.sh" --timeout 120 "${SENTINELS[@]}" \
            >"$PROBE_DIR/wait.log" 2>&1 || true
    fi

    # Check codex probe result
    if [[ "$CODEX_AVAILABLE" == "true" ]]; then
        if [[ -f "$PROBE_DIR/codex-probe.txt.done" ]]; then
            CODEX_EXIT=$(cat "$PROBE_DIR/codex-probe.txt.done")
            if [[ "$CODEX_EXIT" == "0" ]]; then
                # Verify output is non-empty
                if [[ -s "$PROBE_DIR/codex-probe.txt" ]]; then
                    CODEX_HEALTHY="true"
                fi
            fi
        fi
    fi

    # Check cursor probe result
    if [[ "$CURSOR_AVAILABLE" == "true" ]]; then
        if [[ -f "$PROBE_DIR/cursor-probe.txt.done" ]]; then
            CURSOR_EXIT=$(cat "$PROBE_DIR/cursor-probe.txt.done")
            if [[ "$CURSOR_EXIT" == "0" ]]; then
                if [[ -s "$PROBE_DIR/cursor-probe.txt" ]]; then
                    CURSOR_HEALTHY="true"
                fi
            fi
        fi
    fi

    echo "CODEX_HEALTHY=$CODEX_HEALTHY"
    echo "CURSOR_HEALTHY=$CURSOR_HEALTHY"
fi
