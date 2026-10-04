# Dialectic Clarifier Reference

**Consumer**: `/design` Step 2b drafter launchers, Step 4 tail, and Gate C.

**Contract**: surface only genuine bistable design forks at Gate C as advisory, display-only context. The clarifier never rewrites `plan.txt`.

**When to load**: load at `/design` Step 4b only when `skip_approve_requested=false` and fingerprint-valid dialectic candidates, status and digest, or manual candidates and digest exist under `$DESIGN_TMPDIR`. On the common `NO_CONTESTED_DECISIONS` path, on `--skip-approve` runs, or when only stale or unfingerprinted artifacts exist, skip the load and use `approval-gates-gate-c.md` alone.

## Detection bar

A candidate is valid only when all are true:

- It names **two concrete approaches**.
- It has a **material, non-obvious tradeoff**.
- It affects implementation risk or operator intent enough to merit Gate C attention.
- It is one of the top **one or two** decisions.

Do **not** classify scope questions, naming/style choices, ordinary library preferences, or internal implementation preferences as dialectic candidates.

## Candidate JSON

The optional drafter block appears after `LARCH_PLAN_END` and before `LARCH_SCOUT_BEGIN`:

```json
{"decisions":[{"id":"stable-id","title":"decision title","option_a":"approach A","option_b":"approach B","tradeoff":"material tradeoff","drafter_pick":"option_a","why_this_matters":"why Gate C should see this"}]}
```

Promoted files add `plan_fingerprint`, the sha256 of the exact final `plan.txt` bytes. `option_a` and `option_b` are display labels only. `drafter_pick` is `option_a` or `option_b` and names the side aligned with the current plan. **CHOSEN** is the side matching `drafter_pick`; **ALTERNATIVE** is the other side.

## Promotion timing

`parse_drafter_output()` may parse and validate the optional JSON, but it must not write `dialectic-clarifier-candidates.json`. The drafter launcher writes valid raw JSON to `$DESIGN_TMPDIR/.dialectic-raw-pending.json` before the subprocess exits. `dialectic-promote-candidates` consumes that sidecar only after terminal Step 2b postplan success (`POSTPLAN_RC=0`) and fingerprints the final `plan.txt` bytes. Clear `.dialectic-raw-pending.json` at Step 2b drafter start and after successful promotion.

Fallback may call `"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" design dialectic-write-candidates --design-tmpdir "$DESIGN_TMPDIR" --content-file <file>` after its postplan fence succeeds. Candidate absence is non-fatal. `dialectic-resolutions.md` remains an empty legacy placeholder for this clarifier flow.

## Stale invalidation

Any post-Step-2b `plan.txt` rewrite clears auto candidates and cached auto digest after the mutation chain succeeds. Authoritative choke points:

1. `scripts/larch.sh design step35-settle` after successful post-dedup `gate-b-dedup`, and again after successful `step2b-postplan` when `POSTPLAN_RC=0`.
2. The Rust plan-review owner at successful exit of its dedup pipeline when rc is 0, including the unchanged-plan path.
3. `scripts/larch.sh design step2b-postplan` when `plan.txt` bytes change by entry/exit hash compare.

Do not add prompt-side clears before dedup completes. `dialectic-clear-stale` removes stale auto candidates, stale digest/status, and `dialectic-manual-request.txt` unless fingerprint-valid manual candidates and manual digest/status still match the current plan. A lone manual request file never triggers deferred loading.

## Manual request artifact

`$DESIGN_TMPDIR/dialectic-manual-request.txt` is the sole manual-request path. Gate C writes the operator's `Other` text there without shell interpolation. `dialectic-manual --request-file "$DESIGN_TMPDIR/dialectic-manual-request.txt"` reads it. `--request <string>` is reserved for tests and internal callers.

Visible affordances:

- `debate <decision>: <option A> vs <option B>`
- `debate <candidate-id>` when fingerprint-valid candidates supply both options

## Debate profile

The clarifier uses a slim profile, not the old Step 2a.5 six-tag quorum:

- One Claude debater subprocess per side per decision, read-only.
- Compact steelman text per side.
- One `dialectic-ballot.txt` containing all capped decisions.
- Exactly three Claude judge subprocesses for that shared ballot.
- One vote line per `DECISION_N` per judge.
- Binary `THESIS` / `ANTI_THESIS` thresholds, position rotation, attribution stripping, parser tolerance, and disposition enum come from `skills/shared/dialectic-protocol.md`.

Ballot assembly maps **CHOSEN** to `THESIS` and **ALTERNATIVE** to `ANTI_THESIS`. Position rotation affects Defense A/B placement only. Digest labels remain **Option A** / **Option B** and include separate **Drafter pick** and **Panel lean (advisory)** lines.

## Child-process lifecycle

Launch each debater and judge through `scripts/larch.sh agent launch-claude-subprocess` in a `subprocess.Popen(..., start_new_session=True)` wrapper. Track wrapper PIDs as process groups. Run all subprocess work under a shared 300-600 second clarifier budget. On timeout, launch failure, or fail-open exit, terminate then kill every tracked process group, drain outputs, and continue Gate C.

Maintain `$DESIGN_TMPDIR/dialectic-clarifier-generation.txt` as a monotonic integer. Increment at the start of each auto or manual debate round, and record the active generation in `dialectic-clarifier-status.json` before launching subprocesses. Increment again on fail-open kill/timeout. Parent-owned status and digest writers must no-op when their embedded generation does not match the live file. Subprocess sidecars do not observe generation.

## Gate C timing

`design-step3b-tail.sh` runs `scripts/larch.sh design dialectic-gatec` in the foreground. Step 4 backgrounds the tail only when debate may run. `dialectic-gatec` writes `.completed/dialectic-gatec-terminal`; the tail writes `.completed/step-4` after preview completes.

Auto debate runs only when `skip_approve_requested=false` and fingerprint-valid candidates exist. On `--skip-approve`, no new auto debate launches; a fingerprint-valid cached digest may be displayed. On resume or Step 4b entry without fresh tail stdout, Gate C may re-read a fingerprint-valid digest before the prompt. On the normal same-turn path, tail stdout is authoritative and must not be duplicated.

## Digest and outcomes

Debate output is advisory. The operator remains judge of last resort. Digest stdout and markdown are display-only and must not drive orchestrator control flow.

Every untrusted steelman and rationale line is prefixed, and Markdown fence delimiters plus whole-line control rows are escaped so model text cannot break the advisory boundary. **Approve final design** approves the current `plan.txt`, not the panel lean. **Discuss further** is the path to revise the plan. External Codex/Cursor deep dialectic paths are out of scope.
