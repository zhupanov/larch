---
name: admin-upgrade-clients
description: Upgrade client repos (which have dev-tools as a submodule) to the latest version of dev-tools on main. Reads client list from data/client-repos.json, clones each repo, runs upgrade-dev-tools.sh, optionally bumps version, creates PR, posts to Slack, monitors CI, merges, and marks Slack post with :merged:. Use when rolling out latest dev-tools changes to all dependent repos.
allowed-tools: Bash, Read, Edit, Write, Grep, Glob, Agent, Task, Skill
---

# Admin Upgrade Clients Skill

Upgrade all client repos to the latest version of dev-tools (on main). For each client repo: clone, upgrade dev-tools submodule, optionally bump version, create PR, post to Slack, monitor CI, merge, and clean up.

All client repos are processed **in parallel** via background bash scripts.

## Progress Reporting

**Every step MUST print clearly visible status lines.** Use distinct emoji prefixes:

| Step | Emoji | Description |
|------|-------|-------------|
| 0 | 🔧 | Session setup |
| 1a | 🚀 | Phase 1: Upgrade + prepare (parallel bash) |
| 1b | 📋 | Read Phase 1 results |
| 1c | 🏷️ | Version bump (LLM, sequential) |
| 1d | 🔄 | Phase 2: Publish + CI + merge (parallel bash) |
| 1e | 📋 | Read Phase 2 results |
| 2 | 📊 | Final report |

## Step 0 — Session Setup

Run the session setup script which handles temp directory creation, SLACK_BOT_TOKEN check, client repo list reading, and dev-tools SHA resolution:

```bash
$PWD/.claude/skills/admin-upgrade-clients/scripts/session-setup.sh
```

If the script exits non-zero, print the `ERROR` from its output and abort.

Parse the output for `UC_TMPDIR`, `SLACK_TOKEN_OK`, `CLIENT_REPOS`, `CLIENT_COUNT`, `TARGET_SHA`, `TARGET_SHA_SHORT`.

If `SLACK_TOKEN_OK=false`, print: `**⚠ SLACK_BOT_TOKEN is not set. Slack steps will be skipped.**` Set `slack_available=false`.

Print: `🔧 Step 0 — Setup complete. Upgrading $CLIENT_COUNT clients to dev-tools $TARGET_SHA_SHORT`

## Step 1a — Phase 1: Upgrade + Prepare (parallel)

Launch `upgrade-single-client.sh` for **all repos simultaneously** using the Bash tool with `run_in_background: true`. Each invocation runs independently in its own subprocess.

For each repo in `CLIENT_REPOS`:

```bash
$PWD/.claude/skills/admin-upgrade-clients/scripts/upgrade-single-client.sh \
  --repo <REPO> --target-sha $TARGET_SHA --target-sha-short $TARGET_SHA_SHORT --tmpdir $UC_TMPDIR
```

**All Bash calls MUST be in a single message** to ensure true parallelism.

## Step 1b — Read Phase 1 Results

Wait for all Phase 1 `.done` sentinel files. For each repo, check for `$UC_TMPDIR/<repo-name>-phase1.txt.done`. Poll by checking file existence (use `test -f` in a loop with short sleeps, or invoke `wait-for-reviewers.sh` with the sentinel paths).

Read each result file at `$UC_TMPDIR/<repo-name>-phase1.txt`. Parse for `STATUS`, `CLONE_DIR`, `HAS_BUMP`, `BUMP_VERSION_MISSING`, and `ERROR`.

Print a brief status for each repo:
- `STATUS=ready`: `✅ <repo> — ready for PR (HAS_BUMP=<value>)`
- `STATUS=skipped`: `⏭️ <repo> — already up-to-date`
- `STATUS=failed`: `❌ <repo> — failed: <ERROR>`

## Step 1c — Version Bump (LLM, sequential)

For each repo where `STATUS=ready` AND `HAS_BUMP=true`:

1. Read `CLONE_DIR` from the Phase 1 result file
2. Read the `/bump-version` skill file: `$CLONE_DIR/.claude/skills/bump-version/SKILL.md`
3. Follow the instructions in that skill file, executing all commands from `$CLONE_DIR`
4. After the bump, push the new commit: run `cd "$CLONE_DIR" && git push -u origin HEAD` via Bash tool
5. Verify the commit was created: run `cd "$CLONE_DIR" && ./dev-tools/.claude/scripts/generic/check-bump-version.sh --mode post --before-count <COMMITS_BEFORE>` where `COMMITS_BEFORE` is read from Phase 1's `check-bump-version.sh --mode pre` output (saved in the result file or re-derived)

**Important**: Handle version bumps sequentially (one repo at a time). This is the only LLM-driven step — all other work is done by bash scripts.

If a repo has `BUMP_VERSION_MISSING=true`, skip it silently.

## Step 1d — Phase 2: Publish + CI + Merge (parallel)

Launch `publish-and-merge-client.sh` **only for repos with `STATUS=ready`** from Phase 1. Skip repos with `STATUS=skipped` or `STATUS=failed`.

For each eligible repo:

```bash
$PWD/.claude/skills/admin-upgrade-clients/scripts/publish-and-merge-client.sh \
  --repo <REPO> --target-sha-short $TARGET_SHA_SHORT --tmpdir $UC_TMPDIR \
  --slack-available <true|false>
```

Launch all Bash calls in a single message with `run_in_background: true`.

## Step 1e — Read Phase 2 Results

Wait for Phase 2 `.done` sentinel files **only for repos actually launched in Step 1d** (those with `STATUS=ready`). Do not wait for repos that were skipped or failed in Phase 1 — they have no Phase 2 scripts running. Check `$UC_TMPDIR/<repo-name>-result.txt.done` for each launched repo.

Read each result file at `$UC_TMPDIR/<repo-name>-result.txt`. Parse for `STATUS`, `PR_NUMBER`, `PR_URL`, `SLACK_TS`, `BUMP_VERSION_MISSING`, `PR_STATUS`, and `ERROR`.

## Step 2 — Final Report

Print a summary table combining Phase 1 (skipped/failed) and Phase 2 (merged/bailed) results:

```
## Upgrade Results

| Repo | Status | PR | Slack |
|------|--------|----|-------|
| chat-stack | ✅ Merged | #123 | ✅ Posted + :merged: |
| lmserve | ❌ Bailed | #456 | ⚠ Posted (no :merged:) |
| other-repo | ⏭️ Skipped | — | — |
```

### Missing /bump-version Skill

If any repos had `BUMP_VERSION_MISSING=true`, print:

```
## Repos Missing /bump-version Skill

The following repos were upgraded without a version bump because they lack
the `/bump-version` skill:

- myorg/service-a
- myorg/service-b

To create the `/bump-version` skill in a client repo, open a Claude session
in that repo and give it this prompt:

    Create a `/bump-version` skill at `.claude/skills/bump-version/SKILL.md`.
    This skill should:
    1. Determine the current version from the repo's version file
    2. Classify the bump type (major/minor/patch) based on the git diff
    3. Compute the new version
    4. Edit the version file with the new version
    5. Commit with message "Bump version to <NEW_VERSION>"
    The skill should be self-contained and take no arguments. Register it in
    `.claude/settings.json` under `permissions.allow`.
```

### Cleanup

Remove the session temp directory:

```bash
$PWD/.claude/scripts/generic/cleanup-tmpdir.sh --dir "$UC_TMPDIR"
```

Print: `🏁 Upgrade complete!`
