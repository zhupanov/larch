# GitHub repository migration

The migration from `character-ai/larch` to `zhupanov/larch` is tracked in
[umbrella #1](https://github.com/zhupanov/larch/issues/1). The existing fork,
planning issues, and Git history are retained. Historical PRs become attributed,
closed copies without merging their code. Source PR 9076 is recreated as an
open destination PR only after the other migration leaves finish.

[github-migration.patch](github-migration.patch) contains the reviewed adaptation
of [NicholasBoll/github-migration](https://github.com/NicholasBoll/github-migration)
at commit `241c203953fb516609b16f454098daa1b48d7d73`.
[tooling.json](tooling.json) records the patch digest and retained adaptation
commit. [configuration-dispositions.json](configuration-dispositions.json)
records the reviewed settings plan, including personal-account differences.
Its dispositions describe intended settings; issue #2 records application and
normal-CI verification.

The patch includes the inventory and replay commands, resume ledger, independent
history audit, reference repair, native relationship restoration, and focused
tests. It also includes `LARCH_MIGRATION.md`, the operator runbook. Keep the
operator checkout and raw data outside the shipped plugin tree:

```sh
MIGRATION_ROOT=$(mktemp -d "${TMPDIR:-/tmp}/larch-github-migration.XXXXXX")
git clone https://github.com/NicholasBoll/github-migration.git \
  "$MIGRATION_ROOT/github-migration"
git -C "$MIGRATION_ROOT/github-migration" checkout \
  241c203953fb516609b16f454098daa1b48d7d73
git -C "$MIGRATION_ROOT/github-migration" apply \
  "$PWD/docs/repository-migration/github-migration.patch"
npm --prefix "$MIGRATION_ROOT/github-migration" ci \
  --ignore-scripts --no-audit --no-fund
```

Restore the owner-held baseline, source export, checkpoints, and verified
rehearsal reports before resuming a stage. Credentials come from authenticated
GitHub CLI state or the documented token environment variables. No credential
values belong in these committed artifacts.

Actions remains disabled through historical replay, reference repair, archive
verification, and synthetic-ref cleanup. Configuration replay checks the exact
audited archive and completed leaves #4 and #5 before restoring Actions. The
personal repository uses required checks, up-to-date branches, and protected
squash merges. It has no organization merge queue.

The release policy copies only the latest upstream release as an attributed
baseline, then publishes a new destination-built release through the checked-in
release procedure. The copied assets retain upstream build provenance. Install
or upgrade from the subsequent destination-built release, whose attestations
name `zhupanov/larch`.

Raw GitHub exports, historical review context, source Git objects, mapping
checkpoints, and detailed access inventories remain in owner-controlled private
storage. The retained archive has bidirectional object and relationship indexes,
per-record coverage, and checksums. Its retention report records the private
object location and a verified download. This public directory contains the
operator code and sanitized configuration decisions.

Before cutover, reconcile a fresh source watermark and complete clean installation,
upgrade, contributor, maintainer, and recovery checks. Issue and PR mutations use
direct GitHub CLI or API operations for this migration. Source archive, rename,
deletion, or other source mutations need separate authorization.

Use the [recovery procedure](recovery.md) for interrupted stages, configuration
or release failures, and the final open-PR handoff.
