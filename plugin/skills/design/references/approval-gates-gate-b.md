# Approval Gate B Reference

**MANDATORY: READ ENTIRE FILE before composing Gate B prose: `${CLAUDE_PLUGIN_ROOT}/skills/shared/readability-style.md`.**

**Consumer**: `/design` Step 3.5.

**Contract**: Gate B post-review chooser: severity/count CLI authority, zero-findings short-circuit, auto-apply vs explicit mode, apply-pipeline brakes, settle-wrapper dispatch, and plan revision.

**When to load**: unconditionally at Gate B, after the shared approval-gate core. Never skip this slice because the shared core was loaded earlier.

## Gate B: Post-Review Chooser (Step 3.5)

**When**: after Step 3 review completes or the script-internal Step 3 loop bails out. On the happy path, the Rust plan-review owner applies accepted findings in-loop via `scripts/larch.sh plan revise-waterfall --patch-format file-replacement`. Prompt-side Gate B handles `STEP3_REVIEW_LOOP_STATUS=main-agent-apply-required` and `per-round-approval-required`. `NEXT_ACTION=step3b-bypass` bypasses Step 3.5 before Step 3b. `panel-init-failed` hard-stops before Step 3b.

### Severity classification contract

Gate B severity mode, counts, ordered ids, table rows, and per-finding prompt fields are Rust-owned. Use these commands as authority:

- `scripts/larch.sh plan-review gate-b-counts --design-tmpdir "$DESIGN_TMPDIR"`
- `scripts/larch.sh plan-review preview --design-tmpdir "$DESIGN_TMPDIR" --variant gate-b`
- `scripts/larch.sh plan-review gate-b-finding-line --design-tmpdir "$DESIGN_TMPDIR" --finding-id <N>`

Parse KVs and emit CLI output. Do not re-read or manually classify `### FINDING_N:` blocks.

KV binding:

- Structured mode: bind `N=ACCEPTED_COUNT`, `H=HIGH_ACCEPTED_COUNT`, `M=MEDIUM_ACCEPTED_COUNT`, and `L=LOW_ACCEPTED_COUNT`. There is no structured Critical bucket.
- Fallback mode: bind `C=CRITICAL_ACCEPTED_COUNT`, plus `H=HIGH_ACCEPTED_COUNT`, `M=MEDIUM_ACCEPTED_COUNT`, and `L=LOW_ACCEPTED_COUNT`.
- Go-through-each mode: parse `FINDING_IDS` from `gate-b-counts`; it is comma-separated and in document order. Iterate that list only. Never assume a contiguous `1..ACCEPTED_COUNT` range.

### Zero-findings short-circuit

When `$DESIGN_TMPDIR/accepted-plan-findings.md` is empty, Gate B prints `⏩ 3.5: Gate B: no accepted findings; nothing to apply`. This fires before mode resolution, presentation, prompts, or plan apply.

- **Loop mode** (`STEP3_REVIEW_LOOP_STATUS` is set): bind `STEP3_RESUME_ROUND="${FINAL_ROUND_NUM:-${STEP3_REVIEW_ROUND_NUM:-${ROUND_NUM:-}}}"` per `SKILL.md`'s shared Step 3 resume rule. If empty or non-numeric, treat that as a Step 3 routing error. Resume through `design-step3-review.sh --starting-round "$STEP3_RESUME_ROUND" --phase awaiting-continuation` using the Step 3 bgjob resume fence from `SKILL.md`; require `BGJOB_RC=0` plus route KVs.

#### Gate B mode (auto-apply default; `--per-round-approval` for explicit)

Resolve mode only after the zero-findings short-circuit proves at least one accepted in-scope finding remains. The Rust script-internal controller applies accepted findings on the happy path before returning `STEP3_REVIEW_LOOP_STATUS=complete`; Prompt-side Gate B apply runs only on loop bail-outs (`main-agent-apply-required`, `per-round-approval-required`, `postplan-operator-required`). `--manual` / persisted manual mode no longer exists. Select UX from `approve_requested` (bound by the Step 3.5 fence from `run-params.json`; default `false`):

- **`approve_requested=false` (default): auto-apply.** Run `scripts/larch.sh design render-gate --gate B --accepted-count "$N" --approve-requested false`, print `AUTO_APPLY_MESSAGE`, then Execute `### Apply-all body` verbatim. Skip the `AskUserQuestion` entirely. No operator prompt fires before the plan is revised.
- **`approve_requested=true` (`--per-round-approval`): explicit.** Use the deferred explicit-mode reference load after Presentation below. Gate B prompts before any finding changes `plan.txt`, and `approval-gates-explicit.md` loads only after the zero-findings short-circuit and resume idempotency guard prove this entry will prompt.

**Resume idempotency guard**: loop mode records `$DESIGN_TMPDIR/.step3-round-N.phase` and writes `$DESIGN_TMPDIR/.gate-b-postapply-ready-N` only after dedup succeeds. `awaiting-apply` resumes at apply, `awaiting-post-apply` resumes at mechanical dedup/postplan without re-applying findings, and `awaiting-continuation` runs only `plan-review-continuation.sh`. Prompt-side Gate B uses the same marker to avoid double-applying during `main-agent-apply-required` recovery. Before executing the Gate B body, bind `_gate_b_round` from `FINAL_ROUND_NUM`, then `STEP3_REVIEW_ROUND_NUM`, then `ROUND_NUM`; fail closed if it is empty or non-numeric. When `$DESIGN_TMPDIR/.gate-b-postapply-ready-$_gate_b_round` exists and `.completed/step-3.5` does not, do not re-apply accepted findings. Route through the same settle wrapper with `--round-num "$_gate_b_round"` without reapplying. Bind `STEP3_RESUME_ROUND="$_gate_b_round"` before any later Step 3 resume fence. Do not jump directly to Step 3b from this post-apply resume branch; the script-internal loop at `awaiting-continuation` handles continuation before any Step 3b transition.

The zero-findings short-circuit still precedes apply UX selection: nothing is applied, no prompt fires, and the loop resumes through the Step 3 fence.

#### Apply-pipeline prompts under auto-apply

Under default auto-apply (`approve_requested=false`), Gate B fires **no** finding-acceptance prompt. Only these brakes can prompt inside `### Shared post-apply pipeline`, independent of `approve_requested`:

1. **Plan-size trigger** (`scripts/larch.sh design postplan-emit` rc=12): the in-loop controller returns `NEXT_ACTION=postplan-operator` before Step 3b. Gate B's idempotent settle re-entry emits `SETTLE_NEXT_ACTION=gate-b-hard-size`, then the unified Split-path single question fires.
2. **Plan-command validator escalation** (rc=10): cross-vendor auto-correction runs first with the `SKILL.md` shared validator contract. Fix-and-retry / Override / Cancel fires only after auto-fix is exhausted.

Plan drift (`DRIFT_TRIGGER_FIRED=true`) records a warning in `execution-issues.md` and exits `0`; it no longer halts.

**Step 3 outcomes** (read `NEXT_ACTION` first from `$DESIGN_TMPDIR/bgjob/design-step3-review.result.env`, with legacy `$DESIGN_TMPDIR/.step3-review-result.env` fallback only when the bgjob result env is absent; raw status fields are diagnostic):

After every `BGJOB_STATUS=DONE`, read the result env first. Require `BGJOB_RC=0` plus route KVs from final wait stdout and/or `$DESIGN_TMPDIR/bgjob/design-step3-review.result.env` for normal continuation. `DONE` alone, launcher stdout, wait shell exit 0, and the sentinel are not success.

- `NEXT_ACTION=step3b`: the loop already applied accepted findings, ran postplan, and ran continuation; skip Gate B.
- `NEXT_ACTION=gate-b`: prompt-side Gate B owns apply/postplan recovery, then resumes the recorded phase.
- `NEXT_ACTION=postplan-operator`: prompt-side Gate B owns the postplan operator brake without re-applying accepted findings. A hard-size rc 12 routes through `SETTLE_NEXT_ACTION=gate-b-hard-size` to the unified Split-path before Step 3b.
- `NEXT_ACTION=mav`: delegate MainAgent vote and re-tally directly to `scripts/larch.sh plan-review step3-mav --phase pre` and `--phase post`, with the PID-keyed current design env. Parse only trusted scalars from `DESIGN_STEP3_MAV_KV_BEGIN` / `DESIGN_STEP3_MAV_KV_END`; do not bind prompt-side retally anchors or invoke tally, persist-retally, or timing helpers inline. After successful post, resume once through the Step 3 bgjob wrapper: `design-step3-review.sh --starting-round "$STEP3_RESUME_ROUND" --phase awaiting-continuation` for zero accepted findings or `--phase awaiting-apply` when accepted findings remain; if live, rejoin with `bgjob wait`. If post emits `NEXT_ACTION=step3b-bypass`, run the Gate-B-bypass helper and continue to Step 3b.
- `NEXT_ACTION=step3b-bypass`: Gate B is **bypassed**. `NEXT_ACTION=final-summary:*`: Gate B is not reached.

### Presentation

1. Run `scripts/larch.sh plan-review gate-b-counts --design-tmpdir "$DESIGN_TMPDIR"` and bind counts from stdout KVs.
2. Run `scripts/larch.sh plan-review preview --design-tmpdir "$DESIGN_TMPDIR" --variant gate-b` and emit stdout verbatim. Preview owns the `## Plan Review Findings: Review` header, findings rows, and rejected/OOS context. Do not print that header again in Presentation.

### Explicit-mode load gate

Run only after accepted findings exist, the Resume idempotency guard does not route to the post-apply-only settle path, and Presentation completes.

- **`approve_requested=false` (default):** do not load `skills/design/references/approval-gates-explicit.md`; continue directly to `### Apply-all body`.
- **`approve_requested=true` (`--per-round-approval`):** **MANDATORY: READ ENTIRE FILE**: Read `skills/design/references/approval-gates-explicit.md` completely immediately before firing the explicit `AskUserQuestion` or one-by-one iteration.

### Prompt

Explicit-mode prompt details live in `skills/design/references/approval-gates-explicit.md`. Load that file only through `### Explicit-mode load gate`.

### Apply-all body

Before any Write, copy `$DESIGN_TMPDIR/plan.txt` to `$DESIGN_TMPDIR/plan-pre-apply-round-N.txt` for the bound Gate B round if absent. Then apply accepted in-scope findings, rewrite `plan.txt` preserving `diff_lines: <N>` and optional size/override trailers in the final metadata block, then Execute `### Shared post-apply pipeline` verbatim.

### One-by-one iteration prompt

Explicit-mode one-by-one details live in `skills/design/references/approval-gates-explicit.md`. Load that file only through `### Explicit-mode load gate`.

### Shared post-apply pipeline

Prompt-side Gate B owns the pre-apply snapshot and inline rewrite. The settle wrapper runs post-rewrite dedup under `set +e`; on a dedup-revise result it restores `plan-pre-apply-round-N.txt` to `plan.txt` when present, returns `STEP3_REVIEW_LOOP_STATUS=main-agent-apply-required` with `DEDUP_RC`, and does not write `.gate-b-postapply-ready-N`. `.gate-b-postapply-ready-N` is written only after dedup succeeds. Operator-brake resumes (`POSTPLAN_RC=10/12/13`) persist phase `awaiting-postplan-operator`. Non-plan-changing Override/Continue writes `$DESIGN_TMPDIR/.postplan-operator-continue-N`; the loop consumes it and promotes to `awaiting-continuation`. Plan-changing Fix-and-retry/autofix overwrites phase to `awaiting-post-apply`.

After the chosen findings have been applied to `plan.txt` (full accepted set or one-by-one subset), run the same launcher-owned post-apply sequence for both Gate B branches:

1. **Optional trailer guard (direct rewrites)**: before prompt-side `plan.txt` replacement or dedup rewrite, run `plan-review gate-b-dedup --design-tmpdir "$DESIGN_TMPDIR" --snapshot-trailers` to snapshot `diff_added`, `diff_deleted`, `mechanical_churn`, and `oversize_override`. An empty snapshot forbids later optional trailers.
2. Re-read the revised `plan.txt` and remove semantically duplicate lines or short blocks (the same constraint, requirement, or instruction stated more than once, not just byte-identical text).
3. Preserve intentional repetition in distinct context sections (for example, a constraint in both Approach and Edge cases); remove only true redundancy within or across the same section.
4. Rewrite `plan.txt` via the Write tool with duplicates removed.
5. Run settle through the launcher: `"$HOME/.cache/larch/sessions/design-run-$PPID.sh" design-step35-settle.sh --site gate-b --round-num "$_gate_b_round"` (maps to `scripts/larch.sh design step35-settle`).
6. Do not pass `STEP3_RESUME_ROUND` before it is bound. If surrounding prose already has a validated round variable, pass it with `--round-num`; otherwise let the wrapper derive the Gate B round from `FINAL_ROUND_NUM`, `STEP3_REVIEW_ROUND_NUM`, then `ROUND_NUM`.
7. `scripts/larch.sh design step35-settle` calls `scripts/larch.sh design step2b-postplan --site gate-b` in-process after dedup succeeds. Settle owns the post-dedup apply-ready marker, Gate B phase writes, `POSTPLAN_RC=` parsing, and no-`plan-after-round-N.txt` contract. It forwards the Rust action row. Scout-manifest clearing remains owned by `scripts/larch.sh design step2b-postplan`.
8. Settle-wrapper dispatch:
   1. **MANDATORY: READ ENTIRE FILE**: Read `skills/design/references/settle-rc-dispatch.md` completely.
   2. Require `SETTLE_NEXT_ACTION`; stop for repair if it is absent. If the action row and wrapper rc disagree, stop for repair. Branch only on the matching `SETTLE_NEXT_ACTION` row in `settle-rc-dispatch.md`.
9. Before leaving the post-apply path, bind `STEP3_RESUME_ROUND="${FINAL_ROUND_NUM:-${STEP3_REVIEW_ROUND_NUM:-${ROUND_NUM:-}}}"` per `SKILL.md`'s shared Step 3 resume rule. If empty or non-numeric, stop for operator repair as a Step 3 routing error. Do not call `design-step3-review.sh` yet; step 9 only determines or binds `STEP3_RESUME_ROUND`.
10. Only when the settle wrapper returns rc `0`, a retained drift Continue settles, or a non-exiting Split/Override path completes without skill exit, resume once through `design-step3-review.sh --starting-round "$STEP3_RESUME_ROUND" --phase awaiting-continuation` using the Step 3 bgjob resume fence from `SKILL.md`. The script-internal loop runs continuation from `awaiting-continuation` and owns any terminal Step 3b transition.

### Gate B plan revision and Step 2b.5

Gate B's plan revision may branch the merged driver fence. `--partition` maps to Split-path with no prompt. Hard triggers are body `> 800`, firm headings `> 25`, surfaces `> 4`, or `diff_added > 2000` / fallback `diff_lines > 1500`; `mechanical_churn: true` softens only the diff trigger. `SIZE_TRIGGER_FIRED=true` enters the unified Split-path directly. Its single question owns Partition, Override, and Other/chat. Override writes the oversize trailer, deletes `composed-plan.md`, and writes postplan completion. Drift is advisory. Standalone Step 2b.5 is only for Override-after-defects and recovery. Contract: `scripts/larch.sh plan check-size`.

`plan set-oversize-override` records the operator token, plan hash, and trigger reasons. After a Gate B rewrite, `gate-b-dedup --dedup` re-arms only when recomputed reasons are a subset of that record, printing `ℹ oversize override carried forward (reasons: REASONS)` once. A new reason or direct Step 3 re-entry revokes authority and re-asks through Split.

---
