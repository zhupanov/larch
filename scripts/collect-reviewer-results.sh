#!/usr/bin/env bash
# collect-reviewer-results.sh — Collect, validate, and optionally retry external reviewer outputs.
#
# Consolidates the post-launch validation+retry pattern used across all skills.
# Wraps wait-for-reviewers.sh, validates each output, retries once on empty via
# .meta files written by run-external-reviewer.sh, and emits structured results.
#
# Usage:
#   collect-reviewer-results.sh --timeout <seconds> [--write-health <path>] \
#     <output-file> [<output-file> ...]
#
# Options:
#   --timeout <seconds>    Timeout for wait-for-reviewers.sh (e.g., 1860)
#   --write-health <path>  Write updated CODEX_HEALTHY/CURSOR_HEALTHY to file.
#                          Health is monotonic per tool: any failure sets the tool
#                          permanently unhealthy. A later successful instance does
#                          NOT flip it back to healthy.
#
# Arguments:
#   One or more output file paths (from run-external-reviewer.sh invocations).
#   Sentinel paths are derived by appending .done to each output file.
#   Metadata paths are derived by appending .meta to each output file.
#
# Output (KEY=value blocks on stdout, one block per reviewer, separated by blank lines):
#   REVIEWER_FILE=<output-path>
#   TOOL=<codex|cursor|unknown>
#   STATUS=<OK|TIMED_OUT|FAILED|EMPTY_OUTPUT|SENTINEL_TIMEOUT>
#   EXIT_CODE=<N>
#   HEALTHY=<true|false>
#
# Exit codes:
#   0 — always (results are informational, not errors)

# No -e: exit codes from reviewer subprocesses and retries are informational.
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

TIMEOUT=""
WRITE_HEALTH=""
OUTPUT_FILES=()

while [[ $# -gt 0 ]]; do
    case "$1" in
        --timeout)
            TIMEOUT="${2:?--timeout requires a value}"; shift 2 ;;
        --write-health)
            WRITE_HEALTH="${2:?--write-health requires a path}"; shift 2 ;;
        --help)
            echo "Usage: collect-reviewer-results.sh --timeout <seconds> [--write-health <path>] <output-file>..." >&2
            exit 0 ;;
        -*)
            echo "collect-reviewer-results.sh: unknown option: $1" >&2; exit 1 ;;
        *)
            OUTPUT_FILES+=("$1"); shift ;;
    esac
done

if [[ -z "$TIMEOUT" ]]; then
    echo "collect-reviewer-results.sh: --timeout is required" >&2
    exit 1
fi

if [[ ${#OUTPUT_FILES[@]} -eq 0 ]]; then
    echo "collect-reviewer-results.sh: at least one output file is required" >&2
    exit 1
fi

# --- Derive tool name from output filename ---
derive_tool() {
    local basename
    basename=$(basename "$1")
    if [[ "$basename" == *codex* ]]; then
        echo "codex"
    elif [[ "$basename" == *cursor* ]]; then
        echo "cursor"
    else
        echo "unknown"
    fi
}

# --- 1. Build sentinel paths and wait ---
SENTINELS=()
for f in "${OUTPUT_FILES[@]}"; do
    SENTINELS+=("${f}.done")
done

WAIT_OUTPUT=$("$SCRIPT_DIR/wait-for-reviewers.sh" --timeout "$TIMEOUT" "${SENTINELS[@]}" 2>/dev/null) || true

# Parse wait output for TIMEOUT indicators
declare -A SENTINEL_TIMED_OUT
while IFS= read -r line; do
    if [[ "$line" == TIMEOUT\ * ]]; then
        # Extract the sentinel basename for matching
        local_sentinel="${line#TIMEOUT }"
        # Remove trailing colon if present
        local_sentinel="${local_sentinel%%:*}"
        SENTINEL_TIMED_OUT["$local_sentinel"]=true
    fi
done <<< "$WAIT_OUTPUT"

# --- 2. Validate each output and collect results ---
# Track health per tool (monotonic: once false, stays false)
declare -A TOOL_HEALTHY
RETRY_FILES=()
RETRY_INDICES=()

RESULTS=()
for i in "${!OUTPUT_FILES[@]}"; do
    OUTPUT="${OUTPUT_FILES[$i]}"
    SENTINEL="${OUTPUT}.done"
    META="${OUTPUT}.meta"
    TOOL=$(derive_tool "$OUTPUT")
    STATUS="OK"
    EXIT_CODE="0"
    HEALTHY="true"

    SENTINEL_BASE=$(basename "$SENTINEL")
    if [[ "${SENTINEL_TIMED_OUT[$SENTINEL_BASE]+_}" ]]; then
        # wait-for-reviewers.sh reported TIMEOUT (sentinel never appeared)
        STATUS="SENTINEL_TIMEOUT"
        EXIT_CODE="124"
        HEALTHY="false"
    elif [[ -f "$SENTINEL" ]]; then
        EXIT_CODE=$(cat "$SENTINEL" 2>/dev/null || echo "99")
        if [[ "$EXIT_CODE" == "124" ]]; then
            STATUS="TIMED_OUT"
            HEALTHY="false"
        elif [[ "$EXIT_CODE" != "0" ]]; then
            STATUS="FAILED"
            HEALTHY="false"
        elif [[ ! -s "$OUTPUT" ]]; then
            # Exit 0 but empty output — candidate for retry
            STATUS="EMPTY_OUTPUT"
            HEALTHY="false"
            # Queue for retry if .meta exists
            if [[ -f "$META" ]]; then
                RETRY_FILES+=("$OUTPUT")
                RETRY_INDICES+=("$i")
            fi
        fi
    else
        # Sentinel doesn't exist (shouldn't happen after wait, but be defensive)
        STATUS="SENTINEL_TIMEOUT"
        EXIT_CODE="124"
        HEALTHY="false"
    fi

    # Monotonic health: if this tool was already marked unhealthy, keep it
    if [[ "${TOOL_HEALTHY[$TOOL]+_}" && "${TOOL_HEALTHY[$TOOL]}" == "false" ]]; then
        HEALTHY="false"
    fi
    TOOL_HEALTHY["$TOOL"]="$HEALTHY"

    RESULTS+=("REVIEWER_FILE=$OUTPUT|TOOL=$TOOL|STATUS=$STATUS|EXIT_CODE=$EXIT_CODE|HEALTHY=$HEALTHY")
done

# --- 3. Retry empty outputs using .meta files ---
if [[ ${#RETRY_FILES[@]} -gt 0 ]]; then
    RETRY_SENTINELS=()
    for j in "${!RETRY_FILES[@]}"; do
        ORIG_OUTPUT="${RETRY_FILES[$j]}"
        META="${ORIG_OUTPUT}.meta"
        RETRY_OUTPUT="${ORIG_OUTPUT%.txt}-retry.txt"

        # Parse .meta file
        META_TOOL=""
        META_TIMEOUT=""
        META_CAPTURE=""
        META_CMD=""
        META_ORIG_OUTPUT=""
        while IFS='=' read -r key value || [[ -n "$key" ]]; do
            case "$key" in
                TOOL)         META_TOOL="$value" ;;
                TIMEOUT)      META_TIMEOUT="$value" ;;
                CAPTURE_STDOUT) META_CAPTURE="$value" ;;
                OUTPUT_FILE)  META_ORIG_OUTPUT="$value" ;;
                CMD)          META_CMD="$value" ;;
            esac
        done < "$META"

        if [[ -z "$META_CMD" || -z "$META_TOOL" ]]; then
            continue
        fi

        # Build retry command: run-external-reviewer.sh with updated output path
        RETRY_ARGS=(--tool "$META_TOOL" --output "$RETRY_OUTPUT" --timeout "${META_TIMEOUT:-120}")
        if [[ "$META_CAPTURE" == "true" ]]; then
            RETRY_ARGS+=(--capture-stdout)
        fi
        RETRY_ARGS+=(--)

        # Reconstruct command from shell-quoted CMD, replacing original output path
        # The CMD was saved via printf '%q', so eval reconstructs the original args
        RECONSTRUCTED_CMD="$META_CMD"
        # Replace original output path with retry path in the reconstructed command
        if [[ -n "$META_ORIG_OUTPUT" ]]; then
            RECONSTRUCTED_CMD="${RECONSTRUCTED_CMD//$META_ORIG_OUTPUT/$RETRY_OUTPUT}"
        fi

        # Launch retry in background — eval is intentional: CMD was serialized via
        # printf '%q' and must be re-expanded to reconstruct original arg boundaries.
        # shellcheck disable=SC2294
        eval "$SCRIPT_DIR/run-external-reviewer.sh $(printf '%q ' "${RETRY_ARGS[@]}") $RECONSTRUCTED_CMD" >/dev/null 2>&1 &
        RETRY_SENTINELS+=("${RETRY_OUTPUT}.done")
    done

    # Wait for retry sentinels (short timeout)
    if [[ ${#RETRY_SENTINELS[@]} -gt 0 ]]; then
        "$SCRIPT_DIR/wait-for-reviewers.sh" --timeout 180 "${RETRY_SENTINELS[@]}" >/dev/null 2>&1 || true

        # Check retry results and update
        for j in "${!RETRY_FILES[@]}"; do
            ORIG_OUTPUT="${RETRY_FILES[$j]}"
            RETRY_OUTPUT="${ORIG_OUTPUT%.txt}-retry.txt"
            RETRY_SENTINEL="${RETRY_OUTPUT}.done"
            IDX="${RETRY_INDICES[$j]}"
            TOOL=$(derive_tool "$ORIG_OUTPUT")

            if [[ -f "$RETRY_SENTINEL" ]]; then
                RETRY_EXIT=$(cat "$RETRY_SENTINEL" 2>/dev/null || echo "99")
                if [[ "$RETRY_EXIT" == "0" && -s "$RETRY_OUTPUT" ]]; then
                    # Retry succeeded — update result
                    HEALTHY="true"
                    # Respect monotonic health
                    if [[ "${TOOL_HEALTHY[$TOOL]+_}" && "${TOOL_HEALTHY[$TOOL]}" == "false" ]]; then
                        HEALTHY="false"
                    else
                        TOOL_HEALTHY["$TOOL"]="true"
                    fi
                    RESULTS[IDX]="REVIEWER_FILE=$RETRY_OUTPUT|TOOL=$TOOL|STATUS=OK|EXIT_CODE=0|HEALTHY=$HEALTHY"
                fi
                # If retry also failed, keep original EMPTY_OUTPUT result
            fi
        done
    fi
fi

# --- 4. Emit structured results ---
FIRST=true
for result in "${RESULTS[@]}"; do
    if [[ "$FIRST" == "true" ]]; then
        FIRST=false
    else
        echo ""
    fi
    # Convert pipe-delimited to newlines
    echo "$result" | tr '|' '\n'
done

# --- 5. Write health file (if requested, monotonic per tool) ---
if [[ -n "$WRITE_HEALTH" ]]; then
    HEALTH_CODEX="${TOOL_HEALTHY[codex]:-true}"
    HEALTH_CURSOR="${TOOL_HEALTHY[cursor]:-true}"

    HEALTH_TMPFILE=$(mktemp "${WRITE_HEALTH}.tmp.XXXXXX")
    {
        echo "CODEX_HEALTHY=$HEALTH_CODEX"
        echo "CURSOR_HEALTHY=$HEALTH_CURSOR"
    } > "$HEALTH_TMPFILE"
    mv "$HEALTH_TMPFILE" "$WRITE_HEALTH"
fi
