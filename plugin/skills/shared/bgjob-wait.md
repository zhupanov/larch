# Bgjob foreground wait contract

Use this contract for long-running larch helpers that have migrated off Claude background launches.

1. Launch with `LARCH_CLAUDE_PID="$PPID" "${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" bgjob start --step <step> --tmpdir "$TMPDIR" --budget-s <seconds> -- <command...>` from a foreground Bash tool call when the harness has not already exported a durable `LARCH_CLAUDE_PID` / `CLAUDE_PID` / `LARCH_BGJOB_OWNER_PID`. `$PPID` must be the durable agent-session parent, never a nested one-shot wrapper that exits after `STARTED`. The only harness-visible stdout from the launcher is `BGJOB_STATUS=STARTED STEP=<step> PGID=<n>`.
2. If the child writes step result KVs for the orchestrator to consume, truncate or recreate that merge-input env immediately before every `bgjob start`, then pass it with `--merge-result-env <path>`. A stale env from a prior attempt must never satisfy a fresh wait's required-key gate.
3. Then call `"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" bgjob wait --step <step> --tmpdir "$TMPDIR" --max-wait-s 270` with tool timeout `330000`. Only wait after the matching launch printed `BGJOB_STATUS=STARTED STEP=<step> PGID=<n>`; if the launch did not print that marker, route directly to the step's failure or stall handling instead of waiting. Each wait poll refreshes a session-local wait lease; while that lease is fresh, the daemon will not orphan the child solely because the start-time owner PID exited (#8639).
4. If wait prints `BGJOB_STATUS=WAIT`, including `BGJOB_RECOVERY=retryable`, the next action is another identical `bgjob wait`. A retryable recovery retains its durable registry record and is not terminal `DEAD`. Do not emit prose, read task output, use Monitor, call TaskOutput, or sleep between waits.
5. If wait prints `BGJOB_STATUS=DEAD`, route through the step's existing failure or stall handling.
6. If wait prints `BGJOB_STATUS=DONE`, read the full KV block and the result env at `$TMPDIR/bgjob/<step>.result.env` before continuing. The result env is the completion source of truth. Continue normal branch handling only when `BGJOB_RC=0` and the step's required KVs are present in the final wait output and/or result env. Treat `BGJOB_RC=timeout`, `BGJOB_RC=orphaned`, any other non-zero `BGJOB_RC`, or missing required KVs as step failure or stall.

Never treat the `bgjob wait` shell exit code, `BGJOB_STATUS=DONE` alone, launcher stdout, wrapper stdout, or compatibility sidecars as sufficient for continuation.

## Clock and sleep invariant

The daemon measures runtime budgets, owner grace, and wake grace with a
suspend-pausing monotonic clock. It writes `HEARTBEAT_EPOCH` on every monitor
poll. Wall-clock epochs are used only for logs and cross-process TTLs such as
registry-heartbeat staleness and foreground wait-lease freshness.

When a wall-clock jump reveals that the host slept, the daemon refreshes the
registry heartbeat, resets owner validation, and grants one wait-lease window
before it may orphan the child. Advancing the wall clock without advancing the
monotonic clock never spends the runtime budget. A foreground waiter must still
repeat the documented wait command to refresh its lease after resume.

## Wrapper launch example

```bash
: >"$IMPLEMENT_TMPDIR/.step-5-review-merge.env"
LARCH_CLAUDE_PID="$PPID" "${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" bgjob start \
  --step implement-step5-review \
  --tmpdir "$IMPLEMENT_TMPDIR" \
  --budget-s 21600 \
  --merge-result-env "$IMPLEMENT_TMPDIR/.step-5-review-merge.env" \
  -- \
  bash "${CLAUDE_PLUGIN_ROOT}/skills/implement/scripts/step-5-review.sh" --tmpdir "$IMPLEMENT_TMPDIR"
```

Expected launcher stdout is exactly one line shaped like:

```text
BGJOB_STATUS=STARTED STEP=implement-step5-review PGID=12345
```

No banner, summary, or extra progress text may be printed by the launcher wrapper.

## Repeated wait example

```bash
"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" bgjob wait \
  --step implement-step5-review \
  --tmpdir "$IMPLEMENT_TMPDIR" \
  --max-wait-s 270
```

If stdout is:

```text
BGJOB_STATUS=WAIT
ELAPSED_S=270
```

run the same wait command again immediately. Between WAITs: no prose, no Read, no Monitor, no TaskOutput, no sleep, and no alternate progress probe.

## DONE parsing example

A successful wait can print:

```text
BGJOB_STATUS=DONE
BGJOB_RC=0
BGJOB_ELAPSED_S=42
STEP=implement-step5-review
STEP5_REVIEW_STATUS=complete
```

After `DONE`, parse all rows and read `$IMPLEMENT_TMPDIR/bgjob/implement-step5-review.result.env`. The step may continue only when the required step KVs are present and valid. `DONE` plus missing required KVs is a failure or stall path, not success.

## Step 8 handoff carve-out

Do not apply the generic `BGJOB_RC=0` success gate to `ship route-exit`. Step 8 reads the direct ship outcome KVs and numeric driver rc from the bgjob result env. The orchestrator validates those route inputs rather than treating `BGJOB_RC=0` alone as success.

## Long leaf wait carve-out

`/complete-umbrella` Step 1 is the only shipped caller that raises `--max-wait-s` above the default 270-second chunk. It uses `--max-wait-s 7200` with Bash `run_in_background: true` so a typical hour-scale leaf finishes in one or two wait calls without exceeding the Bash foreground timeout ceiling. This supersedes #8650's earlier `--max-wait-s 540` and tool timeout `600000` proposal, and reduces wait turns further. The wait still refreshes the wait lease on every poll. Other skills keep the 270-second chunk and foreground timeout `330000` contract above.

## Parallel external lanes

Every concurrent external lane must use a unique `--step` slug, such as `review-codex-1` and `review-cursor-1`. Shared slugs clobber registry rows, stdout/stderr logs, and `$TMPDIR/bgjob/<step>.result.env`.
