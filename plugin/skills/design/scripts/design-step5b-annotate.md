# design-step5b-annotate.sh

## Purpose

Thin launcher-compat wrapper for the `/design` Step 5b annotate block.

## Primary callers

- `skills/design/SKILL.md`

## Invariants

- The `.sh` file only derives and exports `CLAUDE_PLUGIN_ROOT`, then execs `scripts/larch.sh design step5b-annotate`.
- `scripts/larch.sh design step5b-annotate` owns the OOS annotate behavior.
- The Rust entrypoint hydrates the wrapper environment before reading session keys.
- The `DESIGN_TMPDIR` guard rejects only an empty value, matching the retired Bash annotate prelude.
- The annotate entrypoint binds `oos_issue_stdout = design_tmpdir / "oos-issue.stdout.txt"` immediately after the tmpdir guard.
- The same `oos_issue_stdout` path is used for `--issue-stdout-file` and `ISSUES_FAILED` detection.
- The annotate entrypoint returns immediately through pause-save when `.pause-requested` exists.
- Normal annotate writes `oos-issues-created.md` before GitHub priority-label calls.
- Pending marker `.oos-priority-label-pending` is written at label-phase entry when any high-risk OOS URL still needs `oos-correctness`.
- Label-only mode skips empty-stdout, missing-accepted, and missing-order sequencing errors. It requires the sentinel, combined OOS file, and `REPO`.
- Annotate failure emits `STEP5B_STATUS=annotate-failed`; `FILE_DESIGN_OOS_STATUS=annotate-failed-empty-stdout` uses the once-only empty-stdout retry sentinel before Step 5b.5. `.completed/step-5b` is also written when `oos-issue.stdout.txt` is present and non-empty for partial `/larch:issue` failures without label-retry state. `annotate-label-failed` returns non-zero, does not write `.completed/step-5b`, blocks Step 5b.5, and leaves label-only retry available even when `oos-issue.stdout.txt` is non-empty.
- Full normal annotate success and label-only retry success write `.completed/step-5b`.

## Harness

Covered by the inline tests in `crates/larch-cli/src/design_oos_commands.rs` and `make test-design-structure`.
