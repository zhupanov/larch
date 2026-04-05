#!/usr/bin/env bash
# add-merged-emoji.sh — Add :merged: emoji to a Slack PR announcement.
#
# Wraps dev-tools/scripts/add-slack-emoji.sh with token cleaning.
# This wrapper exists so that callers (skills, ad-hoc commands) can invoke
# it without $() command substitution, which triggers Claude Code's
# interactive permission prompt.
#
# Usage:
#   add-merged-emoji.sh --slack-ts TIMESTAMP --channel NAME [--dev-tools-dir PATH]
#
# Environment:
#   SLACK_BOT_TOKEN — required
#
# Exit codes:
#   0 — emoji added successfully
#   1 — SLACK_BOT_TOKEN not set
#   2 — missing arguments
#   3 — add-slack-emoji.sh failed

set -euo pipefail

usage() { echo "Usage: add-merged-emoji.sh --slack-ts TIMESTAMP --channel NAME [--dev-tools-dir PATH]" >&2; }

SLACK_TS=""
CHANNEL=""
DEV_TOOLS_DIR="dev-tools"
DEV_TOOLS_DIR_EXPLICIT=false
while [[ $# -gt 0 ]]; do
    case "$1" in
        --slack-ts) SLACK_TS="${2:?--slack-ts requires a value}"; shift 2 ;;
        --channel) CHANNEL="${2:?--channel requires a value}"; shift 2 ;;
        --dev-tools-dir) DEV_TOOLS_DIR="${2:?--dev-tools-dir requires a value}"; DEV_TOOLS_DIR_EXPLICIT=true; shift 2 ;;
        --help) usage; exit 0 ;;
        *) echo "Unknown option: $1" >&2; usage; exit 2 ;;
    esac
done

# Auto-detect dev-tools directory when running from within the dev-tools repo itself
# (where there is no dev-tools/ subdirectory). Only applies when --dev-tools-dir was not explicit.
if [[ "$DEV_TOOLS_DIR_EXPLICIT" == "false" ]] && [[ ! -f "$DEV_TOOLS_DIR/scripts/add-slack-emoji.sh" ]] && [[ -f "scripts/add-slack-emoji.sh" ]]; then
    DEV_TOOLS_DIR="."
fi

if [[ -z "$SLACK_TS" ]] || [[ -z "$CHANNEL" ]]; then
    echo "ERROR: --slack-ts and --channel are required" >&2
    usage; exit 2
fi

if [[ -z "${SLACK_BOT_TOKEN:-}" ]]; then
    echo "⚠ SLACK_BOT_TOKEN not set. Skipping :merged: emoji."
    exit 1
fi

CLEAN_TOKEN=$(echo -n "$SLACK_BOT_TOKEN" | tr -d '[:space:]')

bash "$DEV_TOOLS_DIR/scripts/add-slack-emoji.sh" \
    --channel "$CHANNEL" \
    --emoji ":merged:" \
    --slack_timestamp "$SLACK_TS" \
    --token "$CLEAN_TOKEN"
