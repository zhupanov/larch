# Migration recovery

Use the owner-held migration directory and its verified private backups. Keep
the source snapshot, source Git mirror, adaptation bundle, mapping ledger,
configuration plan, and continuation manifest together. Verify their recorded
SHA-256 digests before using a restored copy.

## Interrupted import

Stop the recorded worker and verify its process identity before replacing it.
Keep Actions disabled until historical import, audit, archive retention, and
synthetic-ref cleanup finish. Preserve the destination fork and planning issues.

Resume the failed stage from its checkpoint. Reconcile an uncertain GitHub
write against its stable provenance marker before retrying. Read back the
actual destination number; never derive it from a numeric offset. A changed
destination body or an unknown branch owner requires reconciliation before
another write. Do not rerun `fetch` over the original snapshot, run `increment`,
or reset the ledger to bypass a failure.

Creation responses with HTTP 500, 502, 503, or 504 trigger bounded recovery.
The importer checks the pending record for a remote success. If it is absent,
it allows 90 seconds for GitHub to settle and checks again before retrying.
Each invocation permits at most three POST attempts. Persistent failures,
unreadable recovery data, duplicate markers, and changed bodies stop the stage.
Authentication, validation, and uncertain transport failures require operator
recovery. Keep the pending checkpoint when investigating a stopped stage.

The bulk import stages skip completed per-record checkpoints on restart.
They check saved identities locally and resume unfinished work without remote
rereads of completed records. Uncertain writes and unfinished body restoration
still receive remote checks. Independent full audits run before the historical
leaves are completed.

Synthetic branches may be removed only when their recorded commits still
match and every associated historical PR is verified closed and unmerged.
Retain the source Git mirror, archived diffs, and original GitHub metadata
after cleanup. Re-run the independent audit after cleanup and verify a
download of the retained archive before completing leaf #5.

## Configuration or code failure

The settings inventory records each source value, destination value, and
approved disposition. Repair only the failing setting, checking for later
owner edits before writing. Leave the migration leaf open until its complete
read-back comparison passes.

Canonical identity and release-version changes use ordinary protected PRs.
Keep required checks and branch protection in place when repairing failed CI.
A code rollback uses a reviewed revert PR with the same checks. Do not
force-push `main` or merge historical PR copies as a recovery shortcut.

## Release or installation failure

Retain the existing candidate, tag, and draft while repairing a failed asset
build. Resume the release helpers for that exact version and merged source
commit. A published immutable release cannot be repaired by replacing its
assets. Resume verification, Latest promotion, and the `stable` pin for the
same release when publication succeeded only in part.

Follow the checked-in
[projection release recovery runbook](../../.claude/skills/release/references/first-projection-release-runbook.md)
for the exact stage and finish contracts. Preserve the previous installation
and project-scoped pins during a user upgrade. Record restart requirements
even when a later installation check fails.

## Final cutover

Compare fresh source data with the retained snapshot before completing #8.
Record and reconcile every content delta through the verified map. Preserve
the original exports. Source archive, rename, deletion, or other source writes
remain a separate owner decision.

Recreate source PR 9076 only after #2 through #8 are verified complete. Preserve
its original commits and leave its destination PR open. Recovery of this last
step uses its own creation marker and comment map; it never merges or closes
that PR. Complete the umbrella only after the final mapping, discussion,
references, CI status, and private retention proof are verified.
