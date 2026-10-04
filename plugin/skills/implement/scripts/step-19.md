# step-19.sh

`step-19.sh` is the cleanup-only wrapper for `/implement` Step 19.

It requires the regular `$IMPLEMENT_TMPDIR/.run-log-terminalized` record written
by Step 18 after successful publication, storage-disabled terminalization, or
explicit `--no-logs-commit` suppression. If that record is absent or invalid,
cleanup fails closed and preserves the session.

The wrapper delegates to:

```bash
"$CLAUDE_PLUGIN_ROOT/scripts/larch.sh" implement step-19 \
  --implement-tmpdir "$IMPLEMENT_TMPDIR"
```

Step 19 may restore `finalize-state.sh`, clear the process-scoped implement
pointer, deactivate run state, rename the tracking issue, stash a stalled work
tree, and remove session material. It does not call a run-log writer,
checkpoint, snapshot preparer, or publisher.

The teardown tail remains on stdout for orchestrator relay. It includes
`ISSUE_URL`, `RENAME_BRANCH`, `RENAME_STATUS`, `STASH_REF`,
`SENTINEL_WRITTEN`, `FINALIZE_SUBCOMMAND`, `FINALIZE_WARNINGS`, and sibling
`FINALIZE_*` KVs.
