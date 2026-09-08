# step-8-ship.sh

Step 8+ Rust ship-dispatch bgjob launcher. Foreground mode starts or rejoins bgjob step `implement-step8-ship`; child mode rehydrates durable ship argv from `$IMPLEMENT_TMPDIR/ship-pr-state.sh`, derives `EXPECTED_TMPDIR_BASENAME_PREFIX` through the shared Rust clone-tag helper, runs the advisory `8-pre-ship` phantom probe, and invokes Rust `ship pr` in process with the canonical argv.

## Caller

`skills/implement/SKILL.md` invokes this wrapper from Step 8+ as a foreground bgjob launcher. Pre-driver orchestration may run guard, initial seeding, and `${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh oos file` first; post-driver continuations invoke only this wrapper.

## Stdout and result-env contract

Foreground mode composes the shared Rust bgjob adapter for start-or-reattach, completed-result replacement, and merge-result publication. The child rehydrates the durable ship argv, runs the advisory phantom probe, and invokes the Rust lifecycle owner, which writes the adapter-provided merge-result env before forwarding the JSON contract.

The Rust child writes ship outcome KVs before it forwards the driver's human-readable JSON contract. The bgjob daemon merges those KVs with `BGJOB_RC` and `STEP` into `$IMPLEMENT_TMPDIR/bgjob/implement-step8-ship.result.env`; `ship route-exit` reads that one authoritative result env after bgjob `DONE`.

Result-env publication is fail-closed: a result write failure gives the bgjob no ship outcome to route, so `route-exit` stops rather than using stdout. Step 8 is persist-and-resume by design; `bgjob adapt` owns live-job reattachment and completed-result replacement.

## Invariants

- The Bash wrapper is a portable, single-exec compatibility surface.
- Rust rehydrates `CLAUDE_PLUGIN_ROOT` from `$IMPLEMENT_TMPDIR/plugin-root.env` where needed.
- Rust rehydrates `BRANCH_NAME`, `ISSUE_NUMBER`, `RUN_ID`, `REPO`, `MERGE`, `DRAFT`, `FORKED_TARGET`, `REPO_UNAVAILABLE`, `MANIFEST_PATH`, `TOOL_LABEL`, `NO_ADMIN_FALLBACK`, and `NO_LOGS_COMMIT` from `ship-pr-state.sh` before invoking the active driver.
- `EXPECTED_TMPDIR_BASENAME_PREFIX` comes from the shared Rust clone-tag helper; the ship dispatcher and initial state seeder share the same prefix.
- Rust `ship pr` composes the JSON result and owns result-env validation and private atomic publication.
- `bgjob adapt` refuses a second start when an identity-valid `implement-step8-ship` job is live and replaces only a completed result on a deliberate reship.
- Setup failures without a ship outcome do not reach `route-exit`; they use the existing bgjob failure branch.

## Edit-in-sync

Update `skills/implement/SKILL.md`, `step-8-seed-initial.md`, `skills/implement/references/ship-pr-exit-matrix.md`, the Rust black-box parity tests, and the shell-wrapper structure tests when this contract or argv changes.
