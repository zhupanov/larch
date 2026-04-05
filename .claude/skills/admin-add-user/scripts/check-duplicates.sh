#!/usr/bin/env bash
# check-duplicates.sh — Check if a user already exists in identity maps.
#
# Usage:
#   check-duplicates.sh --email <EMAIL> --slack-id <ID> --github-username <HANDLE>
#
# Output (stdout, KEY=VALUE):
#   SKIP_EMAIL_MAP=true|false
#   SKIP_GITHUB_MAP=true|false
#   ALL_DUPLICATE=true|false
#
# Exit codes:
#   0 — ok (proceed) or all duplicates (nothing to do)
#   1 — conflicting entry found (abort)

set -euo pipefail

EMAIL=""
SLACK_ID=""
GITHUB_USERNAME=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --email)             EMAIL="$2"; shift 2 ;;
    --slack-id)          SLACK_ID="$2"; shift 2 ;;
    --github-username)   GITHUB_USERNAME="$2"; shift 2 ;;
    *) echo "ERROR=Unknown argument: $1" >&2; exit 1 ;;
  esac
done

if [[ -z "$EMAIL" || -z "$SLACK_ID" || -z "$GITHUB_USERNAME" ]]; then
  echo "ERROR=Missing required arguments: --email, --slack-id, --github-username" >&2
  exit 1
fi

SKIP_EMAIL_MAP=false
SKIP_GITHUB_MAP=false

EMAIL_MAP="data/email_to_slack_userid_map.json"
GITHUB_MAP="data/github_username_to_email_map.json"

# Check email map
if [[ -f "$EMAIL_MAP" ]]; then
  EMAIL_EXISTS=$(jq --arg email "$EMAIL" 'has($email)' "$EMAIL_MAP")
  if [[ "$EMAIL_EXISTS" == "true" ]]; then
    EXISTING_SID=$(jq -r --arg email "$EMAIL" '.[$email]' "$EMAIL_MAP")
    if [[ "$EXISTING_SID" == "$SLACK_ID" ]]; then
      SKIP_EMAIL_MAP=true
    else
      echo "SKIP_EMAIL_MAP=false"
      echo "SKIP_GITHUB_MAP=false"
      echo "ALL_DUPLICATE=false"
      echo "ERROR=Email $EMAIL already exists with a different Slack ID ($EXISTING_SID). Resolve manually."
      exit 1
    fi
  fi
fi

# Check GitHub map
if [[ -f "$GITHUB_MAP" ]]; then
  GH_EXISTS=$(jq --arg gh "$GITHUB_USERNAME" 'has($gh)' "$GITHUB_MAP")
  if [[ "$GH_EXISTS" == "true" ]]; then
    EXISTING_EMAIL=$(jq -r --arg gh "$GITHUB_USERNAME" '.[$gh]' "$GITHUB_MAP")
    if [[ "$EXISTING_EMAIL" == "$EMAIL" ]]; then
      SKIP_GITHUB_MAP=true
    else
      echo "SKIP_EMAIL_MAP=$SKIP_EMAIL_MAP"
      echo "SKIP_GITHUB_MAP=false"
      echo "ALL_DUPLICATE=false"
      echo "ERROR=GitHub username $GITHUB_USERNAME already exists with a different email ($EXISTING_EMAIL). Resolve manually."
      exit 1
    fi
  fi
fi

ALL_DUPLICATE=false
if [[ "$SKIP_EMAIL_MAP" == "true" && "$SKIP_GITHUB_MAP" == "true" ]]; then
  ALL_DUPLICATE=true
fi

echo "SKIP_EMAIL_MAP=$SKIP_EMAIL_MAP"
echo "SKIP_GITHUB_MAP=$SKIP_GITHUB_MAP"
echo "ALL_DUPLICATE=$ALL_DUPLICATE"
