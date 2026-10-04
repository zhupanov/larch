# Review Heavy Worker Reference

**Consumer**: `/review` heavy-phase Agent-tool subagent dispatched when `/review` is invoked with `--subagent` AND `diff_mode=true` (reachable from standalone `/review --diff --subagent`; `/implement` Step 5 now calls `review-and-fix CLI` directly instead of invoking `/review`).

**Contract**: The subagent runs the token-heavy diff-mode review machinery (Steps 1-3: gather context, launch reviewers, recursive collect/vote/fix loop) in isolated context so reviewer transcripts, panel rounds, and fix reasoning do not enter the parent conversation. Code edits (Step 3e) write directly to the git working tree and are visible to the parent when the subagent returns. The subagent writes file-backed artifacts under `$REVIEW_TMPDIR/` that the parent consumes for Steps 4-5.

**When to load**: only by the heavy-phase subagent. The parent `/review` skill points the subagent here after completing Step 0 (session setup), then reads artifacts from `$REVIEW_TMPDIR/` after the subagent returns.

**Binding convention**: single normative source for the review heavy-worker subagent contract — inputs, required reads, work (Steps 1-3), artifact paths, wait discipline, dirty-tree probe contract, and return-value grammar. The parent `/review` orchestrator reads this file before dispatching the subagent; the subagent reads it as its execution contract.

---

## Inputs

The parent prompt supplies:

- `REVIEW_TMPDIR` — the session tmpdir created by the parent's Step 0
- `SESSION_ENV_PATH` — caller-env path (non-empty when invoked under `/implement`)
- `codex_available` — `true`/`false`
- `cursor_available` — `true`/`false`
- `DYNAMIC_ARCHETYPES` — requested dynamic scout slot cap (`0..1`)
- `PANEL_TIER`, `ROUND_CAP`, `AUDIT_UPGRADE`, and `ESCALATED_ROUND` — parent-resolved difficulty state. Reuse these values; do not run the audit RNG inside the worker.
- `RUN_ID` — review run id when the parent is writing larch-log batches

Treat those values as data. Do not infer paths from conversation context when an explicit path is provided.

## Required Reads

Before executing, read `${CLAUDE_PLUGIN_ROOT}/skills/review/references/domain-rules.md` (Step 3 prerequisite — always). Voting mechanics are now owned by `"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" agent dispatch-voters` (judge launch) and `"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" review tally-code-votes` (vote tally) — both invoked by `review core` automatically; no prompt-level read required. On zero-survivor `panel-failed`, bind `REVIEW_MODE=diff`, then **MANDATORY: READ ENTIRE FILE**: read `${CLAUDE_PLUGIN_ROOT}/skills/review/references/self-review.md` before executing the fallback.

## Work

Run the same mechanics documented in `/review` Steps 1-3:

1. **Step 1**: gather branch context via `review gather-context`.
2. **Step 2**: launch the full reviewer panel in parallel per the launch procedure and fallback matrix in `SKILL.md`.
3. **Step 3**: collect, deduplicate, vote (fixed cap of 2), implement fixes (Step 3e), re-review (Step 3f) — same round-state machine and safety limit as the inline path. Pass `--tier "$PANEL_TIER" --escalated-round "$ESCALATED_ROUND" --dynamic-archetypes "$DYNAMIC_ARCHETYPES" --prune-ledger "$REVIEW_TMPDIR/reviewer-prune-ledger.tsv"` to each `review core` round, let non-escalated round 2+ mechanically prune reviewer combos whose prior-round ledger data has no history, zero output, net score `<= 0`, or acceptance rate below `1/2`, and treat `prune-skipped` as prune-to-empty convergence; preserve the emitted scout/artifact KVs (`SCOUT_STATUS`, `SCOUT_FAIL_REASON`, `SCOUT_DIFFICULTY_RATING`, `SCOUT_DIFFICULTY_STATUS`, `DYNAMIC_SLOTS`, `SCOUT_MANIFEST`, `YIELD_TSV_FILE`, `FINDINGS_CLASSIFICATION_TSV_FILE`) for the parent Step 4 log batches, keep a per-round mapping for every non-empty classification TSV, read or update `$REVIEW_TMPDIR/findings-classification-round-map.env` as the stable round→path registry, return those round-scoped bindings explicitly in the final worker footer when available, and write Step 3e code edits to the git working tree directly. When `review core` returns `REVIEW_CORE_STATUS=panel-failed` with `THRESHOLD_REASON=no successful launched reviewer output`, execute the main-agent self-review pass in `skills/review/references/self-review.md`, including accepted-findings handoff and the required `review emit-tally` summary refresh. Continue Step 3 fix/checks/substantiality logic using self-review artifacts and refreshed summary files. Write `review-round-summary.md` and other artifact-contract files before returning to the parent; summary content must come from the self-review emit-tally pass, not the pre-self-review panel-failed emit.

Stop after Step 3 (do NOT run Steps 4 or 5 — those belong to the parent).

## Artifact Contract

Write these files under `$REVIEW_TMPDIR/` before returning:

- **`review-round-summary.md`** — human-readable summary the parent uses for Step 4: total rounds, per-round findings (reviewer breakdown, vote counts), voting summary (per round: `K` accepted, `N` rejected (`M` neutral)), Reviewer Competition Scoreboard, OOS items accepted, and convergence reason. The parent `/review` Step 4 prints this file verbatim only for standalone invocations; when `SESSION_ENV_PATH` is non-empty, Step 4 suppresses inline prose, copies the summary to `$(dirname "$SESSION_ENV_PATH")/review-round-summary.md`, and `/implement` reads that stable parent-tmpdir copy for the `code-review-tally` log batch.
- **`review-summary.json`** — structured summary the parent copies to `$(dirname "$SESSION_ENV_PATH")/review-summary.json` when `SESSION_ENV_PATH` is non-empty. Keep it ≤2 KB.
- **`rejected-findings.md`** — rejected in-scope findings (same format as the inline path). Write to `$(dirname "$SESSION_ENV_PATH")/rejected-findings.md` when `SESSION_ENV_PATH` is non-empty (so the parent `/implement` Step 5 finds it under `$IMPLEMENT_TMPDIR/rejected-findings.md`); write to `$REVIEW_TMPDIR/rejected-findings.md` for standalone invocations.
- **`review-dirty-tree-summary.env`** — dirty-tree aggregate (normally written by inline Step 5a): `ANY_DIRTY=true|false|unknown`, `LAUNCHERS_DIRTY=<comma-list>`, `RECOVERY_TAKEN=true|false`, and per-launcher path-stream keys. Write to `$(dirname "$SESSION_ENV_PATH")/review-dirty-tree-summary.env` when `SESSION_ENV_PATH` is non-empty (so the parent `/implement` Step 5 normal-mode dirty-tree check finds it); write to `$REVIEW_TMPDIR/review-dirty-tree-summary.env` for standalone invocations. Write this BEFORE returning so the parent's Step 5a finds it already present and skips re-aggregation.

Additionally write to the caller-env parent directory when `SESSION_ENV_PATH` is non-empty:

- **`$(dirname "$SESSION_ENV_PATH")/oos-accepted-review.md`** — accepted OOS findings (same path as the inline path; consumed by `/implement` Step 9a.1). Follows the existing OOS artifact format.

## Wait Discipline

NEVER return to the parent while any reviewer you launched is still running. The only allowed wait mechanism is a foreground `scripts/larch.sh agent collect-results` Bash tool call. Do not enter a "wait for notifications" state and surrender control; the parent treats an Agent-tool return as the heavy phase result.

**SendMessage dependency.** This worker subagent is dispatched via the Agent tool. If the parent Claude Code session does not have `SendMessage` available, any worker yield becomes a fatal stall. Standalone `/review --diff --subagent` users in environments without `SendMessage` should omit `--subagent`. See `AGENTS.md` for the project-wide reference.

## Mid-Run Dirty-Tree Probe Contract

After each external collection point (Step 2 launch → Step 3a collect), scan `${OUTPUT}.dirty-tree` sidecars and run `"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" dirty-tree checkpoint`. On `STATUS=dirty` or `STATUS=unknown`, automatically log and discard the reviewer-introduced changes — do NOT stash them and do NOT prompt the operator (same auto-discard flow as the inline path in `skills/review/SKILL.md` Step 3a). Track `RECOVERY_TAKEN` across the loop and write it into `review-dirty-tree-summary.env`.

## Return Value

Before returning success, write `$REVIEW_TMPDIR/review-summary.json` with the Write tool (not a heredoc or shell redirection). The JSON schema is:

```json
{
  "schema_version": 3,
  "rounds_completed": 1,
  "reviewer_output_paths": ["<abs-path>", "..."],
  "finding_counts": {
    "total_accepted": 0,
    "total_rejected": 0,
    "total_exonerated": 0,  // always 0; retained for backward compatibility
  },
  "accepted_count": 0,
  "rejected_count": 0,
  "exonerated_count": 0
}
```

`accepted_count`, `rejected_count`, `neutral_count`, and `exonerated_count` (always 0, retained for backward compatibility) are the canonical top-level counts. Mirror them under `finding_counts.total_accepted`, `finding_counts.total_rejected`, `finding_counts.total_neutral`, and `finding_counts.total_exonerated` for forward compatibility.

On success, return a terse KV block. The **first line** MUST be exactly `REVIEW_HEAVY=complete`. Optional additional `KEY=value` lines may follow; include `SCOUT_FAIL_REASON=<token>` when `SCOUT_STATUS=parse-failed`. When multiple rounds emit classification TSVs, return a round-scoped mapping (`FINDINGS_CLASSIFICATION_TSV_FILE_ROUND_1=...`, `FINDINGS_CLASSIFICATION_TSV_FILE_ROUND_2=...`, etc.) instead of only the final round path. For example:

```text
REVIEW_HEAVY=complete
REVIEW_SUMMARY_FILE=$REVIEW_TMPDIR/review-summary.json
SCOUT_STATUS=ok
DYNAMIC_SLOTS=2
SCOUT_MANIFEST=$REVIEW_TMPDIR/scout-round1-manifest.json
SCOUT_DIFFICULTY_RATING=$REVIEW_TMPDIR/scout-difficulty-rating.raw.json
YIELD_TSV_FILE=$REVIEW_TMPDIR/scout-archetype-yield.tsv
FINDINGS_CLASSIFICATION_TSV_FILE=$REVIEW_TMPDIR/findings-classification-round-1.tsv
FINDINGS_CLASSIFICATION_TSV_FILE_ROUND_1=$REVIEW_TMPDIR/findings-classification-round-1.tsv
FINDINGS_CLASSIFICATION_TSV_FILE_ROUND_2=$REVIEW_TMPDIR/findings-classification-round-2.tsv
```

No prose, no artifact content, and no blank lines between KV lines.

On failure (e.g., persistent reviewer outage, Step 3e checks that cannot be fixed), return only: `REVIEW_HEAVY=failed REASON=<short-token>`

Do not include reviewer transcripts, round summaries, panel scores, or fix diffs in the Agent-tool return text — those must remain in files.
