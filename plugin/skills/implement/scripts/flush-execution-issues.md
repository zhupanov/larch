# flush-execution-issues.sh

`flush-execution-issues.sh` appends `$IMPLEMENT_TMPDIR/execution-issues.md` to
the staged `execution-issues` larch-log batch before terminal archive publication.
Primary callers: `/implement` Step 7a checkpoint and Step 18 terminal snapshot.

Usage:

```bash
flush-execution-issues.sh \
  --log-root PATH \
  --run-id RUN_ID \
  --issue-log PATH \
  [--batch execution-issues]
```

`--log-root` must be absolute. `--run-id` is restricted to letters, numbers, and
hyphens before constructing `PATH/implement/RUN_ID/execution-issues.ndjson`.
`--skill` is hardcoded to `implement`.

Output envelope:

- `FLUSH_STATUS=skip|already-flushed|no-records|ok|failed`
- `RECORDS=<N>`
- `APPEND_LOG_FILE=<path>` when an append was attempted or failed during record
  composition; the emitted file path remains readable after process exit

Optional flags:

- `--step-label VALUE` overrides the default record step (`7a`)
- `--source-label VALUE` overrides the default record source

Invariants:

- Empty or absent `--issue-log` is a successful skip.
- Default Step 7a calls create `$IMPLEMENT_TMPDIR/.execution-issues-step7a-reached`
  (or the issue-log directory equivalent) even when the flush is a skip, so
  later archive-tail helpers know the pre-publication checkpoint already ran.
- Idempotency uses both `$IMPLEMENT_TMPDIR/.execution-issues-flushed.sha` and an
  existing batch `source_sha256` probe. When the sentinel is missing, the batch
  probe matches the normalized per-entry hashes the record composer stores,
  with a whole-file SHA fallback for backward compatibility.
- Records are composed by `scripts/larch.sh execution-issues flush` with
  `step="7a"` and `source="execution-issues.md pre-bump"` (historical label;
  no version bump occurs — kept for data contract compatibility) unless
  overridden by `--step-label` / `--source-label`.
- On `FLUSH_STATUS=ok` or `FLUSH_STATUS=no-records`, the flushed
  `execution-issues.md` file is cleared so later flushes append only the
  unflushed tail entries.
- `run-log append` failures are non-fatal to `/implement`: the helper logs
  the captured append output back to `execution-issues.md` through
  `run-log append-failure` and exits 1 so the caller can record a wrapper
  failure if desired.

Makefile wiring: `make test-flush-execution-issues`, included in
`test-harnesses-3`.

Harness coverage: empty input, single-section record composition, multi-section
record composition, idempotent rerun, partial-flush retry, and `run-log` failure
logging, all in `crates/larch-cli/src/execution_issue_commands.rs`.

Edit In Sync:

- `crates/larch-cli/src/execution_issue_commands.rs`
- `crates/larch-cli/src/implement_finalize_commands.rs`
- `skills/implement/SKILL.md`
