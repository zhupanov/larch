# Discussion Rounds Reference

**MANDATORY: READ ENTIRE FILE before composing Step 1c clarifying questions, Step 1d discussion-round writes, or the post-plan Round 2 sub-round body: `${CLAUDE_PLUGIN_ROOT}/skills/shared/readability-style.md`.**

**Consumer**: `/design` Steps 1c, 1d, and the Step 1e Gate A post-plan discussion body reached from Gate B(c) / Gate C(b).

**Contract**: owns the three discussion bodies: Step 1c questions, Step 1d Round 1, and post-plan Round 2. It defines decision walks, caps, schemas (`$DESIGN_TMPDIR/discussion-round1.md`, `$DESIGN_TMPDIR/discussion-round2.md`), and terse answers. Round 2 is no longer automatic at Step 3.5; Step 3.5 is Gate B. Gate A still uses Round 2 for each post-plan "Discuss more" re-entry.

**When to load**: before Steps 1c, 1d, or a Gate A post-plan discussion sub-round.

**Binding convention**: single source for discussion behavior: decision walks, caps, schemas, and terse answers.

---

<!-- step:1c: Clarifying Questions -->

Before drafting the plan, use `AskUserQuestion` to resolve ambiguities that affect scope, constraints, or done criteria.

Consider asking about:
- **Scope boundaries**: What is in-scope vs. out-of-scope? Which related changes does the user NOT want?
- **Key decisions**: When meaningful alternatives exist, including architectural approaches or file organization, present options and ask which direction to take.
- **Unclear requirements**: Which parts are vague, multi-interpretable, or based on implicit assumptions?

**Guidelines**:
- Ask when scope, requirements, or done criteria are uncertain. Suppress only when the feature is fully unambiguous; one extra clarification is cheaper than planning from the wrong interpretation.
- Batch 1-4 questions into one `AskUserQuestion` call instead of multiple sequential calls.
- **Semantic sprawl heuristic (best-effort)**: if clarifying answers reveal several distinct sub-features or cross-cutting infrastructure changes, enter the unified **Split-path** directly. Its single question owns Partition, Override, and Other/chat; Other/chat exits the structured path, so do not offer a preliminary Split/Cancel question. Semantic only: when uncertain, do not fire. At most **once** per Step 1c invocation.
- If the feature is clear, proceed to Step 1d.

After the user responds, carry those answers through all later steps.

---

<!-- step:1d: Design Discussion Round 1 -->

Before drafting, stress-test scope and requirements by walking a decision tree one question at a time. Unlike Step 1c batching, Step 1d is sequential: each answer may reshape later questions.

## Behavior

Identify key **scope and requirements decisions** from the feature description by exploring the codebase (Read/Grep/Glob). Cover **scope boundaries** (in-scope vs. out-of-scope), **hard constraints** (what must not break and what behavior must remain), **non-goals** (what the user explicitly does NOT want), and **must-have requirements** (minimum viable outcome).

Walk each branch one question at a time via sequential `AskUserQuestion` calls, providing a **recommended answer** for each question. If codebase exploration can answer the question, report the finding instead of asking.

After each answer, apply the **same semantic sprawl heuristic** as Step 1c (direct unified Split-path entry, with no preliminary prompt; on Cancel export `SUMMARY_OUTCOME=cancelled-sprawl`, run `### Final summary block` through Read/cache, print the operator line, then emit the cached summary as terminal text). **Cap**: at most **once** per Step 1d invocation; if it already fired during Step 1c or earlier in Step 1d, do not re-fire.

**Explicit prohibition**: Do NOT ask about implementation approach, architectural preferences, library choices, or file organization. Those belong to Step 2b plan drafting and Step 3 plan review. Round 1 is strictly requirements/scope clarification.

## Short-circuit

If the feature is straightforward with fewer than 2 scope decision branches, print `⏩ 1d: discussion r1: no scope decisions require discussion (<elapsed>)` and proceed to Step 1d.5 (brainstorm panel, when enabled) or Step 1d.7 (outline) when brainstorm is off. Step 1d.7 always fires on new-plan runs after Step 1d / Step 1d.5, including this short-circuit path; users may use **Refine outline** there to add context before plan drafting.

## Output

Write resolved decisions to `$DESIGN_TMPDIR/discussion-round1.md` using a simple Q&A format:

```markdown
## Decision 1: <short title>
- **Question**: <the question asked>
- **Resolution**: <the answer: from user or codebase>
- **Source**: user / codebase
```

This file captures scope boundaries and hard constraints only, NOT architectural preferences.

## Cap

At most **7 `AskUserQuestion` calls** in this step. If more than 7 decision branches remain, print: `⏩ Remaining scope questions deferred to implementation.` and proceed to Step 1d.5 (brainstorm panel, when enabled) or Step 1d.7 (outline) when brainstorm is off; users may pick **Refine outline** there to surface deferred branches before plan drafting.

## Terse answers

If the user gives a terse or non-responsive answer (e.g., "I don't know", "your recommendation is fine", "sure"), accept the recommended answer and move on without re-asking.

Record `<N>` decisions resolved.

---

<!-- post-plan discussion sub-round body (invoked from Step 1e Gate A on re-entry; the legacy <!-- step:3.5 marker is intentionally retained below for tooling that anchors on it) -->

<!-- step:3.5: Post-Plan Discussion Sub-Round body (referenced from Gate A re-entry) -->

After plan review, stress-test remaining design decisions that were not covered in Round 1, were challenged by reviewers, or were introduced by the plan itself. Gate A invokes this body on each re-entry from Gate B(c) "switch to discussion mode" or Gate C(b) "discuss further"; it is no longer automatic.

## Inputs

Read these artifacts:
- `$DESIGN_TMPDIR/discussion-round1.md`: If it exists and is non-empty, use it to identify decisions already covered in Round 1. **If it does not exist or is empty** (Round 1 short-circuited or was skipped), treat all candidate decisions as uncovered by Round 1 and proceed normally.
- `$DESIGN_TMPDIR/plan.txt`: Latest plan: initial Step 2b output or a post-plan re-entry plan with Gate B findings applied. Read this file, not conversation context.
- `$DESIGN_TMPDIR/accepted-plan-findings.md`: If it exists and is non-empty, use it to identify decisions reviewers challenged as suboptimal or that required plan revision.

## Behavior

Identify implementation-plan decisions **not covered in Round 1** (emerged from plan design, not the original feature description) or **challenged by reviewers** (appear in `accepted-plan-findings.md`).

Walk each uncovered branch one question at a time via sequential `AskUserQuestion` calls, providing a **recommended answer** for each question. If codebase exploration can answer the question, report the finding instead of asking.

Unlike Round 1, Round 2 MAY ask about architectural decisions and implementation approach because the current plan and reviewer feedback provide concrete context.

## Short-circuit

If all plan decisions are covered by Round 1 and no reviewer findings challenged them, print `⏩ post-plan discussion: no additional decisions require discussion (<elapsed>)` and return to the calling Gate A prompt (re-fire the "ready for review / discuss more" `AskUserQuestion`). This body is invoked from Gate A's "Discuss more" branch on a post-plan re-entry; control returns to Gate A, NOT to Step 3b. Gate A decides the next destination. "Ready for review" on a post-plan re-entry proceeds to Step 3, not Step 3b.

## Output

The caller (Gate A) selects the target file: post-plan Gate A re-entries from Gate B(c) or Gate C(b) write resolved decisions to `$DESIGN_TMPDIR/discussion-round2.md`. Step 1d remains the only first-time writer for `$DESIGN_TMPDIR/discussion-round1.md`. Use the same Q&A format as Round 1:

```markdown
## Decision 1: <short title>
- **Question**: <the question asked>
- **Resolution**: <the answer: from user or codebase>
- **Source**: user / codebase
```

**Plan revision authority**: This Gate A re-entry body MAY revise `$DESIGN_TMPDIR/plan.txt` directly from user-resolved decisions, because each change follows from explicit user answers. Preserve or recompute optional `diff_added:`, `diff_deleted:`, `mechanical_churn:`, and `oversize_override: operator` trailers in the final contiguous metadata block immediately above the required final `diff_lines:` line. Before direct replacement, run `"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" plan-review gate-b-dedup --design-tmpdir "$DESIGN_TMPDIR" --snapshot-trailers` as the pre-rewrite snapshot authority; do not rely on prompt-side keys-only checks. After rewriting, run `"$HOME/.cache/larch/sessions/design-run-$PPID.sh" design-step35-settle.sh --site discussion-round2` (maps to `scripts/larch.sh design step35-settle`). Settle runs dedup, clears stale dialectic artifacts, parses `POSTPLAN_RC=`, forwards `SETTLE_NEXT_ACTION=`, and delegates scout clearing to `scripts/larch.sh design step2b-postplan`. Do not call `dialectic-clear-stale` before settle dedup completes. `gate-a` and `discussion-round2` both map internally to `scripts/larch.sh design step2b-postplan --site discussion-round2`.

1. **MANDATORY: READ ENTIRE FILE**: Read `skills/design/references/settle-rc-dispatch.md` completely.
2. Require `SETTLE_NEXT_ACTION`; stop for repair if it is absent. Branch only on the matching `SETTLE_NEXT_ACTION` row in `settle-rc-dispatch.md`.

Reviewer findings are NEVER applied here. Gate B owns those. Print the revised plan only if substantive changes were made.

## Cap

At most **7 `AskUserQuestion` calls** in this step. No-response refires retry the current branch and do not advance the seven-call decision counter. If more than 7 decision branches remain, print: `⏩ Remaining design questions deferred to implementation.` and proceed.

## Terse answers

If the user gives a terse or non-responsive answer, accept the recommended answer and move on without re-asking.

Record `<N>` decisions resolved.

Round 2 postplan validation re-enters through `scripts/larch.sh design step35-settle --site discussion-round2`, which maps internally to `scripts/larch.sh design step2b-postplan --site discussion-round2`.

Compatibility grep note: `gate-a` and `discussion-round2` both map to `step2b-postplan --site discussion-round2` internally through the launcher mapping to `scripts/larch.sh design step2b-postplan --site discussion-round2`.
