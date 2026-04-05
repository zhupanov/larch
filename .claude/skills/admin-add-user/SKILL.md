---
name: admin-add-user
description: Add a new user to dev-tools identity maps (email-to-Slack and GitHub-to-email). Takes --slack-id, --github-username, and --email, edits the JSON data files, creates a PR, posts to Slack, monitors CI, merges, and cleans up. No design or code review.
argument-hint: "--slack-id <SLACK_ID> --github-username <GITHUB_USERNAME> --email <EMAIL>"
allowed-tools: Bash, Read, Edit, Write, Grep, Glob, Skill
---

# Admin Add User Skill

Add a new user to the dev-tools identity mapping files. Edits `data/email_to_slack_userid_map.json` and `data/github_username_to_email_map.json`, then follows the post-change shazam flow (branch, commit, PR, Slack, CI+merge, cleanup) with no design or code review.

`$ARGUMENTS` contains the three required arguments: `--slack-id`, `--github-username`, and `--email`.

## Progress Reporting

**Every step MUST print clearly visible status lines.** Use distinct emoji prefixes:

| Step | Emoji | Description |
|------|-------|-------------|
| 0 | 🔧 | Parse and validate arguments |
| 1 | 🔧 | Session setup |
| 2 | 🔍 | Duplicate check |
| 3 | 🔀 | Create branch |
| 4 | 🛠️ | Edit JSON files |
| 5 | 🧪 | Validate |
| 6 | 💾 | Commit |
| 7 | 🏷️ | Version bump |
| 8 | 🚀 | Create PR |
| 9 | 📋 | Post to Slack |
| 10 | 🔄 | CI + rebase + merge |
| 11 | ✨ | :merged: emoji |
| 12 | 🧹 | Local cleanup |
| 13 | ✅ | Verify main |

## Step 0 — Parse and Validate Arguments

Parse and validate `$ARGUMENTS` using the dedicated script:

```bash
$PWD/.claude/skills/admin-add-user/scripts/parse-args.sh $ARGUMENTS
```

Parse the output for `VALID`, `SLACK_ID`, `GITHUB_USERNAME`, `EMAIL`, and `ERROR`. If `VALID=false`, print `**❌ $ERROR**` and abort.

Print: `🔧 Step 0 — Arguments validated: email=<EMAIL>, slack_id=<SLACK_ID>, github=<GITHUB_USERNAME>`

## Step 1 — Session Setup

Run the shared session setup script:

```bash
$PWD/.claude/scripts/generic/session-setup.sh --prefix claude-admin-add-user --skip-branch-check
```

Parse the output for `SESSION_TMPDIR`, `SLACK_TOKEN_OK`, `REPO`, `REPO_UNAVAILABLE`. Set `AAU_TMPDIR` = `SESSION_TMPDIR`.

If `SLACK_TOKEN_OK=false`, print: `**⚠ SLACK_BOT_TOKEN is not set. Slack steps will be skipped.**` Set `slack_available=false`.
If `REPO_UNAVAILABLE=true`, print `**❌ Could not determine repository name.**` and set `repo_unavailable=true`.

Read the Slack channel:

```bash
$PWD/.claude/scripts/generic/read-slack-channel.sh
```

Parse the output for `SLACK_CHANNEL`. If empty, print `**⚠ slackChannel not configured. Slack steps will be skipped.**` and set `slack_available=false`.

Print: `🔧 Step 1 — Session setup complete.`

## Step 2 — Check for Duplicates

```bash
$PWD/.claude/skills/admin-add-user/scripts/check-duplicates.sh \
  --email "$EMAIL" --slack-id "$SLACK_ID" --github-username "$GITHUB_USERNAME"
```

Parse the output for `SKIP_EMAIL_MAP`, `SKIP_GITHUB_MAP`, `ALL_DUPLICATE`, and `ERROR`.

- If exit code is 1 (conflict): Print `**❌ $ERROR**` and abort.
- If `ALL_DUPLICATE=true`: Print `**⚠ User already fully mapped. Nothing to do.**` and abort gracefully (not an error).
- If `SKIP_EMAIL_MAP=true`: Print `**⚠ Email <EMAIL> already mapped. Skipping email map.**`
- If `SKIP_GITHUB_MAP=true`: Print `**⚠ GitHub username <GITHUB_USERNAME> already mapped. Skipping GitHub map.**`

Print: `🔍 Step 2 — Duplicate check passed.`

## Step 3 — Create Branch

```bash
$PWD/.claude/scripts/generic/create-branch.sh --check
```

Parse `USER_PREFIX` from the output. Then create the branch:

```bash
$PWD/.claude/scripts/generic/create-branch.sh --branch "$USER_PREFIX/add-user-$GITHUB_USERNAME"
```

Print: `🔀 Step 3 — Branch created: <branch-name>`

## Step 4 — Edit JSON Files

```bash
$PWD/.claude/skills/admin-add-user/scripts/edit-json-files.sh \
  --email "$EMAIL" --slack-id "$SLACK_ID" --github-username "$GITHUB_USERNAME" \
  --tmpdir "$AAU_TMPDIR" [--skip-email-map] [--skip-github-map]
```

Include `--skip-email-map` and/or `--skip-github-map` flags based on the values from Step 2.

Parse the output for `FILE_MODIFIED` lines (one per modified file). Use these paths in Step 6 for staging.

Print: `🛠️ Step 4 — JSON files updated.`

## Step 5 — Validate

Run the existing data integrity tests:

```bash
$PWD/.claude/skills/admin-add-user/scripts/run-user-tests.sh
```

If tests fail (exit 1), print `**❌ Data validation failed. Aborting.**`, show the failure output, then clean up using the abort script:

```bash
$PWD/.claude/skills/admin-add-user/scripts/abort-and-cleanup.sh --branch <branch-name> --tmpdir "$AAU_TMPDIR"
```

Then abort.

Print: `🧪 Step 5 — Validation passed.`

## Step 6 — Commit

Stage only the files that were actually modified and commit. Build the file list conditionally based on `skip_email_map` and `skip_github_map` flags — only include files that were actually modified:

```bash
$PWD/.claude/scripts/generic/git-commit.sh -m "Add user $GITHUB_USERNAME to identity maps" <modified-files>
```

Where `<modified-files>` includes `data/email_to_slack_userid_map.json` (unless `skip_email_map`) and `data/github_username_to_email_map.json` (unless `skip_github_map`).

Print: `💾 Step 6 — Changes committed.`

## Step 7 — Version Bump

Check if the repo has a `/bump-version` skill and capture commit count:

```bash
$PWD/.claude/scripts/generic/check-bump-version.sh --mode pre
```

Parse the output for `HAS_BUMP` and `COMMITS_BEFORE`.

**If `HAS_BUMP=false`**: Print `**⚠ VERSION BUMP SKIPPED: No /bump-version skill found.**` and skip to Step 8.

**If `HAS_BUMP=true`**:
1. Invoke `/bump-version` via the Skill tool.
2. Verify:
   ```bash
   $PWD/.claude/scripts/generic/check-bump-version.sh --mode post --before-count $COMMITS_BEFORE
   ```
   Parse for `VERIFIED`. If `VERIFIED=false`, print: `**⚠ /bump-version did not create a commit.**`

Print: `🏷️ Step 7 — Version bump complete.`

## Step 8 — Create PR

Write the PR body to `$AAU_TMPDIR/pr-body.md`:

```markdown
## Summary
- Add user <GITHUB_USERNAME> (<EMAIL>) to dev-tools identity maps

## Test plan
- [x] Ran `pytest python/tests/test_user_data.py` — all validations pass

Generated with [Claude Code](https://claude.com/claude-code)
```

Create the PR:

```bash
$PWD/.claude/scripts/generic/create-pr.sh --title "Add user $GITHUB_USERNAME to identity maps" --body-file "$AAU_TMPDIR/pr-body.md"
```

Parse `PR_NUMBER` and `PR_URL` from the output.

Print: `🚀 Step 8 — PR created: <PR_URL>`

Then print machine-parseable output:
```
PR_NUMBER=<N>
PR_URL=<url>
```

## Step 9 — Post to Slack

**If `slack_available=false`**: Print `⏭️ Step 9 — Skipped (Slack not available).` and proceed to Step 10.

Post to Slack using the shared script:

```bash
$PWD/.claude/scripts/generic/post-pr-announce.sh --pr $PR_NUMBER
```

Parse the output for `SLACK_TS=<value>`.

If the script exits non-zero or `SLACK_TS` is empty, print `**⚠ Slack announcement failed. Continuing.**` and set `SLACK_TS` to empty.

## Step 10 — CI + Rebase + Merge Loop

**If `repo_unavailable=true`**: Print `⏭️ Step 10 — Skipped (repository name unavailable).` and skip to Step 12.

Monitor CI and merge using the consolidated `ci-wait.sh` script. No code fixes for data-only changes — if CI fails for a non-transient reason, bail.

Track these counters (all start at 0):
- `iteration` — incremented by the caller after each successful rebase and each evaluate_failure/transient-retry, passed to `ci-wait.sh` via `--iteration`
- `rebase_count` — incremented after each successful rebase
- `transient_retries` — incremented after each transient rerun, reset after successful rebase or a non-transient failure; when exceeds 2, treat as real failure

**Wait for CI** using `ci-wait.sh`:

```bash
$PWD/.claude/scripts/generic/ci-wait.sh --pr $PR_NUMBER --repo $REPO \
  --rebase-count "$rebase_count" --fix-attempts 0 --iteration "$iteration"
```

Use `timeout: 1860000` on the Bash tool call (31 minutes, matching the script's 1800s default + grace).

Parse the output for: `ACTION`, `CI_STATUS`, `BEHIND_COUNT`, `FAILED_RUN_ID`, `BAIL_REASON`, `ITERATION`, `ELAPSED`. Update `iteration` from the returned `ITERATION` value.

**Execute the action** returned by `ci-wait.sh`:

   - **`rebase`**: Run `$PWD/.claude/scripts/generic/rebase-push.sh`. Handle exit codes:
     - Exit 0: Success. Increment `rebase_count`, increment `iteration`, reset `transient_retries`. Sleep 30s, then re-invoke `ci-wait.sh`.
     - Exit 1 (conflicts): Bail — `$PWD/.claude/scripts/generic/git-rebase-abort.sh`. Data-only changes should not conflict.
     - Exit 2 (push failed): Retry once. If second failure, bail.
     - Exit 3 (rebase error): Bail.

   - **`rebase_then_evaluate`**: Rebase first (same exit-code handling as `rebase` above). On exit 0: increment `rebase_count`, increment `iteration`, reset `transient_retries`. Sleep 30s, then re-invoke `ci-wait.sh` to wait for fresh CI on the rebased HEAD (do NOT evaluate the stale `FAILED_RUN_ID` from the pre-rebase run).

   - **`merge`**: Use the shared merge script: `$PWD/.claude/scripts/generic/merge-pr.sh --pr $PR_NUMBER --repo $REPO`. Parse the output for `MERGE_RESULT` and `ERROR`. Handle: `merged` → success; `admin_merged` → success with warning; `main_advanced` → increment `iteration`, re-invoke `ci-wait.sh`; `ci_not_ready` → increment `iteration`, re-invoke `ci-wait.sh`; `admin_failed` or `error` → bail. **CRITICAL: The `--admin` safety invariant is enforced inside `merge-pr.sh`. (Keep in sync with `/shazam` Step 2b and `/admin-upgrade-clients` Sub-Step 7.)**

   - **`already_merged`**: Print `✅ PR already merged externally.` Proceed to Step 11.

   - **`evaluate_failure`**: Use `FAILED_RUN_ID` from the output. If `FAILED_RUN_ID` is empty, resolve the failed run from `$PWD/.claude/scripts/generic/gh-pr-checks.sh --pr $PR_NUMBER --repo $REPO`. If no valid run ID can be found after the fallback, treat as a non-rerunnable failure and bail. Check if transient:
     - Transient AND `transient_retries < 2`: Run `$PWD/.claude/scripts/generic/sleep-seconds.sh 60`, then `$PWD/.claude/scripts/generic/ci-rerun-failed.sh --run-id $FAILED_RUN_ID --repo $REPO`, increment `transient_retries`, increment `iteration`. Re-invoke `ci-wait.sh`.
     - If transient but `transient_retries >= 2`: Treat as real failure (below).
     - Otherwise (real failure): Bail. Print `❌ CI failed. PR #$PR_NUMBER left open for manual triage.`

   - **`bail`**: Print `BAIL_REASON`. PR stays open.

Print on success: `✅ Step 10 — PR #<NUMBER> merged!`

Save expected commit title for Step 13.

## Step 11 — Add :merged: Emoji

**Only if** the PR was merged **AND** `SLACK_TS` is non-empty **AND** `slack_available=true`.

Add the :merged: emoji using the shared script:

```bash
$PWD/.claude/scripts/generic/post-merged-emoji.sh --slack-ts "$SLACK_TS"
```

If it fails (exit non-zero), print `**⚠ Failed to add :merged: emoji. Continuing.**`

## Step 12 — Local Cleanup

**If the PR was merged:**

```bash
$PWD/.claude/scripts/generic/local-cleanup.sh --branch <branch-name>
```

Parse the output for `CLEANUP_SUCCESS`, `CURRENT_BRANCH`, and `BRANCH_DELETED`. If `CLEANUP_SUCCESS=true`, print: `🧹 Step 12 — Switched to main, deleted local branch.` If `CLEANUP_SUCCESS=false`, print: `**⚠ Step 12 — Cleanup partially failed. Current branch: <CURRENT_BRANCH>, branch deleted: <BRANCH_DELETED>.**`

**If bailed (PR not merged):**

Print: `⚠️ Step 12 — Skipped cleanup (PR not merged). You are still on branch <branch-name>.`

## Step 13 — Verify Main and Cleanup

**Verify (only if the PR was merged):**

```bash
$PWD/.claude/scripts/generic/verify-main.sh --expected-title "Add user $GITHUB_USERNAME to identity maps (#$PR_NUMBER)"
```

Parse the output for `VERIFIED`, `COMMIT_HASH`, and `COMMIT_MESSAGE`.

- If `VERIFIED=true`: `✅ Step 13 — Verified: main is at <COMMIT_HASH> "<COMMIT_MESSAGE>"`
- If `VERIFIED=false`: `**⚠ Step 13 — Unexpected HEAD on main.**`

**Always clean up temp directory** (regardless of merge/bail outcome):

```bash
$PWD/.claude/scripts/generic/cleanup-tmpdir.sh --dir "$AAU_TMPDIR"
```

Print: `🏁 Done! User <GITHUB_USERNAME> added to identity maps.`
