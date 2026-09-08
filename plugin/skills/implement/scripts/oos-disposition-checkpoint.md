# oos disposition-checkpoint

Rust owns `scripts/larch.sh oos disposition-checkpoint`. The command resolves one `/implement` session's disposition inputs, evaluates the Rust gate in process, and records refusals. `skills/implement/scripts/oos-disposition-checkpoint.sh` remains only a thin compatibility wrapper for direct path invocation and the delegation smoke.

## Ownership trace

The live Step 8 route is:

1. `skills/implement/scripts/step-8-oos-checkpoint.sh` enters the Rust workflow router through `scripts/larch.sh implement step-8-oos-checkpoint`.
2. `crates/larch-cli/src/implement_ship_commands.rs` invokes `scripts/larch.sh oos disposition-checkpoint`, preserves its refusal artifact, appends a de-duplicated Tool Failures fallback when needed, and, only after a zero command exit, owns `run-statistics.md`, the `steps_ran.step9a1` stamp, and allowlisted `OOS_PENDING=false` bookkeeping.
3. `crates/larch-cli/src/main.rs` dispatches the OOS verb to `crates/larch-cli/src/oos_commands.rs`.
4. The command composes `crates/larch-core/src/issue/oos_disposition.rs` and `crates/larch-core/src/issue/oos_record.rs` for gate state, evidence counters, and accepted-block grammar.

The Step 8 router and the lower-level OOS checkpoint remain distinct Rust workflow responsibilities. The OOS checkpoint calls the shared gate evaluator in process and does not spawn `oos-disposition-gate.sh` or a nested `oos disposition-gate` command.

## Invocation

```text
scripts/larch.sh oos disposition-checkpoint --implement-tmpdir DIR [--design-tmpdir DIR]
```

Legacy direct-wrapper invocation accepts the same arguments:

```text
oos-disposition-checkpoint.sh --implement-tmpdir DIR [--design-tmpdir DIR]
```

- `--implement-tmpdir` is required. It names the session directory containing state files, accepted-OOS Markdown, filing evidence, and `larch-logs/implement/`.
- `--design-tmpdir` is optional. The command also accepts exported `DESIGN_TMPDIR` when the flag is absent.

## Input resolution

`crates/larch-cli/src/oos_commands.rs` reads `ship-pr-state.sh` followed by `finalize-state.sh` with the shared legacy KV codec. The last value wins. `FORKED_TARGET=true` or `REPO_UNAVAILABLE=true` skips the disposition evaluation and the non-security batch precondition after session identity resolution. The carve-outs do not bypass ambiguous run-batch validation: no recorded identity plus multiple discoverable batches returns exit 2 before the skip.

The accepted design path resolves in this order:

1. `<design-tmpdir>/oos-accepted-design.md` when the flag or environment value names an existing file;
2. `$IMPLEMENT_TMPDIR/design-export/oos-accepted-design.md` when present;
3. `$IMPLEMENT_TMPDIR/oos-accepted-design.md`.

The other accepted inputs are `$IMPLEMENT_TMPDIR/oos-accepted-review.md` and `$IMPLEMENT_TMPDIR/oos-accepted-main-agent.md`.

The commit range comes from the Rust Git adapter. It uses `<merge-base>..HEAD` when `HEAD` and `origin/main` have a merge base, `origin/main..HEAD` when the base resolves without a merge base, `HEAD^..HEAD` when `origin/main` is absent but `HEAD^` resolves, and `HEAD` otherwise.

## NDJSON discovery

Run identity resolves from the last state-file `RUN_ID`, then non-empty `$IMPLEMENT_TMPDIR/session-id`, then the parent directory of exactly one discoverable `larch-logs/implement/*/oos-issues.ndjson` batch.

| Identity inputs | Batch resolution |
|-----------------|------------------|
| Recorded or session identity is non-empty | Use only `$IMPLEMENT_TMPDIR/larch-logs/implement/<RUN_ID>/oos-issues.ndjson`. No fallback may bind another run's batch. |
| No recorded or session identity, one batch | Derive its parent-directory name as the run identity and adopt that batch. |
| No recorded or session identity, multiple batches | Exit 2 rather than guessing which run owns disposition evidence. |
| No recorded or session identity, no batch | Continue with no NDJSON path; non-security accepted OOS then triggers the required-batch refusal. |

Outside fork and repo-unavailable carve-outs, a non-zero accepted non-security count requires the resolved NDJSON path to be a regular file. A recorded but stale run identity therefore cannot fall back to a foreign run's sole batch.

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | Gate cleared, or the fork / repo-unavailable carve-out skipped evaluation. |
| 1 | Accepted non-security OOS lacks filing, inline-triage, or rejection evidence. |
| 2 | Invalid arguments or session inputs, ambiguous or missing required NDJSON, unusable Git history, or malformed gate evidence. |
| 3 | Non-security disposition cleared, but a non-empty private security sidecar still requires disposition. |

The Rust #7681 Step 8 router translates any non-zero command exit into `OOS_CHECKPOINT_RC=<code>` plus `NEXT_ACTION=stall`. On zero it performs post-pass bookkeeping and emits `NEXT_ACTION=reship`. See `skills/implement/references/ship-pr-oos-checkpoint-router.md` for that router contract.

## Refusal artifacts

The Rust command writes its own bounded diagnostics and `execution-issues.md` Tool Failures entries:

- `$IMPLEMENT_TMPDIR/oos-disposition-checkpoint.stderr.log` records argument, session-resolution, missing-batch, and security-sidecar refusals.
- `$IMPLEMENT_TMPDIR/oos-disposition-gate.stderr.log` records gate-counter validation or an undisposed-counter failure line.
- `execution-issues.md` uses `step-8-oos-checkpoint`, `step-8-oos-checkpoint-validation`, or `step-8-oos-checkpoint-security-sidecar` according to the refusal.

An invalid command line with no recoverable `--implement-tmpdir` prints usage on process stderr. A parsed path that does not exist also reports on stderr. When an invalid line contains a path hint that cannot accept writes, exit 2 may be the only reliable refusal evidence; callers must not assume a session artifact was persisted.

## Test authority

Rust unit tests in `crates/larch-cli/src/oos_commands.rs` cover input resolution, carve-outs, gate refusal, run identity, design export, security-sidecar state, and logging. Core unit tests in `crates/larch-core/src/issue/oos_disposition.rs` and `crates/larch-core/src/issue/oos_record.rs` cover gate semantics and record parsing. `skills/implement/scripts/test-oos-disposition-gate.sh` covers only the two thin wrappers' plugin-root selection, root entrypoint, arguments, exit status, stdout, and stderr.

```text
cargo test --locked --package larch-cli --bin larch oos_commands::tests
cargo test --locked --package larch-core --lib issue::oos_disposition::tests
cargo test --locked --package larch-core --lib issue::oos_record::tests
make oos-disposition-gate-bash-harness
```
