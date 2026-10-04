# OOS Acceptance Rubric (Legitimacy Standard)

An out-of-scope (OOS) observation should be accepted when it is genuine,
concrete, and non-duplicate. Accepted OOS is not implemented in the current
change. It enters the filing path only when the severity gate below also passes.

## YES

Vote YES when all of these are true:

- The observation describes a real problem or useful follow-up, not a vague
  preference.
- The trigger is concrete: a named path, behavior, workflow, or reviewed output
  explains why the item exists.
- The item is not a duplicate of an already filed, already accepted, or clearly
  equivalent OOS item.
- The item is worth tracking outside the current scope, even if it would not beat
  an in-scope finding under the stricter necessity rubric.

## NO

Vote NO for:

- Pure style, polish, wording, or taste preferences.
- Noise, cleanup with no named future cost, or consistency-only churn.
- Duplicates of existing accepted/filed OOS items.
- False positives.
- Speculative or hypothetical issues with no concrete trigger.
- Suggested fixes that are the only objection; remedies are informational and
  the future implementer chooses the fix.

## Thresholds

OOS uses the OOS-specific panel thresholds: one YES accepts in a one-judge panel,
one or more YES votes accept in a two-judge panel, and two or more YES votes
accept in a three-judge panel. In-scope findings keep their stricter acceptance
thresholds.

## Filing gate

Acceptance is not enough to file an OOS item. File it only when the accepted OOS
also has a strict majority of YES voters rating severity `major`. Accepted OOS
with YES voters rating only `minor` or `nit` stays in logs and is not filed.

## Suggested fixes

Suggested fixes are informational only. Do not vote NO because you prefer a
different remedy when the underlying OOS observation is legitimate.

## Update triggers

Keep this rubric in sync with:

- `scripts/larch.sh render voter`
- `skills/shared/reviewer-templates.md`
- `crates/larch-core/src/review/findings_ledger.rs`
- `skills/implement/SKILL.md`
- `skills/implement/references/step5-review-branches.md`
- `skills/design/SKILL.md`
- `skills/shared/review-acceptance-rubric.md`
- `skills/shared/voting-protocol.md`
