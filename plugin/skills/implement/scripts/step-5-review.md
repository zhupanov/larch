# step-5-review.sh

Step 5 review bgjob launcher. The default mode is a foreground bgjob start wrapper whose harness-visible stdout is either the fresh-start line `BGJOB_STATUS=STARTED STEP=implement-step5-review PGID=<n>`, cached completion wait output, or, on same-step re-entry with a live identity-valid registry row, the immediate `bgjob wait` rejoin output.

## Caller

`skills/implement/SKILL.md` invokes this wrapper from the scripted review loop, then repeatedly invokes `scripts/larch.sh bgjob wait --step implement-step5-review` until `DONE` or `DEAD`.

## KV grammar

Fresh launch stdout is exactly one bgjob start line. The child writes Step 5 review KVs through `review-and-fix step5` into `$IMPLEMENT_TMPDIR/.step5-review-result.env`; bgjob merges that file into `$IMPLEMENT_TMPDIR/bgjob/implement-step5-review.result.env` at completion.

Normal continuation requires `BGJOB_RC=0`, `STEP5_REVIEW_STATUS=complete`, and the required Step 5 KVs in the final wait stdout and/or result env. A valid stall envelope (`STEP5_REVIEW_STATUS=stall` plus the required Step 5 KVs) from an active or final wait remains terminal for the current run. A cached canonical stall result env is restartable recovery state: without a live registry row, the wrapper clears it before starting a fresh review; with a live registry row, the wrapper clears it before rejoining the live bgjob. `DONE` alone, the launcher stdout, the shell exit code from `bgjob wait` are not sufficient.

## Invariants

- Bash 3.2 portable; no associative arrays or namerefs.
- Self-rehydrates `CLAUDE_PLUGIN_ROOT` from `$IMPLEMENT_TMPDIR/plugin-root.env` where needed.
- Telemetry marking is best-effort and runs in the bgjob child.
- `dynamic_archetypes_cap` resolves from `$IMPLEMENT_TMPDIR/session-env.sh`, then from process `LARCH_DYNAMIC_ARCHETYPES_MAX`, then the implement-mode default `1`.
- Truncates `$IMPLEMENT_TMPDIR/.step5-review-result.env` immediately before every fresh `bgjob start`.
- Reuses `$IMPLEMENT_TMPDIR/bgjob/implement-step5-review.result.env` only when it is canonical completion (`BGJOB_RC=0`, `STEP5_REVIEW_STATUS=complete`, and the required Step 5 KVs); stale, malformed, or stall result envs are cleared before a fresh start, and registry probe failures fail closed instead of relaunching.
- Removes legacy detach sidecars (`.step5-wrapper-detached`, `.step5-reattach-active`) before a fresh launch; migrated Step 5 does not create them.
- Delegates owner-death, orphan, timeout, process-group cleanup, stdout/stderr logs, and terminal result env publication to bgjob.
- Routes completion through the bgjob result env; routing keys on `BGJOB_RC=0` plus `STEP5_REVIEW_STATUS=complete` for completion and on a valid stall envelope from active or final waits for stall handling.
- Same-step re-entry with a live identity-valid registry row runs `bgjob wait` instead of launching a second review daemon; stale or dead rows are cleared before a fresh start, and non-complete cached result envs are cleared before the wait.

## Edit-in-sync

Update `skills/implement/SKILL.md`, `skills/implement/references/step5-review-branches.md`, `make test-implement-structure`, and the inline tests in `crates/larch-cli/src/implement_review_commands.rs` when this contract or argv changes.
