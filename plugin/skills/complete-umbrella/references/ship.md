# Ship Phase

**Consumer**: The fourth fresh general-purpose Agent spawned by the `/complete-umbrella` leaf orchestrator.

**Contract**: Route only through the deterministic leaf ship driver, spawn a bounded CI fixer only after an actual failed-check outcome, spawn a bounded conflict fixer only after a DIRTY-main handoff, and verify terminal shipping state.

**When to load**: **MANDATORY: READ ENTIRE FILE** only for the primary ship phase.

Read `phase-common.md` in this directory in full before acting.

Read `$SESSION_TMPDIR/review-summary.md`. Require its final HEAD to match the clean current branch. Do not read the issue bodies, design brief, implementation diff, or repository source.

Run the standalone driver in ship mode:

```bash
"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" complete-umbrella ship-leaf \
  --mode ship \
  --repository "<REPOSITORY>" \
  --repo-root "$PWD" \
  --handoff-root "$SESSION_TMPDIR" \
  --umbrella "<UMBRELLA>" \
  --leaf "<LEAF>"
```

The driver owns the deterministic sequence: rebase onto the latest `origin/main` when safe, push, create or verify a PR with the leaf closing link, refresh CI once every 300 seconds, distill a failed run, detect the default-branch merge queue after green checks, enqueue and wait when it is enabled or squash-merge with `--admin --delete-branch` otherwise, verify the merge, retitle the closed leaf `[DONE]`, switch to `main`, fetch and rebase `origin/main`, delete the feature branch, and verify every postcondition. When a CI-green PR is `DIRTY` against `main`, the driver either rebases cleanly itself or hands off `needs_conflict_fix` with `CONFLICT_FILES` while leaving the rebase in progress.

Route only on `SHIP_STATUS`:

- `complete`: run the same command with `--mode verify`. Require another `SHIP_STATUS=complete`.
- `ci_failed`: require `CI_ERRORS_FILE` to be a regular file below `$SESSION_TMPDIR`. Spawn one fresh general-purpose Agent with only the identifiers from your prompt, the positive fix round, `CI_ERRORS_FILE`, and `PHASE_CONTRACT=$CLAUDE_PLUGIN_ROOT/skills/complete-umbrella/references/ci-fix.md`. Await its task notification. Accept when `PHASE_STATUS=complete` is present in the returned text and `$SESSION_TMPDIR/ci-fix-round-<N>.md` exists as a regular file; ignore surrounding narration and cosmetic `HANDOFF_FILE` path slips. On a missing status or missing handoff file, re-spawn that fixer in a fresh context up to two additional times, then fail. Then rerun ship mode. The driver's persisted state enforces the fix-attempt cap.
- `needs_conflict_fix`: require the `CONFLICT_FILES` key from the driver stdout (may be empty). Spawn one fresh general-purpose Agent with only the identifiers from your prompt, the positive conflict round, `CONFLICT_FILES`, and `PHASE_CONTRACT=$CLAUDE_PLUGIN_ROOT/skills/complete-umbrella/references/conflict-fix.md`. Await its task notification. Accept when `PHASE_STATUS=complete` is present in the returned text and `$SESSION_TMPDIR/conflict-fix-round-<N>.md` exists as a regular file; ignore surrounding narration and cosmetic `HANDOFF_FILE` path slips. On a missing status or missing handoff file, re-spawn that fixer in a fresh context up to two additional times, then fail. Then rerun ship mode. The driver's persisted state enforces the conflict-fix attempt cap.
- Any other value or nonzero exit: fail. Do not repair deterministic shipping state by hand.

Do not poll while the driver runs. Do not spawn a CI fixer when checks are pending or green. Do not spawn a conflict fixer unless the driver returned `needs_conflict_fix`.

When you are a re-spawned ship attempt after an earlier interrupted or failed ship, resume from the durable handoff instead of restarting from scratch: the deterministic driver reads the persisted `complete-umbrella-ship.env` and re-enters `ship-leaf`, which pushes any local-only CI-fix commit on the leaf branch before refreshing CI. Do not discard the existing `complete-umbrella-ship.env`, `ci-fix-round-*.md`, or local branch commits. The orchestrator, not this phase, owns the bounded five-attempt ship-retry policy and the 180-second wait between attempts.

After verified completion, write `$SESSION_TMPDIR/ship-summary.md` with only the PR number, PR URL, final issue state, final local HEAD, and `SHIP_STATUS=complete`.

After verified completion, end with only:

```text
PHASE_STATUS=complete
HANDOFF_FILE=ship-summary.md
```
