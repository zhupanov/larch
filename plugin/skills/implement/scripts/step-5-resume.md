# step-5-resume.sh

Step 5 resume bgjob launcher. It records main-agent handoff timing and, for review-resume work, starts `implement-step5-resume` through bgjob with merge-result env capture.

## Caller

`skills/implement/SKILL.md` invokes `step-5-resume.sh --checks-site step5-review-fixes --final-round-num "$FINAL_ROUND_NUM"` after main-agent vote or coder-main-agent fixes, then repeatedly invokes `scripts/larch.sh bgjob wait --step implement-step5-resume` until `DONE` or `DEAD`.

The `--record-only` mode remains foreground because stall and durable-bail branches need only idempotent timing capture, not a long resume loop.

## KV grammar

Fresh launch stdout is exactly one bgjob start line:

```text
BGJOB_STATUS=STARTED STEP=implement-step5-resume PGID=<n>
```

The bgjob child tees `checks-step5-resume` stdout into `$IMPLEMENT_TMPDIR/bgjob/implement-step5-resume.merge.env`; bgjob merges it into `$IMPLEMENT_TMPDIR/bgjob/implement-step5-resume.result.env` with `BGJOB_RC`, `BGJOB_ELAPSED_S`, and `STEP`.

Normal continuation requires `BGJOB_RC=0` plus the required checks/resume KVs in the final wait stdout and/or result env. `DONE` alone, launcher stdout is not sufficient.

## Invariants

- Bash 3.2 portable; no associative arrays or namerefs.
- Self-rehydrates `CLAUDE_PLUGIN_ROOT` and token/timing context from `$IMPLEMENT_TMPDIR/session-env.sh`.
- Truncates `$IMPLEMENT_TMPDIR/bgjob/implement-step5-resume.merge.env` immediately before each fresh start.
- Delegates owner-death, orphan, timeout, process-group cleanup, stdout/stderr logs, and terminal result env publication to bgjob.
- Routes Step 5 resume completion through the bgjob result env; routing keys on `BGJOB_RC=0` and required KVs.
- `--record-only` records timing once and exits without bgjob launch.

## Edit-in-sync

Update `skills/implement/SKILL.md`, `skills/implement/references/step5-review-branches.md`, `make test-implement-structure`, and the inline tests in `crates/larch-cli/src/implement_review_commands.rs` when this contract or argv changes.
