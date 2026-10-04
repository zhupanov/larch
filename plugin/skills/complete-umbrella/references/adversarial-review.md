# Adversarial Review Phase

**Consumer**: The third fresh general-purpose Agent spawned by the `/complete-umbrella` leaf orchestrator.

**Contract**: Independently review the implementation against the brief, repair every in-scope finding, prove stale-caller and parity-success coverage, and persist final review handoffs.

**When to load**: **MANDATORY: READ ENTIRE FILE** only for the primary adversarial-review phase.

Read `phase-common.md` in this directory in full before acting.

Start from only `$SESSION_TMPDIR/design-brief.md`, `$SESSION_TMPDIR/implementation.diff`, and `$SESSION_TMPDIR/plan.md`. Do not read the issue bodies or the prior phase summary. This is an independent review, not a continuation of the implementer's reasoning.

Review the diff against every brief requirement. Check correctness, recovery paths, trust boundaries, architecture, tests, and companion artifacts. Inspect exact changed files when needed.

For every entrypoint, command, symbol, or file removed or renamed by the diff, run a repository-wide stale-caller sweep with the `Grep` tool. Classify every match as migrated, intentionally retained, generated, fixture-only, or stale. Fix every stale production caller.

For every differential or parity harness in scope, verify that it asserts a real success path executed. An authorization-refusal-only comparison is not parity evidence. Add the assertion or test when missing, then run the focused success case.

Apply every in-scope fix you find. Run affected checks. Commit review fixes in one commit when the diff changed. Require a clean worktree.

After the final commit, run the managed-leaf line-budget read:

```bash
"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" complete-umbrella ship-leaf \
  --mode line-budget \
  --repository "<REPOSITORY>" \
  --repo-root "$PWD" \
  --handoff-root "$SESSION_TMPDIR" \
  --umbrella "<UMBRELLA>" \
  --leaf "<LEAF>"
```

Require one of these outcomes:

- `RUST_LINE_BUDGET_STATUS=not-managed` or `within-limit`: record the checked
  count in the review summary and continue.
- `RUST_LINE_BUDGET_STATUS=over-limit`: record a visible warning in the review
  summary with the leaf, count, limit, base SHA, and head SHA, then continue.
  This is the automatic continue-with-warning path; do not edit or publish the
  issue plan for the advisory. The ship driver independently remeasures and
  emits a second warning with the PR number immediately before queue submission
  or direct admin merge.

Any other output or nonzero exit is a hard stop.

Regenerate `$SESSION_TMPDIR/implementation.diff` from the final `git diff main...HEAD`. Write `$SESSION_TMPDIR/review-summary.md` with findings, fixes, stale-caller results, parity-success evidence, final HEAD, Rust line-budget status/count, and checks. Keep it below 2,000 tokens.

End with only:

```text
PHASE_STATUS=complete
HANDOFF_FILE=review-summary.md
```
