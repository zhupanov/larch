# /implement Step 5 self-review

**Consumer**: Step 5 when `self_review=true`.
**Contract**: Authoritative body for Claude-subagent self-review (`larch:claude-self-reviewer`).
**When to load**: **MANDATORY: READ ENTIRE FILE** when `self_review=true` or `STEP5_REVIEW_STATUS=self-review-required`.

Entry conditions: this reference is used for explicit `--self-review` and runtime zero-survivor fallback when `STEP5_REVIEW_STATUS=self-review-required`. The same artifacts remain authoritative: `$IMPLEMENT_TMPDIR/self-review-accepted.md`, `$IMPLEMENT_TMPDIR/rejected-findings.md`, `$IMPLEMENT_TMPDIR/oos-accepted-main-agent.md`, self-review tally, and the checks-commit route. The `larch:claude-self-reviewer` subagent writes the accepted/rejected/OOS artifact files; this reference owns dispatch, the composite route, and tally reconciliation.

When `self_review=true` (or after `self-review-required` fallback), self-review runs via a Claude Agent-tool subagent. First, mark Step 5 telemetry best-effort, then print the Step 5 banner.

```bash
"$HOME/.cache/larch/sessions/implement-run-$PPID.sh" scripts/larch.sh timing telemetry-mark --implement-tmpdir "$IMPLEMENT_TMPDIR" --label "Step 5: code review" || true
```

Print `> **🔶 /implement 5: code review: self-review mode (Claude subagent)**` after the telemetry mark returns.

1. Spawn one Agent-tool subagent with `subagent_type` `larch:claude-self-reviewer`. Prompt contains only: the repository root, the working branch `$BRANCH_NAME`, `forked_target` true|false, plan path `$IMPLEMENT_TMPDIR/plan.txt`, implement tmpdir `$IMPLEMENT_TMPDIR`, merge-base remote (`upstream/main` when `forked_target=true`, else `origin/main`), and the contract reminders from `agents/claude-self-reviewer.md`. No plan body or diff content is inlined. The main agent does **not** perform the review Edit/Write pass itself.
2. Parse the three trailing `SELF_REVIEW_*` lines from the subagent return:
   - `SELF_REVIEW_RESULT=complete`: continue with the composite checks-commit route below. Set `FILES_CHANGED_HINT` from `SELF_REVIEW_FIXES=true|false`.
   - `SELF_REVIEW_RESULT=bail` or missing/malformed trailer: log `Step 5: self-review subagent bail: $SELF_REVIEW_SUMMARY` to `Warnings`, set prompt-side `STALL_TRACKING=true` and `STALL_STEP=5` when durable seed is absent, and skip to Step 18.
3. Run captured relevant checks and the self-review commit route as one bgjob-owned composite fence:

> **Continue after bgjob `DONE`.** The launcher stdout is only `BGJOB_STATUS=STARTED STEP=implement-checks-step5-self-review PGID=<n>`. Then call the wait fence. If wait returns `BGJOB_STATUS=WAIT`, the next action is the identical wait fence again with no intervening prose or tools. If wait returns `BGJOB_STATUS=DEAD`, route through the existing self-review failure/stall branch. On final `DONE`, read the full wait KV block and `$IMPLEMENT_TMPDIR/bgjob/implement-checks-step5-self-review.result.env`; continue only when `BGJOB_RC=0` and required composite KVs are present. On composite `NEXT_ACTION=continue`, continue the self-review flow. On composite `NEXT_ACTION=stall`, skip to Step 18 (durable stall state is already seeded by commit-route). On composite `NEXT_ACTION=checks-failed`, apply **Checks Failure Entry Macro** with pinned `--site step5-self-review`.

**⚠ Bgjob foreground launch required: use the foreground bgjob launcher, not legacy immediate-background mode. Expected launcher stdout is exactly `BGJOB_STATUS=STARTED STEP=implement-checks-step5-self-review PGID=<n>`.**

```bash
"$HOME/.cache/larch/sessions/implement-run-$PPID.sh" skills/implement/scripts/run-step-checks.sh --site step5-self-review --commit-site step5-self-review # lint-consecutive-bash: ok self-review bgjob launch precedes the repeated wait fence
```

The self-review launcher uses `BUDGET_S=14700` and routes completion through the bgjob result env.

Wait with the shared bgjob contract. Repeat this exact fence on `BGJOB_STATUS=WAIT`.

```bash
"$HOME/.cache/larch/sessions/implement-run-$PPID.sh" scripts/larch.sh bgjob wait --step implement-checks-step5-self-review --tmpdir "$IMPLEMENT_TMPDIR" --max-wait-s 270
```

After the self-review composite bgjob returns `DONE` with `BGJOB_RC=0`, parse exactly one line-anchored composite `NEXT_ACTION=` record from the final `DONE` stdout and/or bgjob result env. Continue only on `NEXT_ACTION=continue`. On `NEXT_ACTION=main-agent-edit`, follow the reference's in-step Edit/Write and re-entry contract (orchestrator repair only for structural composite failures), then re-run this same composite launcher with identical argv. On missing, duplicated, malformed, seed-failed, non-zero `BGJOB_RC`, or non-zero-without-`NEXT_ACTION` output, treat it as an invalid composite envelope: log to `Warnings`, set prompt-side `STALL_TRACKING=true` and `STALL_STEP=5` when durable seed is absent, and skip to Step 18. Do not proceed to the next self-review step or Step 6.

4. Do not record a successful self-review in `$IMPLEMENT_TMPDIR/execution-issues.md`.
That log is for warnings and failures that need operator attention.
Normal review artifacts record successful completion.

5. Emit self-review Step 5 run-log artifacts so final report and `audit_runs` Step 5 detection treat a clean self-review as review ran. The CLI reconciles accepted and rejected counts from durable self-review artifacts under `$IMPLEMENT_TMPDIR`. This verb is best effort: writer failure records a Warnings entry in `$IMPLEMENT_TMPDIR/execution-issues.md` and returns `0`, so it never blocks Step 6.

```bash
"$HOME/.cache/larch/sessions/implement-run-$PPID.sh" "$CLAUDE_PLUGIN_ROOT"/scripts/larch.sh review-and-fix write-self-review-tally --implement-tmpdir "$IMPLEMENT_TMPDIR" --run-id "$RUN_ID"
```

6. Proceed directly to `### Cross-Skill Presence Propagation` in `skills/implement/SKILL.md`, then `### Track Rejected Code Review Findings` in `skills/implement/SKILL.md`, then Step 6, same chain as `STEP5_REVIEW_STATUS=complete`. Set `FILES_CHANGED_HINT=true` if `SELF_REVIEW_FIXES=true` or fixes were committed by the composite, otherwise `false`.

> **Continue after self-review completes.** Do NOT end the turn, summarize, or write a handoff message. → shared/subskill-invocation.md#anti-halt
