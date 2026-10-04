# step-8-seed-initial.sh

Thin wrapper for the Rust-owned, create-if-absent initial `/implement` `ship-pr-state.sh` seed.

## Caller

The pre-driver Step 8 path in `skills/implement/SKILL.md` calls this wrapper with one physical Bash fence. The Step 5 `stall` missing-state path in `skills/implement/references/step5-review-branches.md` calls the same wrapper with stall flags.

## Inputs

The Rust command requires `IMPLEMENT_TMPDIR`, rehydrates `CLAUDE_PLUGIN_ROOT` from `plugin-root.env`, derives `EXPECTED_TMPDIR_BASENAME_PREFIX` with the shared Rust clone-tag helper, and reads durable inputs from `$IMPLEMENT_TMPDIR/bootstrap-routing.env`, `$IMPLEMENT_TMPDIR/ship-seed-input.env`, `$IMPLEMENT_TMPDIR/session-env.sh`, `$IMPLEMENT_TMPDIR/parent-issue.md`, and `$IMPLEMENT_TMPDIR/session-id`.

Source order is fixed:

- `BRANCH_NAME`: `bootstrap-routing.env`, then `parent-issue.md` sentinel, then empty.
- `ISSUE_NUMBER`: `bootstrap-routing.env`, then `parent-issue.md`, then empty.
- `RUN_ID`: `bootstrap-routing.env`, then `session-env.sh` `LARCH_RUN_ID`, then `parent-issue.md`, then empty.
- `REPO`: `bootstrap-routing.env`, then `session-env.sh` `REPO`, then empty.
- `REPO_UNAVAILABLE`: `bootstrap-routing.env`, then `session-env.sh`, then `false`.
- `FORKED_TARGET`: `ship-seed-input.env`, then `session-env.sh`, then `false`.
- `DEFERRED`: `bootstrap-routing.env`, then `ship-seed-input.env`, then `false`.
- `MERGE` and `DRAFT`: command flags, then `ship-seed-input.env`, then `false`.
- `NO_ADMIN_FALLBACK` and `NO_LOGS_COMMIT`: command flags, then `ship-seed-input.env`, then `false`.
- `MANIFEST_PATH`: command flag, then `ship-seed-input.env`, then empty.
- `TOOL_LABEL`: command flag, then `ship-seed-input.env`, then mapped `bootstrap-routing.env` `coder` (`codex` to `Codex`, `cursor` to `Cursor`, anything else to `claude`), then `claude`.
- `EXPECTED_SESSION_ID`: `$IMPLEMENT_TMPDIR/session-id`, then empty.

Do not invoke the retired session-env reader. Use the shared Rust legacy-KV and session-rehydration helpers when adding inputs.

## Create-if-absent gate

If `$IMPLEMENT_TMPDIR/ship-pr-state.sh` exists, is non-empty, and contains a `KEY=value` line, the Rust command exits non-zero before writing. Initial seeding must not overwrite driver-progressed state.

## Stall flags

The Step 5 missing-state path may pass `--stall-tracking`, `--stall-step`, `--bail-reason`, `--bail-failure-detail-log`, `--merge false`, and `--draft false`. The Rust state builder forces `DRAFT=false` whenever `--stall-step` is non-empty and preserves the requested `MERGE` value.

## Delegation

The wrapper enters `scripts/larch.sh implement step-8-seed-initial`. Rust assembles the canonical request, including `--expected-tmpdir-basename-prefix`, then invokes the Rust `ship seed-initial-state` owner in process.

Do not put multi-line seeder examples, line continuations, inline `if`, or retired Python seeder invocations in `SKILL.md`. Argv assembly lives here.

`ship-seed-input.env` is written by Step 0 and extended after Step 2 dispatch with manifest/tool context.
