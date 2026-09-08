# Approval Gate A Reference

**MANDATORY: READ ENTIRE FILE before composing Gate A prose: `${CLAUDE_PLUGIN_ROOT}/skills/shared/readability-style.md`.**

**Consumer**: `/design` Step 1e re-entry only.

**Contract**: Gate A discussion-mode loop: render-gate shape, Ready-for-review route to Step 3, Discuss-more sub-round body, and `discussion-round2.md` accumulation. Loads only on re-entry from Gate B(c) or Gate C(b).

**When to load**: unconditionally when Gate A is entered from Gate B or Gate C. Do not load on the default path.

## Gate A: Discussion Mode Loop (Step 1e)

**When**: **Re-entry-only** from Gate B option (c) "switch to discussion mode" or Gate C option (b) "discuss further". First-time Step 1d / Step 1d.5 entry is replaced by the **Step 1d.7 outline-approval gate**; see `${CLAUDE_PLUGIN_ROOT}/skills/design/references/design-outline.md` for Approve/Refine/Cancel.

**Behavior**: when post-plan scope or requirements questions appear discussed, prompt via `AskUserQuestion`.

**Shape 2: re-entry from Gate B(c) or Gate C(b) (post-plan)**: run `scripts/larch.sh design render-gate --gate A`. Pass the rendered `HEADER`, `QUESTION`, and option rows directly to `AskUserQuestion`.

- **See full plan**: if `$DESIGN_TMPDIR/plan.txt` is missing or empty, print `**⚠ plan.txt missing or empty; nothing to show.**` and re-prompt with `--without-see-full-plan` anyway. Otherwise re-display the current plan under `## Latest Design Plan` (verbatim, no diff vs. prior version), then run `scripts/larch.sh design render-gate --gate A --without-see-full-plan` and re-fire with those rows. This option never mutates state or advances control.
- **Ready for review**: **MANDATORY: READ ENTIRE FILE** `${CLAUDE_PLUGIN_ROOT}/skills/design/references/plan-review-runtime.md` completely before invoking `design-step3-entry.sh --reentry`. Route to the single Step 3 entry fence and proceed directly to Step 3 with the current `$DESIGN_TMPDIR/plan.txt`. Do not add a separate Gate A wrapper. Step 3 consumes the marker to restore the direct-review bypass package and clear stale review/final-approval sentinels before pause-check.
- **Discuss more**: remain in Gate A; conduct another discussion sub-round, then re-render Gate A.

The Shape 2 trigger is exactly "Gate A entered from Gate B(c) or Gate C(b)", the same trigger that routes the discussion sub-round body to `discussion-round2.md`.

### Discussion sub-round body

When the user picks **Discuss more**, ask what else to discuss or walk a deferred Step 1d branch. Append resolved decisions to `$DESIGN_TMPDIR/discussion-round2.md` using the `discussion-rounds.md` Q&A schema, then re-prompt with Shape 2.

Re-entry is post-plan. Write new resolved decisions to `$DESIGN_TMPDIR/discussion-round2.md`, not `discussion-round1.md` (Round 1 closes once Step 2a begins). `discussion-round2.md` records user-approved discussion outcomes, not patch instructions. Gate A may revise `plan.txt` only for user-resolved design decisions recorded during that discussion flow; Gate B alone applies accepted review findings. Do not run a Gate B rollback pass from `discussion-round2.md`. If discussion changes the plan after an explicit apply or changes whether an earlier finding should still stand, exit through **Ready for review** so Step 3 re-runs and regenerates `accepted-plan-findings.md` before any later Gate B entry.

---
