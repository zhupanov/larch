---

# larch-run-lifecycle: shared-v1 skill=block-issue
name: block-issue
description: "Use when expressing a native GitHub blocked-by relationship between two issues. Takes the blocked issue number and the blocking issue number as arguments."
argument-hint: "<ISSUE_A> <ISSUE_B> [--repo owner/name] --operator-invoked [--triage-controlled --expected-updated-at TIMESTAMP]"
allowed-tools: Bash
---

**MANDATORY: Follow the complete shared lifecycle contract in `${CLAUDE_PLUGIN_ROOT}/skills/shared/run-lifecycle.md` with declared skill `block-issue`.**

# block-issue

**MANDATORY: READ ENTIRE FILE before composing user-facing prose: `${CLAUDE_PLUGIN_ROOT}/skills/shared/readability-style.md`.**

Express a native GitHub blocking relationship: issue ISSUE_A is blocked by issue ISSUE_B.

## Arguments

Positional: `ISSUE_A ISSUE_B` — plain issue numbers (≥1). Optional: `--repo owner/name` (auto-detected from `gh repo view` when omitted). Live mutations require `--operator-invoked`. Optional: `--run-id <ID>`; shared flag details are in `${CLAUDE_PLUGIN_ROOT}/skills/shared/run-id-flag.md`. Example: `/block-issue 1842 1827 --operator-invoked` marks #1842 as blocked by #1827.

`/triage` callers also pass `--triage-controlled --expected-updated-at <timestamp>`. This mode re-reads the target immediately before mutation, requires exact freshness, rejects security-sensitive or protected lifecycle state (including a title-only stale lifecycle prefix), and performs an exact relationship and fresh-timestamp read-back. It does not weaken ordinary argument validation.

<!-- step:1 — Add blocked-by relationship -->

Strip `--run-id <ID>` from `$ARGUMENTS` before invoking the script (the script does not accept this flag). Command contract: `crates/larch-cli/src/issue_dependency_commands.rs`.

```bash
"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" block-issue add-blocked-by $ARGUMENTS
```

Parse `SUCCESS`, `RELATION_VERIFIED`, optional `UPDATED_AT`, and the confirmation line from stdout without `eval`/`source`. Verify the relationship was established before reporting:

- **`SUCCESS=true` and `RELATION_VERIFIED=true`**: Print the confirmation line (e.g., `✓ #1842 is now blocked by #1827`). A triage-controlled call also requires a non-empty fresh `UPDATED_AT` and returns it unchanged to the parent.
- Non-zero exit: Surface the `ERROR=` message from stderr and stop.
