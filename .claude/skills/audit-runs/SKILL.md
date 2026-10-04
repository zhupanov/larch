---
# larch-run-lifecycle: shared-v1 skill=audit-runs
name: audit-runs
description: "Use when auditing recently merged larch run logs for anomalies, filing the chain-of-history audit issue, and proposing user-approved bug follow-ups."
allowed-tools: Bash, Read
---

**MANDATORY: Follow the complete shared lifecycle contract in `${CLAUDE_PLUGIN_ROOT}/skills/shared/run-lifecycle.md` with declared skill `audit-runs`.**

# audit-runs

**MANDATORY: READ ENTIRE FILE before composing user-facing prose: `$PWD/skills/shared/readability-style.md`.**

Audit recently-merged larch run logs for the selected skill (`--skill=design|implement`) for anomalies. The current implement registry covers EXON/OOS/missing-file/NS-retry/self-deploying checks; the current design registry is intentionally narrower and ships cache-freshness plus guideline-assessment coverage. Always file a chain-of-history audit-report issue; record bug-issue candidates as proposals at scan time and act on them only after explicit user direction in chat.

This is a **dev-only** operator skill (`.claude/skills/`). It is NOT shipped with the plugin.

## Usage

```
/larch:audit-runs --skill <name> [<verbal-description>] [--repo owner/name] [--allow-concurrent]
```

(Plugin slash-alias: `/audit-runs …` may also resolve to this skill depending on marketplace wiring; prefer `/larch:audit-runs` when unsure.)

### Args

- `--skill <name>` (**required**): which skill’s run logs to audit. Closed enum: `design` or `implement`. Reject missing or out-of-enum values before any side effect (no GitHub call, no tmpdir). Parse from argv before preflight.
- `<verbal-description>` (optional positional): when present, describes which PRs to audit. When omitted or empty, treat as `since last audit` (same error paths as that form). Supported forms:
  - `last N PRs` — N most-recently-merged PRs targeting `main`
  - `since last audit` — PRs merged after the prior audit report's `audited_pr_range.last`; error if no prior report exists or no new PRs have merged (do NOT file an empty report)
  - `since <ISO8601-instant>` — PRs merged after that instant (same interpretable forms as GitHub `mergedAt`: `…Z` or explicit `±HH:MM` offset). This filter is **not** tied to the Pacific wall-clock convention used for audit report titles and `audit_timestamp`.
  - `#N` or `PR #N` — exactly one PR
  - Default when empty: treat as `since last audit` (requires a prior audit-report issue; continues to error if no prior report or malformed frontmatter — same as the explicit `since last audit` form)
- `--repo <owner/name>`: target repo. Default: `zhupanov/larch`
- `--allow-concurrent`: override the shared 5-minute `audit-report` concurrency guard (not skill-scoped)
- **Removed flag**: `--no-fix-issues` is not supported. If any token in the skill argv is exactly `--no-fix-issues`, refuse immediately with a clear usage error (flag removed); do not proceed or silently ignore it.

## Pre-flight

After parsing and validating `--skill` into `$SKILL`:

```bash
PREFLIGHT_OUT=$("${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" audit-runs preflight \
  --skill "$SKILL" --repo "<owner/name>" [--allow-concurrent])
```

Read `PREFLIGHT_OK`, `REASON`, and (on success) exactly one whole-line
`CORPUS_ROOT`. Preflight synchronizes immutable inputs exactly once, refuses a
partial or unsynchronized corpus, and no later audit step contacts storage.

## Bugs-backlog advisory

After `PREFLIGHT_OK=true` and before `resolve-prs`, run the advisory once:

```bash
NUDGE_RC=0
NUDGE_OUT=$("${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" audit-runs bugs-backlog-nudge \
  --repo "<owner/name>" \
  --root "$PWD" 2>"$TMPDIR/bugs-backlog-nudge.err") || NUDGE_RC=$?
```

If stdout is non-empty, print it to chat as advisory text only and retain it for early-exit paths. If the command fails, print a short non-fatal advisory failure, including stderr when helpful, and continue the audit. Do not include this output in the filed audit report body, YAML frontmatter, `compute-counters` input, or scan NDJSON.

## Verbal-Description Resolution

```bash
RESOLVE_OUT=$("${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" audit-runs resolve-prs \
  --skill "$SKILL" --repo "<owner/name>" [--verbal-description "<verbal-description>"])
```

Read `PR_LIST`, `PR_COUNT`, `IMPLICIT_SINCE_LAST_AUDIT`, `PRIOR_REPORT_NUMBER`, `RESOLVED_ECHO`, and `ERROR` from stdout. Fail-fast when `ERROR` is non-empty; print any non-empty `NUDGE_OUT`, then print `ERROR` to the user. Print `RESOLVED_ECHO` before scanning. Contract: `scripts/larch.sh audit-runs resolve-prs`.

## Scan Registry

The scan list is externalized in `.claude/skills/audit-runs/scans-$SKILL.tsv` (one row per scan: `name`, `type`, `pattern`, `expected_outcome`, `severity`). JSON emission, category filtering, and scan implementations live in `crates/larch-cli/src/audit_runs_commands.rs` behind `scripts/larch.sh audit-runs scan-run`; Rust unit and parity coverage owns the wire. **Adding a scan** requires coordinated updates: (1) a new `scans-$SKILL.tsv` row in the relevant per-skill registry, (2) matching Rust scan logic, (3) any Rust counter wiring when the scan feeds cumulative totals, (4) this `SKILL.md` scan table if the operator-facing baseline changes, and (5) hermetic Rust coverage for the new NDJSON shape and counter path. **Plan fidelity**: substantive changes to that surface (new counters, new cumulative YAML keys, or registry-wide behavior) should be tracked in their own issue/PR when they go beyond a routine scan-row + test update. Routine `changelog-rebase-conflicts` / `changelog_rebase_conflicts` / `ns_retries_cursor_specialist` wiring is part of this skill’s maintained baseline, not an ad-hoc add-on. **Operator parity with run-log audit-title hygiene on `main`**: audit-title and search-exclusion work assumes run logs and issue titles stay aligned with the same `^\[(Run Logs Audit |Implement Run Logs Audit |Design Run Logs Audit ).* Report\]` title regex used by the audit-report writer; the pre-lock `"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" git check-main-sync` probe (KV stdout: `SYNC_STATUS=not-main|ok|blocked|probe-error`, plus optional `AHEAD_COUNT` and `ERROR`) uses the locally cached `origin/main` ref (no fetch). If the probe fails with `SYNC_STATUS=probe-error`, operators must refresh the cached ref with a manual `git fetch origin main` before locking. This is the same freshness requirement as the audit-report title migration. Treat main-sync as a first-class preflight gate next to audit-title hygiene, not an undocumented side effect.

Read the registry at runtime:
```bash
SCANS_TSV="$PWD/.claude/skills/audit-runs/scans-$SKILL.tsv"
```

### Scans (baseline — see `scans-implement.tsv` / `scans-design.tsv` for the machine-readable registry)

Implement currently uses the full table below. Design currently uses `cache-freshness` and `guideline-assessment` unless/until `scans-design.tsv` grows additional rows.

| Scan | What | Where |
|---|---|---|
| Required-file presence | Compare against `docs/run-logs-required-files.tsv` (NDJSON `result` is `pass` / `fail` / `skip` / `error`) | run-log root |
| EXON misclassification | `\| FINDING_.* \| 0 \| 0 \| [1-9]+ \|.*\| rejected \|` | `round-*/voting-tally.md` |
| OOS category mangle | plan-review **accepted** rows only: non-empty `category` not in `{code-quality, risk-integration, correctness, architecture, security}` (code-review accepted prose categories are ignored by design) | `review-findings-full.jsonl` |
| NS-retry sidecars | `reviewer_signals[].ns_retry_reason` when present; legacy `*-ns-retry*` file fallback when the concise carrier is missing (`result:"skip"` only when both are absent) | `round-*/round-meta.json` (+ legacy `round-*/`) |
| Cursor CI stall causes | `cursor-ci-stall-*.json` sidecars: informational histogram of `.channel` values (pass when none) | `round-*/` |
| Codex round-1 adherence | round 2+ panel-manifest should not contain `tool=codex` | `round-N/panel-manifest.ndjson` |
| Codex generalist waste | `reviewer_signals` entry for `codex-generalist-output.txt` is `NO_ISSUES_FOUND` only AND timing > 120s (`result:"skip"` when carrier missing) | `round-1/round-meta.json` + `timing-report.json` |
| Execution-issues categories | non-Warnings entries in `execution-issues.ndjson` | `execution-issues.ndjson` |
| Cache freshness | `manifest.json::larch_version` vs latest plugin version (`result: informational` when the run lags current; empty `larch_version` remains `fail`; other rows may emit `skip`/`error`) | `manifest.json` |
| Guideline assessment | published Gate C guideline assessment is present and non-empty when the artifact exists (`result:"informational"` when absent) | `architectural-guideline-assessment.md` |
| Changelog rebase/conflicts (heuristic) | `execution-issues.ndjson` bodies mentioning changelog + rebase/conflict | `execution-issues.ndjson` |
| Coder tool | `CODER_TOOL` field | `round-*/coder.env` |
| Trailing-content NO_ISSUES_FOUND | `reviewer_signals[].first_pass_trailing_content == true` (`result:"skip"` when carrier missing; legacy `*-first-pass.txt` no longer primary) | `round-*/round-meta.json` |
| OOS silent drop | accepted non-security `### OOS_` blocks vs filed GitHub URLs, Inline-triage commit lines, and rejected-OOS markers in `oos-issues.ndjson` | `oos-accepted-*.md`, `oos-issues*.ndjson`, `oos-issues-created.md`, typed commit history of the run-log repo root |

## Scanning

```bash
# lint-consecutive-bash: ok map output feeds per-run scan loop
# Map each PR to its run-log directory
RUN_MAP_TSV=$("${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" audit-runs map-runs \
  --skill "$SKILL" --pr-list "$PR_LIST" --repo "<owner/name>" \
  --log-root "$CORPUS_ROOT/$SKILL")
# TSV: pr_number<TAB>run_id<TAB>started_at<TAB>larch_version<TAB>closes_issue
```

Then for each PR row in the TSV:

```bash
"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" audit-runs scan-run \
  --skill "$SKILL" \
  --run-dir "$CORPUS_ROOT/$SKILL/<RUN_ID>" \
  --pr <PR_NUM> \
  --scans-tsv "$SCANS_TSV" \
  --required-files-tsv "$PWD/docs/run-logs-required-files.tsv" \
  --current-version "<latest-plugin-version>" \
  > "$TMPDIR/scan-results-<PR_NUM>.ndjson"
```

Read `scan-results-*.ndjson` files as NDJSON (one JSON object per scan per line). Each line’s `result` is not limited to pass/fail: treat **`informational`**, **`skip`**, and **`error`** as first-class outcomes when writing the report (for example `cache-freshness` behind current vs missing inputs vs manifest/registry drift). Contract: `scripts/larch.sh audit-runs scan-run`.

**Cross-cutting checks (NDJSON + operator judgment):** the synthetic `cross-cutting` object (and `cache-freshness` / manifest fields) flags **manifest integrity** — empty `ended_at` / `pr_number`, and `manifest_pr_number_mismatch_with_audited_pr` / legacy `self_deploying_gap` when `manifest.json`’s `pr_number` disagrees with the audited PR (run-log vs merge skew / self-deploying version gaps). When `run_version < current_version`, `cache-freshness` emits **`result: informational`** (not `fail`): treat it as a self-deploying lens on the batch, not a defect signal versus the fix stream. **`proposed_new_issues` / `proposed_augmentations`** must be reconciled against **actually filed or closed** bug issues after the report (per **Post-report user prompt**); do not assume a proposal row implies an open issue without live verification through `scripts/larch.sh audit-runs issue-search`.

## Proposed bug-issue actions

At scan time, **only** record findings as proposals. **Never** auto-file a bug issue and **never** auto-post augmentation comments during the scan.

- **`proposed_new_issues`**: findings that warrant a new bug issue after classification: no matching **open** issue, and when the only matches are **closed**, the version-window check (below) does not suppress the proposal. When searching, use `scripts/larch.sh audit-runs issue-search`, which excludes only audit-report noise locally (all three audit-report families, the same regexes as `audit-runs title-match`). It **never** excludes `[IMPLEMENTING]` — those issues are open and match the search; route those hits to **`proposed_augmentations`** instead. Always present in the audit-report frontmatter (possibly empty).
- **`proposed_augmentations`**: findings that match at least one **open** issue (same keyword search). This includes titles beginning with `[IMPLEMENTING]` (still open on GitHub). Always present in the audit-report frontmatter (possibly empty).

For each finding, classify it into one of these two lists using GitHub + repo history; do not file or comment until after the post-report user prompt below.

1. **Search issues (open + closed):**
   ```bash
   "${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" audit-runs issue-search \
     --repo "<repo>" --keywords "<finding keywords>"
   ```
   Stdout is KV-shaped: `MATCH_COUNT=N`, then one `MATCH=<number>\t<state>\t<closedAt>\t<title>` row per surviving hit (empty `closedAt` for open issues). The search is capped at the 100 newest matching issues. The helper drops pull-request rows and audit-report-family titles locally; `[IMPLEMENTING]` titles are always retained. On `ISSUE_SEARCH_FAILED=true`, surface `REASON=` and fail the classification loudly.
2. **Open matches** (including `[IMPLEMENTING] …` titles): **`proposed_augmentations`**. In **`## Open issues snapshot`**, when an augmented issue’s title starts with `[IMPLEMENTING]`, note that the finding **recurred in this batch (pre-fix)** for that issue number.
3. **Closed matches only** (no open match for this finding): apply the **version-window** check before proposing `proposed_new_issues`:
   - Resolve fix merge time:
     ```bash
     "${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" audit-runs fix-merge \
       --repo "<repo>" --issue <N>
     ```
     Read `ISSUE_CREATED_AT=`, `ISSUE_CLOSED_AT=`, `MATCHED_PR=`, `MERGED_AT=`, `AMBIGUOUS=`, and `CANDIDATES=` from KV stdout. The helper reads the issue timing and the bounded merged-to-main PR history, keeps PRs referencing `#<N>`, and applies **PR disambiguation** below to pick **one** merged PR attributed to the fix. `MATCHED_PR=unknown` with `AMBIGUOUS=false` (zero candidates — including a fix PR older than the bounded history window, the documented degraded read) means: fall back to issue timing only (`ISSUE_CLOSED_AT`) and set `matched_pr` / merge metadata to unknown in your notes / `version_window_checks` rationale.
   - **PR disambiguation (normative, implemented by `audit-runs fix-merge`):** When multiple merged PRs match, the helper prefers the PR whose `body`/`title` contains an explicit closing reference for `#<N>` (`closes`, `fixes`, `resolved`, case-insensitive). If still tied, it prefers the PR with `mergedAt` closest **after** the issue `createdAt` (smallest positive delta). **If no candidate has `mergedAt` strictly after `createdAt`**, it uses the candidate with the latest `mergedAt`. If still ambiguous, it emits `AMBIGUOUS=true` with every surviving candidate in `CANDIDATES=` — **do not** silently suppress: treat as **in-scope** (`decision: propose`, `in_scope: true`) and record both PR numbers plus a one-line operator rationale in the audit-report prose (or in the `finding` slug row’s implied narrative). When `MATCHED_PR=unknown`, use issue `closedAt` only; keep `fix_shipped_in: unknown` unless you can attribute a merge elsewhere.
   - Find the next plugin version shipped after that instant and apply the version-window rule (file-shaped contract, not subject-line guessing alone):
     ```bash
     "${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" audit-runs version-window \
       --repo-root "$PWD" --after "<mergedAt-or-closedAt-ISO>" \
       --audited-versions "<comma-separated manifest.json::larch_version values for this batch>"
     ```
     Read `BUMP_SHA=` (`none` when no bump commit follows the instant), `FIX_SHIPPED_VERSION=` (`X.Y.Z` or `unknown`), `IN_SCOPE=`, and `DECISION=skip|propose` from KV stdout. The helper walks local first-parent history for the **earliest** commit after the instant whose subject contains `Bump version` and which changes `.claude-plugin/plugin.json`, reads `.version` from that commit's tree, and normalizes strictly: strip a single leading `v`, trim ASCII whitespace, require three **integer** components `MAJOR.MINOR.PATCH`, then compare **numerically per component** (so `1.10.0` is greater than `1.9.999`; never a naive string sort on the dotted token).
   - The helper's `DECISION` implements the normative rule; the semantics stay binding:
     - If **no** bump commit exists after that instant **or** `.version` is empty or malformed, `FIX_SHIPPED_VERSION=unknown` and `DECISION=propose` — **do not** skip the proposal solely for missing bump metadata (treat as in-scope for recurrence unless other reasoning applies). Record `fix_shipped_in: unknown`.
     - If either `FIX_SHIPPED_VERSION` or an audited `larch_version` fails the three-integer parse (for example `34.0.0-rc1`, extra dotted segments, or odd strings), that side is `unknown` for the inequality: you cannot prove the fix is strictly newer than **every** audited run, so `DECISION=propose` — record the parse gap in `version_window_checks`.
     - `DECISION=skip` (`IN_SCOPE=false`) only when `FIX_SHIPPED_VERSION` is known and **strictly greater than** every audited `larch_version` from this batch's run-map / scan inputs — the fix post-dates all audited runs, so **do not** propose a new issue for this closed-only match.
     - Otherwise (`FIX_SHIPPED_VERSION=unknown`, **or** `FIX_SHIPPED_VERSION ≤` any audited `larch_version`) the closed fix was in scope for at least one run → propose **`proposed_new_issues`** (recurrence).
4. **Record** each closed-issue evaluation in **`version_window_checks`** (see **Frontmatter**). Use `version_window_checks: []` when no closed issue was evaluated for any finding.

**Precedence:** any **open** match for the finding → `proposed_augmentations` only (even if older closed duplicates exist).

### Post-report user prompt

After the audit report issue is filed and prior reports are handled per **Close Prior Reports**:

1. Print the **full audit-report body** verbatim to chat (the same markdown submitted as the issue body), then print the **audit-report URL**.
2. **Zero-findings short-circuit**: if `proposed_new_issues` and `proposed_augmentations` are both empty, state `No findings — no bug issues to file.` and exit — do **not** ask the 3-way question.
3. **Otherwise**, ask the operator a 3-way question: (1) file/augment all, (2) discuss specific findings first, or (3) skip filing. Act on the response:
   - **File/augment all**: file new issues via `/larch:issue` (dedup ON); post augmentation comments with `"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" audit-runs comment --repo "<repo>" --issue <N> --body-file "$TMPDIR/audit-augment-<N>.md" --operator-invoked` (write the **Augmentation comment shape** markdown to that file first — same `--body-file` pattern as `issue create-one`; do not pass multi-line tables inline). The `--operator-invoked` flag is required; the shared issue-mutation owner authorizes, redacts, and read-back-verifies the comment. Read `COMMENT_POSTED=true` + `COMMENT_URL=` from KV stdout; on `COMMENT_FAILED=true`, surface `REASON=` to the operator — augmentation failures stay visible.
   - **Discuss first**: wait for operator direction; file or augment per finding only as approved.
   - **Skip filing**: exit cleanly; the audit report already captures proposed findings for the historical record.

4. **Post-report session summary (audit-report issue only):** **only when** an audit-report issue was actually filed (`issue create-one` returned a non-empty issue number / `AUDIT_REPORT_NUMBER`) **and** you did **not** end the run at step 2’s **zero-findings short-circuit** (that path exits immediately after the chat message — no 3-way walkthrough, **no** session-summary). After step 3’s per-finding walkthrough completes (filed, augmented, skipped, or mixed), compose `$TMPDIR/session-summary.md` and post it as a single comment on that audit-report issue (supplementary history). **Skip** this entire step whenever **no** audit-report issue exists — for example the zero-PR `since last audit` short-circuit (no `issue create-one` call), preflight/resolve failures before filing, or any other path that never yields an audit-report issue number (there is nothing to comment on).

   ```markdown
   ## Post-report session summary

   **3-way decision**: <file-all | discuss-first | skip-filing>

   **Per-finding actions**:

   | Finding | Decision | Filed as | URL |
   |---|---|---|---|
   | ... | filed-as-drafted \| modified \| skipped | #N or — | url or — |

   **Augmentations**:

   | Target issue | Action | Comment URL |
   |---|---|---|
   | #N | posted \| skipped | url or — |

   ---
   *Posted by /audit-runs post-report session-summary step.*
   ```

   ```bash
   "${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" audit-runs comment \
     --repo "<repo>" --issue "$AUDIT_REPORT_NUMBER" \
     --body-file "$TMPDIR/session-summary.md" --operator-invoked
   ```

   **When** this step runs, even **skip-filing** should populate the tables with **skipped** rows (useful operator history). Omit empty **Augmentations** table section when there were no augmentation rows. If the comment fails (`COMMENT_FAILED=true` or a refusal), print its `REASON=` to chat but **do not** fail the overall audit run (this comment is supplementary, best-effort).

The audit report issue is **never** edited after creation (chain-of-history).

### Augmentation comment shape

```markdown
**Additional data from <PR list>:**

| PR | Count |
|---|---|
| #N | M occurrences |

Previous cumulative: X → Now: X+M
```

## Audit Report

Always file an audit report after the scan, EXCEPT when the scope is `since last audit` (including an empty/omitted positional normalized to that form per **Verbal-Description Resolution**) and the query yields zero new PRs (exit cleanly without filing).

### Title Format

```bash
PACIFIC_OUT=$("${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" audit-runs pacific-timestamp)
PACIFIC_TIMESTAMP=$(printf '%s\n' "$PACIFIC_OUT" | sed -n 's/^PACIFIC_TIMESTAMP=//p')
# → PACIFIC_TIMESTAMP=2026-05-20T21:59-07:00

TITLE_OUT=$("${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" audit-runs title \
  --skill "$SKILL" --pr-list "$PR_LIST" --timestamp "$PACIFIC_TIMESTAMP")
# stdout is KV-shaped: each line is `KEY=value`. The title script prints `TITLE=...` (not a bare title string).
TITLE=$(printf '%s\n' "$TITLE_OUT" | sed -n 's/^TITLE=//p')
```

Contracts: `scripts/larch.sh audit-runs pacific-timestamp`, `scripts/larch.sh audit-runs title`.

### Label

`audit-report` (must exist in the repo; verify before filing):

```bash
"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" audit-runs label-check \
  --repo "<repo>" --label audit-report
```

Require `LABEL_PRESENT=true` (exact, case-sensitive local match). On `LABEL_PRESENT=false` or `LABEL_CHECK_FAILED=true`, stop before filing.

### Filing Method

Use `issue create-one` directly (bypasses the batch parser's `###` heading-trap):
```bash
"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" issue create-one \
  --title "<title>" \
  --body-file "$TMPDIR/audit-report-body.md" \
  --label "audit-report" \
  --repo "<repo>" \
  --operator-invoked
```

The `--operator-invoked` flag is required for direct operator-invoked filing. Do not omit it. Read `ISSUE_NUMBER=` (plus `ISSUE_URL=`) from KV stdout; `ISSUE_FAILED=true` + `ISSUE_ERROR=` reports a failed filing.

### Counter Computation

Before composing the report body, run `scripts/larch.sh audit-runs compute-counters` to get cumulative totals:

```bash
COUNTERS_OUT=$("${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" audit-runs compute-counters \
  --scan-results-dir "$TMPDIR" \
  [--prior-frontmatter "$TMPDIR/prior-report-body.md"])
```

Read `SCAN_FILES_FOUND`, `EXON_MISCLASSIFICATIONS`, `EXON_DELTA`, `OOS_CATEGORIES_MANGLED`, `OOS_MANGLED_DELTA`, `OOS_CATEGORIES_CLEAN`, `OOS_CLEAN_DELTA`, `OOS_CATEGORIES_BLANK`, `OOS_BLANK_DELTA`, `NS_RETRIES_CURSOR_SPECIALIST`, `NS_RETRIES_DELTA`, `CHANGELOG_REBASE_CONFLICTS`, `CHANGELOG_DELTA`, and `CATEGORY_STATS_PARTIAL`. `CATEGORY_STATS_PARTIAL=true` when any PR’s `category-stats` line has `partial_data: true`: missing JSONL, or malformed/mangled aggregate unavailable. `OOS_*_DELTA` for clean/blank omit category-stats only for the missing-file case. Contract: `scripts/larch.sh audit-runs compute-counters`.

### Frontmatter (YAML block between `---` markers at top of body)

`audit_timestamp` matches **Title Format** `<Pacific-ISO-timestamp>`: Pacific wall time with explicit `-07:00` or `-08:00` and minute precision from `scripts/larch.sh audit-runs pacific-timestamp` (`PACIFIC_TIMESTAMP_SOURCE=tz_america_los_angeles`). It is **not** the `since <ISO8601-instant>` filter convention. Populate `cumulative_counters` from `scripts/larch.sh audit-runs compute-counters` output keys below.

```yaml
audit_schema_version: 1
audit_timestamp: <Pacific-ISO-timestamp>
audited_repo: <owner/name>
audited_pr_range:
  first: N
  last: M
  count: K
audited_prs: [N, ..., M]   # explicit list when range has gaps
prior_report_issue: <N | null>
proposed_new_issues: [...]        # always present; findings with no matching open issue
proposed_augmentations: [...]     # always present; findings matched to an existing open issue
version_window_checks:           # always present; one row per closed-issue match evaluated (possibly empty [])
  - finding: <slug>
    matched_issue: <N>
    matched_state: closed
    fix_shipped_in: vX.Y.Z | unknown
    audited_versions: [34.0.0, 34.0.1]
    in_scope: false
    decision: skip | propose
  # Example A — fix shipped strictly after every audited run → suppress new issue
  - finding: exon-regression-example
    matched_issue: 2401
    matched_state: closed
    fix_shipped_in: v35.0.0
    audited_versions: [34.0.0, 34.0.1]
    in_scope: false
    decision: skip
  # Example B — unknown shipped version → cannot prove post-dates batch → propose recurrence
  - finding: oos-mangle-example
    matched_issue: 2402
    matched_state: closed
    fix_shipped_in: unknown
    audited_versions: [34.0.0]
    in_scope: true
    decision: propose
  # Example C — fix version overlaps at least one audited run → in scope
  - finding: ns-retry-example
    matched_issue: 2403
    matched_state: closed
    fix_shipped_in: v34.0.0
    audited_versions: [34.0.0, 34.0.1]
    in_scope: true
    decision: propose
cumulative_counters:
  exon_misclassifications: N
  oos_categories_mangled: N
  oos_categories_clean: N
  oos_categories_blank: N
  ns_retries_cursor_specialist: N
  changelog_rebase_conflicts: N
```

### Report Sections (in this order, exact `##` headers with a trailing space before the title; use `####` for internal subheadings)

- `## Summary` (when any audited run’s `cache-freshness` line shows `run_version` strictly less than `--current-version`, prepend a bold one-line banner immediately under the heading: **Self-deploying lens:** runs in this batch were on `<run_version(s)>`; current `main` is `<current-version>`. Use the scan NDJSON values; do not invent versions.)
- `## Delta from prior audit` (omit when `prior_report_issue` is null)
- `## Per-PR findings` (one `####` subsection per PR)
- `## Open issues snapshot` (list every open audit-eligible issue: number, title, last-seen-symptom-count; when an issue title begins with `[IMPLEMENTING]` and received an augmentation for this batch, annotate that it **recurred this batch (pre-fix)**)
- `## Scan results` (table: scan-name → pass/fail/finding count, plus issue cross-references)

## Close Prior Reports

After the new audit report is filed:

```bash
"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" audit-runs close-priors \
  --skill "$SKILL" --new-issue-number "<ISSUE_NUMBER>" --repo "<repo>" \
  --operator-invoked
```

The `--operator-invoked` flag is required. Omitting it causes a refusal before any network call.

Stdout is KV-shaped. A verified close emits `CLOSED_NUMBER=<N>`. A close failure can still exit `0` while emitting `CLOSE_FAILED=<N>` then a **TAB**-separated `REASON=...` continuation on the same line. If the bounded issue list fails up front, the CLI prints `ISSUE_LIST_FAILED=true` plus `REASON=...` and exits non-zero. If more than one matching prior is found, it refuses before any comment or close with `CLOSE_PRIORS_REFUSED=true`, `REASON=ambiguous-match`, and the mutation-refusal exit code. The shared issue-mutation owner authorizes the mutation and verifies the comment and close read-back. After any `scripts/larch.sh audit-runs close-priors` invocation, scan stdout for `CLOSE_FAILED=` / `ISSUE_LIST_FAILED=` / `CLOSE_PRIORS_REFUSED=` even when the exit code is `0`. Do not treat `CLOSED_NUMBER=` as unconditional full success.

Contract: `scripts/larch.sh audit-runs close-priors`.

## Output to chat

Only after the new audit report issue is filed and prior reports are handled per **Close Prior Reports** (same sequencing precondition as `### Post-report user prompt` above), the orchestrator MUST surface to chat (in order):

1. The **full audit-report body**, verbatim (same content as the filed issue body).
2. The **audit-report URL**.
3. Either the **zero-findings short-circuit** message (`No findings — no bug issues to file.`) when both proposal lists are empty, **or** the **3-way question** about filing/augmenting when there is at least one proposed item.

Bug-issue filing and augmentation happen only after operator response to that question (unless skipped); they are not part of this mandatory chat tail when the short-circuit applies.

## Output

Optional stdout-style summary after the chat contract (for example per-scan PASS/FAIL counts). Lists of bug issues filed or augmented belong here only **after** the operator has directed those actions — not at scan time.

## Preconditions

- Working tree must be a clone of `--repo`
- `audit-report` label must exist in the target repo (created by the bootstrap audit report — treat its existence as a precondition assertion, not something to create on each invocation)
- `docs/run-logs-required-files.tsv` must exist in the repo root (for the Required-file presence scan)
- parsed `CORPUS_ROOT/<skill>/` must exist after the one synchronization pass

## Revised Orchestrator Flow

```
parse --skill (design|implement) → $SKILL; fail-fast if missing/invalid
scripts/larch.sh audit-runs preflight --skill $SKILL → PREFLIGHT_OK + CORPUS_ROOT / fail-fast
scripts/larch.sh audit-runs bugs-backlog-nudge --repo <owner/name> --root $PWD → chat-only advisory
scripts/larch.sh audit-runs resolve-prs --skill $SKILL → full stdout KV contract
scripts/larch.sh audit-runs map-runs --skill $SKILL → run-map.tsv
for each PR:
  scripts/larch.sh audit-runs scan-run --skill $SKILL → scan-results-NNNN.ndjson
scripts/larch.sh audit-runs compute-counters    → COUNTERS_OUT (KV lines on stdout; treat as counters input)
scripts/larch.sh audit-runs issue-search / fix-merge / version-window → classification KV inputs
[LLM: classify proposed_new_issues / proposed_augmentations from those KV outputs and record version_window_checks]
scripts/larch.sh audit-runs pacific-timestamp → PACIFIC_TIMESTAMP (extract from stdout KV)
scripts/larch.sh audit-runs title --skill $SKILL → TITLE
[LLM: write report prose — Summary, Delta, Per-PR findings, Open issues, Scan results table
       reading from COUNTERS_OUT + scan-results-*.ndjson as structured input]
scripts/larch.sh audit-runs label-check → LABEL_PRESENT=true precondition
scripts/larch.sh issue create-one → file audit report
scripts/larch.sh audit-runs close-priors --skill $SKILL → close one unambiguous prior audit-report issue for this skill
[LLM: post-report 3-way question if proposed issues exist — or zero-findings short-circuit when both proposal lists are empty]
[LLM: if audit-report issue number exists: post session-summary via scripts/larch.sh audit-runs comment; else skip (no report filed)]
```

## Scripts

- `scripts/larch.sh audit-runs title-match`: per-skill audit-report title matching
- `scripts/larch.sh audit-runs preflight`: typed Git fetch/pull, repo-identity, concurrency guard, and one corpus synchronization
- `scripts/larch.sh audit-runs bugs-backlog-nudge`: chat-only `/learn-from-bugs` backlog advisory
- `scripts/larch.sh audit-runs resolve-prs`: verbal-description → PR_LIST
- `scripts/larch.sh audit-runs map-runs`: PR → run-log directory mapping (TSV)
- `scripts/larch.sh audit-runs scan-run`: all scans against one run-log dir; NDJSON output
- `scripts/larch.sh audit-runs compute-counters`: sum scan deltas + prior totals; KV output
- `scripts/larch.sh audit-runs pacific-timestamp`: portable Pacific timestamp
- `scripts/larch.sh audit-runs title`: generate report title string
- `scripts/larch.sh audit-runs close-priors`: close one unambiguous prior audit-report issue
- `scripts/larch.sh audit-runs issue-search`: open+closed proposal-classification search with local audit-title exclusion
- `scripts/larch.sh audit-runs fix-merge`: fix-PR disambiguation plus issue timing (KV output)
- `scripts/larch.sh audit-runs version-window`: earliest post-instant plugin bump plus the strict version-window decision
- `scripts/larch.sh audit-runs label-check`: exact `audit-report` label precondition
- `scripts/larch.sh audit-runs comment`: authorized, redacted, read-back-verified supplementary comment
- `crates/larch-cli/tests/audit_runs.rs`: Rust audit scan, counter, timestamp, title, advisory, closure, and cutover-verb wire coverage

## Anti-patterns

- Do NOT file an empty audit report (zero PRs audited)
- Do NOT recurse: the skill must not audit its own audit-report issues
- Do NOT close prior reports before the new one is confirmed filed (ISSUE_NUMBER from issue create-one is non-empty)
- Do NOT `gh issue create` directly — use `issue create-one` for audit reports and `/larch:issue` for bug issues
- Do NOT auto-file or auto-augment bug issues — only file the audit report itself at scan/report time. Bug-issue actions require explicit user direction in chat.
- Do NOT ask the 3-way question when there are zero findings — state `No findings — no bug issues to file.` and exit.
