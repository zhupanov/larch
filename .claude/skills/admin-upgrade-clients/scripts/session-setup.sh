#!/usr/bin/env bash
# session-setup.sh — Session setup for /admin-upgrade-clients.
#
# Combines: mktemp, SLACK_BOT_TOKEN check, read client-repos.json,
#           git fetch origin main, and git rev-parse origin/main.
#
# Usage:
#   session-setup.sh
#
# Output (stdout, KEY=VALUE):
#   UC_TMPDIR=<path>
#   SLACK_TOKEN_OK=true|false
#   CLIENT_REPOS=<json array>
#   CLIENT_COUNT=<N>
#   TARGET_SHA=<full sha>
#   TARGET_SHA_SHORT=<7 char sha>
#
# Exit codes: 0 success, 1 failure (prints ERROR=<message>)

set -euo pipefail

# Create temp directory
UC_TMPDIR=$(mktemp -d /tmp/claude-admin-upgrade-clients-XXXXXX)
echo "UC_TMPDIR=$UC_TMPDIR"

# Check SLACK_BOT_TOKEN
if [[ -n "${SLACK_BOT_TOKEN:-}" ]]; then
  echo "SLACK_TOKEN_OK=true"
else
  echo "SLACK_TOKEN_OK=false"
fi

# Read client repos
CLIENT_REPOS_FILE="$PWD/data/client-repos.json"
if [[ ! -f "$CLIENT_REPOS_FILE" ]]; then
  echo "ERROR=Client repos file not found: $CLIENT_REPOS_FILE"
  exit 1
fi

CLIENT_REPOS=$(jq -c . "$CLIENT_REPOS_FILE")
CLIENT_COUNT=$(echo "$CLIENT_REPOS" | jq 'length')
echo "CLIENT_REPOS=$CLIENT_REPOS"
echo "CLIENT_COUNT=$CLIENT_COUNT"

# Fetch and get dev-tools main SHA
if ! git fetch origin main 2>/dev/null; then
  echo "ERROR=Could not fetch origin/main. Network or remote configuration issue."
  exit 1
fi

TARGET_SHA=$(git rev-parse origin/main)
TARGET_SHA_SHORT="${TARGET_SHA:0:7}"
echo "TARGET_SHA=$TARGET_SHA"
echo "TARGET_SHA_SHORT=$TARGET_SHA_SHORT"
