---
name: issue
description: "Use when creating a new GitHub issue from a free-form description. Optional --go flag posts a 'GO' comment on the new issue so it becomes immediately eligible for /fix-issue automation."
argument-hint: "[--go] <issue description>"
allowed-tools: Bash
---

# Issue Skill

Create a new GitHub issue in the current repository from a free-form description. Optionally appends a final comment containing only `GO`, which marks the issue as approved for automated processing by `/fix-issue`.

## Step 1 — Parse Arguments

Parse flags from the start of `$ARGUMENTS` before treating the remainder as the issue description. Stop at the first non-flag token.

- `--go`: Set `go_mode=true`. Default: `go_mode=false`. When `true`, a final comment with the exact body `GO` is posted on the new issue after creation.

After flag stripping, the remainder of `$ARGUMENTS` is the **issue description**. Save it as `DESCRIPTION`.

If `DESCRIPTION` is empty (only flags were provided), print `**ERROR: Usage: /issue [--go] <issue description>**` and abort.

## Step 2 — Derive Title and Body

The **title** is a concise one-line summary derived from `DESCRIPTION`:

- Take `DESCRIPTION` up to the first newline (or the whole string if single-line).
- Trim leading/trailing whitespace.
- If the result is longer than 80 characters, truncate at the last word boundary ≤ 80 chars and append `…`.
- If after trimming the title is empty, abort with `**ERROR: Could not derive a title from the description.**`

The **body** is the full original `DESCRIPTION` verbatim (including any multi-line content).

## Step 3 — Verify Repository Access

```bash
REPO=$(gh repo view --json nameWithOwner --jq '.nameWithOwner' 2>/dev/null)
```

If the command fails or `REPO` is empty, print `**ERROR: Could not determine the current repository. Ensure 'gh' is authenticated and you are inside a GitHub-backed repo.**` and abort.

## Step 4 — Create the Issue

Invoke `gh issue create` with the derived title and full body:

```bash
gh issue create --title "<title>" --body "<body>"
```

Pass `<body>` via a heredoc or a temp file if it contains characters that would be awkward to quote inline. Capture stdout — `gh` prints the new issue URL as the last line.

**On failure** (non-zero exit): print `**ERROR: Failed to create issue: <stderr excerpt>**` and abort.

**On success**: Parse the issue number from the URL (format `https://github.com/OWNER/REPO/issues/N`).

Save `ISSUE_URL` and `ISSUE_NUMBER`.

## Step 5 — Post GO Comment (conditional)

Run this step only when `go_mode=true`.

```bash
gh issue comment "$ISSUE_NUMBER" --body "GO"
```

- On success: proceed to Step 6 — the final summary will note the GO comment was posted.
- On failure: print `**⚠ Issue was created but GO comment failed: <stderr excerpt>. You can add 'GO' as a final comment manually to approve it for /fix-issue.**` — still proceed to Step 6 (issue exists and is useful on its own).

## Step 6 — Report

Print a final summary on a single line:

- If `go_mode=true` and the GO comment succeeded: `✅ Created issue #<ISSUE_NUMBER> with GO comment — <ISSUE_URL>`
- If `go_mode=true` and the GO comment failed: `**⚠ Created issue #<ISSUE_NUMBER> — <ISSUE_URL> (GO comment failed; add it manually to approve for /fix-issue)**`
- If `go_mode=false`: `✅ Created issue #<ISSUE_NUMBER> — <ISSUE_URL>`
