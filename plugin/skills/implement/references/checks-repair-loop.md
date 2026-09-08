**Consumer**: `/implement` checks-failure orchestrator at folded Step 3, Step 5 self-review, Step 5 MAV, Step 5 coder-main-agent-required, and Step 6 sites.
**Contract**: normative `checks repair-loop` invocation, stdout KV parse-and-branch rules (`NEXT_ACTION`, optional tail and ledger keys), structural main-agent-edit re-entry, and delegated-waterfall stall routing.
**When to load**: **MANDATORY: READ ENTIRE FILE** before handling `STATUS=fail` at any of those sites; do not invoke `checks repair-loop` or branch on repair outcomes without loading this file first.

## 1. Structural gate, all sites

On `STATUS=fail`, or folded-site composite `NEXT_ACTION=checks-failed`, first check for `FAILURE_REASON`.

Structural reasons include `tmpdir-validation`, `site-validation`, `repo-root-unresolved`, `check-script-not-executable`, `check-script-symlink-broken`, and `redaction-failed`.

Act on the reason.
Do not invoke repair-loop when no `REDACTED_LOG_FILE` exists.
At folded sites, key-scan the full composite stdout for both `DIGEST_FILE` and `REDACTED_LOG_FILE`, not only the first physical composite line. Do not Read either file on this path. Reserve `REDACTED_LOG_FILE` for repair-loop input and later bounded evidence materialization.
Before skipping to Step 18 on this no-log path, whitespace-token-scan the first physical line of captured composite stdout for `EXIT_CODE`, `FAILURE_REASON`, and `PHASE`. Mirror `FAILURE_REASON` into `IMPLEMENT_BAIL_REASON` and `FINAL_BAIL_REASON`. Set `STALL_STEP` from the pinned site (`3` for `--site step3`, `6` for `--site step6`, `5` for Step 5 self-review). Default `PHASE` to `checks` when the composite line omits it.
Then route to the default stall semantics in section 4: set `STALL_TRACKING=true`, skip to Step 18, and do not proceed on the site success path.

## 2. Repair-loop invocation

When `REDACTED_LOG_FILE` is present, launch the repair-loop as a bgjob (not a bare foreground command). Foreground `checks repair-loop` can exceed the Bash tool's 600 s ceiling when a lint-fix lane runs up to `FIXER_LANE_TIMEOUT_SEC` (1800 s); the bgjob path keeps the launcher return under that ceiling and preserves the `NEXT_ACTION` envelope in the result env.

```bash
"$HOME/.cache/larch/sessions/implement-run-$PPID.sh" scripts/larch.sh checks repair-loop --bgjob-launch true --tmpdir "$IMPLEMENT_TMPDIR" --site <lint-site> [--checks-site <capture-site>] --checks-log "$REDACTED_LOG_FILE"
```

Launcher stdout is `BGJOB_STATUS=STARTED STEP=implement-<lint-site>-repair PGID=<n>` (site-qualified slug, for example `implement-step3-repair` or `implement-step6-repair`). Then wait:

```bash
"$HOME/.cache/larch/sessions/implement-run-$PPID.sh" scripts/larch.sh bgjob wait --step implement-<lint-site>-repair --tmpdir "$IMPLEMENT_TMPDIR" --max-wait-s 270
```

On `BGJOB_STATUS=WAIT`, repeat the identical wait with no intervening prose or tools. On `DEAD`, route through the site stall path. On final `DONE`, parse section 3 keys from the wait stdout and/or `$IMPLEMENT_TMPDIR/bgjob/implement-<lint-site>-repair.result.env`. Require `BGJOB_RC=0` for `NEXT_ACTION=continue` or `NEXT_ACTION=main-agent-edit`; `NEXT_ACTION=stall` may arrive with a non-zero child rc and still routes per section 4. Do not treat launcher stdout, shell exit 0, or `DONE` alone as success. Ship-pr internal CI repair-loop call sites keep bare foreground invocation (no `--bgjob-launch`); they already run inside a longer-lived ship bgjob.

Bind and reuse the pinned site pair for every invocation in section 4, including post-main-agent re-entries:

- Step 3: `--site step3`. The folded composite launcher is `skills/implement/scripts/run-step-checks.sh --site step3 --commit-site step4 --rebase-checkpoint-4r --forked-target "${forked_target:-false}"`, followed by `scripts/larch.sh bgjob wait --step implement-step3-checks --tmpdir "$IMPLEMENT_TMPDIR" --max-wait-s 270` until `DONE`. Repair-loop step slug: `implement-step3-repair`.
- Step 5 self-review: `--site step5-self-review`. The folded composite launcher is `scripts/larch.sh implement checks-commit-route --checks-site step5-self-review --commit-site step5-self-review`. Repair-loop step slug: `implement-step5-self-review-repair`.
- Step 5 MAV and coder-main-agent-required: `--site step5-mav --checks-site step5-review-fixes`. The folded composite launcher is `scripts/larch.sh implement checks-step5-resume --checks-site step5-review-fixes --final-round-num "$FINAL_ROUND_NUM"`. Repair-loop follows the lint-fix site, not the capture site. **Never** omit `--checks-site` on re-entry. Defaulting would run internal re-checks under `step5-mav` instead of `step5-review-fixes`. This text is already shaped for the Step 5 bgjob chunk: that chunk must make the wrapper a thin bgjob launcher, truncate its merge env before start, and gate resume continuation on `BGJOB_RC=0` plus `STEP5_REVIEW_STATUS` in the result env. Repair-loop step slug: `implement-step5-mav-repair`.
- Step 6: `--site step6`. The initial orchestrator folded composite launcher is `skills/implement/scripts/step-6-entry.sh --forked-target "${forked_target:-false}"` with the change gate active, followed by `scripts/larch.sh bgjob wait --step implement-step6-checks --tmpdir "$IMPLEMENT_TMPDIR" --max-wait-s 270` until `DONE`. All Step 6 post-repair re-entries, including `NEXT_ACTION=continue` and `NEXT_ACTION=main-agent-edit`, use `skills/implement/scripts/step-6-entry.sh --forked-target "${forked_target:-false}" --force-checks true` plus the same `implement-step6-checks` bgjob wait, so the checks leg always re-runs even when repair leaves the tree matching pre-review baselines. Step 6 repair re-entry must not use the bare `checks-commit-route` launcher and must not omit `--force-checks true`. Repair-loop step slug: `implement-step6-repair`.

## 3. Parse stdout before branching on exit code

Use key-based extraction for these keys before checking the Bash exit code. After a bgjob-backed repair-loop, read the same keys from the final `DONE` wait stdout and/or `$IMPLEMENT_TMPDIR/bgjob/implement-<lint-site>-repair.result.env`:

- `NEXT_ACTION`
- `STDERR_TAIL_PATH`
- `CODER_LOG_FILE`
- All `LINT_FIX_LEDGER_*` keys when present
- `FAILURE_REASON`
- `LINT_FIX_TIER_LEDGER_PATH`

Exit-code contract:

- Exit `0` with `NEXT_ACTION=continue` or `NEXT_ACTION=main-agent-edit` is success.
- Exit `1` with `NEXT_ACTION=stall` is the normal terminal stall path. Parse KVs from captured stdout (or the bgjob result env) and route to stall. Do not treat non-zero exit alone as an orchestrator hard failure before KV parse.
- Exit `2` for argument or validation failure still prints `NEXT_ACTION=stall`. Parse KVs first, then route to stall.
- For bgjob launches: gate on `BGJOB_RC` plus the parsed `NEXT_ACTION` as in section 2; do not use the launcher shell exit alone.
## 4. Branch semantics

### `NEXT_ACTION=stall` after delegated lint-fix exhaustion

For pre-ship sites, these exact terminal reasons are ordinary delegated-waterfall exhaustion:

- `lint-fix-no-selectable-tier`
- `lint-fix-budget-exhausted`

Require both `FAILURE_REASON` and `LINT_FIX_TIER_LEDGER_PATH` as terminal evidence. Route to the normal stall path. Do not edit inline and do not reinterpret `LOOP_STATUS=exhausted` or `LOOP_STATUS=no-changes-stale` as `NEXT_ACTION=main-agent-edit`. The ship-pr CI sites keep their existing internal lint-fix handoff and are outside this pre-ship remapping.

For `lint-fix-all-tiers-no-useful-delta`, route to `NEXT_ACTION=main-agent-edit`. Require the emitted `LINT_FIX_LEDGER_*` fields and record the escalation before handing the failure to the main agent. The ship-pr CI sites keep their existing internal lint-fix handoff and are outside this pre-ship remapping.

### `NEXT_ACTION=continue`

Use this site split as the sole normative rule.

- Folded sites (Step 3, Step 5 self-review, Step 5 MAV/coder, Step 6): re-run the section 2-pinned composite launcher with identical argv before any success-path routing. For bgjob-migrated Step 3 and Step 6, truncate/recreate the launcher merge env through the wrapper, then repeat `bgjob wait` until `DONE`; continue only when `BGJOB_RC=0` and required composite KVs are present in the final `DONE` stdout and/or `$IMPLEMENT_TMPDIR/bgjob/<step>.result.env`; when `CHECKPOINT_NEXT=load-routing` is present, allow the non-zero child rc and route through the rebase macro before judging the probe as failed. For Step 6 only, identical argv means the post-repair re-entry launcher `skills/implement/scripts/step-6-entry.sh --forked-target "${forked_target:-false}" --force-checks true`, not the initial orchestrator argv without `--force-checks true`.

### `NEXT_ACTION=main-agent-edit`

When `LINT_FIX_LEDGER_READY=true`, record one escalation before the ci-fixer handoff. Pass the parsed `LINT_FIX_LEDGER_*` fields from section 3 verbatim; do not invent site/trigger tokens. Pass the literal absolute `IMPLEMENT_TMPDIR` value parsed from Step 0 bootstrap output; a prompt-side Bash call cannot rely on a shell variable set in an earlier call. See **Escalation recording owners** in `${CLAUDE_PLUGIN_ROOT}/skills/implement/SKILL.md` and `${CLAUDE_PLUGIN_ROOT}/skills/implement/references/stall-recovery.md` for ownership and dedup rules.

```bash
"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" stall-recovery record-escalation --implement-tmpdir "<literal IMPLEMENT_TMPDIR from Step 0 bootstrap>" --site "$LINT_FIX_LEDGER_SITE" --trigger "$LINT_FIX_LEDGER_TRIGGER" --step "$LINT_FIX_LEDGER_STEP" --phase "$LINT_FIX_LEDGER_PHASE" --dispatcher "$LINT_FIX_LEDGER_DISPATCHER" --exit-code "$LINT_FIX_LEDGER_EXIT_CODE" --failure-detail-log "$LINT_FIX_LEDGER_FAILURE_DETAIL_LOG"
```

Stable lint-fix site/trigger tokens come from repair-loop stdout (for example `step3` / `main-agent-required`, `step5-self-review` / `main-agent-required`, `step5-mav` / `main-agent-required`, `step6` / `main-agent-required`). Use the parsed values, not the capture-site label.

This branch is a ci-fixer subagent handoff. It is never a main-agent repair path. The main agent does not Read `DIGEST_FILE`, `LINT_FIX_LEDGER_FAILURE_DETAIL_LOG`, `STDERR_TAIL_PATH`, or `CODER_LOG_FILE`, and it does not Edit/Write repository files.

Keep `$IMPLEMENT_TMPDIR/checks-fix-round-<site>.count`, starting at 1 and capped at 10. Exhaustion follows the existing terminal stall path. Bind `CHECKS_FIX_SITE` to the parsed `LINT_FIX_LEDGER_SITE` when present, otherwise the pinned lint site. For each allowed round, use the parsed `LINT_FIX_LEDGER_FAILURE_DETAIL_LOG` as the source when it is present; otherwise use the current `REDACTED_LOG_FILE`. Materialize the bounded, redacted evidence file before spawning the fixer. The command verifies session containment and does not expose the evidence to the main agent:

```bash
"$HOME/.cache/larch/sessions/implement-run-$PPID.sh" scripts/larch.sh checks fixer-evidence --tmpdir "$IMPLEMENT_TMPDIR" --site "$CHECKS_FIX_SITE" --round "$CHECKS_FIX_ROUND" --checks-log "$CHECKS_FIX_SOURCE_LOG"
```

Require `CHECKS_FIXER_EVIDENCE_STATUS=ok` and bind `CHECKS_FIXER_EVIDENCE_FILE`. A failed or malformed evidence envelope follows the existing terminal stall path; do not substitute a raw log, tail, or prompt-side diagnosis.

Spawn `larch:ci-fixer` with only: `REPO_ROOT`, `BRANCH_NAME`, `MODE=checks`, the lint site token, `CHECKS_FIXER_EVIDENCE_FILE`, `$IMPLEMENT_TMPDIR/checks-fixer-rounds.md`, and `CHECKS_FIX_ROUND`. No evidence contents are inlined. Record `MODE=subagent` and `TIER=subagent` using the existing subagent-attribution shape. Parse only the three trailing `FIXER_*` lines.

- `FIXER_RESULT=committed`: require a full `FIXER_COMMIT` SHA, append `FIXER_SUMMARY` to the rounds file, and re-run the section 2-pinned composite launcher.
- `FIXER_RESULT=no-progress`, `FIXER_RESULT=bail`, or a malformed trailer: use the existing terminal stall path.
- After a fixer return or death, inspect only `git status --porcelain`. If the tree is dirty, salvage one `CI fix round <N> salvage` commit. In `MODE=checks`, never push the salvage commit. Do not discard fixer work.

For Step 6 after a checks-fixer commit, re-run `skills/implement/scripts/step-6-entry.sh --forked-target "${forked_target:-false}" --force-checks true`, wait on `implement-step6-checks` until `DONE`, and require `BGJOB_RC=0` plus required KVs before any success-path routing or subsequent `checks repair-loop` invocation; when `CHECKPOINT_NEXT=load-routing` is present, allow the non-zero child rc and route through the rebase macro before treating the probe as failed. Do not reuse the initial orchestrator argv without `--force-checks true`.
On `STATUS=fail` or composite `NEXT_ACTION=checks-failed` with `REDACTED_LOG_FILE`, re-invoke `checks repair-loop --bgjob-launch true` with the same pinned `--site` and optional `--checks-site` pair from section 2 for this call site and the updated `--checks-log`, then repeat the section 2 `bgjob wait` for `implement-<lint-site>-repair`.
Keep the updated `REDACTED_LOG_FILE` as the repair-loop input; do not Read a new `DIGEST_FILE` on re-entry.
Do not pass only `--checks-log`.
Step 5 MAV and coder must repeat `--site step5-mav --checks-site step5-review-fixes`.
Repeat until repair-loop `NEXT_ACTION` is `continue`, `main-agent-edit`, or `stall`; `continue` still means re-run the same composite launcher before success routing. On bgjob `WAIT`, run the identical wait again with no intervening prose or tools. On Step 6, both `continue` and `main-agent-edit` repair paths must use `skills/implement/scripts/step-6-entry.sh --forked-target "${forked_target:-false}" --force-checks true`; never re-enter Step 6 repair via bare `checks-commit-route`.
Preserve the structural `FAILURE_REASON` handling in section 1 on each re-entry.

### `NEXT_ACTION=stall`

Before skipping to Step 18, bind `EXIT_CODE`, `FAILURE_REASON` (into `IMPLEMENT_BAIL_REASON` and `FINAL_BAIL_REASON`), `STALL_STEP`, and `PHASE` from captured composite stdout when those prompt-side values are not already set. Whitespace-token-scan the first physical line the same way as section 1.

Use the default implement contract: set `STALL_TRACKING=true` and skip to Step 18.
Applies to Step 3, Step 6, and Step 5 self-review only.
Stall recovery runs before the final report.
Step-local SKILL deltas may override this path.
Those overrides apply only at their sites.

Step 5 self-review has no override beyond the default stall routing.
Step 5 MAV and coder-main-agent-required terminal checks stalls are routing summaries at the repair-loop site. Do **not** skip to Step 18 at this site. Continue to the main-agent handoff paragraph in `${CLAUDE_PLUGIN_ROOT}/skills/implement/SKILL.md`; that paragraph performs `--record-only` timing capture, then applies the **Durable Bail** body in `${CLAUDE_PLUGIN_ROOT}/skills/implement/references/step5-review-branches.md` before skipping to Step 18. Do not invoke `step-5-resume.sh --final-round-num "$FINAL_ROUND_NUM" --record-only` or durable-seed inline here. Do not re-invoke the Step 5 loop wrapper.

## 5. In-step contract

The failure path is in-step.
It is not a halt.
Do not end the turn, summarize, or write a handoff message.

## 6. Self-edit attribution (never halt on your own subprocess's edit)

The `checks repair-loop` lint-fix tiers and the Step 3 pre-commit ruff autofix edit tracked files in place. Every path they change is recorded to `$IMPLEMENT_TMPDIR/self-edit-log.tsv` (one row per path: recorded epoch seconds, source, path, post-edit sha256).

Before concluding that a tracked file which changed between two of your own actions was touched by a concurrent or external runner — and before halting or asking the operator about a parallel session on that basis — consult the log:

```bash
"$HOME/.cache/larch/sessions/implement-run-$PPID.sh" scripts/larch.sh checks self-edit-log --tmpdir "$IMPLEMENT_TMPDIR" --path <changed-path> --repo-root "$(git rev-parse --show-toplevel)"
```

- `SELF_EDIT_ATTRIBUTED=true`: one of this run's own spawned subprocesses changed that path. Do not treat it as an external edit and do not halt for a concurrent runner on that path.
- `SELF_EDIT_CONTENT_MATCHES=true`: the file's current content is exactly what your subprocess produced, confirming nothing has changed it since.
- `SELF_EDIT_ATTRIBUTED=false`: the path is absent from the log; only then may you treat the change as external.

`ps` and `stat` mtime cannot attribute an edit once the spawning subprocess has exited, so the log is the authority. Omit `--path` to dump every recorded self-edit (`SELF_EDIT_COUNT` plus one `SELF_EDIT` line per row).
