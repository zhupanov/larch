# Voting Protocol

Shared voting protocol for adjudicating review findings. Used by `/design` (plan review) and `/review` (code review). This protocol **replaces** the Negotiation Protocol for `/design` and `/review`. `/research` continues using the Negotiation Protocol in `external-reviewers.md`.

## Overview

After deduplication, a panel casts YES/NO votes on each finding. `/design` plan review normally uses three voters (Claude, Codex, Cursor). `/review` and `/implement` Step 5 code review use three fixed slots: `codex-validity`, `codex-plan-fidelity`, and `codex-pragmatism`; each waterfalls Codex, then Cursor, then Claude. Full-tier findings need 2+ YES votes. Unavailable voters degrade through the tier table and never fail open. `/review` dispatch is `"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" agent dispatch-voters`; tally is `"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" review tally-code-votes`. Original reviewers earn points from vote outcomes.

## Ballot Format

Assign each deduplicated finding a stable sequential ID before voting. Ballots use one `### FINDING_N:` markdown block per finding. `/design` plan review (`scripts/larch.sh plan-review tally`, implemented by the Rust CLI) also splits `### OOS_N:` blocks. `/review` code review (`review tally-code-votes`) accepts both `### FINDING_N:` and `### OOS_N:` headings; legacy OOS code-review rows may still be `### FINDING_N:` headings with `[OUT_OF_SCOPE]` in the title:

```markdown
### FINDING_1: <short title>
- **Reviewer**: anonymous
- **Concern**: <finding description>
- **Suggested revision**: <what to change>

### FINDING_2: <short title>
- **Reviewer(s)**: anonymous
- **Concern**: <finding description>
- **Suggested revision**: <what to change>
```

Prepend voter instructions as parser-ignored prose before the first `### FINDING_N:` block. Voter-facing ballots must hide proposer identity: keep `Reviewer` / `Reviewer(s)` labels but set them to `anonymous`, while scoring and audit use `proposer-map.tsv`. Body text is not scrubbed. Tally attribution stays skill-specific: `/design` uses `Code` / `Codex` / `Cursor`; `/review` uses specialist labels (`Correctness`, `Testing`, `Edge-cases`, `Codex-Correctness`, `Codex-Testing`, `Codex-Edge-cases`). Simple panels add `Claude-Generic` and a smaller external-specialist set. `/research` uses the Negotiation Protocol, not voting.

## Voter Output Format

Each voter outputs one line per ballot item, **using the same ID that appears on that run's ballot heading**:

- **`/design` plan review**: in-scope headings are `### FINDING_N:`, OOS headings are `### OOS_N:` — vote lines use `FINDING_N:` and `OOS_N:` respectively.
- **`/review` code review**: vote lines use the same ID form as the ballot heading. In-scope headings use `FINDING_N:`; OOS headings may use `OOS_N:`. Legacy `[OUT_OF_SCOPE]` rows under `FINDING_N:` still vote with `FINDING_N:`.

YES votes require no reason; NO votes require a one-line reason:

```
FINDING_1: YES CORRECTNESS=true SEVERITY=major QUALITY=good UNCERTAIN=false
FINDING_2: NO CORRECTNESS=false-positive SEVERITY=nit QUALITY=no-fix UNCERTAIN=false — <one-line reason>
OOS_1: YES CORRECTNESS=true SEVERITY=minor QUALITY=adequate UNCERTAIN=false
OOS_2: NO CORRECTNESS=false-positive SEVERITY=nit QUALITY=no-fix UNCERTAIN=false — <one-line reason>
...
```

Valid vote tokens are `YES` and `NO`; legacy stray `EXONERATE` maps to `NO`. If output has valid votes for some findings and misses others, keep the valid votes; missing entries produce per-voter `JUDGE_ERROR` parser fallback. `JUDGE_ERROR` does not lower the panel tier; quorum is based on available voter files for the round.

## Threshold Rules

| Eligible Voters | YES Votes Required | Notes |
|---|---|---|
| 3 | 2+ | Standard majority |
| 2 | 2 (unanimous) | When one voter unavailable/timed out |
| 1 | 1 | Binding single-judge decision; YES accepts, NO rejects |
| 0 | Main agent decides | No automated vote; main agent reads ballot as untrusted data and adjudicates |

Dispatchers warn when effective voters fall below the expected panel size. For `/design`, `/review`, and `/implement` Step 5, expected size is three when Cursor or Codex lanes are available, or **one Claude fallback voter** when neither external lane is active. Voters waterfall Codex, then Cursor, then Claude. The code-review single-Claude floor warns only if that expected judge fails; plan review preserves its `1/3 ... quota hit` warning even when the floor succeeds. `effective` means not `failed` and substantive enough to contribute valid vote lines after retries. On the three-slot path, `ELIGIBLE_VOTERS` and `EFFECTIVE_VOTERS` count only substantive non-empty voter files after parse-rate removal; empty placeholders keep `vN_tool` attribution but do not inflate quorum.

After thresholding, each finding becomes `accepted`, `neutral` (≥1 YES but below threshold; -0.25 points unless neutral rescue routes it to OOS), or `rejected` (0 YES; −1 point). `crates/larch-core/src/review/voting.rs::classify_result` owns classification; tally commands map labels to KV and JSON at emission. Neutral rescue keeps `Result=neutral` in the vote table, but routes a single-YES `major` neutral to OOS artifacts with `scope=oos`. Single-YES `minor`, `nit`, missing, or invalid severities stay dropped.

## Voter Panel Composition

**For plan review** (`/design` Step 3), `scripts/larch.sh plan-review voter-dispatch` launches the same Codex-primary waterfall as the code-review panel below. If both external tools are unavailable, it launches only slot 1 as a Claude floor voter and marks slots 2 and 3 failed, `not-run`:
- **Voter 1** (`v1`): `codex-validity`: `render voter --voter-tool <active-tool>`
- **Voter 2** (`v2`): `codex-plan-fidelity`: `render voter --voter-tool <active-tool>`
- **Voter 3** (`v3`): `codex-pragmatism`: `render voter --voter-tool <active-tool>`

**For code review** (`/review` Step 3 and `/implement` Step 5): `"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" agent dispatch-voters` launches three fixed voter slots, using **canonical slot indexing** (`v1`/`v2`/`v3` always map to validity/plan-fidelity/pragmatism, never to compacted surviving voters). All three voters use Codex-primary waterfall dispatch (Codex, then Cursor, then Claude) and may fall through to the configured external fallback labels:
- **Voter 1** (`v1`): `codex-validity`: `render voter --archetype validity-correctness`
- **Voter 2** (`v2`): `codex-plan-fidelity`: `render voter --archetype plan-fidelity-completeness`
- **Voter 3** (`v3`): `codex-pragmatism`: `render voter --archetype pragmatism-cost`

When both external voter lanes are unavailable, the panel falls back to a **single Claude floor voter** at slot 1 (`agent launch-claude-review --role voter`, binding-single tier); slots 2-3 are empty placeholders that keep their `vN_tool` attribution but do not count toward quorum. The active per-slot archetype label is recorded in the `vN_tool` cells (`codex-validity`/`codex-plan-fidelity`/`codex-pragmatism`, fallback semantic labels, or `claude` on fallback) even when a slot's vote file is empty or failed. The code-review classification TSV has **22 columns** (`reviewer_slots`, three voter groups of five rating cells plus `vN_tool`, and trailing `scope`; no `body_severity`). `/design` plan review uses the separate **23-column** schema (`finding_reviewers`, the same voter groups, `body_severity`, and trailing `scope`). The canonical headers are Rust-owned `scripts/larch.sh voting code-review-classification-header` and `scripts/larch.sh voting findings-classification-header`. `scope` is `in_scope` or `oos`; consumers prefer explicit `scope=oos` over ballot id prefixes. Legacy TSVs without `scope` remain readable with flat accepted +1 scoring and `OOS_` prefix fallback.

**MAV/legacy tally exception:** the fixed length-3 `--voter-files` + `--voter-tools` contract applies only on the normal three-slot dispatch path. When `--voter-tools` is omitted, `review tally-code-votes` keeps compacted multi-voter semantics for one to three `--voter-files` entries (main-agent-vote re-tally, zero-findings, and other legacy callers) and the legacy 18-column rows; those callers are unchanged.

All voters vote on **all** findings. No self-voting exclusion. Neutralized ballots are the structural mitigation: voters see `anonymous` reviewer lines while tally code restores proposer attribution from the sidecar after voting.

## Voter Prompt Template

Customize the `{VOTER_ROLE}` and `{REVIEW_CONTEXT}` per skill:

<!-- OOS voter rubric: canonical runtime voter text is emitted by scripts/larch.sh render voter. Keep OOS paragraph parity across skills/design/SKILL.md (Step 3 MAV), skills/implement/references/step5-review-branches.md (Step 5 MAV), and this voting-protocol template manually. -->

For items prefixed with `[OUT_OF_SCOPE]`: apply the OOS Acceptance Rubric (`skills/shared/oos-acceptance-rubric.md`). Vote YES when the OOS observation is legitimate, concrete, and non-duplicate. Vote NO for false positives, duplicates, style or polish noise, and speculative items with no concrete trigger. Suggested remedies are informational only; do not vote NO for remedy disagreement. The future implementer of the OOS issue chooses the remedy.

```
You are a {VOTER_ROLE} on a voting panel. For each proposed change to {REVIEW_CONTEXT}, vote YES or NO:
- **YES**: The finding is NECESSARY for the feature per the Review Acceptance Rubric (`skills/shared/review-acceptance-rubric.md`): the feature would be incomplete, broken, unverifiable, or regressed without it. This includes an independent implementation of behavior already owned in-repo when reuse or shared extraction fits the approved scope. Red or flapping default-branch CI actively blocks verification for every run; `/implement` owns executing that repair.
- **NO**: The finding does not clear the necessity gate — it may be real or valuable, but the feature ships correctly without it. Route it to Out-of-Scope instead.

Default-deny. If unsure, vote NO. "Legitimate but not necessary" is a NO.

**Severity floor (mandatory):** Vote **NO** on any *in-scope* nit; nits never clear necessity. OOS rows are judged only for filing-worthiness.

Do NOT vote YES for cleaner, more robust, more consistent, more flexible, more idiomatic, best-practice, already-met performance, or speculative portability changes. Those are Out-of-Scope signals, not acceptance signals.

**OOS / `[OUT_OF_SCOPE]` / plan `OOS_N:` rows:** Runtime prompts use `scripts/larch.sh render voter` for grammar-specific OOS wording; the paragraph above is the canonical shared clause. Here, YES means a legitimate, concrete, non-duplicate observation worth tracking; NO means false, duplicate, style/polish noise, or speculative with no concrete trigger. OOS items are never implemented in this PR.

{BALLOT}

For each ballot item, output exactly one line using the same ID from the ballot heading:
FINDING_N: YES
or
FINDING_N: NO — <one-line reason>
or
OOS_N: YES
or
OOS_N: NO — <one-line reason>

Note: for /review code review, use `OOS_N:` only when the ballot heading itself is `### OOS_N:`; `[OUT_OF_SCOPE]` rows under `### FINDING_N:` still use `FINDING_N:`.

You must vote on every item. Do NOT skip any. Do NOT modify files.
```

## Launching Voters

Voter dispatch is owned by runtime dispatchers, not prompt-side launch scaffolding.

- `/design` plan review voter dispatch is owned by `scripts/larch.sh plan-review voter-dispatch` in `crates/larch-cli/src/plan_review_commands.rs`.
- `/review` and `/implement` Step 5 code-review voter dispatch is owned by `scripts/larch.sh agent dispatch-voters`.
- Tally ownership is explicit per workflow: `scripts/larch.sh plan-review tally` and `scripts/larch.sh review tally-code-votes`, `emit-tally`, and `log-phase` are Rust-owned.
- The live Codex dispatch surface and output stem are documentary tokens here only: `${CLAUDE_PLUGIN_ROOT:?}/scripts/larch.sh agent launch-codex-exec` and `codex-vote-output.txt`.

Do not launch voters directly from the orchestrator on `/design`, `/review`, or `/implement` Step 5 paths. The dispatchers own availability checks, fallbacks, sentinel waits, external result validation, and status emission.

## Competition Scoring

After tallying votes, compute a score for each **original reviewer** (not voters):

| Vote pattern | Points | Description |
|---|---|---|
| Accepted in-scope finding with a strict majority of YES voters rating `major` on their `vN_severity` cell | +2 | High-impact finding validated by YES voters |
| Other accepted in-scope finding | +1 | Finding was validated by the panel |
| Neutral (≥1 YES, not accepted) | -0.25 | Insufficient support, but not unanimously dismissed. Single-YES `major` neutrals route to OOS instead. |
| Rejected (0 YES) | −1 | Finding was unanimously dismissed by the panel |

Severity for competition points comes from panel `vN_severity` cells attached to recorded panel votes. `body_severity` never affects points. If a deduplicated finding was proposed by multiple reviewers, **all** contributing reviewers receive the same weighted points for that finding. Reviewer pruning remains unweighted accepted-minus-rejected count math and does not apply the neutral penalty.

`LARCH_UNIQUE_FINDER_BONUS` is an experimental additive bonus and is off by default. A positive float enables the bonus and sets its size; the suggested experimental value is `0.25`. It applies only when an accepted in-scope finding has exactly one restored proposer. Deduplicated multi-reviewer findings keep shared base credit and receive no uniqueness bonus. OOS scoring remains flat and unaffected. Reviewer pruning remains unweighted accepted-minus-rejected math and does not use this bonus.

## Scoreboard

After voting, print the scoreboard. Branch on `SESSION_ENV_PATH`:

- **When `SESSION_ENV_PATH` is empty (standalone run)**: print the full scoreboard table to the session.
- **When `SESSION_ENV_PATH` is non-empty (nested run under `/implement`)**: print only a one-line count summary of the form `Round <N>: <A> accepted, <R> rejected (<N> neutral)` (in-scope findings only). The full scoreboard is suppressed at all levels in nested mode — per-round printing here and the Step 4a final summary (both inline and via `review-round-summary.md` in subagent runs).

Full scoreboard format (used in standalone mode):

```
## Reviewer Competition Scoreboard

| Reviewer | Findings | Accepted | Neutral | Rejected | OOS Proposed | OOS Accepted | OOS-Neutral | OOS-Rejected | Score |
|----------|----------|----------|---------|----------|--------------|--------------|-------------|--------------|-------|
| _label1_ | 3        | 2        | 1       | 0        | 1            | 0            | 1           | 0            | +2.75 |
| _label2_ | 2        | 1        | 1       | 0        | 0            | 0            | 0           | 0            | +0.75 |
| _label3_ | 2        | 1        | 0       | 1        | 1            | 0            | 0           | 1            | 0     |
```

The **Neutral** column counts all non-accepted in-scope findings that cost **-0.25** points to the proposer (≥1 YES but below acceptance threshold). The **Rejected** column counts non-accepted findings that cost **−1** point (0 YES). A single finding is counted in **at most one** of these two columns.

When `LARCH_UNIQUE_FINDER_BONUS` is active and rewards at least one accepted in-scope finding, print one note below the reviewer scoreboard with the bonus value and rewarded sole-finder finding count. Do not add a scoreboard column.

Attribution labels are skill-specific (e.g., `/design` uses `Code`/`Codex`/`Cursor`; `/review` hard panel uses `Correctness`/`Testing`/`Edge-cases`/`Codex-Correctness`/`Codex-Testing`/`Codex-Edge-cases`). One row per independent reviewer. Future token allocation should use precision-value, not cumulative reviewer `Score`: measure in-scope `net-score-per-finding` as `(accepted_weight - Rejected) ÷ Proposed` on scoreboard columns, where `Proposed` is the in-scope `Findings` count and OOS is excluded from both numerator and denominator.

## Out-of-Scope Observations

Reviewers may return **out-of-scope observations**: pre-existing issues or concerns beyond the PR scope that merit future attention. They are handled beside in-scope findings with different semantics:

### OOS on the Ballot

OOS ballot format depends on the skill:

- **`/design` plan review** (`scripts/larch.sh plan-review tally`): OOS items get `OOS_` prefixed IDs (e.g., `OOS_1`, `OOS_2`) and appear as `### OOS_N:` heading blocks on the ballot:

  ```markdown
  ### OOS_1: <short title of pre-existing issue>
  - **Reviewer**: anonymous
  - **Concern**: <description of pre-existing issue>
  ```

- **`/review` code review** (`review collect-findings` / `review tally-code-votes`): ballots may contain legacy `### FINDING_N: [OUT_OF_SCOPE] <title>` blocks or direct `### OOS_N:` blocks. Voters must use the matching ballot ID (`FINDING_N:` for legacy OOS headings, `OOS_N:` for direct OOS headings), and `review tally-code-votes` accepts both forms.

### OOS Vote Semantics

For OOS items, votes mean:
- **YES**: This observation deserves a GitHub issue for future attention.
- **NO**: Not worth tracking — the observation is trivial or incorrect.

OOS uses OOS-specific thresholds: one YES accepts in a one-judge panel, one or more YES votes accept in a two-judge panel, and two or more YES votes accept in a three-judge panel. Accepted non-security OOS enters `/implement`'s Rust Step 9a.1 filing batch. `/design` files accepted non-security OOS during Step 5b. In `/review` description mode, accepted OOS items are local artifacts for the operator to file manually via `/issue`; no automatic filing occurs. Otherwise, the item remains an audit observation.

**OOS items are never implemented in the current PR**. Accepted OOS creates issues only, separating "fix now" in-scope findings from "fix later" OOS observations.

### OOS Scoring

Out-of-scope items stay flat in the live voting classifier: accepted OOS earns a provisional +1, non-accepted OOS with a split-panel or OOS neutral (≥1 YES, not accepted) vote pattern scores 0, and dismissed OOS costs −1. `crates/larch-core/src/review/voting.rs::classify_result` is the live classifier and does not inspect GitHub issue fate.

| OOS vote pattern | Points | Description |
|---|---|---|
| OOS accepted (meets YES threshold for the tier) | +1 provisional | Reviewer surfaced an issue worth tracking |
| OOS neutral (≥1 YES, not accepted) | 0 | Insufficient support, but not dismissed |
| OOS rejected (0 YES) | −1 | Observation was unanimously dismissed by the panel |

`/analyze-issues` can render a separate fate-adjusted OOS report after the fact. In that diagnostic report, open filed OOS issues remain provisional, PR-closed filed OOS issues keep +1, and filed OOS issues closed unfixed or combined away score 0. The fate-adjusted report adds no retroactive −1 penalty and does not change live voting outputs.

### OOS Scoreboard

The scoreboard adds OOS columns:

```
| Reviewer | ... | OOS Proposed | OOS Accepted | OOS-Neutral | OOS-Rejected | ...
```

### OOS Security Tag

Security-tagged OOS items are held locally and never filed as public GitHub issues, whether accepted, neutral, or rejected. The detection contract is shared between the Rust `/design` tally (`scripts/larch.sh plan-review tally`) and `/review` code review (`review tally-code-votes`) through the common review classifier:

- **Canonical token**: a block is security-tagged when its body contains at least one **unfenced** occurrence of `focus-area\s*=\s*security` (case-insensitive, optional whitespace around `=`).
- **Dedicated field token**: a line-start `focus-area` field also routes as security when its value begins with `security` (including `security-hardening` style values), with optional bold/backtick markup around the label or value and either `:` or `=` as the separator.
- **Heading tag token**: the block-opening heading may start its title with `[security]` or `<security>` (optionally after `[OUT_OF_SCOPE]` / `[OOS]`). Later `### ... [security] ...` headings inside prose are not routing tags.
- **Match discrimination (false-positive guard)**: canonical-token occurrences inside backtick or triple-backtick regions are fenced and do not count — only unfenced occurrences mark a finding as security-tagged.
- **Security counter-invariant**: a real security finding MUST carry at least one routing token recognized by `is_security_block` — an unfenced canonical token, a dedicated `focus-area` field line, or a block-opening heading tag; otherwise it will not be held locally.
- Accepted OOS items where the block matches are written ONLY to the local `oos-accepted-*.md` artifact and to the local-only artifact path; security-tagged findings (focus-area=security) are held locally and NEVER filed publicly — the canonical Rust filing pipeline (`/implement` Step 9a.1 → `scripts/larch.sh oos file`) is skipped for them.

### OOS Reporting

OOS items are **not** written to `rejected-findings.md`; they use this separate pipeline:

- **Accepted OOS items — reviewer voting path** (2+ YES): Plan-review OOS accepted by the `/design` panel is written to `$DESIGN_TMPDIR/oos-accepted-design.md` (and visibility text to `$DESIGN_TMPDIR/oos.md`) during `/design` Step 3 tally/finalize. Code-review OOS accepted by the `/review` panel is written to `$REVIEW_TMPDIR/oos-accepted-review.md` during review tally; `review core` mirrors a copy at `$IMPLEMENT_TMPDIR/oos-accepted-review.md` for `/implement` Step 9a.1 and disposition gates.
- **Accepted OOS items — main-agent dual-write path** (no vote required): Written to `oos-accepted-main-agent.md` in `$IMPLEMENT_TMPDIR` by the main agent at discovery time, every time it logs a `Pre-existing Code Issues` entry to `execution-issues.md`. This is the mechanical enforcement of `/implement`'s Follow-up Work Principle for the `Pre-existing Code Issues` category — see `/implement` SKILL.md → "Follow-up Work Principle" and "Mechanical enforcement of the principle: `Pre-existing Code Issues` dual-write". Durable follow-up work outside that category is not auto-filed via this path — the main agent files it manually via `/issue` per the principle. This path is unconditional and runs in every mode (`--quick`, `--merge`, `--draft`, `--no-merge`, or any future flag). It does NOT pass through a voting panel — main-agent classification is the policy gate.
- **Unified filing**: `/implement` Step 9a.1 reads accepted OOS from the main-agent artifact, the plan-review artifact (`$DESIGN_TMPDIR/oos-accepted-design.md` when `/design` ran in-session, with implement-local fallbacks documented in `/implement` SKILL.md for disposition gates and ship-pr), and `$IMPLEMENT_TMPDIR/oos-accepted-review.md`. Rust-owned `scripts/larch.sh oos file` recovers identities, combines/caps pending blocks, and creates/wires them through the typed owner without `/issue` semantic dedup. All three artifacts share the same `### OOS_N:` schema (Description, Reviewer, Vote tally, Phase). Main-agent items use Reviewer=`Main agent`, Vote tally=`N/A — auto-filed per policy`, Phase=`implement`.
- **Non-accepted OOS items**: Collected and reported in a dedicated `<details><summary>Out-of-Scope Observations</summary>` section in the PR body for future reference.

External reviewers **in diff mode** vary by slot type. **Specialist external slots** (Cursor and Codex specialists from `agents/reviewer-*.md`) use dual-list output (`### In-Scope Findings` and `### Out-of-Scope Observations`) and can contribute OOS items via voting. **In `/review` description mode**, all external reviewers use the Claude subagent dual-list contract and contribute OOS via voting; see `${CLAUDE_PLUGIN_ROOT}/skills/review/SKILL.md` Step 3a. Claude subagent reviewers use `reviewer-templates.md` dual-list templates and produce OOS items via voting in both modes. The main agent dual-write path produces OOS items without voting.

## Zero Accepted Findings

If voting filters out **all** in-scope findings (every in-scope finding rejected by the panel), print: `**ℹ Voting panel rejected all in-scope findings. No changes to implement.**` and skip the implementation/revision step. Proceed directly to the rejected findings report. (OOS items accepted for issue filing are processed separately — by `/implement` Step 9a.1 — and do not count as implementation work.)
