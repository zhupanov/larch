# step-8-oos-checkpoint.sh

Thin Step 8+ OOS checkpoint relay. Rust owns the #7681 workflow routing and post-pass bookkeeping in `implement step-8-oos-checkpoint`, and composes the existing Rust `oos disposition-checkpoint` command through the verified bootstrap.

## Caller

`skills/implement/SKILL.md` invokes this wrapper from the named `/implement` step so the prompt-side Bash fence remains a plugin-root source guard plus one script call.

## KV grammar

The wrapper forwards Rust stdout unchanged and exits with the Rust process rc only. The disposition-checkpoint rc is diagnostic in `OOS_CHECKPOINT_RC`; it is not the wrapper exit code.

Rust emits these keys when routing succeeds:

- `OOS_CHECKPOINT_RC=0` and `NEXT_ACTION=reship` only when disposition rc 0 and all bookkeeping succeeds.
- `OOS_CHECKPOINT_RC=<n>` and `NEXT_ACTION=stall` when disposition is non-zero.
- `OOS_CHECKPOINT_RC=<nonzero>` and `NEXT_ACTION=stall` when disposition rc 0 but run-statistics, manifest stamp, or `OOS_PENDING=false` patching fails. It never pairs `OOS_CHECKPOINT_RC=0` with `NEXT_ACTION=stall`.

## Rust-owned work

The Rust verb runs `${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh oos disposition-checkpoint` without forwarding child stdout, preserves child-written `oos-disposition-checkpoint.stderr.log` when captured stderr is empty, appends Tool Failures rows when needed, writes `run-statistics.md`, stamps `steps_ran.step9a1=true` on full success, best-effort stamps `step9a1=false` on bookkeeping failure, and atomically clears only the allowlisted `OOS_PENDING=false` state key.

OOS-checkpoint `NEXT_ACTION=stall` is not the post-driver Step 16 stall path. It halts Step 8+ until the checkpoint gap or bookkeeping failure is resolved.

The Rust command writes the machine contract directly, so inherited quiet mode cannot suppress `NEXT_ACTION`.

## Invariants

- Bash 3.2 portable; no associative arrays or namerefs.
- Self-rehydrates `CLAUDE_PLUGIN_ROOT` from `$IMPLEMENT_TMPDIR/plugin-root.env` where needed.
- OOS checkpoint telemetry follows `skills/shared/session-setup-output.md`; values come from `$IMPLEMENT_TMPDIR/session-env.sh`.

## Edit-in-sync

Update `skills/implement/SKILL.md` and the implement structure/timing harnesses when this contract or argv changes.
