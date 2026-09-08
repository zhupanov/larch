---
# larch-run-lifecycle: shared-v1 skill=combine-issues
name: combine-issues
description: "Use when combining open issues to reduce issue count and token cost. Use /combine-issues --oos for OOS issues; checks actuality and merit before aggressive combinations."
allowed-tools: Bash, Read, Write
---

**MANDATORY: Follow the complete shared lifecycle contract in `${CLAUDE_PLUGIN_ROOT}/skills/shared/run-lifecycle.md` with declared skill `combine-issues`.**

# Combine Issues

**MANDATORY: READ ENTIRE FILE before composing user-facing prose: `${CLAUDE_PLUGIN_ROOT}/skills/shared/readability-style.md`.**

Reduce open issue count by merging related issues into combined ones. The primary goal is saving tokens — fewer, broader issues mean fewer `/design` + `/implement` execution cycles and less duplicated context loading.

## Flags

| Flag | Default | Purpose |
|------|---------|---------|
| `--oos` | off | OOS mode: operate only on open issues with a `[OOS]` title prefix. Checks each item for actuality and merit, discards stale items, stages low-merit rejections for approval, then proposes an aggressive combination scheme. See **OOS Mode** section below for the full step flow. |

When `--oos` is present, skip the standard Steps 1–3 below and follow the **OOS Mode** section instead.

## When to Combine

Good candidates share at least one of:

- **Same code area** — multiple issues touching the same file(s) or module.
- **Similar change pattern** — issues applying analogous edits to different files (e.g., "add error handling to script A" + "add error handling to script B").
- **Overlapping scope** — one issue is a subset of another, or both contribute to the same goal.
- **Sequential dependency** — issues that must land in order and are small enough to ship as one unit.

Do NOT combine issues that are genuinely independent and benefit from separate review (e.g., a bug fix and an unrelated feature).

## Session Temp Directory

Before writing any body files, create a unique temp directory for this run:

```bash
COMBINE_TMPDIR=$(mktemp -d)
```

Use `$COMBINE_TMPDIR/` for all body temp files. Never write to a hardcoded `/tmp/<name>` path; those paths collide when a prior run left a file at the same location.

<!-- step:1 — Fetch Eligible Issues -->

```bash
CLAUDE_PLUGIN_ROOT="$CLAUDE_PLUGIN_ROOT" "${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" combine-issues fetch
```

The helper fetches at most the 100 most recent open issues before title-prefix filtering. This is the complete candidate window for both standard and OOS modes. Title-prefix filtering and its Rust parity coverage live in `crates/larch-cli/src/combine_issues_commands.rs`.

Parse `ISSUES_FILE` and `COUNT` from stdout. If `COUNT=0`, print `No open issues eligible for combination.` and stop.

Read the JSON file at `$ISSUES_FILE` to get the full issue list (number, title, body, labels).

<!-- step:2 — Analyze and Propose Groups -->

Read each issue's title and body. Identify groups of 2+ issues that meet the combination criteria above. For each proposed group:

1. List the source issue numbers and titles.
2. State the combination rationale (which criterion from "When to Combine" applies).
3. Draft a combined title and a combined body that preserves all actionable content from the source issues.

Present all proposed groups to the user in a numbered list. If no groups are identified, print `No combination candidates found among <COUNT> open issues.` and stop.

Ask the user which groups to apply (e.g., "all", "1,3", or "none").

<!-- step:3 — Apply Approved Combinations -->

For each approved group, write the combined body to `$COMBINE_TMPDIR/group-<N>.md`, then invoke:

```bash
CLAUDE_PLUGIN_ROOT="$CLAUDE_PLUGIN_ROOT" "${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" combine-issues apply \
  --title "<combined title>" \
  --body-file "$COMBINE_TMPDIR/group-<N>.md" \
  --source-issues "<comma-separated issue numbers>" \
  --operator-invoked
```

Parse `COMBINED_ISSUE` and `CLOSED_ISSUES` from stdout. Print a summary line per group: `Combined #X, #Y, #Z → #<new> (<N> issues closed)`.

After all groups are applied, print a final tally: `Done — <N> issues combined into <M>, net reduction: <N-M>`.

## OOS Mode (`--oos`)

Operates only on open issues whose title starts with the `[OOS]` prefix followed by a space (prefix match, not substring). Fetches them, checks actuality and merit item-by-item, discards stale items, stages low-merit rejections for approval, and proposes an aggressive combination.

Resolve the target repository once before OOS commands that require `--repo`:

```bash
# lint-consecutive-bash: ok repo resolution and tempdir setup are separate documented setup steps
REPO=$("${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" gh resolve-repo 2>/dev/null || true)
if [ -z "$REPO" ]; then
  REPO=$(gh repo view --json nameWithOwner --jq '.nameWithOwner' 2>/dev/null || true)
fi
if [ -z "$REPO" ]; then
  echo "Could not determine repository."
  exit 1
fi
```

Create a unique temp directory for body files and state files for this run:

```bash
COMBINE_TMPDIR=$(mktemp -d)
```

Set these variable paths under `$COMBINE_TMPDIR` before running any OOS step:

- `SOURCE_TO_COMBINED_JSON=$COMBINE_TMPDIR/source-to-combined.json`
- `COMBINED_ISSUES_JSON=$COMBINE_TMPDIR/combined-issues.json`
- `BLOCKED_SOURCES_JSON=$COMBINE_TMPDIR/blocked-sources.json`
- `WRITE_RESULTS_JSON=$COMBINE_TMPDIR/write-results.json`
- `EXCEPTION_DECISIONS_JSON=$COMBINE_TMPDIR/exception-decisions.json`
- `DEPS_JSON=$COMBINE_TMPDIR/deps.json`
- `OPEN_ISSUES_JSON=$COMBINE_TMPDIR/open-issues.json`
- `INHERITED_PLAN_JSON=$COMBINE_TMPDIR/inherited-plan.json`
- `REFRESHED_OPEN_ISSUES_JSON=$COMBINE_TMPDIR/open-issues-refreshed.json`
- `FINAL_INHERITED_PLAN_JSON=$COMBINE_TMPDIR/inherited-plan-final.json`
- `AUDIT_OPEN_ISSUES_JSON=$COMBINE_TMPDIR/audit-open-issues.json`
- `EXISTING_EDGES_JSON=$COMBINE_TMPDIR/existing-edges.json`
- `DECIDED_EDGES_JSON=$COMBINE_TMPDIR/decided-edges.json`
- `PROSE_CANDIDATES_JSON=$COMBINE_TMPDIR/prose-candidates.json`
- `TIER2_CANDIDATES_JSON=$COMBINE_TMPDIR/tier2-candidates.json`
- `CLOSE_ELIGIBLE_JSON=$COMBINE_TMPDIR/close-eligible.json`
- `AUDIT_PLAN_JSON=$COMBINE_TMPDIR/audit-plan.json`

<!-- step:oos-1 — Fetch OOS Issues -->

```bash
CLAUDE_PLUGIN_ROOT="$CLAUDE_PLUGIN_ROOT" "${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" combine-issues fetch --repo "$REPO" --oos
```

OOS title-prefix filtering lives in the Rust `combine-issues` command owner.

Parse `ISSUES_FILE` and `COUNT` from stdout. If `COUNT=0`, print `No open [OOS] issues found.` and stop.

Read the JSON file at `$ISSUES_FILE`.

<!-- step:oos-2 — Actuality and Merit Check (per item) -->

OOS issues contain one or more items. Items follow two formats:

- **Single-item**: the entire body is one item (common for auto-filed single findings; body typically has `- **Description**: ...` and optional `- **Location**: ...` fields).
- **Multi-item**: body contains multiple `### Item N — <title>` sections, each with its own `**Location**:`, `**Severity**:`, etc. fields.

For each issue, parse its body and extract the individual items. Then for each item:

1. Read the `Location:` field (file path, optionally with a line number after `:`). Strip any trailing `:line` suffix to get the repo-relative path.
2. If the file does not exist in the repo:
   a. Search for in-flight implementing work through the typed issue command (never interpolate untrusted `Location:` text into shell command prose):
      ```bash
      "${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" issue search-implementing \
        --file-path "<repo-relative-path>" \
        --repo "$REPO"
      ```
      The command sanitizes the path to `[A-Za-z0-9/._-]`, searches at most 100 open issue candidates through the typed GitHub client, and requires the implementing issue title to match `^\[(DESIGNING|IMPLEMENTING)\]` followed by a space, with an explicit reference to the full sanitized path in title or body. `STATUS=ambiguous` or `STATUS=invalid_path` means the item is **not** blocked.
   b. If `STATUS=blocked` and `IMPLEMENTING_ISSUE=<M>` (a positive integer from the command output), the item is **blocked** — emit `Keeping item "<title>" from #<N>: referenced file <path> not yet created — blocked by #<M> ("<implementing title>").` Wire the blocked-by relationship using only the validated `IMPLEMENTING_ISSUE` value:
      ```bash
      "${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" block-issue add-blocked-by <N> <M> --repo "$REPO" --operator-invoked
      ```
      On failure, still keep the item as **actual** (not blocked) and emit a warning.
   c. If `STATUS=none` or `STATUS=invalid_path`, the item is **stale** — emit `Discarding item "<title>" from #<N>: referenced file <path> no longer exists.` and skip it.
   d. If `STATUS=ambiguous`, the item is **actual** (not blocked) — emit `Keeping item "<title>" from #<N>: referenced file <path> not yet created — implementing issue match ambiguous.` and continue to the merit gate.
3. If the file exists, read the relevant lines (±20 lines around the stated line when a line number is given). Assess whether the concern is still present:
   - If the code the item describes has been removed or the issue is clearly fixed, mark **stale** and emit a discard message.
   - If uncertain, default to **actual** (keep the item).
4. For each **actual** item that is **not blocked**, judge merit before adding it to the oos-3/oos-4 flat list:
   - Reject only when the rejection has a concrete cause tied to code you read or to documented repo principles. Use the simplicity-first (minimum code, nothing speculative) and surgical-changes (touch only what the task requires) principles as named anchors.
   - Proposed low-merit causes include speculative features, single-use abstractions, unrequested flexibility, defensive handling for impossible states, refactors of unbroken code, pure nits, lopsided cost/benefit, or concrete contradictions of documented decisions.
   - If the evidence is weak, subjective, or uncited, default to **keep**.
   - For each low-merit item, stage a proposed rejection with the source issue, item title, stable display key, and a 1-3 sentence cause. Do not combine, deduplicate, close, or discard the item until the oos-4 merit batch confirms rejection.
5. Collect only items that passed both actuality and merit, and are not blocked, into the flat list for oos-3/oos-4. **Blocked** items stay on their source issues. Proposed low-merit items stay only in the staged rejection list until the operator decides the merit batch. Do not include blocked or pending low-merit items in deduplication or combination, and never close a source issue solely because its remaining items are blocked or pending merit confirmation.

Track pending fully discarded source issues while checking actuality and merit. A source is fully discarded only when every item in that source is stale auto-discarded or confirmed merit-rejected, and no item is blocked, rescued, still actionable, pending merit confirmation, or consumed by a combined host. During oos-2, merit rejections are only proposed, so sources with proposed low-merit items remain pending and must not close yet. Build a pending discarded-source list with the source issue number, a redacted stale discard summary, proposed merit rejection keys and causes, proposed close reason `not planned`, and whether a comment file will be used. Sources whose items are all stale still behave like the previous stale-only closure list.

Do not call raw `gh issue close` from prompt prose. Reserve `scripts/larch.sh combine-issues close-sources` for **post-combination** source closures in oos-7 only: each close comment must keep the human-readable `Combined into #<target>` line and write the durable `larch:combined-away` marker used by `/analyze-issues`. Stale-only and confirmed fully discarded closures in oos-2 and oos-4 use `combine-issues close-stale` only and must **not** carry the combined-away marker. Do not invoke `combine-issues close-stale` during oos-2 before approval.

If no kept items remain after actuality and merit checks and no proposed merit rejections are pending, proceed to an approval prompt that shows only stale-only closures. After approval, close approved fully stale sources. Prefer per-issue `close-stale` calls when comments differ by issue:

```bash
CLAUDE_PLUGIN_ROOT="$CLAUDE_PLUGIN_ROOT" "${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" combine-issues close-stale \
  --repo "$REPO" \
  --issues "<issue>" \
  --reason "not planned" \
  --comment-file "<file>" \
  --operator-invoked
```

The comment file must be redacted and should summarize only that issue's stale discard lines. For batch calls, use a shared comment only when it is correct for every issue in the batch. Parse `CLOSED_ISSUES` and `PARTIAL` from stdout, and parse redacted `WARNING=` records from stderr. Report the stale closure tally and stop without entering combination planning. If approval is denied, leave the stale-only sources open, report them as not closed, and stop.

If proposed merit rejections are pending, continue to oos-4 even when zero kept items remain. The oos-4 merit batch must run before any merit-based discard or fully discarded source closure. If kept items remain, carry the pending discarded-source list forward into oos-4.

Emit a summary: `Actuality and merit check: <K> kept, <M> stale, <P> low-merit pending confirmation across <N> OOS issues.` When any blocked items were found, append: `(<B> item(s) kept as blocked by in-flight implementing issues.)`

<!-- step:oos-3 — Deduplicate -->

Remove duplicate items: two items are duplicates when they reference the same `Location:` and describe the same concern (identical or near-identical description). Keep the more detailed copy. Emit `Deduplication: removed <D> duplicate(s).` when `D > 0`.

<!-- step:oos-4 — Propose Aggressive Combination Scheme -->

Combine the remaining kept items into the **minimal** number of combined issues subject to one constraint: no single combined issue should be notably too large or risky to implement in one pass (rough heuristic: more than ~15 items or covering >5 unrelated subsystems is too large). If no kept items remain because all actual items are pending merit decisions, present zero groups and still run the merit and discarded-source approval gate.

Within that constraint, combine **aggressively** — unlike the standard mode, unrelated items may be grouped together if it reduces issue count. Prefer grouping by:

1. Same file or module (strongest signal).
2. Same severity or focus area.
3. Any remaining items: pack into a single catch-all combined issue unless it would exceed the size heuristic.

When any proposed merit rejections exist, show them before the combination scheme as one consolidated list:

```text
Rejected items (merit):
  <key> (#<source>) - <item title>: <1-3 sentence cause of rejection>
```

Include every staged merit rejection. The key must be stable within the proposal, such as `A`, `B`, or `item-3`. Do not prompt per item.

For each proposed combined issue:

1. List source issue numbers and item titles.
2. Draft a combined title (e.g., `[OOS] <brief theme> — <N> items`) and a combined body that uses `### Item N — <title>` format, preserving all actionable content.
3. Note which source issues will be closed. Use the oos-5 handled-item definition: stale auto-discarded, confirmed merit-rejected, or consumed into an approved group. Pending merit, blocked, rescued unconsumed, or otherwise unhandled items keep the source open.

Include stale-only and pending fully discarded source closures in the operator proposal. Show each source issue number, close reason `not planned`, the stale discard summary, proposed merit rejection keys and causes when present, and whether the close will use a comment file. Make clear that sources with pending merit items cannot close unless the operator confirms those merit rejections.

Present the proposed scheme to the user. State that approval covers three independent decisions: creating combined issues for kept items, closing stale-only or fully discarded source issues, and confirming the merit rejection batch. Ask: "Apply all groups, discarded-source closures, and merit rejections (yes), apply specific groups or closures and explicitly decide merit rejections, rescue by stable key (e.g. A or #12/A), or cancel (no)?"

Rescue matching (key-only):

- **Rescue matching uses stable display keys only.** Match against the keys shown in the `Rejected items (merit)` list (e.g., `A`, `B`, `item-3`). Do not match item titles or description substrings. When bare keys collide across sources, require `#source/key` form (e.g., `#12/A`).
- **Zero-match rescue**: keep all staged merit rejections pending; emit `Rescue matched no keys; all rejections remain pending.` Return to the operator for explicit keys or cancel. Do not confirm any rejection from that rescue text.
- **Multi-match rescue**: matched items stay on the staged rejection list. They are neither rescued nor confirmed rejected until the operator confirms the exact intended keys. Emit a re-confirmation prompt listing the matched items and their sources. Matched-but-unconfirmed items stay merit-pending and close-blocking.
- **Unambiguous unique-key rescue**: the named item moves to the kept-item set; unrescued listed items remain for the merit batch confirmation step.

Single-response ordering rule: in a single operator response, resolve rescue matching, including any multi-match key confirmation, before any merit-batch confirmation, including Apply all. Exclude confirmed-rescued keys from rejection. Apply all does not confirm merit rejections while any rescue text is zero-match or multi-match ambiguous.

Merit batch timing:

- Batch approval confirms rejection only for keys neither rescued nor left pending from unresolved rescue matching.
- Merit batch cannot run while any multi-match rescue awaits key confirmation.
- Confirmed rescued keys move from the staged rejection list to the kept-item set. Cancel leaves all merit rejections pending.

Deduplication timing:

- Rerun deduplication and grouping **only after confirmed rescues**. Do not trigger dedup after a zero-match or multi-match rescue attempt before key confirmation.

Post-rescue regrouping:

- After any confirmed rescue, rerun oos-3 deduplication and rebuild groups from the final kept-item set.
- If rescue changed kept-item membership or grouping partitions, re-present the combination scheme (groups, bodies, and tied stale-only closures) and require explicit operator approval again. A prior Apply all or partial approval does not authorize oos-5 apply or stale-only closes tied to the pre-rescue scheme.
- Do not enter oos-5 or close stale-only sources tied to a changed scheme until the operator approves the updated proposal with bodies matching the final kept set.
- Ordering: confirmed rescue → regroup → re-present and re-approve when the scheme changed → merit-batch confirmation for keys still staged → oos-5 only after regroup re-approval (when needed) and merit-batch resolution.

After approval and before dependency phases, close approved stale-only or fully discarded sources. Prefer per-issue `close-stale` calls when comments differ by issue:

```bash
CLAUDE_PLUGIN_ROOT="$CLAUDE_PLUGIN_ROOT" "${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" combine-issues close-stale \
  --repo "$REPO" \
  --issues "<issue>" \
  --reason "not planned" \
  --comment-file "<file>" \
  --operator-invoked
```

The comment file must be redacted and should summarize only that issue's stale discard lines and confirmed merit rejection causes. The comment must say the items were discarded as stale or out of line with repo principles when merit contributed. For batch calls, use a shared comment only when it is correct for every issue in the batch. Parse `CLOSED_ISSUES` and `PARTIAL` from stdout, and parse redacted `WARNING=` records from stderr. Include partial stale or fully discarded closes in final left-open tallies. Do not let a `close-stale` partial failure hide later combine results.

<!-- step:oos-5 — Apply with Deferred Closure -->

For each approved group, write the combined body to `$COMBINE_TMPDIR/oos-group-<N>.md`, then invoke:

```bash
CLAUDE_PLUGIN_ROOT="$CLAUDE_PLUGIN_ROOT" "${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" combine-issues apply \
  --repo "$REPO" \
  --title "<combined title>" \
  --body-file "$COMBINE_TMPDIR/oos-group-<N>.md" \
  --source-issues "<comma-separated issue numbers to close>" \
  --defer-close \
  --operator-invoked
```

Only list a source issue in `--source-issues` when all of its items were handled by this run. Handled means stale auto-discarded, confirmed merit-rejected, or consumed into an approved group. Blocked-on-source items, pending merit items, rescued items not consumed by an approved group, and uncombined kept items are not handled. If a source issue had stale items discarded or merit items confirmed rejected and every remaining item was consumed into combined issues, it can be deferred for closure. If a source issue contributed items to multiple groups, defer closure until all groups are applied. Never list a source issue in `--source-issues` when it still has blocked items, pending merit items, rescued unconsumed items, or any other unhandled item.

Parse `COMBINED_ISSUE`, `SOURCE_ISSUES`, `SOURCE_TO_COMBINED_JSON_FRAGMENT`, and `CLOSING_DEFERRED` from stdout. Stop using `CLOSED_ISSUES` from apply output as either a per-group tally or the final source-closure tally. Print: `Combined <source refs> → #<new> (source closure deferred).`

Record these state files for the dependency phases:

- `source_to_combined.json`: JSON object accumulated from `SOURCE_TO_COMBINED_JSON_FRAGMENT` values. The first fragment for a source stores a scalar combined issue number. A later fragment for the same source promotes the value to a sorted unique array. An existing array stays an array and receives the sorted unique union. Duplicate combined issue numbers are deduplicated. Preserve scalar values for sources that map to exactly one combined host. Keep the accumulated shape accepted by `_parse_source_to_combined`.
- `combined_issues.json`: JSON list of objects with `number`, `title`, and `source_issues`.
- `blocked_sources.json`: JSON object with `blocked_sources`, one object per source that remains blocked, pending, rescued but unconsumed, or otherwise unhandled. Each object has `source_issue`, `reason`, and optional `blocked_items`. Use reasons such as `blocked_on_source`, `merit_pending`, `rescued_unconsumed`, or `uncombined_kept_item`.

Materialize every dependency workflow JSON file before the first dependency command, even on no-op paths:

- `write_results.json`: `{"write_results":[]}`
- `exception_decisions.json`: `{"decisions":[]}`
- `blocked_sources.json`: `{"blocked_sources":[]}` when no sources are blocked, pending, rescued but unconsumed, or otherwise unhandled.
- `tier2_candidates.json`: `{"candidates":[]}` when Tier-2 is skipped or has no candidates.
- `existing_edges.json`: `[]` until populated from successful inherited writes.
- `decided_edges.json`: `{"decisions":[]}` until operator decisions are recorded.

<!-- step:oos-6 — Inherit Source Dependencies -->

Run the Rust planner for inherited native dependencies. Do not redo remap, classification, dedupe, or source-eligibility logic in prompt prose.

```bash
CLAUDE_PLUGIN_ROOT="$CLAUDE_PLUGIN_ROOT" "${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" combine-issues fetch-deps \
  --repo "$REPO" \
  --issues "<all source issues>" > "$DEPS_JSON"
CLAUDE_PLUGIN_ROOT="$CLAUDE_PLUGIN_ROOT" "${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" combine-issues list-open --repo "$REPO" > "$OPEN_ISSUES_JSON"
CLAUDE_PLUGIN_ROOT="$CLAUDE_PLUGIN_ROOT" "${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" combine-issues plan-inherited \
  --repo "$REPO" \
  --deps-file "$DEPS_JSON" \
  --source-to-combined-file "$SOURCE_TO_COMBINED_JSON" \
  --open-issues-file "$OPEN_ISSUES_JSON" \
  --combined-issues-file "$COMBINED_ISSUES_JSON" > "$INHERITED_PLAN_JSON"
```

Hard-stop if `fetch-deps`, `list-open`, or `plan-inherited` exits non-zero, or if any emitted JSON status is not `ok`. Do not feed empty, stale, or failed prerequisite files into the next dependency command.

Use `plan-inherited` as the only source of remapped inherited edge classification.

- Mark sources with dependency read failures as not close-eligible.
- Mark sources tied to unknown inherited classifications as not close-eligible.
- Write `safe_edges` with `"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" issue add-blocked-by --client-issue <client> --blocker-issue <blocker> --repo "$REPO" --operator-invoked`.
- Treat `satisfied_edges` as closed-blocker dependencies. Do not write them and do not treat them as close-blocking.
- Record safe-edge write success, idempotent already-present success, write failures, and unresolved writes in one write-results JSON file.

Write-results JSON is a top-level object with `write_results`. Each entry has `edge`, `client_issue`, `blocker_issue`, `phase`, `status`, `source_issues`, and optional `error`. Use phases `inherited_safe`, `inherited_exception`, `inherited_reclassified_safe`, `inherited_reclassified_exception`, `audit_tier1_safe`, or `audit_approved`. Use statuses `written`, `already_present`, `failed`, or `unresolved`. Only `written` and `already_present` count as successful writes for source closure.

<!-- step:oos-6b — Inherited Exception Gate -->

Run this gate before any source closure. For each inherited exception edge, show:

- Client issue number and title.
- Blocker issue number and title.
- Contributing source issues.
- Reason.

Ask the operator to approve or reject each edge. Write only approved exception edges. Record rejected inherited exceptions as deliberate non-inheritance decisions. Treat cancellation or missing answers as unresolved.

Exception-decisions JSON is a top-level object with `decisions`. Each entry has `edge`, `decision`, `phase`, `source_issues`, and `reason`. Use decisions `approved`, `rejected`, or `unresolved`. `unresolved` blocks source closure. `rejected` resolves the inherited exception without writing it.

Do not close sources tied to unresolved inherited exception decisions. Do not close sources tied to approved exception edges whose write failed.

<!-- step:oos-6c — Refresh Inherited Unknowns -->

Refresh metadata before source closure, then rerun the inherited planner.

```bash
CLAUDE_PLUGIN_ROOT="$CLAUDE_PLUGIN_ROOT" "${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" combine-issues list-open --repo "$REPO" > "$REFRESHED_OPEN_ISSUES_JSON"
CLAUDE_PLUGIN_ROOT="$CLAUDE_PLUGIN_ROOT" "${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" combine-issues plan-inherited \
  --repo "$REPO" \
  --deps-file "$DEPS_JSON" \
  --source-to-combined-file "$SOURCE_TO_COMBINED_JSON" \
  --open-issues-file "$REFRESHED_OPEN_ISSUES_JSON" \
  --combined-issues-file "$COMBINED_ISSUES_JSON" > "$FINAL_INHERITED_PLAN_JSON"
```

Hard-stop if refreshed `list-open` or `plan-inherited` exits non-zero, or if any emitted JSON status is not `ok`.

Compare the refreshed plan with the initial plan by edge tuple. The refresh resolves metadata races for newly created combined hosts or open blockers that become visible. Closed blockers classify as terminal `satisfied`, not unknown. Write newly safe inherited edges that were previously unknown. Record those writes in the shared write-results file with phase `inherited_reclassified_safe`. Surface newly classified exception edges through the same approval gate schema as `oos-6b`; record approved, rejected, and cancellation or missing-answer outcomes in the shared exception-decisions file with phase `inherited_reclassified_exception`. Write approved reclassified exception edges only after approval, and record those writes in the shared write-results file with phase `inherited_reclassified_exception`. Treat cancellation or missing answers as `unresolved`. Keep unresolved and still-unknown inherited edges close-blocking. Do not put unresolved or still-unknown inherited edges in `existing_edges.json`.

<!-- step:oos-7 — Close Consumed Source Issues -->

Compute closure eligibility in Rust.

```bash
CLAUDE_PLUGIN_ROOT="$CLAUDE_PLUGIN_ROOT" "${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" combine-issues close-eligible \
  --inherited-plan-file "$FINAL_INHERITED_PLAN_JSON" \
  --write-results-file "$WRITE_RESULTS_JSON" \
  --exception-decisions-file "$EXCEPTION_DECISIONS_JSON" \
  --source-to-combined-file "$SOURCE_TO_COMBINED_JSON" \
  --blocked-sources-file "$BLOCKED_SOURCES_JSON" > "$CLOSE_ELIGIBLE_JSON"
```

Close only sources emitted in `eligible_by_combined`. Sources mapped to multiple combined hosts remain ineligible until a canonical closure host is chosen. Sources listed in `blocked_sources.json` for `merit_pending`, `blocked_on_source`, `rescued_unconsumed`, `uncombined_kept_item`, or any other unhandled reason are not close-eligible. Partition eligible sources by their `source_to_combined` host. Invoke `close-sources` once per combined issue with only that combined issue's eligible source issues.

`close-eligible` ignores `satisfied_edges`; closed-blocker dependencies do not require writes and do not block source closure.

```bash
CLAUDE_PLUGIN_ROOT="$CLAUDE_PLUGIN_ROOT" "${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" combine-issues close-sources \
  --repo "$REPO" \
  --combined-issue "<combined issue>" \
  --source-issues "<comma-separated eligible source issues>" \
  --operator-invoked
```

Each `close-sources` invocation writes a two-line close comment: human-readable `Combined into #<target>`, blank line, then the exact HTML envelope `<!-- larch:combined-away source=#<source> target=#<target> -->` (for example `<!-- larch:combined-away source=#12 target=#99 -->`). `/analyze-issues` matches that HTML comment shape when docking combined-away filed OOS issues.

Run every partitioned `close-sources` invocation. Parse `CLOSED_ISSUES` and `PARTIAL` from stdout, and parse `WARNING=` lines from stderr. Treat partial closure as a warning path, not a hard stop. Reserve non-zero exit or `ERROR=` for argument and repository failures. Aggregate `CLOSED_ISSUES` from all invocations for the final source-closed tally. Keep ineligible, skipped, and failed-close source issues open. Summarize every source left open with the reason from `close-eligible` or the `WARNING=` text. Always continue to `oos-8`.

<!-- step:oos-8 — Audit Open Issues -->

After source closure, refresh the full open-issue set for audit.

```bash
CLAUDE_PLUGIN_ROOT="$CLAUDE_PLUGIN_ROOT" "${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" combine-issues list-open --repo "$REPO" > "$AUDIT_OPEN_ISSUES_JSON"
```

Hard-stop if audit `list-open` exits non-zero, or if its JSON status is not `ok`.

Include newly combined issues and archival open issues. Exclude closed source issues.

Build `existing_edges.json` as a JSON list of `[client_issue, blocker_issue]` pairs from actual inherited write results only. Include inherited edges with status `written` or `already_present`. Exclude rejected inherited exceptions, unresolved inherited exceptions, and still-unknown inherited edges.

Build `decided_edges.json` separately from inherited rejected exception decisions, inherited unresolved exception decisions, and audit decisions already made in this run. Do not use `decided_edges.json` as proof that an edge exists.

Run Tier-1 prose audit:

```bash
CLAUDE_PLUGIN_ROOT="$CLAUDE_PLUGIN_ROOT" "${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" combine-issues prose-audit \
  --repo "$REPO" \
  --combined-issues "$COMBINED_ISSUES_CSV" \
  --open-issues-file "$AUDIT_OPEN_ISSUES_JSON" \
  --existing-edges-file "$EXISTING_EDGES_JSON" \
  --source-to-combined-file "$SOURCE_TO_COMBINED_JSON" > "$PROSE_CANDIDATES_JSON"
```

`prose-audit` parses issue bodies and comments. It remaps consumed source references through `source_to_combined.json` before state checks, self-edge checks, dedupe, and output. It covers `Blocked by #N`, `Blocks #N`, and `Blocking #N` directions.

Then do best-effort Tier-2 semantic reasoning over bounded trigger pairs only. Build trigger pairs only from:

- Prose-audit candidate pairs.
- Inherited plan safe, exception, and unknown edge pairs.
- Explicit issue references involving a combined issue or consumed source issue.
- Shared exact file-path-like tokens between combined issue context and open issue context.

If the bounded trigger set exceeds 50 pairs, skip Tier-2 for the excess and summarize the skipped count. Record Tier-2 candidates in the same candidate schema with `source_kind=tier2_semantic`, `confidence=low|medium|high`, `edge=[client, blocker]`, and `reason`. Treat Tier-2 output as proposals only.

<!-- step:oos-9 — Audit Exception Gate and Writes -->

Plan audit writes in Rust.

```bash
CLAUDE_PLUGIN_ROOT="$CLAUDE_PLUGIN_ROOT" "${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" combine-issues plan-audit \
  --prose-candidates-file "$PROSE_CANDIDATES_JSON" \
  --tier2-candidates-file "$TIER2_CANDIDATES_JSON" \
  --existing-edges-file "$EXISTING_EDGES_JSON" \
  --decided-edges-file "$DECIDED_EDGES_JSON" \
  --open-issues-file "$AUDIT_OPEN_ISSUES_JSON" \
  --combined-issues-file "$COMBINED_ISSUES_JSON" > "$AUDIT_PLAN_JSON"
```

Hard-stop if `prose-audit` or `plan-audit` exits non-zero, or if any emitted JSON status is not `ok` when a status field is present.

Scope this gate to audit-derived candidates not already decided in `oos-6b` or `oos-6c`. Auto-write only Tier-1 safe audit edges from `auto_write_edges`. Ask the operator before writing every edge in `approval_required_edges`.

Approval-required audit edges include:

- Audit exception edges where the client is non-OOS and the blocker is a newly combined `[OOS]` issue.
- All Tier-2 semantic edges.

Before prompting, show the client issue number and title, blocker issue number and title, source kind, confidence when present, and reason. Do not write rejected edges. Write approved audit edges with `"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" issue add-blocked-by --client-issue <client> --blocker-issue <blocker> --repo "$REPO" --operator-invoked`. Treat idempotent already-present responses as success. Record audit write failures separately from inherited write failures. Count Tier-1 safe writes and approved audit writes in `audit_edges_written`.

<!-- step:oos-10 — Dependency Summary -->

Print dependency summary counts:

- Inherited safe edges written.
- Inherited exception edges surfaced, approved, and rejected.
- Inherited unknown edges and unknown edges reclassified.
- Audit Tier-1 safe edges written.
- Audit Tier-2 trigger pairs considered and skipped by bound.
- Audit Tier-2 edges surfaced.
- Audit exception edges surfaced.
- Audit approval-required edges approved and rejected.
- Total audit edges written.
- Duplicate edges skipped and self-edges skipped.
- Sources closed and sources left open.
- Stale-only or fully discarded sources closed and stale-only or fully discarded sources left open.
- Sources left open due to pending merit rejections.
- Dependency read failures and dependency write failures.

Count `close-stale CLOSED_ISSUES` separately from `close-sources CLOSED_ISSUES`, then include both in the total closed-source count. If any `close-stale` call returns `PARTIAL=true`, list the affected source issue as left open unless a later successful close is observed. Include redacted stale or fully discarded close warnings in the run summary. List sources left open with reasons.

## Anti-patterns

- **NEVER combine issues without user confirmation.** The analysis is advisory; the user decides which groups to merge. Combining the wrong issues loses important context that is hard to recover.
- **NEVER combine an issue that has a `[DESIGNING]`, `[IMPLEMENTING]`, `[STALLED]`, or `[DONE]` title prefix, nor legacy `[PLANNED]` / `[IN PROGRESS]` busy titles.** The fetch script filters these out, but if one slips through (e.g., prefix applied after fetch), skip it and warn. Note: `[DESIGNED]` issues are intentionally NOT excluded — they are valid combine candidates (design complete, implementation not yet started).
- **NEVER discard actionable content while merging source issues into a combined body.** The combined body must preserve every concrete task, file reference, and reproduction step from the originals. Confirmed merit rejections after oos-4 batch approval are permitted discards; summarizing away kept specifics is not.
- **NEVER auto-apply merit rejections before oos-4 confirmation.** Pending merit items stay open and close-blocking until the operator explicitly approves the merit batch.
- **NEVER let a newly combined `[OOS]` issue block a non-OOS issue without explicit operator approval.** The safe default is OOS work blocked by non-OOS work.
- **NEVER write Tier-2 semantic audit edges without explicit operator approval.** Tier-2 output is a proposal, not an auto-write source.
- **NEVER close a source issue when dependency reads, inherited writes, inherited classifications, blocked-source status, or inherited exception decisions for that source are unresolved.** Fail closed and list the source in the summary.
- **NEVER close sources for one combined issue using another combined issue's close command.** Partition source closure by `source_to_combined.json`.
- **NEVER duplicate inherited remap, classification, or closure-eligibility logic in prompt prose when CLI plan outputs are available.** Use `plan-inherited`, `close-eligible`, and `plan-audit` as the source of truth.
- **NEVER add rejected, unresolved, or still-unknown inherited edges to `existing_edges.json`.** That file is proof of existing edges only.
- **NEVER run Tier-2 semantic audit across the full open-issue Cartesian product.** Keep Tier-2 bounded to explicit trigger pairs.
