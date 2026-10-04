---

# larch-run-lifecycle: shared-v1 skill=file-bug
name: file-bug
description: "Use when filing, investigating, or root-causing a software bug. Reads the repo, drafts a detailed GitHub issue, and invokes /issue with dedup enabled."
argument-hint: "[--urgent] <bug description>"
allowed-tools: Bash, Read, Grep, Glob, Write, Skill
hooks:
  PreToolUse:
    - matcher: "Write"
      hooks:
        - type: command
          command: "${CLAUDE_PLUGIN_ROOT}/scripts/deny-edit-write.sh file-bug"
          timeout: 5
---

**MANDATORY: Follow the complete shared lifecycle contract in `${CLAUDE_PLUGIN_ROOT}/skills/shared/run-lifecycle.md` with declared skill `file-bug`.**

# File-Bug Skill

**MANDATORY: READ ENTIRE FILE before composing user-facing prose: `${CLAUDE_PLUGIN_ROOT}/skills/shared/readability-style.md`.**

Investigate a user-described bug inline, compose a detailed issue body, then delegate creation to `/issue` with dedup enabled. This skill is for issue filing only. It never edits the repository.

**Anti-halt continuation reminder.** After every child `Skill` tool call (e.g., `/issue`) returns, IMMEDIATELY continue with this skill's NEXT numbered step - do NOT end the turn on the child's cleanup output, and do NOT write a summary, handoff, status recap, or "returning to parent" message - those are halts in disguise. The rule is strictly subordinate to any explicit non-sequential control-flow directive in THIS file (e.g., `bail`, `skip to Step N`). A normal sequential `proceed to Step N+1` instruction is the default continuation this rule reinforces, NOT an exception. → shared/subskill-invocation.md#anti-halt

## Contract

- `--urgent` is the only flag.
- Remove one or more leading `--urgent` tokens from the description before validation.
- If any other leading `--...` token remains after removing `--urgent`, treat it as prose, not as an option.
- Use `[BUG]` as the default issue title prefix.
- Use `[BUG] (URGENT)` as the issue title prefix when `--urgent` was present.
- Never pass `--no-dedup` to `/issue`.
- Rely on `/issue` to assign the filed bug to the GitHub user authenticated in `gh`.
- Use only `Read`, `Grep`, `Glob`, and safe read-only `Bash` discovery for investigation.
- Use `Write` only for files under `$BUG_TMPDIR`.
- The `Write` hook is active only after Step 2 writes a fresh `bug-*` activation sentinel.
- Do not use `Edit`, `NotebookEdit`, external agents, or repo-writing Bash commands.
- If root cause is uncertain, say so in the issue body and list the evidence and next steps.
- Include exactly one `Origin:` line under `## Root cause analysis`. Use `regression #N`, `new-code`, or `spec-gap`.

<!-- step:1 - Validate input -->
## Step 1 - Validate input

Trim `$ARGUMENTS` mentally. Remove one or more leading `--urgent` tokens before validation. If at least one leading `--urgent` was present, remember the issue title prefix `[BUG] (URGENT)`. Otherwise remember the issue title prefix `[BUG]`.

If the remaining description is empty or whitespace-only, print:

```text
**⚠ /file-bug: bug description is required. Aborting.**
```

Stop before creating any temp directory.

**Security triage (mandatory).** After validating input, assess whether `$ARGUMENTS` describes a **security vulnerability** (exploitable weakness, credential exposure, auth bypass, injection, RCE, etc.) rather than ordinary functional breakage. If the report is security-sensitive, or if you are uncertain whether it is security-sensitive, **do not proceed**. Print:

```text
**⚠ /file-bug: this report appears to describe a security vulnerability. Do not file a public GitHub issue. Report it responsibly per SECURITY.md (email disclosure). Aborting before /issue.**
```

Stop before creating any temp directory. See `${CLAUDE_PLUGIN_ROOT}/SECURITY.md` § Reporting a Vulnerability.

<!-- step:2 - Create temp directory -->
## Step 2 - Create temp directory

Create a canonical `/tmp` scratch directory before writing any artifacts:

```bash
BUG_TMPDIR=$(mktemp -d "/tmp/claude-bug-XXXXXX")
printf 'BUG_TMPDIR=%s\n' "$BUG_TMPDIR"
```

Parse the Bash output for `BUG_TMPDIR=<path>` and bind that path for all later steps. Bash tool calls do not preserve shell variables across fences; retain the parsed value mentally (mirror `/research` Step 0 parsing of `SESSION_TMPDIR`).

Activate the `Write` hook after `$BUG_TMPDIR` exists and before the first `Write`:

```bash
if [[ -z "${XDG_CACHE_HOME:-}" && -z "${HOME:-}" ]]; then
  echo "**⚠ /file-bug: failed to activate Write hook. Aborting.**"
  rm -rf "$BUG_TMPDIR"
  exit 1
fi
BUG_DENY_ACTIVE_DIR="${XDG_CACHE_HOME:-${HOME}/.cache}/larch/deny-edit-write-active"
BUG_DENY_ACTIVE_SENTINEL="$BUG_DENY_ACTIVE_DIR/file-bug-$PPID"
if ! mkdir -p "$BUG_DENY_ACTIVE_DIR" || ! : > "$BUG_DENY_ACTIVE_SENTINEL"; then
  echo "**⚠ /file-bug: failed to activate Write hook. Aborting.**"
  rm -rf "$BUG_TMPDIR"
  exit 1
fi
printf 'BUG_DENY_ACTIVE_SENTINEL=%s\n' "$BUG_DENY_ACTIVE_SENTINEL"
```

All scratch files and the `/issue` sentinel file must stay under `$BUG_TMPDIR`. This placement keeps the active skill-scoped `Write` hook on the allowed side of its scratch-only policy (canonical `/tmp` or the larch cache sessions root).

<!-- step:3 - Investigate -->
## Step 3 - Investigate

Investigate the report inline. Prefer direct reads and targeted search:

- Use `Glob` to find likely files.
- Use `Grep` for error text, function names, commands, config keys, and related tests.
- Use `Read` to inspect the smallest set of files that can explain the behavior.
- Use safe read-only `Bash` discovery when needed, such as `git status --short`, `git branch --show-current`, `git rev-parse --short HEAD`, or test listing commands.

Do not edit the repo. Do not run mutating commands. If running a reproduction would be expensive, destructive, or dependent on unavailable services, describe the best reproduction scenario instead of forcing it.

If investigation reveals a security vulnerability rather than ordinary functional breakage, abort before Step 4 using the same security message and `SECURITY.md` guidance from Step 1. Remove `"$BUG_DENY_ACTIVE_SENTINEL"` and `$BUG_TMPDIR` if they exist, then stop.

<!-- step:4 - Compose issue body -->
## Step 4 - Compose issue body

**Sanitize before writing.** The body will be filed as a **public** GitHub issue. Before `Write`, apply compose-time redaction to every section (especially **Evidence** and **Original report**): secrets / API keys / OAuth / JWT / passwords / certificates → `<REDACTED-TOKEN>`; internal hostnames / URLs / private IPs → `<INTERNAL-URL>`; PII (emails, names, account IDs linked to a real user) → `<REDACTED-PII>`. Follow `${CLAUDE_PLUGIN_ROOT}/skills/implement/references/execution-issues-tracking.md` and `${CLAUDE_PLUGIN_ROOT}/docs/security/artifacts-redaction-and-publication.md`. `/issue`'s shell scrubber covers token-shaped secrets only; prompt-level sanitization is required and `/issue` redaction is defense-in-depth, not sufficient alone.

Use `Write` to create `$BUG_TMPDIR/bug-issue-body.md` with exactly these ten `##` headings, in this order:

```markdown
## Summary

<One-paragraph bug summary.>

## Original report

<The user's bug description, preserving important details. Neutralize literal larch HTML comment control markers before writing this section and any other user-controlled or quoted-untrusted section: for each `<!-- larch:` … `-->` substring, insert U+200B (zero-width space) immediately after `<!--` so downstream tooling cannot parse it as a real marker (see `docs/issue-anchored-plan.md`).>

## Reproduction scenario

<Concrete steps, command, or scenario. Say what could not be reproduced if applicable.>

## Expected behavior

<What should happen.>

## Observed behavior

<What appears to happen instead.>

## Root cause analysis

Origin: <Choose exactly one: regression #N, new-code, or spec-gap.>

<Likely root cause. If uncertain, state uncertainty explicitly and explain why. When the introducing change is known, name it with one canonical origin phrase: "introduced by #N", "introduced by PR #N", "introduced in #N", "incomplete fix of #N", "persists after #N", or "residual of #N".>

## Evidence

<Bulleted evidence from files, commands, logs, tests, or code paths.>

## Affected files

<Repo-relative paths and why each matters.>

## Suggested fix(es)

<Specific fix ideas, or "No concrete fix identified yet" with next investigation steps.>

## Open questions

<Questions for /design or implementers. Use "None identified" only when true.>
```

The body should give `/design` enough context to produce a good implementation plan. Do not invent certainty. Separate observations from inferences. Apply the larch-marker neutralization rule from `## Original report` to every user-controlled or quoted-untrusted section before `Write`.

<!-- step:5 - Invoke issue -->
## Step 5 - Invoke issue

**Security re-check (mandatory, fail-closed).** Immediately before any `/issue` Skill-tool call, re-assess whether the bug report or investigation results describe a **security vulnerability** (exploitable weakness, credential exposure, auth bypass, injection, RCE, etc.) rather than ordinary functional breakage. If the report is security-sensitive, or if you are uncertain whether it is security-sensitive, **do not call** `/issue`. Print:

```text
**⚠ /file-bug: this report appears to describe a security vulnerability. Do not file a public GitHub issue. Report it responsibly per SECURITY.md (email disclosure). Aborting before /issue.**
```

Remove `"$BUG_DENY_ACTIVE_SENTINEL"` and `$BUG_TMPDIR`, then stop. Do not run Steps 6 or 7. See `${CLAUDE_PLUGIN_ROOT}/SECURITY.md` § Reporting a Vulnerability.

Derive a concise, descriptive issue title from the original bug report, not from `$BUG_TMPDIR/bug-issue-body.md`. If the derived title is empty or whitespace-only after trimming, use `Bug report`. If the title starts with `-`, prefix `Bug:` followed by a space so `/issue` does not parse it as a flag.

Invoke `/issue` via the Skill tool:

- Default: `/issue --title-prefix "[BUG]" --body-file "$BUG_TMPDIR/bug-issue-body.md" --sentinel-file "$BUG_TMPDIR/issue-completed.sentinel" "<descriptive-title>"`
- Urgent: `/issue --title-prefix "[BUG] (URGENT)" --body-file "$BUG_TMPDIR/bug-issue-body.md" --sentinel-file "$BUG_TMPDIR/issue-completed.sentinel" "<descriptive-title>"`

Pass exactly one `--title-prefix` value. Do not reimplement prefix de-duplication; `/issue` owns that behavior.

Do not include `--no-dedup`.

> **Continue after child returns.** When the child Skill returns, execute Step 6 - do NOT end the turn, and do NOT write a summary, handoff, or "returning to parent" message. → shared/subskill-invocation.md#anti-halt

<!-- step:6 - Verify issue outcome -->
## Step 6 - Verify issue outcome

Parse `/issue` stdout as machine text. Do not use `eval` or `source`. Extract:

- `ISSUES_CREATED=<N>`
- `ISSUES_FAILED=<N>`
- `ISSUES_DEDUPLICATED=<N>`
- `ISSUE_1_URL=<url>` for a created issue
- `ISSUE_1_DUPLICATE_OF_URL=<url>` for a deduplicated issue

Then verify the sentinel file:

```bash
"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" verify skill-called --sentinel-file "$BUG_TMPDIR/issue-completed.sentinel"
```

Parse `VERIFIED` from stdout.

Success requires both of these conditions:

- `ISSUES_FAILED=0`
- `VERIFIED=true`

A created issue and a deduplicated issue are both successful outcomes. Prefer `ISSUE_1_URL` for the final report. Fall back to `ISSUE_1_DUPLICATE_OF_URL`.

If `/issue` fails, if `ISSUES_FAILED` is nonzero, if `VERIFIED` is not `true`, or if neither `ISSUE_1_URL` nor `ISSUE_1_DUPLICATE_OF_URL` is present: remove `"$BUG_DENY_ACTIVE_SENTINEL"`, surface the failure and parsed counters when available, stop without claiming that an issue was filed, and **do not run Step 7**. Leave `$BUG_TMPDIR` in place for debugging.

> **Continue to Step 7 IMMEDIATELY** only when Step 6 bound `ISSUE_1_URL` or `ISSUE_1_DUPLICATE_OF_URL` and both success conditions hold. → shared/subskill-invocation.md#step-boundary

<!-- step:7 - Cleanup and report -->
## Step 7 - Cleanup and report

**Entry guard.** Run this step only when Step 6 bound `ISSUE_1_URL` or `ISSUE_1_DUPLICATE_OF_URL` and verification succeeded. Otherwise skip Step 7 entirely.

Remove the scratch directory:

```bash
rm -f "$BUG_DENY_ACTIVE_SENTINEL"
rm -rf "$BUG_TMPDIR"
```

Report the issue URL selected in Step 6. If the issue was deduplicated, say that the existing issue was reused.
