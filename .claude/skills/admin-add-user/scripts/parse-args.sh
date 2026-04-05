#!/usr/bin/env bash
# parse-args.sh — Parse and validate /admin-add-user arguments.
#
# Usage:
#   parse-args.sh --slack-id <ID> --github-username <HANDLE> --email <EMAIL>
#
# Output (stdout, KEY=VALUE):
#   SLACK_ID=<value>
#   GITHUB_USERNAME=<value>
#   EMAIL=<value>
#   VALID=true|false
#   ERROR=<message>   (only when VALID=false)
#
# Exit codes: 0 success, 1 validation failure

set -euo pipefail

SLACK_ID=""
GITHUB_USERNAME=""
EMAIL=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --slack-id)
      [[ $# -ge 2 ]] || { echo "VALID=false"; echo "ERROR=--slack-id requires a value"; exit 1; }
      SLACK_ID="$2"; shift 2 ;;
    --github-username)
      [[ $# -ge 2 ]] || { echo "VALID=false"; echo "ERROR=--github-username requires a value"; exit 1; }
      GITHUB_USERNAME="$2"; shift 2 ;;
    --email)
      [[ $# -ge 2 ]] || { echo "VALID=false"; echo "ERROR=--email requires a value"; exit 1; }
      EMAIL="$2"; shift 2 ;;
    *)
      echo "VALID=false"
      echo "ERROR=Unknown argument: $1"
      exit 1 ;;
  esac
done

# Check all required
if [[ -z "$SLACK_ID" || -z "$GITHUB_USERNAME" || -z "$EMAIL" ]]; then
  echo "VALID=false"
  echo "ERROR=Missing required argument. Usage: /admin-add-user --slack-id <ID> --github-username <HANDLE> --email <EMAIL>"
  exit 1
fi

# Validate email
if [[ ! "$EMAIL" =~ ^[^@]+@[^@]+\.[^@]+$ ]]; then
  echo "VALID=false"
  echo "ERROR=Invalid email format: $EMAIL"
  exit 1
fi

# Validate Slack ID (human user IDs: U followed by 8-11 uppercase alphanumeric)
if [[ ! "$SLACK_ID" =~ ^U[A-Z0-9]{8,11}$ ]]; then
  echo "VALID=false"
  echo "ERROR=Invalid Slack ID format: $SLACK_ID. Expected format: U followed by 8-11 uppercase alphanumeric characters (human users only, not bots)."
  exit 1
fi

# Validate GitHub username (no spaces)
if [[ "$GITHUB_USERNAME" =~ [[:space:]] ]]; then
  echo "VALID=false"
  echo "ERROR=GitHub username should be a login handle (e.g., 'octocat'), not a display name (e.g., 'Mona Lisa')."
  exit 1
fi

echo "SLACK_ID=$SLACK_ID"
echo "GITHUB_USERNAME=$GITHUB_USERNAME"
echo "EMAIL=$EMAIL"
echo "VALID=true"
