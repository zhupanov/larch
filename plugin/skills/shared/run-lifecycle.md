# Shared run lifecycle

This contract applies only to a skill whose `SKILL.md` contains
`# larch-run-lifecycle: shared-v1 skill=<name>` in its YAML frontmatter.
Before running a lifecycle command, resolve `<name>` in the machine-checked
`skills/shared/run-lifecycle-ownership.tsv` registry. The `*` row selects the
generic start and terminal commands below. A named row selects its registered
owners instead. Do not run a second lifecycle path for a named skill.

Only for a skill resolved through the `*` row, run this command at invocation
start before the skill performs work:

Use the generic `$PWD` fallback only when this Bash call starts in the client
repository. Bash tool calls do not inherit a `cd` from an earlier call. If this
call may start in a worktree, scratch directory, or other checkout, pass the
previously resolved absolute client repository root instead. Do not use `$PWD`
to rediscover it.

```bash
"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" run-log lifecycle-start --repo-root "${CLAUDE_PROJECT_DIR:-$PWD}" --skill "<name>"
```

A nested child may lead with
`--lifecycle-parent-context <absolute-context-path>`. Consume one pair, bind
`LIFECYCLE_PARENT_CONTEXT`, and remove it before public parsing. Root,
duplicate, missing, or later flags fail. When set, pass
`--lifecycle-parent-context "$LIFECYCLE_PARENT_CONTEXT"` to the start command.
The lifecycle CLI validates it and derives the immutable parent; no other parent
IDs or environment variables are allowed.

Parse `RUN_ID`, `SKILL`, `LOG_ROOT`, `RUN_DIR`, `CONTEXT_FILE`,
`RUN_LOG_STORAGE`, `RUN_LOG_STORAGE_REASON`, `STORAGE_BASE_URI`,
`CLIENT_REPO`, `TOOL_REPO_URI`, `RUN_LOGS_URI`, `STORAGE_PREFLIGHT`,
`PREFLIGHT_OK`, and `LIFECYCLE_STARTED` from stdout without `eval` or `source`.
Stop if the command fails, either success value is not `true`, or the storage
state is not one of these pairs:

- `RUN_LOG_STORAGE=enabled` and `STORAGE_PREFLIGHT=ok`
- `RUN_LOG_STORAGE=disabled` and `STORAGE_PREFLIGHT=skipped-disabled`

Disabled storage is an intentional local-only mode. Keep run-log staging and
bookkeeping active. Do not invent provider, URI, remote, cache, or pending
publication values.

Callers that already own a run ID pass `--run-id "<id>"`. Specialized owners
also pass their absolute `--log-root` and `--adopt-existing` when rich artifact
setup created the manifest first. A specialized owner whose Step 0 `session
setup` parse binds `REPO_ROOT` passes `--repo-root "$REPO_ROOT"` to the start
and terminal commands instead of the generic fallback shown here. The context
file persists the validated identity, staging root, publication mode and
reason, client repository, and either the enabled storage identity or disabled
local namespace ID. Later subprocesses rehydrate that pinned state without
shell state.

After start succeeds, run exactly one matching terminal command before the
skill returns. Require exit zero, `LIFECYCLE_TERMINALIZED=true`, and one valid
terminal pair:

Invoke that terminal command through
`"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh"`, as with lifecycle start.

- `RUN_LOG_PUBLICATION=published` and `LIFECYCLE_FLUSHED=true`
- `RUN_LOG_PUBLICATION=skipped-disabled` and `LIFECYCLE_FLUSHED=false`

Treat any other combination as a loud terminal failure. The
`skipped-disabled` pair is successful terminalization, not a publication
failure, and has no pending remote state.

- Success: `run-log lifecycle-finalize`
- Failure: `run-log lifecycle-failure`
- Operator cancellation: `run-log lifecycle-cancel`
- Non-error early return: `run-log lifecycle-early-return`

Pass the exact `--repo-root` value used for lifecycle start to the selected
terminal command. Also pass `--skill "<name>" --run-id "$RUN_ID"`. The generic
`${CLAUDE_PROJECT_DIR:-$PWD}` fallback remains valid only when the terminal call
starts in the client repository. Run the command before emitting terminal
user-facing prose.

For a nested child invocation, give the child the parent skill and run ID. The
child passes them to start as `--parent-skill "<parent-name>" --parent-run-id
"<parent-run-id>"`. Parent and child keep distinct `RUN_ID` values and distinct
archives.

Aliases are parent invocations, not alternate names for the target run. Start
and finish the alias under its alias name. When invoking its target through the
Skill tool, pass the parsed `CONTEXT_FILE` as the target's leading internal
`--lifecycle-parent-context` argument before all target arguments, so the target
starts with a distinct child ID and immutable parent metadata. Apply the same
handoff to every other child Skill call.
