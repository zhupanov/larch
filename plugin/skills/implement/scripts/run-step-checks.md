# run-step-checks.sh

Captured relevant-checks wrapper and Step 3 checks/commit composite bgjob launcher.
The wrapper resolves persisted `REPO_ROOT`, computes a checks-input identity (`HEAD` + worktree fingerprint covering staged, unstaged, and untracked content), re-joins a live identity-valid bgjob row or a validated-complete canonical result env when present, clears stale canonical result env state before fresh launch, seeds the merge-result env with that identity, and launches a bgjob whose foreground stdout is exactly `BGJOB_STATUS=STARTED STEP=<name> PGID=<n>`.

## Caller

`skills/implement/SKILL.md` invokes this wrapper for active Step 3 with `--site step3 --commit-site step4 --rebase-checkpoint-4r`. Step 5 self-review uses `--site step5-self-review --commit-site step5-self-review`. Legacy helper-only call sites may still pass only `--site SITE` to launch `scripts/larch.sh checks run-relevant` through the same bgjob transport.

## Identity contract

Persisted identity fields (owned by `crates/larch-core/src/implement/identity.rs`):

- `CHECKS_INPUT_HEAD_SHA`
- `CHECKS_INPUT_TREE_FP`
- `CHECKS_INPUT_FP_SCHEMA` (`v1`)

Fingerprint coverage includes binary staged and unstaged diffs plus untracked path and content hashes. Matching failed results (`NEXT_ACTION=checks-failed`) may rejoin when the identity still matches. Any committed, staged, unstaged, or untracked drift forces a fresh checks run. Arbitrary non-empty `NEXT_ACTION` values are not treated as complete.

Live-row mismatch fails closed: the wrapper emits an error and exits without deleting active state or launching a duplicate. Stale completed results are cleared (result + merge env) after cleanup verification, then a fresh launch proceeds.

Child mode receives the validated `REPO_ROOT` and immutable launch identity, executes with cwd and `CLAUDE_PROJECT_DIR` bound to that root, revalidates identity immediately before checks and again before publishing the terminal merge envelope, and publishes a non-reusable `NEXT_ACTION=identity-integrity-failed` envelope on drift.

Shared classifier: `scripts/larch.sh implement checks-result-identity`.

## KV grammar

The bgjob child writes helper stdout plus verified identity fields into the merge-result env. After `scripts/larch.sh bgjob wait` returns `DONE`, the orchestrator reads `$IMPLEMENT_TMPDIR/bgjob/<step>.result.env` and gates continuation on `BGJOB_RC=0` plus the required site KVs. A live registry row or identity-valid completed result env reuses `bgjob wait`; stale completed result envs are cleared before a fresh `bgjob start`.

## Invariants

- Bash 3.2 portable; no associative arrays or namerefs.
- Self-rehydrates `CLAUDE_PLUGIN_ROOT` from `$IMPLEMENT_TMPDIR/plugin-root.env` where needed.
- Session telemetry key names live in `skills/shared/session-setup-output.md`; check wrappers consume the `$IMPLEMENT_TMPDIR/session-env.sh` copy.
- When `--site step3`, uses bgjob step slug `implement-step3-checks`, uses bgjob result-env completion.
- Rejoins `bgjob wait` for live registry rows or identity-valid completed result envs before starting a fresh composite run.
- The wrapper does not write legacy wait markers; bgjob owns completion state.

## Edit-in-sync

Update `skills/implement/SKILL.md` and the implement structure/timing harnesses when this contract or argv changes.
