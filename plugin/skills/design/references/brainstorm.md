# Brainstorm panel (Step 1d.5)

**Consumer**: `/design` Step **1d.5**: runs after Step **1d** (Round 1 discussion) and before Step **1d.7** (Design Outline: Gate A re-entry only post-plan) when `brainstorm_requested` is true in `$DESIGN_TMPDIR/run-params.json` or set in Step 0 by argv or the Step 0b `Brainstorm:` title-prefix auto-enable.

**Contract**: one-shot per invocation via `$DESIGN_TMPDIR/.brainstorm-done`. Produces additive `$DESIGN_TMPDIR/brainstorm.md` (never load-bearing for downstream automation). Downstream readers: **Step 2b** (plan drafting) and **Step 3** (plan-review feature context merged by `plan-review-loop.sh` when `brainstorm.md` exists).

**When to load**: only when Step **1d.5** executes: do not preload during Step 0.

---

## Style preamble expansion

Before launching each external slot (framing, scope) and before composing the always-Claude pragmatic slot, read `${CLAUDE_PLUGIN_ROOT}/skills/shared/readability-style.md` once and substitute every literal `<READABILITY_STYLE>` token in the assembled prompt with the full preamble contents. The `<READABILITY_STYLE>` expansion remains limited to these existing `/design` brainstorm prompt surfaces. The pragmatic slot is parent-session, but it receives the same substitution so all three slots see identical style guidance.

---

## Anti-halt override (Step 1d.5 only)

Step 1d.5 **overrides** the generic “never halt after Bash” anxiety **only** for the narrow case: after externals return and you print the **brainstorm synthesis digest** once, you may yield the turn so the operator can speak in the discussion loop below.

**Hard prohibition (non-negotiable)**: Do **NOT** use `ScheduleWakeup`, wall-clock `sleep` polling loops, Monitor-driven polling, or Claude background waits for brainstorm externals or operator replies. Use foreground `bgjob start` plus chunked `bgjob wait` for external lanes. Do not add summary/handoff prose that masquerades as a parent-skill terminal.

---

## Entry guard

1. Read `$DESIGN_TMPDIR/run-params.json` → boolean `brainstorm_requested` (default **false** when absent).
2. If `brainstorm_requested` is not true: print `⏩ 1d.5: brainstorm: skipped` and **skip** this entire step (go to Step **1d.7**).
3. If `$DESIGN_TMPDIR/.brainstorm-done` exists: print `⏩ 1d.5: brainstorm: skipped (already complete; .brainstorm-done present)` and **skip** this entire step (go to Step **1d.7**).
4. Print `> **🔶 /design 1d.5: brainstorm**`.

---

## MANDATORY: read prompts file first

**MANDATORY: READ ENTIRE FILE**: Read `${CLAUDE_PLUGIN_ROOT}/skills/design/references/brainstorm-prompts.md` completely before assembling slot prompts. It holds `<BRAINSTORM_FRAMING_PROMPT>`, `<BRAINSTORM_SCOPE_PROMPT>`, and `<BRAINSTORM_PRAGMATIC_PROMPT>`.

---

## Optional Round-1 context

If `$DESIGN_TMPDIR/discussion-round1.md` exists and is non-empty, read it and prepend a short quoted excerpt block into each external slot prompt assembly (bounded length; paraphrase if huge). If absent, proceed without Round-1 text.

---

## Three-slot ideation matrix

| Slot | Tool order | Output file (deterministic) | Timing kind | Prompt body token |
|------|------------|------------------------------|-------------|-------------------|
| Framing | Read `ORDER=` from `design.brainstorm_framing` | **`$DESIGN_TMPDIR/cursor-brainstorm-output.txt`**: canonical **framing** staging file; parent **Write**s here no matter which external actually ran (waterfall / Agent fallback). | `cursor-brainstorm` / `codex-brainstorm` | `<BRAINSTORM_FRAMING_PROMPT>` |
| Scope | Read `ORDER=` from `design.brainstorm_scope` | **`$DESIGN_TMPDIR/codex-brainstorm-output.txt`**: canonical **scope** staging file; parent **Write**s here no matter which external actually ran. | `codex-brainstorm` / `cursor-brainstorm` | `<BRAINSTORM_SCOPE_PROMPT>` |
| Pragmatic | Always Claude (primary) | in-session compose (no external path) | _(none)_ | `<BRAINSTORM_PRAGMATIC_PROMPT>` |

Before each external slot launch, run `"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" external-defaults role --role design.brainstorm_framing` or `"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" external-defaults role --role design.brainstorm_scope`. Parse `ORDER=` as the registry-backed waterfall for that slot. Iterate the order with the existing availability gates and Agent-text fallbacks, then pick the first eligible external tool. Pragmatic brainstorming stays parent-session Claude and has no registry lookup.

Spawn slowest-first when two externals are queued in one wave, based on the resolved tools for the selected framing and scope slots. Keep Claude Agent text generation as the fallback when no external is eligible.

### Agent-returns-text + parent-writes-file (mandatory)

External review **Agent** fallbacks return **text only** to the parent session. The parent MUST **Write** that returned text to the canonical staging file for that ideation slot (`cursor-brainstorm-output.txt` for framing, `codex-brainstorm-output.txt` for scope) **before** synthesis: never instruct a subagent to write these files directly.

---

## External launches (representative)

Read `${CLAUDE_PLUGIN_ROOT}/skills/shared/bgjob-wait.md` before launching or waiting on external brainstorm lanes. Each parallel external lane MUST use a unique bgjob `--step` slug and its own merge-result env so registry rows, stdout/stderr logs, and result envs cannot clobber each other. Use `design-brainstorm-framing` for the framing lane and `design-brainstorm-scope` for the scope lane. Capture failures under the launch-failure sink that matches the slot's canonical output path, and append via `run-log append-failure` during collection.

Canonical pairings:

- **Framing** output: `$DESIGN_TMPDIR/cursor-brainstorm-output.txt`; matching failure sink: `$DESIGN_TMPDIR/cursor-brainstorm-launch.failure.log`; bgjob step: `design-brainstorm-framing`; merge input: `$DESIGN_TMPDIR/.design-brainstorm-framing-result.env`.
- **Scope** output: `$DESIGN_TMPDIR/codex-brainstorm-output.txt`; matching failure sink: `$DESIGN_TMPDIR/codex-brainstorm-launch.failure.log`; bgjob step: `design-brainstorm-scope`; merge input: `$DESIGN_TMPDIR/.design-brainstorm-scope-result.env`.

Before every fresh external lane start, truncate or recreate that lane's merge input. The launcher `.meta` file's `STDERR_SINK=` value must point at the matching failure log for the same output path. Mismatched sink/output pairs can create `External Reviewer Issues` rows that collect mode cannot ingest.

**Framing** (when the registry-selected tool is external and available):

```bash
"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" bgjob start \
  --step design-brainstorm-framing \
  --tmpdir "$DESIGN_TMPDIR" \
  --budget-s 1260 \
  --merge-result-env "$DESIGN_TMPDIR/.design-brainstorm-framing-result.env" \
  -- \
  "${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" agent launch-review --tool <resolved> --output "$DESIGN_TMPDIR/cursor-brainstorm-output.txt" --stderr-sink "$DESIGN_TMPDIR/cursor-brainstorm-launch.failure.log" --timeout 1200 --timing-task-kind <resolved>-brainstorm --prompt "<BRAINSTORM_FRAMING_ASSEMBLED_PROMPT>" # lint-consecutive-bash: ok framing and scope examples use distinct outputs
```

**Scope** (when the registry-selected tool is external and available):

```bash
"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" bgjob start \
  --step design-brainstorm-scope \
  --tmpdir "$DESIGN_TMPDIR" \
  --budget-s 1260 \
  --merge-result-env "$DESIGN_TMPDIR/.design-brainstorm-scope-result.env" \
  -- \
  "${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" agent launch-review --tool <resolved> --output "$DESIGN_TMPDIR/codex-brainstorm-output.txt" --stderr-sink "$DESIGN_TMPDIR/codex-brainstorm-launch.failure.log" --timeout 1200 --timing-task-kind <resolved>-brainstorm --prompt "<BRAINSTORM_SCOPE_ASSEMBLED_PROMPT>"
```

Fresh-launch stdout for each lane must be exactly `BGJOB_STATUS=STARTED STEP=<lane-step> PGID=<n>`. Wait on each launched lane with `"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" bgjob wait --step <lane-step> --tmpdir "$DESIGN_TMPDIR" --max-wait-s 270` and timeout `330000`. `BGJOB_STATUS=WAIT` means the next action for that lane is the identical wait command with no intervening prose, reads, Monitor, TaskOutput, or sleep. After `BGJOB_STATUS=DONE`, read `$DESIGN_TMPDIR/bgjob/<lane-step>.result.env`; continue to collection only when `BGJOB_RC=0`. `DEAD`, missing `BGJOB_RC`, non-zero `BGJOB_RC`, `BGJOB_RC=timeout`, or `BGJOB_RC=orphaned` uses the existing launch-failure and dirty-tree recovery path.

**Always-Claude pragmatic**: run in the parent session (Agent or inline) using `<BRAINSTORM_PRAGMATIC_PROMPT>` embedded in `<CLAUDE_BRAINSTORM_ASSEMBLED_PROMPT>`; merge result into synthesis input (no `scripts/larch.sh agent collect-results` row required for a purely in-session path).

---

## Collection (`design-run-$PPID.sh step1d5 --mode collect`) - externals only

**Do not copy-paste a fence verbatim.** The argv below is illustrative only: list **only** the canonical staging paths (`cursor-brainstorm-output.txt` / `codex-brainstorm-output.txt`) for slots you **actually launched as externals** this wave (parent-only / Agent-text fallbacks are **not** launches). Use dynamic argv: one path when a single external ran, two when both ran. Use `timeout: 1260000` on the foreground Bash tool call so the orchestrator does not kill collection before `agent collect-results --timeout 1260` finishes inside the launcher.

**Example: one external** (e.g. Cursor framing ran; Codex scope was parent-written in-session):

```bash
# lint-consecutive-bash: ok one-external and two-external collect examples are intentionally distinct
"$HOME/.cache/larch/sessions/design-run-$PPID.sh" step1d5 --mode collect -- \
  "$DESIGN_TMPDIR/cursor-brainstorm-output.txt"
```

**Example: two externals** (both Cursor framing and Codex scope launched as externals):

```bash
"$HOME/.cache/larch/sessions/design-run-$PPID.sh" step1d5 --mode collect -- \
  "$DESIGN_TMPDIR/cursor-brainstorm-output.txt" \
  "$DESIGN_TMPDIR/codex-brainstorm-output.txt"
```

Guard this call by launched external paths: **omit paths** for slots that were not launched as externals (tool unavailable with parent-written Agent fallback is **not** an external launch). **Never** invoke collect mode with zero paths. The launcher-owned collect call ingests dirty-tree sidecars, runs the post-collection checkpoint, writes `dirty-tree-detected.env`, and appends external launch failures idempotently.

## Post-collection dirty-tree recovery

Immediately after `design-run-$PPID.sh step1d5 --mode collect` returns:

1. Consult `${OUTPUT}.dirty-tree` sidecars for each canonical staging path you supplied to `--mode collect`.
2. Read `$DESIGN_TMPDIR/dirty-tree-detected.env`.

If the env file contains `RECOVERY_REQUIRED=true`, run the non-skippable operator recovery flow **before** synthesis or Step 1d.7:

- Use `$DESIGN_TMPDIR/.dirty-tree-prompted-brainstorm-collection` as the once-per-boundary sentinel; do not fire `AskUserQuestion` when the sentinel already exists.
- When prompting, offer exactly **Restore a clean tree and continue** and **Cancel this design run**.
- On **Restore a clean tree and continue**: re-run `"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" dirty-tree checkpoint` and continue only when it reports `STATUS=clean`; then rewrite `dirty-tree-detected.env` with `RECOVERY_REQUIRED=false` and proceed to synthesis.
- On **Cancel this design run**: preserve `$DESIGN_TMPDIR` and exit `/design`.
- Do not proceed to synthesis, the discussion loop, or Step 1d.7 while `RECOVERY_REQUIRED=true`.

---

**MANDATORY: READ ENTIRE FILE before composing the synthesis and any free-form discussion-loop response: `${CLAUDE_PLUGIN_ROOT}/skills/shared/readability-style.md`.**

## Synthesis → `brainstorm.md`

1. Read slot outputs (externals from disk; Claude slot from session result).
2. Deduplicate near-identical bullets; **order** ideas: framing → scope → pragmatic clusters.
3. **Write** `$DESIGN_TMPDIR/brainstorm.md` using this schema:

```markdown
## Brainstorm Synthesis

### <Idea short title>
**Source:** cursor-brainstorm | codex-brainstorm | claude-brainstorm
<1–3 sentences>

### <Next idea>
**Source:** …
…
```

`## Brainstorm Synthesis` is required once; each idea uses `###` heading + literal `**Source:**` line exactly as shown (pipe-separated tool tags for traceability).

---

## Free-form discussion loop (after synthesis)

### Branch order: classify-message-first

1. **Terminal / ready**: If the operator message is **standalone primary-intent** (“done”, “ready for gate”, “let’s move on”) **and** it is **not** negated, conditional, or carrying an embedded refinement (“don’t X yet, but …”), then: write `$DESIGN_TMPDIR/.brainstorm-done`, print a one-line acknowledgment, and **continue to Step 1d.7 in the same turn** without re-printing the full synthesis document.
2. **Refinement**: If they want edits (add idea, merge, reorder): **mutate** `brainstorm.md`, print an `## Updated Brainstorm Digest` with changed bullets only, then **end the turn**.
3. **Ambiguous**: If intent is unclear, `AskUserQuestion` with exactly two clarified options (no secrets in option text).

**Termination vocabulary disambiguation**: Treat “done / ready / proceed” as terminal **only** when it is the **standalone primary intent** of the message. Messages that negate, defer, or bundle refinements (“not yet”, “also change …”, “done but fix …”) → **refinement** path, not terminal.

When the loop ends via terminal path, ensure `.brainstorm-done` exists before entering Step **1d.7**.

---

## Downstream consumer contract (additive)

- **Step 2b**: Read `brainstorm.md` when present; fold ideas only when they do not conflict with explicit user refusals.
- **Step 3**: `plan-review-loop.sh` may stage non-empty `brainstorm.md` as optional feature context, but `plan-review-scope-anchor.txt` remains the binding issue scope for scout, reviewer, voter, and MainAgent fallback decisions.

## Plan-review binding scope

Plan review uses the staged issue plus approved-outline scope anchor. Brainstorm synthesis may be present as optional context, but it is non-binding and does not redefine reviewer, voter, scout, or MainAgent fallback scope.
