# /implement Step 0 bootstrap recovery

**Consumer**: Step 0 routing rows for `degraded-prompt` and `dirty-recovery`.
**Contract**: Authoritative degraded-prompt handling and dirty-tree recovery gate after bootstrap returns a non-`step2` directive.
**When to load**: **MANDATORY: READ ENTIRE FILE** before executing either routing row.

## Bootstrap edit gate (NEVER #21)

**Bootstrap edit gate (NEVER #21)**: do not call Edit, Write, or repo-mutating Bash on git-tracked paths while `BOOTSTRAP_NEXT=degraded-prompt` or `BOOTSTRAP_NEXT=dirty-recovery` is active. Repo edits remain forbidden until a resumed `step-0-bootstrap.sh --mode resume` run returns exit 0 and `BOOTSTRAP_NEXT=step2`.

## Degraded prompt handling

**Degraded prompt handling.** When `DEGRADED_PROMPT_REQUIRED=true`, the explanation block was already relayed to operator-visible stderr during Step 0 bootstrap. Present the relayed degraded explanation block verbatim (from bootstrap stderr during Step 0). Fire `AskUserQuestion` with exactly two choices: **Continue (reduced panel — unavailable tools dropped, no cross-tool or Claude padding)** / **Abort**.

On **Continue**, write `$IMPLEMENT_TMPDIR/.degraded-tools-gate-prompted`, preserve `$IMPLEMENT_TMPDIR`, and rerun `step-0-bootstrap.sh --mode resume`. On **Abort**, set `STALL_TRACKING=true` and skip to Step 18 terminalization, followed by Step 19 cleanup.

If `PRESENCE_INPUT_EMPTY=true` appears in the envelope, append a `Warnings` entry to `$IMPLEMENT_TMPDIR/execution-issues.md` and preserve the gate diagnostics in operator-visible output. If `DEGRADED_PROMPT_REQUIRED=true` surfaces from an absorbed continue-tail on a resume path, run this same degraded-prompt branch before treating missing `ROUTE=` / `REBASE_RC=` details as rebase failure. A one-down result without `$IMPLEMENT_TMPDIR/.degraded-tools-gate-prompted` emits `DEGRADED_PROMPT_REQUIRED=true` and does not auto-continue. A both-down result emits `DEGRADED_HARD_FAIL=true` and stops before checkpoint `1.r`; stale sentinels never permit both-down continuation. The gate is not a later vendor-routing input.

## Step 0 dirty-tree recovery gate

Step 0 dirty-tree recovery gate:

1. Write `$IMPLEMENT_TMPDIR/dirty-tree-detected.env` with `STATUS=dirty-or-unknown`, `STAGE=step0-plan-materialize`, and `RECOVERY_REQUIRED=true`.
2. If `$IMPLEMENT_TMPDIR/.dirty-tree-prompted-step0-plan-materialize` is absent, create it and fire `AskUserQuestion` with exactly two operator paths: **Restore a clean tree and continue** / **Cancel this implement run**.
3. On **Restore a clean tree and continue**: the operator cleans the worktree back to the Step 0 checkpoint state (for example by stashing, discarding scratch edits they do not want in this run, or otherwise restoring a clean `git status`), then the orchestrator re-runs `scripts/larch.sh dirty-tree checkpoint` and only continues when it returns `STATUS=clean`. Keep `RECOVERY_REQUIRED=true` until the clean re-check succeeds. Once clean, rewrite `$IMPLEMENT_TMPDIR/dirty-tree-detected.env` with `RECOVERY_REQUIRED=false`, `unset IMPLEMENT_BAIL_REASON`, export the existing `IMPLEMENT_TMPDIR`, and immediately re-run the resume fence below.
4. On **Cancel this implement run**: keep `RECOVERY_REQUIRED=true`, set `STALL_TRACKING=true`, and skip to Step 18 terminalization, followed by Step 19 cleanup.

The resumed bootstrap tail re-runs `scripts/larch.sh dirty-tree checkpoint` before any Phase 3 tail helper. If that re-probe returns `STATUS=dirty` or `STATUS=unknown`, stay in recovery mode and do not branch/log. Parse the resumed wrapper stdout before continuing so `IMPLEMENT_BAIL_REASON`, `BRANCH_NAME`, `BRANCH_ACTION`, and `PLAN_FILE` come from the resumed tail rather than the pre-recovery pass. Parse the resumed wrapper stdout before re-evaluating `BOOTSTRAP_NEXT`.

Use this shape:

```bash
[ -z "${CLAUDE_PLUGIN_ROOT:-}" ] && [ -n "${IMPLEMENT_TMPDIR:-}" ] && [ -f "$IMPLEMENT_TMPDIR/plugin-root.env" ] && . "$IMPLEMENT_TMPDIR/plugin-root.env"
export IMPLEMENT_TMPDIR
[ -z "${CLAUDE_PLUGIN_ROOT:-}" ] && [ -n "${IMPLEMENT_TMPDIR:-}" ] && [ -x "$IMPLEMENT_TMPDIR/larch-run.sh" ] && CLAUDE_PLUGIN_ROOT=$("$IMPLEMENT_TMPDIR/larch-run.sh" --print-plugin-root 2>/dev/null || true)
export CLAUDE_PLUGIN_ROOT
# Dirty-tree resume rehydrates implementer selection and lifecycle parent context from the tmpdir.
LARCH_CLAUDE_PID="$PPID" "${CLAUDE_PLUGIN_ROOT}/skills/implement/scripts/step-0-bootstrap.sh" --mode resume
```
