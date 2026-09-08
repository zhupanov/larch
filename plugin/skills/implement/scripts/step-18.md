# step-18.sh

`step-18.sh` is the two-phase terminal log wrapper for `/implement` Step 18.
It never restores repository state or invokes teardown.

## Invocation

Gate phase:

```bash
"$HOME/.cache/larch/sessions/implement-run-$PPID.sh" \
  skills/implement/scripts/step-18.sh \
  --phase gate \
  --stall-tracking-memory "${STALL_TRACKING:-false}"
```

Terminal logs phase:

```bash
"$HOME/.cache/larch/sessions/implement-run-$PPID.sh" \
  skills/implement/scripts/step-18.sh \
  --phase logs-flush \
  --step17-emitted "${STEP17_EMITTED_FOR_STEP18:-false}"
```

The normal path uses the composite
`scripts/larch.sh implement step-18-gate-logs-flush`. The standalone phases remain
for the active-stall breakout path.

## Gate phase

The gate resolves the in-memory, `ship-pr-state.sh`, `finalize-state.sh`, and
`session-env.sh` stall layers plus the abandoned-checks marker. It emits the
`STALL_TRACKING_*` KVs and `STALL_RECOVERY_REQUIRED=true|false`. Active recovery
remains prompt-side and must finish before the terminal logs phase starts.

## Logs flush phase

The logs phase runs the Step 18b final-report refresh, records closing token and
timing marks, then invokes:

```bash
"$CLAUDE_PLUGIN_ROOT/scripts/larch.sh" run-log prepare-terminal-snapshot \
  --implement-tmpdir "$IMPLEMENT_TMPDIR" \
  --run-id "$RUN_ID"
```

The preparer refreshes the final summary, token and timing reports, vendor
failure diagnostics, architectural outcome batches, ship-route handoff,
session transcript, and execution-issues tail. Transcript capture runs whenever
a source is configured, including a normal path with a completed Step 7a. A
failed recapture keeps the prior staged transcript when one exists, reports the
exact `SESSION_TRANSCRIPT_STATUS`, returns
`RUN_LOG_FINAL_FLUSH_OK=false`, and preserves the session.

After complete snapshot preparation, Step 18 invokes exactly one matching
lifecycle terminal verb. Enabled storage publishes one create-only archive.
Disabled storage emits `RUN_LOG_PUBLICATION=skipped-disabled`. Explicit
`NO_LOGS_COMMIT=true` emits
`RUN_LOG_PUBLICATION=skipped-suppressed`. Every successful state writes
`$IMPLEMENT_TMPDIR/.run-log-terminalized`, which is the Step 19 cleanup fence.

Publication or verification failure returns nonzero without writing that fence.
The orchestrator must not run Step 19 on failure.

## Marker body handoff

After successful terminalization, `EMIT_BODY=true`, `WFR_RC=0`, and a non-empty
`summary-final.md` cause the wrapper to emit the body between:

- `---LARCH-SUMMARY-FINAL-BEGIN---`
- `---LARCH-SUMMARY-FINAL-END---`

The orchestrator caches the body, runs Step 19, relays the teardown tail, then
emits the cached body as the final chat text. There is no disk fallback after
cleanup.

## Stream contract

Do not call `larch_quiet_init`. Stall KVs, terminal snapshot KVs, lifecycle KVs,
Step 18b KVs, and marker lines remain on captured stdout.

## Edit in sync

Update `skills/implement/SKILL.md`, `skills/implement/scripts/step-19.md`,
`crates/larch-cli/src/implement_terminal_commands.rs`, the focused tests,
runtime projection, and `docs/linting.md` when this contract changes.
