# /implement Step 19 cleanup

**Consumer**: `/implement` Step 19.

**Contract**: Cleanup begins only after Step 18 terminalization and performs no
run-log writes.

**When to load**: **MANDATORY READ ENTIRE FILE** after Step 18 returns
`NEXT_ACTION=logs-flush-done` and before `step-19.sh`.

Step 19 requires a regular
`$IMPLEMENT_TMPDIR/.run-log-terminalized` file containing both
`RUN_LOG_TERMINALIZED=true` and `LIFECYCLE_TERMINALIZED=true`. Missing,
symlinked, or invalid evidence returns
`CLEANUP_BLOCKED=run-log-not-terminalized` and preserves the session.

After the fence passes, Step 19 may:

- restore `finalize-state.sh` from the guarded session writer when ship and
  finalize stall state differ;
- clear the process-scoped implement pointer;
- perform final issue-prefix and branch bookkeeping;
- deactivate the run registry entry;
- preserve stalled edits and write the stalled sentinel; and
- remove the session tmpdir on a non-stalled terminal path.

It does not invoke `run-log`, render a batch, capture a transcript, recover or
update the run-log manifest, or recreate the staging run.

Relay the teardown tail from captured Step 19 stdout before emitting the cached
terminal summary body. The tail includes `ISSUE_URL`, `RENAME_BRANCH`,
`RENAME_STATUS`, `STASH_REF`, `SENTINEL_WRITTEN`, `FINALIZE_SUBCOMMAND`,
`FINALIZE_WARNINGS`, and sibling `FINALIZE_*` KVs.
