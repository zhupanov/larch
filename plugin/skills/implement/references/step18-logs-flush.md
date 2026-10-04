# /implement Step 18 logs flush and recovery

**Consumer**: `/implement` Step 18.

**Contract**: Normative stall-gate interpretation, final-report refresh, complete
terminal snapshot preparation, and lifecycle publication.

**When to load**: **MANDATORY READ ENTIRE FILE** at Step 18 entry, after the
Step 18 banner and before
`scripts/larch.sh implement step-18-gate-logs-flush`.

## Stall recovery gate

Step 18a runs first on every Step 18 entry. Stall paths and Step 12d bails skip
directly here, so recovery precedes the Step 16/17 final report on those paths.
The dominant no-stall path uses one composite fence. That fence reads all stall
layers, emits `STALL_RECOVERY_REQUIRED` and `STALL_TRACKING_*`, normalizes the
outcome, and enters terminal snapshot preparation when no prompt-side recovery
is required.

Resolve `STALL_TRACKING` from the in-memory value,
`$IMPLEMENT_TMPDIR/ship-pr-state.sh`,
`$IMPLEMENT_TMPDIR/finalize-state.sh`, and
`$IMPLEMENT_TMPDIR/session-env.sh`. An identity-checked abandoned checks bgjob
is the fifth signal. A layer is active when it is neither empty nor exactly
`false`.

Route active work on `NEXT_ACTION=stall-recovery`. After `CLEARED=true`, invoke
`step-18.sh --phase logs-flush`; do not rerun the composite gate.

## Final report and closing ledgers

The logs phase runs `final-report step18b`, preserving the existing
recover-then-report behavior. It then emits the terse since-last-mark reports
and records the closing `Step 18 — logs flush` token and timing marks. The
terminal snapshot renderer runs after those marks, while the ledgers and
session tmpdir still exist.

`step-18.sh --phase logs-flush --step17-emitted true` records that a non-empty
Step 17 body is pending for deferred chat emission. Step 18 markers take
precedence when `EMIT_BODY=true`, `WFR_RC=0`, and the marker body is valid.
Otherwise, the Step 17 cache remains the candidate. The orchestrator does not
emit either body until Step 19 finishes.

## Complete terminal snapshot

`run-log prepare-terminal-snapshot` owns the last mutable log writes. It
refreshes:

- final-summary projection;
- full token and timing JSON plus derived batches;
- vendor failure diagnostics;
- invariant and guideline ship outcomes;
- ship-route handoff when present;
- the session transcript on every path with a configured source;
- the final execution-issues tail; and
- manifest reachability, including `steps_ran.step18=true`.

The preparer is fail closed. A configured transcript source that cannot be
recaptured reports its exact status, retains any prior staged transcript, and
blocks publication. A missing unconfigured source is recorded as an execution
issue so completeness can waive that unavailable artifact under I-Flush-1.

`RUN_LOG_FINAL_FLUSH_OK=true` means the whole preparation completed and the
terminal files were verified. It never means only the execution-issues append
succeeded.

## Terminal publication

After snapshot preparation, Step 18 calls exactly one lifecycle terminal verb.
The lifecycle owner finalizes the manifest and outcome, verifies required
artifacts, scrubs secrets and paths, creates the archive, and publishes it when
storage is enabled.

Success must be one of:

- `RUN_LOG_PUBLICATION=published`, `LIFECYCLE_FLUSHED=true`, and verified remote
  and cache fields;
- `RUN_LOG_PUBLICATION=skipped-disabled`,
  `LIFECYCLE_FLUSHED=false`, and no remote fields; or
- explicit operator suppression with
  `RUN_LOG_PUBLICATION=skipped-suppressed` and
  `RUN_LOG_PUBLISH_SKIPPED=no-logs-commit`.

Every successful state records
`$IMPLEMENT_TMPDIR/.run-log-terminalized`. Publication failure does not record
the fence, retains recovery material, and blocks Step 19.
