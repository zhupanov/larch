#!/usr/bin/env bash
# edit-json-files.sh — Add user entries to identity map JSON files.
#
# Usage:
#   edit-json-files.sh --email <EMAIL> --slack-id <ID> --github-username <HANDLE> \
#                      --tmpdir <DIR> [--skip-email-map] [--skip-github-map]
#
# Output (stdout, KEY=VALUE):
#   FILE_MODIFIED=<path>   (one line per modified file)
#
# Exit codes: 0 success, 1 jq error

set -euo pipefail

EMAIL=""
SLACK_ID=""
GITHUB_USERNAME=""
TMPDIR_ARG=""
SKIP_EMAIL_MAP=false
SKIP_GITHUB_MAP=false

while [[ $# -gt 0 ]]; do
  case "$1" in
    --email)             EMAIL="$2"; shift 2 ;;
    --slack-id)          SLACK_ID="$2"; shift 2 ;;
    --github-username)   GITHUB_USERNAME="$2"; shift 2 ;;
    --tmpdir)            TMPDIR_ARG="$2"; shift 2 ;;
    --skip-email-map)    SKIP_EMAIL_MAP=true; shift ;;
    --skip-github-map)   SKIP_GITHUB_MAP=true; shift ;;
    *) echo "ERROR=Unknown argument: $1" >&2; exit 1 ;;
  esac
done

if [[ -z "$EMAIL" || -z "$SLACK_ID" || -z "$GITHUB_USERNAME" || -z "$TMPDIR_ARG" ]]; then
  echo "ERROR=Missing required arguments" >&2
  exit 1
fi

MODIFIED=()

if [[ "$SKIP_EMAIL_MAP" != "true" ]]; then
  jq --sort-keys --arg email "$EMAIL" --arg sid "$SLACK_ID" \
    '. + {($email): $sid}' data/email_to_slack_userid_map.json \
    > "$TMPDIR_ARG/email_map.json" && \
    mv "$TMPDIR_ARG/email_map.json" data/email_to_slack_userid_map.json
  MODIFIED+=("data/email_to_slack_userid_map.json")
fi

if [[ "$SKIP_GITHUB_MAP" != "true" ]]; then
  jq --sort-keys --arg gh "$GITHUB_USERNAME" --arg email "$EMAIL" \
    '. + {($gh): $email}' data/github_username_to_email_map.json \
    > "$TMPDIR_ARG/github_map.json" && \
    mv "$TMPDIR_ARG/github_map.json" data/github_username_to_email_map.json
  MODIFIED+=("data/github_username_to_email_map.json")
fi

for f in "${MODIFIED[@]}"; do
  echo "FILE_MODIFIED=$f"
done
