# Conflict Resolution Procedure

**Consumer**: Rebase Checkpoint Macro early checkpoints (Steps 1.r, 4.r, 7.r, 7a.r) and the active Rust Step 8+ ship lifecycle. Early checkpoints enter when the macro's `scripts/larch.sh push rebase --no-push --skip-if-pushed --keep-on-conflict` exits 1 with a rebase still in progress. The Rust pre-push rebase similarly hands off when conflict resolution must move to the orchestrator. **`--keep-on-conflict` / exit-4 routing**: this file owns Markdown procedure routing only for a rebase that remains in progress, covering macro `early_rebase` and `ship_pr_pre_push` **exit-4** handoff. When the driver needs a conflict handoff, it persists `RESUME_PHASE` / `CALLER_KIND` / `CONFLICT_FILES` to `ship-pr-state.sh`; `ship route-exit` emits `NEXT_ACTION=conflict-fix` for orchestrator routing. Retired in Phase 1 (#3364): `step8b_rebase`, `step12_phase4`, `step8_apply_bump_same_version`, and the Rebase + Re-bump Sub-procedure.

**Contract**: Authoritative orchestrator contract for rebase conflict-resolution via the `larch:ci-fixer` subagent in `MODE=conflict`. The main agent must never Read conflicted hunks, never classifies conflicts inline, and never edits conflicted files. Phase 1 classification/resolution, Phase 3 self-review, and the Phase 4 local `--continue` loop run inside the subagent (`agents/ci-fixer.md` `MODE=conflict`); Phase 2 operator escalation (`AskUserQuestion` / `SendMessage`) stays with the main agent on `needs-operator`. Preserve the upstream (main) / feature branch commit labels, never "ours"/"theirs" (enforced in the subagent prompt). Preserve the `early_rebase` Phase 3 skip, the `ship_pr_pre_push` Phase 3 self-review for non-trivial resolutions (now subagent self-review; no external panel), the no-push Phase 4 rule, and the rebase-abort bail invariant. Phase 4 exit 0 for `early_rebase` returns to the macro. For `ship_pr_pre_push`, re-invoke `${CLAUDE_PLUGIN_ROOT}/skills/implement/scripts/step-8-ship.sh` through the Step 8 bgjob start/wait pair so `run_rebase_rebump` can finish verification and force-push. Attribution for this path is `MODE=subagent` / `TIER=subagent`.

**When to load**: when `scripts/larch.sh push rebase` exits 1 in the Rebase Checkpoint Macro's `--no-push --skip-if-pushed --keep-on-conflict` early_rebase path, or when the Rust ship driver emits `NEXT_ACTION=conflict-fix` with `RESUME_PHASE=ship-pr-rrr-phase14` and `CALLER_KIND=ship_pr_pre_push` in `.ship-route-exit-handoff.env`. Read this file before spawning the conflict-mode subagent. Do NOT load on any other `scripts/larch.sh push rebase` exit code or ship routing token.

---

When `scripts/larch.sh push rebase` exits 1, conflicts paused the rebase. Route resolution to `larch:ci-fixer` (`MODE=conflict`). Operator escalation stays with the main agent; the subagent returns `needs-operator` instead of calling `AskUserQuestion`.

**Caller families**:

- `caller_kind=early_rebase`: spawn the conflict-mode subagent with this caller kind. On `FIXER_RESULT=resolved`, return to the Rebase Checkpoint Macro success path (M3). Bail paths abort the rebase (idempotent verify), set `STALL_TRACKING=true`, and skip to Step 18. No panel and no push occur; Step 5 normal review covers correctness later, and no version bump exists yet.
- `caller_kind=ship_pr_pre_push`: spawn the conflict-mode subagent with this caller kind. On `FIXER_RESULT=resolved`, local-only rebase succeeded. Do NOT push from this file. **Re-invoke the active Step 8+ selector**: launch `${CLAUDE_PLUGIN_ROOT}/skills/implement/scripts/step-8-ship.sh` through the Step 8 bgjob start/wait pair. The Rust driver verifies the completed rebase, lease-pushes the new head, clears the handoff, and resumes CI. **Bail** matches `early_rebase`: abort, set `STALL_TRACKING=true`, skip to Step 18.

**Bail invariant**: Any hard bail from this procedure must call `"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" git rebase-abort` before the caller-family bail destination (idempotent; safe when already aborted), because the rebase stays in progress until abort or Phase 4 exit 0.

## Inputs

The caller supplies `CONFLICT_FILES` as a comma-separated list. For `caller_kind=early_rebase`, use the `CONFLICT_FILES=...` line from the macro M1 `scripts/larch.sh push rebase --no-push --skip-if-pushed --keep-on-conflict` stdout. For `caller_kind=ship_pr_pre_push`, use the conflict list from `.ship-route-exit-handoff.env`; Rust `ship pr` writes the same list into `$IMPLEMENT_TMPDIR/ship-pr-state.sh`. If absent, pass an empty `CONFLICT_FILES=` token so the subagent falls back to `git diff --name-only --diff-filter=U`. Do not invent conflict metadata beyond the driver- or macro-provided list. Do not Read conflicted file contents in the main agent.

## Spawn the conflict-mode subagent

Spawn the Agent tool with `subagent_type` `larch:ci-fixer`. The prompt contains only:

- `MODE=conflict`
- repository root
- working branch
- `caller_kind=early_rebase` or `caller_kind=ship_pr_pre_push`
- `CONFLICT_FILES=<comma-separated list>` (may be empty)
- `$IMPLEMENT_TMPDIR` when available
- the contract reminders from `agents/ci-fixer.md` `MODE=conflict` (label rule, trivial classification, self-review for non-trivial `ship_pr_pre_push`, per-hop re-capture, no push, bail invariant)

No conflicted hunk content is inlined. Attribution for this path is `MODE=subagent` / `TIER=subagent`.

Append each round's `FIXER_SUMMARY` and any per-file resolution table from the subagent message body to `$IMPLEMENT_TMPDIR/conflict-fixer-rounds.md` (create if absent). Do not Read conflicted paths while doing so.

## Parse the result

Parse only the final message's three `FIXER_*` lines:

```
FIXER_RESULT=resolved|needs-operator|bail
FIXER_COMMIT=
FIXER_SUMMARY=<one line>
```

### `FIXER_RESULT=resolved`

Route exactly as today's Phase 4 exit 0:

- `caller_kind=early_rebase`: return to the Rebase Checkpoint Macro success path. Do NOT push.
- `caller_kind=ship_pr_pre_push`: re-invoke `${CLAUDE_PLUGIN_ROOT}/skills/implement/scripts/step-8-ship.sh` through the Step 8 bgjob start/wait pair. Do not rerun Step 7a architectural-guidelines Phase A and do not call guideline invalidate or pin helpers here. Pass no resume phase; the Rust driver reads scoped state internally. Do NOT invoke the retired Rebase + Re-bump Sub-procedure.

### `FIXER_RESULT=needs-operator`

The subagent kept the rebase in progress. Run the existing escalation prompt via `AskUserQuestion` once with the upstream (main) version, feature branch commit version, and proposed resolution for each uncertain file (use the per-file context from the subagent message body; do not Read conflicted hunks yourself). Use explicit labels — never "ours"/"theirs".

- On operator guidance: continue the same subagent via `SendMessage` with the guidance text (and the same `MODE=conflict` / caller kind / paths). When `SendMessage` is unavailable, spawn a fresh `larch:ci-fixer` with `MODE=conflict` and the operator-guidance text (same gating pattern as the CI-fixer round loop / `/review --subagent`).
- If the operator says to abort, or guidance cannot resolve the conflict: run `"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" git rebase-abort`, set `STALL_TRACKING=true`, and bail to Step 18.

### `FIXER_RESULT=bail` or an unparseable final message

Verify the rebase was aborted: run `"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" git rebase-abort` idempotently. Then route today's bail path: set `STALL_TRACKING=true` and skip to Step 18. Give the subagent one fresh respawn only when the message was unparseable and a rebase is still in progress with no evidence of a hard failure class; if that also fails, bail as above.

## Dead-subagent salvage

If the subagent dies or returns no usable trailer while a rebase is still in progress: run `"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" git rebase-abort`, set `STALL_TRACKING=true`, and skip to Step 18. The CI-fixer dirty-tree salvage-commit rule (`CI fix round <N> salvage`) does **not** apply mid-rebase; abort is the deterministic safe action and matches the bail invariant.

## Phase ownership (reference)

| Phase | Owner | Notes |
|-------|--------|-------|
| 1 Classification and resolution | `larch:ci-fixer` `MODE=conflict` | Per-file trivial / high-confidence / uncertain |
| 2 Operator escalation | Main agent on `needs-operator` | `AskUserQuestion` + `SendMessage` / fresh-spawn |
| 3 Self-review | `larch:ci-fixer` `MODE=conflict` | `ship_pr_pre_push` non-trivial only; skip for `early_rebase` and trivial-all |
| 4 Continue rebase | `larch:ci-fixer` `MODE=conflict` | `"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" push rebase --continue --no-push --keep-on-conflict`; per-hop `CONFLICT_FILES` re-capture |

The Phase 1-4 procedure text that the subagent executes lives in `agents/ci-fixer.md` under `MODE=conflict`. Do not re-run those phases in the main agent.
