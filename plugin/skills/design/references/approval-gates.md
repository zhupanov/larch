# Approval Gates Shared Core

**MANDATORY: READ ENTIRE FILE before composing approval-gate prose: `${CLAUDE_PLUGIN_ROOT}/skills/shared/readability-style.md`.**

**Consumer**: `/design` Steps 1e, 3.5, and 4b. Load this shared core before the gate-specific slice. Gate slices are always loaded at their gate and never skipped.

**Contract**: shared renderer, review-cap, severity/count, post-apply, and cross-gate state invariants. Gate behavior lives in `approval-gates-gate-a.md`, `approval-gates-gate-b.md`, and `approval-gates-gate-c.md`. Step 3 runtime cumulation and snapshot semantics live in `plan-review-runtime.md`.

**When to load**: at Gate A, or at Gate B/C unless already loaded during Gate A re-entry.

## Review-round cap

Gate C option shaping comes from `scripts/larch.sh design render-gate --gate C --design-tmpdir "$DESIGN_TMPDIR"`. Consume `REVIEW_ROUND_CAP`, option rows, and optional `REVIEW_ROUND_COUNT_WARN`. Do not restate renderer cap math. Step 3 is the counter authority and enforces the fixed cap of 2 on every entry, including Gate C re-runs and Gate A **Ready for review** re-entry. Gate A **Discuss more** loops remain uncapped. Escalation changes panel tier and model role only; it does not add review rounds.

## Renderer parsing contract

Run renderer commands as `"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" design render-gate ...`. Require `GATE_RENDER_STATUS=ok` and every `HEADER`, `QUESTION`, and `OPTION_*` field needed by `AskUserQuestion`; stop for repair on any miss. Do not reconstruct fallback prompt copy in prose. After Gate C `render-gate`, append the bounded Warning to `$DESIGN_TMPDIR/execution-issues.md` when `REVIEW_ROUND_COUNT_WARN=non-numeric` is present.

---

## State invariants across gates

1. **Latest plan to reviewers**: Step 3 always reads `$DESIGN_TMPDIR/plan.txt` from the latest Step 2b write, Gate B applied-set revision, or Gate A user-resolved discussion revision. No prior-version plan is submitted.

2. **No preserved findings across manual review runs**: when Step 3 is re-entered from Gate C(c), prior review artifacts are overwritten. Gate B uses only the latest `accepted-plan-findings.md`. During automatic continuation before Gate C, `accepted-plan-findings-all.md`, `rejected-findings-all.md`, and `oos-accepted-design.md` accumulate for final reporting and terminal status mapping; see `plan-review-runtime.md` § Single-pass review.

3. **Discussion outputs accumulate**: Step 1d writes `discussion-round1.md`. Step 1d.7 writes `design-outline.md`. `discussion-round2.md` accumulates Gate A re-entries from Gate B(c) / Gate C(b). All three remain inputs to later plan revisions.

4. **Gate B apply contract**: by default (`approve_requested=false`) Gate B **auto-applies** every accepted in-scope finding with no prompt. Under `--per-round-approval` (`approve_requested=true`) it prompts before revising `plan.txt`, and rewriting runs only after **Apply all** or applied individual findings in **Go through each**. It never asks again for already-approved apply actions. Gate A and Gate C never auto-revise `plan.txt`; Gate A may revise it only for user-resolved discussion outcomes. Gate B never treats `discussion-round2.md` as patch instructions. The script-internal Step 3 loop applies accepted findings on the happy path via `scripts/larch.sh plan revise-waterfall`; prompt-side Gate B applies only on loop bail-outs. There is no persisted mode state; each Gate B entry recomputes UX from `approve_requested`.

<!-- loop-mode review contract -->
In loop mode, accepted findings are applied inside the Rust plan-review owner before `STEP3_REVIEW_LOOP_STATUS=complete`. Prompt-side Gate B applies only on loop bail-outs; under `--per-round-approval` it asks explicitly: Apply all / Go through each / Switch to discussion mode.

Step 5c missing or empty `$DESIGN_TMPDIR/composed-plan.md` is a file-precondition defect. Recovery must compose Step 5c item 1 first, then re-run `"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" design step5c --session-env-path "$HOME/.cache/larch/sessions/current-design-env-$PPID.sh" --claude-pid "$PPID" --fresh-attempt`. Skip auto-repair and do not offer Override.

For ordinary composed-plan validator defects where the file exists and is non-empty, keep ordinary recovery semantics: auto-repair, then Fix-and-retry / Override / Cancel when auto-repair does not resolve the defect.

Limit `"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" design step5c --session-env-path "$HOME/.cache/larch/sessions/current-design-env-$PPID.sh" --claude-pid "$PPID" --fresh-attempt --skip-validate` to ordinary Step 5c validator defects after operator Override or successful auto-fix validation. Fix-and-retry re-runs the same command without `--skip-validate` so command validation reruns on the operator-edited `composed-plan.md`. Do not imply that `--skip-validate` can repair a missing or empty composed plan.

Compatibility grep note: `scripts/larch.sh design step35-settle` calls `scripts/larch.sh design step2b-postplan --site gate-b` in-process (historical launcher fence: `design-step35-settle.sh`).
