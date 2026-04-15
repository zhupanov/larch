---
name: alias
description: "Use when creating shortcut aliases for existing larch skills with preset flags. Generates a project-level skill in .claude/skills/ that forwards to the target skill."
argument-hint: "[--merge] <alias-name> <target-skill> [preset-flags...]"
allowed-tools: Bash, Skill
---

# Alias Skill

Create a project-level alias skill in `.claude/skills/` that forwards to an existing larch skill with preset flags. Delegates to `/implement --quick --auto` for the full pipeline (implementation, code review, version bump, PR).

Example: `/alias i implement --merge` creates `.claude/skills/i/SKILL.md` so that `/i <feature>` is equivalent to `/implement --merge <feature>`.

Example with merge: `/alias --merge i implement --merge` creates the same alias AND merges the PR after CI passes.

## Step 1 — Parse Arguments

Parse flags from the start of `$ARGUMENTS` before treating the remainder as positional arguments. Flags may appear in any order; stop at the first non-flag token.

- `--merge`: Set `alias_merge=true`. Default: `alias_merge=false`. When true, `--merge` is forwarded to the `/implement` invocation so the resulting PR is also merged.

After flag stripping, parse the remaining positional arguments:
- First token = **alias name**
- Second token = **target skill name** (without `/` prefix)
- Remainder = **preset flags** (may be empty — a pure rename shortcut is valid)

If fewer than 2 positional tokens are provided, print: `**ERROR: Usage: /alias [--merge] <alias-name> <target-skill> [preset-flags...]**` and abort.

## Step 2 — Validate

All validation uses Bash since `${CLAUDE_PLUGIN_ROOT}` is a shell variable not resolvable in Read/Glob.

1. **Alias name format**: Verify alias name matches `^[a-z][a-z0-9-]*$` (lowercase, alphanumeric + hyphens, must start with a letter).
   - If invalid, print: `**ERROR: Alias name '<name>' is invalid. Must start with a lowercase letter and contain only lowercase letters, digits, and hyphens.**` and abort.

2. **Reserved name check**: Reject if alias name matches any of: `design`, `implement`, `review`, `research`, `loop-review`, `alias`, `relevant-checks`, `bump-version`, `fix-issue`.
   - If reserved, print: `**ERROR: Cannot create alias '<name>' — this name is reserved (it matches an existing larch or common project-level skill). Choose a different name.**` and abort.

3. **Target name format**: Verify target skill name matches `^[a-z][a-z0-9-]*$` (same format as alias names).
   - If invalid, print: `**ERROR: Target name '<target>' is invalid. Must contain only lowercase letters, digits, and hyphens.**` and abort.

4. **Target skill exists**: Verify target skill exists:
   ```bash
   test -f "${CLAUDE_PLUGIN_ROOT}/skills/<target>/SKILL.md"
   ```
   - If not found, print: `**ERROR: Target skill '<target>' does not exist.**` Then list valid targets:
     ```bash
     ls "${CLAUDE_PLUGIN_ROOT}/skills/"
     ```
     and abort.

5. **Target is not "alias"**: Forbid alias-to-alias recursion.
   - If target is "alias", print: `**ERROR: Cannot create an alias that targets /alias (no alias-to-alias recursion).**` and abort.

6. **Collision check**: Verify `.claude/skills/<alias-name>/` does not already exist in the current project:
   ```bash
   test -d ".claude/skills/<alias-name>"
   ```
   - If it exists, print: `**ERROR: '.claude/skills/<alias-name>/' already exists. Remove it first or choose a different name.**` and abort.

## Step 3 — Delegate to /implement

Construct a feature description for `/implement`. The description must contain all the information the implementing agent needs to create the alias:

If `<preset-flags>` is non-empty:
```
Create alias skill /<alias-name> for /<target-skill> with preset flags: <preset-flags>. Run: mkdir -p .claude/skills/<alias-name> && ${CLAUDE_PLUGIN_ROOT}/skills/alias/scripts/generate-alias.sh --name <alias-name> --target <target-skill> --flags "<preset-flags>" --version "$(grep '"version"' ${CLAUDE_PLUGIN_ROOT}/.claude-plugin/plugin.json | sed 's/.*"version": *"\([^"]*\)".*/\1/')" > .claude/skills/<alias-name>/SKILL.md
```

If `<preset-flags>` is empty:
```
Create alias skill /<alias-name> for /<target-skill> (pure rename, no preset flags). Run: mkdir -p .claude/skills/<alias-name> && ${CLAUDE_PLUGIN_ROOT}/skills/alias/scripts/generate-alias.sh --name <alias-name> --target <target-skill> --flags "" --version "$(grep '"version"' ${CLAUDE_PLUGIN_ROOT}/.claude-plugin/plugin.json | sed 's/.*"version": *"\([^"]*\)".*/\1/')" > .claude/skills/<alias-name>/SKILL.md
```

Invoke the Skill tool:
- Try skill: `"implement"` first (bare name). If no skill matches, try skill: `"larch:implement"` (fully-qualified plugin name).
- args: `"--quick --auto [--merge] <feature-description>"`

Only include `--merge` in the args if `alias_merge=true`.
