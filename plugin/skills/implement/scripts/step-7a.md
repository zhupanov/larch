# step-7a.sh

`step-7a.sh` is the direct `/implement` Step 7a orchestration helper; active prompt-side Step 7a launches the Rust entrypoint with `--bgjob-launch true`. It rehydrates session context, handles code-flow diagram generation and `larch:diagrams` comment upsert, runs the 7a.r rebase checkpoint, and checkpoints pending execution issues before Step 8.

## Interface

```bash
skills/implement/scripts/step-7a.sh \
  --implement-tmpdir PATH \
  [--issue-number N] \
  [--run-id ID] \
  [--no-logs-commit BOOL] \
  [--forked-target BOOL]
```

`--implement-tmpdir` is required and must be absolute. Optional values fall back to `$IMPLEMENT_TMPDIR/session-env.sh` keys when omitted.

## Stdout contract

| Key | Values |
| --- | --- |
| `DIAGRAM_STATUS` | `ok`, `skipped`, `failed`, or `skip` (`skip` means the small/non-runtime classifier skipped generation) |
| `DIAGRAM_PATH` | Absolute path to `code-flow-diagram.md`, or empty |
| `COMMENT_URL` | Tracking issue comment URL, or empty when upsert is gated, skipped, or failed |
| `LOG_CHECKPOINT_STATUS` | `ok`, `degraded`, or `skip`; this is a local execution-issues checkpoint and never publication |
| `STEP_7A_BAIL_REASON` | Empty on non-argument paths; `argv`, `missing-implement-tmpdir`, or `invalid-implement-tmpdir` on argument validation errors |

The helper re-emits the rebase checkpoint-probe KV envelope before its final KV tail.

## Exit Codes

| Code | Meaning |
| --- | --- |
| `0` | Step completed or degraded non-fatally |
| `1` | Rebase checkpoint reported a conflict and Step 7a preserved that exit |
| `3` | Rebase checkpoint reported a non-conflict failure and Step 7a preserved that exit |
| Other non-zero | Step 7a preserved the probe exit; the orchestrator uses the macro's `unexpected-rc-<n>` / other-non-zero routing |
| `2` | Argument validation failed |

## Bail Reasons

Argument validation may emit `argv`, `missing-implement-tmpdir`, or `invalid-implement-tmpdir`. Diagram generation, comment upsert, rebase probe, and checkpoint degradation do not set a bail reason; they append warnings or tool failures and continue.

## Invariants

- Phases stay in this order: rehydrate, token/timing marks, classifier, Code Flow generation, shared diagrams-comment upsert, 7a.r rebase probe, execution-issues checkpoint, final KV tail.
- The classifier, diagram generator, and 7a.r rebase probe use module-level `base_remote` / `base_ref`, defaulting to `origin/main` and switching to `upstream/main` when `--forked-target true` is on argv or when `LARCH_FORKED_TARGET=true` is rehydrated from `$IMPLEMENT_TMPDIR/session-env.sh` during session-key lookup.
- `LARCH_FORKED_TARGET` has no direct shell-environment fallback; only argv and the session-env file are honored.
- When `REPO` or `UPSTREAM_REPO` is present in `$IMPLEMENT_TMPDIR/session-env.sh`, Step 7a threads the resolved owner/repo to `scripts/larch.sh diagrams upsert` via `--repo`.
- Step 7a writes `$IMPLEMENT_TMPDIR/code-flow-section.md` only when the in-process `implement code-flow-diagram` owner reports `STATUS=ok`. The file contains the `## Code Flow Diagram` section passed to `scripts/larch.sh diagrams upsert`.
- When generation is skipped or failed, Step 7a removes any stale local `code-flow-diagram.md` / `code-flow-section.md`, omits the upsert, and preserves any prior valid Code Flow section on the issue instead of replacing it with a placeholder.
- Empty `ISSUE_NUMBER` still gates the tracking-issue upsert.
- `larch:diagrams` uses the shared stable marker `<!-- larch:diagrams v1 -->`; Step 7a does not call `scripts/larch.sh tracking-issue upsert-summary` directly and does not use a `runid=` marker for diagrams.
- The local execution-issues checkpoint runs after the 7a.r rebase probe on every path. Probe failure preserves the probe rc for orchestrator routing while still staging pending issues when inputs allow.
- Step 7a does not render terminal token, timing, vendor, or transcript batches and never invokes the retired `run-log commit` command. Step 18 owns that complete snapshot.
- The helper does not write a `diagrams` larch-log batch.
- With `--bgjob-launch true`, the Rust entrypoint truncates the merge-result env and starts bgjob step slug `implement-step7a`. The child mirrors required KVs into the merge-result env for the final bgjob `DONE` gate.

## Regression checklist

- Green generation writes `code-flow-section.md` and invokes `scripts/larch.sh diagrams upsert`.
- Prior Architecture content is preserved by the shared helper while Code Flow is replaced.
- No prior diagrams comment produces a Code Flow-only body.
- `STATUS=skipped` and `STATUS=failed` omit `code-flow-section.md` and skip the upsert.
- Legacy `<!-- larch:diagrams v1 runid=... -->` comments do not collide with the stable marker.
- `ARCHITECTURE_DIAGRAM_FILE` has no effect on Step 7a.

## Edit-in-sync

Keep this file aligned with:

- `skills/implement/SKILL.md` Step 7a
- `skills/implement/scripts/test-step-7a.sh`
- `crates/larch-cli/src/implement_review_commands.rs` (behavioral authority and inline tests)
