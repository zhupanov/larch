# Split-path: inline partition

**Consumer**: `/design` skill orchestrator at every Split-path entry: size-triggered routes, explicit `--partition` / `-p`, semantic-sprawl, Gate B, settle-dispatch, and Step 5c publish-size refusal.

**Contract**: the main agent validates the exact proposal, asks once, then delegates original conversion and leaf creation to `/umbrella`.

**When to load**: load when any Split-path entry condition triggers in `SKILL.md` or the referenced routing files.

This file is the normative partition procedure for every size trigger, explicit `--partition` / `-p`, semantic-sprawl route, Gate B route, settle-dispatch route, and Step 5c publish-size refusal.

## 1. Build the proposal inline

The main agent reads `$DESIGN_TMPDIR/plan.txt` when it exists. Before plan materialization, it reads `$DESIGN_TMPDIR/feature-description.txt` and the optional discussion artifact. Do not dispatch decomposition subagents or ask a preliminary Split question.

Write one risk-minimizing proposal to `$DESIGN_TMPDIR/decompose/inline-partition.md`. Put shared and risky foundations first. Use this exact shape for at least two pieces:

```markdown
## Pieces

### Piece 1: <title>
- Scope: <paths and behavior>
- Firm-headings: <bare parent-plan paths, comma-separated; no `###` or backticks>
- Acceptance: <concrete checks>
- Dependencies: none | blocked-by Piece N[, Piece M]
- Size estimate: <lines or effort>
```

Declare only necessary dependencies. Independent pieces stay independent.

## 2. Validate before the question

Run:

```bash
"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" decompose prepare \
  --design-tmpdir "$DESIGN_TMPDIR" \
  --partition-file "$DESIGN_TMPDIR/decompose/inline-partition.md" \
  ${ISSUE_NUMBER:+--issue-number "$ISSUE_NUMBER"}
```

`prepare` rejects one-piece proposals, missing metadata or coverage, bad references, and cycles. It writes only proposal-declared edges to `partition-deps.tsv`; it never adds serial edges. Repair an invalid proposal inline and rerun. Do not ask the operator to resolve partition internals.

## 3. Ask exactly one question

After preparation, make exactly one `AskUserQuestion` call.

- Valid proposal: offer **Partition into the listed pieces (Recommended)**, **Override size guardrail**, and Other/chat.
- Still invalid after inline repair: use the same question. Mark Partition unavailable and include the concise `DECOMPOSE_PARTITION_STATUS` reason. If selected, record the validation failure and end Split-path.
- Other/chat exits the structured Split-path. Do not ask another partition question.

Override keeps the existing warning and caller-specific continuation. No Split-path branch emits a second `AskUserQuestion`.

## 4. Convert through `/umbrella`

`prepare` preserves any leading topical prefix such as `[BUG]`, adds `split-<original-issue-number>-<piece-number>`, and reads the original identity from Step 0 state. Preserve the exact approved batch and TSV after approval.

Require Step 0 `CONTEXT_FILE` and remove any stale `$DESIGN_TMPDIR/decompose/umbrella-complete.sentinel`. Missing context fails closed before child invocation or GitHub mutation and preserves `$DESIGN_TMPDIR`.

Invoke `/umbrella` via the Skill tool:

- Try bare `umbrella` with `--lifecycle-parent-context "$CONTEXT_FILE"` first, then `--skip-approve`, `--prepared-root "$DESIGN_TMPDIR/decompose"`, `--prepared-input-file "$DESIGN_TMPDIR/decompose/partition-input.txt"`, `--prepared-deps-file "$DESIGN_TMPDIR/decompose/partition-deps.tsv"`, `--completion-sentinel "$DESIGN_TMPDIR/decompose/umbrella-complete.sentinel"`, and `$ISSUE_NUMBER`.
- Retry byte-identical arguments as `larch:umbrella` only for `Unknown skill: umbrella`; otherwise preserve the failure.

The child consumes the exact prior approval, keeps `/issue` deduplication, applies the prepared graph, converts the original in place to open `[UMBRELLA]`, and skips independent decomposition or approval.

## 5. Verify the child handoff

After the child returns, validate the completion proof against the live approved artifacts and target identity:

```bash
"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" umbrella verify-completion \
  --sentinel-file "$DESIGN_TMPDIR/decompose/umbrella-complete.sentinel" \
  --sentinel-root "$DESIGN_TMPDIR/decompose" \
  --prepared-input "$DESIGN_TMPDIR/decompose/partition-input.txt" \
  --prepared-deps "$DESIGN_TMPDIR/decompose/partition-deps.tsv" \
  --repo "$REPO" \
  --issue "$ISSUE_NUMBER"
```

Require exit zero, `UMBRELLA_COMPLETION_VERIFIED=true`, and exact `UMBRELLA_NUMBER=$ISSUE_NUMBER`. Any incomplete or stale proof preserves the open original and `$DESIGN_TMPDIR` without claiming success. Keep `decompose annotate`, `decompose migrate-deps`, and `decompose close-original` off the Split-path; `/umbrella` is the single mutation owner.

For Step 5c size refusal, accepted Partition is terminal. Export `SUMMARY_OUTCOME=approved-partition`, run the Final summary block, and exit `0`. Do not rerun Step 5c or continue against the converted original. Only Override reruns `"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" design step5c --session-env-path "$HOME/.cache/larch/sessions/current-design-env-$PPID.sh" --claude-pid "$PPID" --fresh-attempt`.

Panel dispatch and aggregate CLI commands remain available to their existing non-Split callers.
