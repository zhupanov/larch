# Larch Run Logs

On a default `/implement --merge` run with storage enabled, Step 18 closes the live ledgers, rebuilds the complete terminal snapshot, publishes one immutable `.tar.gz` archive, and promotes the same sanitized tree into the unpacked local cache. Step 19 then restores and removes session state without writing logs. The archive is the durable source of truth for voting tallies, code-review tally counters (`code-review-tally.json` self-review `accepted_count` / `rejected_count`), `review-findings-full.jsonl`, rejected findings, OOS observations, execution issues, run statistics, token/timing reports, and the session transcript. The tracking issue and PR body carry only slim projections. `/implement` does not add run-log files or commits to the business PR.

Missing storage configuration is an intentional publication opt-out. The
lifecycle warns at startup and terminalization, keeps local staging and
bookkeeping during the run, and removes them after successful terminalization.
It creates no archive, synchronized cache entry, or pending publication.
`repo_unavailable=true` and the separate `--no-logs-commit` flag also produce
no archive; their existing semantics do not become storage-disabled mode. Fork
dry-run mode (`--forked`) does not create a tracking issue. Session-derived
content passes through secrets and tmpdir-path redaction before enabled
publication, and a scrub failure blocks publication.

The configured storage root, deterministic archive, provider, cache, sync,
error, and Rust-handoff contracts are defined in
[Run-log storage contracts](run-log-archive.md).

Every writer resolves one pinned publication mode and client repository per
invocation. Enabled mode resolves the tool repository from repository-root
`tools-config.toml` or `LARCH_STORAGE_BASE_URI` plus local Git origin:

```text
<storage-base>/larch/<client-repo>/run-logs/<skill>/<run-id>.tar.gz
```

The base may be an S3, GCS, or R2 bucket root or include an optional prefix.
The client repository is derived from `remote.origin.url`; checkout and
worktree names do not affect it. Synchronization returns a v2 cache root bound
to the SHA-256 identity of the canonical tool repository URI. Analysis skills
retain that returned root for all reads. Their explicit `--log-root` options
remain offline-fixture bypasses that do not load storage configuration or use
the network. Without an explicit local corpus, analyzers require enabled
storage and fail with configuration guidance instead of returning an empty
corpus.

## Tool-first layout migration

Issue `character-ai/larch#8081` owns the Rust command that migrates the
retained repository-first S3 corpora to the tool-first layout (the historical
program is #7966):

```text
s3://zhupanov/larch/run-logs/
  -> s3://zhupanov/larch/larch/run-logs/
s3://zhupanov/agent-lint/run-logs/
  -> s3://zhupanov/larch/agent-lint/run-logs/
```

The relative key remains `run-logs/<skill>/<run-id>.tar.gz`. Modern larch and
agent-lint archives are copied byte for byte. Historical manifest-less larch
archives are validated against the pinned migration inventory and rebuilt with
the current canonical archive manifest. Every target then passes the normal
manifest-based materializer.

The migration writes its immutable audit report under
`s3://zhupanov/larch/larch/migration-reports/`, outside `run-logs/`. It retains
the old source prefixes and migration provenance for rollback. Issue
`character-ai/larch#7967` owns any later source deletion after its retention
and verification gates pass. See the
[2026-07-24 operational record](run-log-layout-migration-2026-07-24.md) for
the final plan, report, corpus totals, cache validation, and cutover evidence.

## Universal skill lifecycle

`skills/shared/run-lifecycle.md` defines the shared start and terminal contract,
`scripts/larch.sh` is the sole production entrypoint to its Rust owner, and
`skills/shared/run-lifecycle-ownership.tsv` assigns each skill its start and
terminal owner. Each invocation owns one explicit run ID under its declared
skill. A nested invocation stores its parent's skill and run ID in its manifest
without sharing the parent's run directory. Enabled parent and child runs
publish distinct archives. Disabled parent and child runs retain distinct
local IDs and parent metadata without a remote URI. The universal minimum is `manifest.json`,
`final-report.md`, and `execution-issues.ndjson`. When session transcript
capture is unavailable, the execution-issues record names that omission before
enabled publication or disabled cleanup. I-Flush-1 requires durable
completeness only when publication is enabled.

Specialized owners may adopt an existing rich staging tree by passing the same
run ID and an absolute `--log-root`. The lifecycle stores that choice in a
durable context file, so terminalization does not depend on inherited shell
state. The context pins publication mode, resolution reason, client repository,
and either canonical storage identity or a repository-root-derived local
namespace digest. Design, implement, and review use this adoption path and keep
their rich artifacts in the one lifecycle-owned tree.

A retryable `/design` Step 5c plan-block or receipt-write failure uses the same
design-log selection and redaction owner to refresh the local lifecycle staging
tree. It does not invoke a lifecycle terminal verb or create a remote archive.
`PUBLISH_OK=false` keeps cleanup disabled. A later `--fresh-attempt` reuses the
open run, and only the successful or truly terminal tail records the write-once
lifecycle outcome.

The inventory includes public skills, shipped aliases, internal child skills,
and dev-only skills. An alias owns a run under the alias name, then passes its
identity to the target as parent metadata. Nested review owns a separate child
run linked to its implement parent. The lifecycle lint rejects
temporary migration markers, incomplete declarations, unregistered specialized
owners, and direct archive publishers outside the shared terminal owner.

## Plan scope and Git isolation

Issue-anchored `larch:plan` blocks list the repository files that a
`/implement` run is expected to touch. Run logs are not repository files.
Current workflows stage them under the session root. Enabled runs publish one
remote archive and promote one unpacked cache copy. Disabled runs clean local
staging after terminalization. Neither mode creates a log branch,
commit, push, pull request, or merge.

## Directory structure

The paths below are relative to the session staging root. A synchronized cache
returns the same `<skill>/<RUN_ID>/` corpus shape without the leading
session-only `larch-logs/` directory.

```text
larch-logs/                 # session staging only
  design/
    <RUN_ID>/
      manifest.json
      architectural-invariant-assessment.md
      architectural-guideline-assessment.md
      accepted-plan-findings-audit.md
      (design session artifacts: files from `$DESIGN_TMPDIR` plus `render-cache/` subtree, filtered to exclude raw per-lane transcripts and sidecars via `larch_core::design::log_publish::publish_excluded`, then trimmed and redacted by Rust `design log-publish` in `crates/larch-cli/src/design_log_publish_commands.rs`; `composed-plan.diff` is a unified diff of `composed-plan.md` vs final `plan.txt` — reconstruct with `patch plan.txt composed-plan.diff -o composed-plan.md`)
      plan-review/
        round-<N>/
          findings-classification.tsv
          panel-prompt-sizes.tsv
  implement/
    <RUN_ID>/
      manifest.json
      include-probe-evidence.md
      parent-issue.md
      pre-review-head.txt
      pre-review-untracked.txt
      codex-impl-transcript.txt
      codex-impl-transcript-prompt.txt
      codex-commit-message.txt
      codex-impl-manifest-raw.json
      plan-review-tally.json
      difficulty-rating.json
      architectural-invariant-outcome.json
      architectural-guideline-outcome.json
      code-review-tally.json
      review-findings-full.jsonl
      final-summary.md
      oos-issues.ndjson
      run-statistics.md
      vendor-failure-diagnostics.txt
      token-report.json
      timing-report.json
      execution-issues.ndjson
      checks-digest-sizes.tsv
      session-transcript.jsonl
      breadcrumbs/
        quiet.log
      round-<N>/
        findings-classification.tsv
        rejected-findings.md
        oos-accepted-review.md
        review-round-summary.md
        review-summary.json
        voting-tally.md
        panel-prompt-sizes.tsv
        aggregator-validate.stderr / aggregator-dispatch.stderr (when the findings aggregator fails; staged under the round directory when `write-round` runs)
        *-output.txt
        *-output.txt.meta
        *-output.txt.json
  review/
    <RUN_ID>/
      manifest.json
      session-transcript.jsonl
      review-context.md
      review-panel-manifest.ndjson
      review-findings.ndjson
      review-tally.md
      review-scout-manifest.json
      difficulty-rating.json
      review-round-summary.md
      review-findings-classification-round-<N>.tsv
      panel-prompt-sizes.tsv
      checks-digest-sizes.tsv
```

`<RUN_ID>` is the UUID assigned at the start of each `/implement` session. Batch payload files under a run directory are redacted for secrets and tmpdir paths before archive publication. Universal lifecycle schema version 3 records publication mode and resolution identity. The underlying run-log manifest schema remains version 2 and keeps `operator_cwd` / `operator_repo_root` only as stable redacted placeholders (`"<OPERATOR_CWD>"`, `"<REPO_ROOT>"`) so durable logs preserve schema shape without exposing operator-local absolute paths. `scripts/larch.sh run-log manifest` is the single atomic schema-v2 manifest mutation path; see [Run-log CLI contract](run-log-cli.md#rust-owned-manifest-updates).

Bgjob result envs and daemon logs are session-local routing inputs before
run-log capture. They record `BGJOB_RC`, step-specific KVs, stdout, stderr, and
registry state under the session tmpdir. Ship and publish steps render durable
summaries from the routed outcome. Raw bgjob registry and daemon files are
diagnostics, not stable published batches, unless a caller copies bounded
diagnostics into `execution-issues.ndjson` or another documented batch.

### design architectural invariant and guideline assessments

`larch-logs/design/<RUN_ID>/architectural-invariant-assessment.md` and
`architectural-guideline-assessment.md` are top-level design artifacts written
from Gate C only. The invariant artifact is present only when
`ARCHITECTURAL_INVARIANTS.md` is present, valid, and has parsed `I-*` entries;
it records either the deterministic clean note or a blocking violation
assessment. The guideline artifact keeps the existing clean/deviation contract.

When a knowledge file is absent, invalid, or empty for invariants, Gate C removes any stale assessment
artifact before approval, so no stale copy is published. The artifacts publish
through the existing design-log copy, tmpdir redaction, and secret-scrub flow.
It is auditable through `/fluff-analysis` guideline assessment coverage and
`scripts/larch.sh audit-runs scan-run --skill design`.

### design accepted plan-review findings audit

`larch-logs/design/<RUN_ID>/accepted-plan-findings-audit.md` is a top-level
design artifact written from Gate C only. It records the main-agent audit of
accepted plan-review findings and their application to the final plan. Clean
runs contain a deterministic no-concerns note; mild or strong dissent records a
compact assessment, not raw diffs.

### implement architectural invariant and guideline outcomes

`larch-logs/implement/<RUN_ID>/architectural-invariant-outcome.json` records
the invariant Step 8 compose-time outcome before guideline handling. It records
`clean`, `violation`, or `dropped`; violations are blocking and feed autonomous
remediation. `architectural-guideline-outcome.json` keeps the existing
`pinned`, `clean`, or `dropped` guideline contract with stable reason token,
redacted detail, `head_sha`, `base_ref`, status, and `assessment_kind`.
Unavailable invariant and guideline outcomes store the redacted, bounded
launcher diagnostic in `detail`. Historical outcome files may omit `detail` or
store it as an empty string.

Durable notes record `NOTE_STATE` as `authored`, `deterministic-clean`, or
`unavailable`. `unavailable` is a `/design`-only fallback; the `/implement`
Step 8 subagent path never produces it (a subagent spawn failure gets one
respawn, then Tool Failure). Authored and deterministic-clean notes keep
separate `AUTHORED_DIFF_FINGERPRINT` and `COVERED_DIFF_FINGERPRINT` values.
`DIFF_FINGERPRINT` remains the covered-input compatibility field. Safe HEAD
advancement compares the stored covered `HEAD_SHA` with current HEAD using a
rename-suppressed NUL-delimited path diff. It advances only when every added or
deleted path is under `larch-logs/**` or is `docs/**/*.md`. It then refreshes
the complete base-to-current-HEAD snapshot, covered fingerprint, and HEAD
identity together. The authored fingerprint does not change.

Consumption validates the live HEAD, base, identities, regular-file snapshot,
and snapshot fingerprint even when the stored HEAD already matches. Git errors,
malformed paths, unsafe increments, stale identities, and missing or symlinked
inputs fail closed. Prior metadata with a valid non-empty `DIFF_FINGERPRINT` is
read as an authored note whose authored and covered identities match. Older
notes without enough identity require reassessment. `unavailable` is a
non-violation fallback and cannot replace a recorded invariant violation.

The artifacts are written for terminal Step 8 results. Runs that still
need architectural assessment do not write partial outcomes. The audit scan treats missing artifacts below
`GUIDELINE_SHIP_OUTCOME_MIN_LARCH_VERSION`, and runs that did not reach Step 8,
as informational. At or above that cutover, Step 8-eligible missing, malformed,
empty, or symlinked artifacts fail.

Step 2 also records launch-time architectural-knowledge requiredness in the
session-local `step2-architectural-knowledge.env` file. That snapshot is not a
published run-log batch, but the dispatcher uses it to decide whether the coder's
`manifest.json` must include `architectural_acknowledgment`. Missing or empty
acknowledgment on `complete` or `needs_qa` bails with
`architectural-acknowledgment-missing` instead of recovering as
`manifest-schema-invalid`. Invalid architecture files are omitted fail-closed and
logged under `Warnings` as Step 2 architectural-knowledge omissions.

### In-loop refresh sidecars

In-loop refresh sidecars (`token-report-refresh.json`, `timing-report-refresh.json`,
`session-transcript-refresh.txt`) are volatile in-loop snapshots that are not
published in the run tree. The run-log refresh owner reads them as inputs for
re-rendering canonical batches (`token-report.ndjson`, `timing-report.ndjson`,
`session-transcript.jsonl`) but does not copy the refresh files themselves into
`larch-logs/implement/<RUN_ID>/`. Canonical reports such as `token-report.json`,
`timing-report.json`, `token-report.ndjson`, and `timing-report.ndjson` are still
published normally.

### Design publication selection

`scripts/larch.sh design log-publish` (Rust owner
`crates/larch-cli/src/design_log_publish_commands.rs`, filter
`larch_core::design::log_publish::publish_excluded`) owns design-log selection.
Its exclusions apply by basename at every copied depth. Raw Codex event
streams, plan-review transcripts, rendered prompts, launch stderr, producer
sidecars, collector failure logs, dropped-slot raw diagnostics, token carriers,
and the `plan-autofix/` draft tree stay session-local. Normal final logs also
exclude `.completed/`; pause snapshots retain the top-level completion
sentinels needed for resume provenance. The retired Python
`design_log_ship` helper is gone; the `ship design-log` CLI already exits 2.

The `plan-review/` tree uses the per-round contract below. The publisher drops
the obsolete `round-<N>/revise/` tree and cumulative or redundant round files.
Top-level `issue-body.txt`, `issue.json`, and `architecture-diagram.md` are also
excluded because GitHub owns those public snapshots. `render-cache/` has an
open content schema but keeps the shared suffix denylist and the same root,
ancestor, symlink, file-type, trim, and redaction checks. An unsafe selected
file rejects publication instead of broadening the copied set.

### breadcrumbs/

The tree above shows `implement/<RUN_ID>/breadcrumbs/` as a representative
example. The path shape is shared across publishing skill roots, so
the same directory artifact may exist as `design/<RUN_ID>/breadcrumbs/`,
`review/<RUN_ID>/breadcrumbs/`, or `research/<RUN_ID>/breadcrumbs/` when a
publisher wires that helper for that skill. The landed callers are the
`/implement` terminal publisher and `scripts/larch.sh design log-publish`.

`breadcrumbs/` is a directory artifact, not a larch-log batch. The implement
publisher and `scripts/larch.sh design log-publish` reach the Rust owner through
`scripts/larch.sh run-log publish-breadcrumbs`, and the shared run lifecycle
calls that owner directly. Session-tmpdir
`breadcrumbs/` paths (`$IMPLEMENT_TMPDIR/breadcrumbs/`, `$DESIGN_TMPDIR/breadcrumbs/`,
`$REVIEW_TMPDIR/breadcrumbs/`, or `$RESEARCH_TMPDIR/breadcrumbs/`) are publication
hints only; publication stages quiet logs from the session root, not
live runtime streams under those directories.

Source resolution uses the log-root parent's `breadcrumbs/`. That directory is
a hint only: publication derives the session root from the hint's parent and
stages matching `larch-quiet-<script>-<pid>.log` files from the session root
rather than scanning published inputs from `breadcrumbs/` itself. The derived
session root must resolve under `IMPLEMENT/DESIGN/REVIEW/RESEARCH_TMPDIR` when
any of those is set; otherwise publication skips breadcrumb staging and returns
success without creating or replacing the published `breadcrumbs/` directory.

Per-script session-root quiet logs whose basenames match exactly
`larch-quiet-<script>-<pid>.log` are staged. Each accepted file is individually
redacted through the shared Rust redaction owner (sensitive paths first, then
every secret family), then all redacted content is **concatenated** into a single
`larch-logs/<skill>/<run-id>/breadcrumbs/quiet.log` with per-source header lines
`=== <basename> ===`. The individual source files are not published separately.
Quiet-log sourcing uses `dirname` of the breadcrumbs source path and runs even
when `breadcrumbs/` was never created. Candidates must stay under the active
session tmpdir, must not be symlinks, and must not be hardlinks. Legacy
`*.ndjson` files and other non-quiet-log artifacts under the session
`breadcrumbs/` hint are not published.
When no quiet log stages, the helper returns 0 and does not create, replace,
or clear an existing published `breadcrumbs/` destination.

The enforced-reject and silent-skip split is a security boundary. An invalid
source root, path escape, source-directory symlink, unsafe accepted file,
hardlink, invalid accepted basename, or redactor failure rejects publication
for the whole directory and leaves the prior destination unchanged. Legacy
ndjson files, monitor sidecars, non-regular files, race-disappeared candidates,
and non-matching quiet-log basenames are ignored and not published. See the
canonical [breadcrumb security invariants](security/artifacts-redaction-and-publication.md#breadcrumb-security-invariants).

`round-<N>/` directories are written by `run-log write-round` during
`/implement` code review. They preserve the per-round reviewer and voter
diagnostic artifacts that are otherwise lost with `$IMPLEMENT_TMPDIR` cleanup.
Only registered artifact names are copied. `.meta` files have `CMD_JSON=...`
removed when `CMD_JSON=` is the first non-whitespace token, included
`*-output.txt.json` / `*-output-*.txt.json` sidecars have their top-level
`.result` field removed, and all copied files still pass through the normal
tmpdir and secrets redaction. This trimming is specific to the published round
artifacts; the session tmpdir may still hold raw sidecars for in-run retries.
If JSON trimming fails, `write-round` fails closed instead of copying the raw
sidecar into `larch-logs/`.

### panel prompt-size telemetry

`panel-prompt-sizes.tsv` is count-only telemetry for panel-tier prompts. It records safe identifiers, rendered prompt byte and estimated-token counts, derived scaffold and payload byte/token counts, and agent-file byte and estimated-token counts when a repo-local agent file exists. It never stores rendered prompt text or payload text.

Rows are written only when `LARCH_PANEL_SLOT` is set and the slot class is recognized as specialist, plan-review, voter, aggregator, or implementer. Dispatch producers set the panel environment explicitly in review dispatch, code voters, plan-review dispatch, aggregation, and review-fix coder paths. Appends use a best-effort flock-protected TSV writer, so lock or write failures skip telemetry without failing the parent dispatch.

Current rows include `scaffold_bytes`, `scaffold_tokens`, `payload_bytes`, and `payload_tokens`. `prompt_bytes` remains the rendered prompt size. `payload_bytes` is count-only per-run content that the renderer or dispatcher knows it inlined or attached as prompt payload; `scaffold_bytes` is the non-negative remainder of prompt bytes after subtracting payload bytes. Older published TSVs may lack these columns. `measure-panel-cost` treats missing scaffold as the whole prompt and missing payload as zero.

Published locations are:

- Design plan review: `larch-logs/design/<RUN_ID>/plan-review/round-<N>/panel-prompt-sizes.tsv` only. Top-level design copies are ignored.
- Implement Step 5: `larch-logs/implement/<RUN_ID>/round-<N>/panel-prompt-sizes.tsv`.
- Standalone review: `larch-logs/review/<RUN_ID>/panel-prompt-sizes.tsv`, or `larch-logs/review/<RUN_ID>/round-<N>/panel-prompt-sizes.tsv` when the dispatch is round-local.

`scripts/larch.sh token measure-panel-cost` synchronizes once, then aggregates cached panel TSVs by agent file, plus generated/no-agent buckets for voters and generated prompts. It writes a TSV under the `measure-panel-cost` owner in the [analyzer state tree](analysis-state.md) with dispatch counts, runs observed, loads per run, prompt counts, scaffold and payload counts, agent counts, and total realized counts. Rows rank by scaffold bytes so fixed prompt surface stays visible even when payload-heavy runs dominate realized bytes.

### checks digest-size telemetry

`checks-digest-sizes.tsv` is count-only telemetry for relevant-checks failure digests. It records byte and estimated-token counts for the redacted failure log and the generated digest, plus signed `saved_bytes` and `saved_tokens` values. Savings can be negative when a digest is larger than a tiny redacted log. The file never stores log text, digest text, commands, failure lines, prompts, or absolute paths.

Published locations are:

- Implement checks failures: `larch-logs/implement/<RUN_ID>/checks-digest-sizes.tsv`.
- Standalone review checks failures: `larch-logs/review/<RUN_ID>/checks-digest-sizes.tsv`.

Writes are best-effort. A telemetry lock or write failure prints a warning and does not change the checks result or the `DIGEST_FILE=` failure envelope. The writer skips telemetry unless exactly one active implement or review run directory exists under the session `larch-logs/` tree.

`scripts/larch.sh token measure-checks-digest-savings` synchronizes once, then aggregates cached checks-digest TSVs into the `measure-checks-digest-savings` owner in the [analyzer state tree](analysis-state.md). It reports `status=insufficient-data` until at least 5 valid rows exist. With 5 or more rows, positive aggregate signed token savings yields `recommendation=go-design-validator-extension`; zero or negative aggregate token savings yields `recommendation=no-go-design-validator-extension`. The design-validator digest extension remains gated on a future positive measurement.

### design plan-review `findings-classification.tsv`

`larch-logs/design/<RUN_ID>/plan-review/round-<N>/findings-classification.tsv`
is the per-round forensic export produced by
`scripts/larch.sh plan-review tally`. The file always uses a 23-column,
tab-separated schema:

`finding_id`, `finding_reviewers`, `voting_result`, then three repeated slot
groups of: `vote`, `correctness`, `severity`, `quality`, `uncertain`, `tool`,
then `body_severity` and `scope`.

The canonical header is:

```text
finding_id\tfinding_reviewers\tvoting_result\tv1_vote\tv1_correctness\tv1_severity\tv1_quality\tv1_uncertain\tv1_tool\tv2_vote\tv2_correctness\tv2_severity\tv2_quality\tv2_uncertain\tv2_tool\tv3_vote\tv3_correctness\tv3_severity\tv3_quality\tv3_uncertain\tv3_tool\tbody_severity\tscope
```

Semantics:

- `finding_id` is the ballot heading id (`FINDING_N` or `OOS_N`).
- `finding_reviewers` is proposer attribution copied from the ballot block.
- `voting_result` is the final tally outcome for that row.
- `vN_vote` is the normalized vote token used by the tally (`YES`, `NO`,
  or empty when that slot had no parseable vote for the id; stray `EXONERATE` tokens are mapped to `NO`).
- `vN_correctness`, `vN_severity`, `vN_quality`, and `vN_uncertain` are the
  optional forensic rating axes parsed from the same voter line.
- `vN_tool` is the runtime tool identity for that slot.
- `body_severity` is the optional severity token parsed from the finding body
  (`major`, `minor`, `nit`, or empty). It is forensic metadata only;
  reviewer scoreboards weight accepted in-scope findings from YES-voter panel
  severity, not from `body_severity`.
- `scope` is `in_scope` or `oos`. Producers write `scope=oos` for direct
  `OOS_*` rows, legacy `[OUT_OF_SCOPE]` or `[OOS]` rows, and scope-drift rows.
  Consumers prefer explicit `scope=oos` over id prefixes; legacy TSVs without
  `scope` remain readable with flat accepted +1 scoring and `OOS_` prefix fallback.

Older published design TSVs may use the 21-column shape without
`body_severity`. `/voter-calibration` keeps those readable through
header-driven detection, so `v3_tool` is not shifted into the body-severity
slot.

Slot semantics:

- For explicit `--voter` dispatch, non-`MainAgent` voters preserve canonical
  tool slots from the declared `SLOT` label: `Claude -> v1`, `Codex -> v2`,
  `Cursor -> v3`. Basename heuristics do not override explicit slot labels.
  Missing slots stay empty instead of compacting later voters leftward.
- For sole `--voter MainAgent:<PATH>` adjudication, `v1`/`v2`/`v3` remain empty
  and `voting_result` stays `rejected` for every row even though the accepted /
  rejected / OOS artifact files reflect the MainAgent adjudication result.
- For legacy `--voter-files`, slots are inferred from basename/tool heuristics.
- Missing or degraded rounds preserve empty cells so every data row still has
  the full 23-column schema width. <!-- lint-literal-counts: allow fixed TSV schema --> A 0-finding or tally-error round may therefore publish a
  header-only TSV.

See `crates/larch-cli/src/plan_review_commands.rs` in the source repository
for the authoritative producer contract and harness coverage.

### design plan-review per-round artifacts

Under `larch-logs/design/<RUN_ID>/plan-review/round-<N>/`, each single-pass Step 3 review entry produces forensic artifacts. The list below is a **representative** selection grouped by producer. `larch_core::design::log_publish::publish_excluded` (via `scripts/larch.sh design log-publish`) is the authoritative archive-selection filter, and [Design publication selection](#design-publication-selection) explains its operator-facing contract.

#### Findings

- `findings.md`
- `findings-in-scope.md`
- `findings-oos.md`
- `findings-classification.tsv`

#### Voting

- `oos.md`
- `oos-accepted-design.md`
- `ballot.txt` (session snapshot; excluded from the published archive)
- `voting-tally.md`

`voting-tally.md` includes the per-finding vote table, reviewer competition
scoreboard, and voter agreement scoreboard. The voter agreement section is a
diagnostic view over the same classification rows. It does not introduce a new
published artifact.

`accepted-plan-findings.md` and `rejected-findings.md` are excluded from
published round directories (#3721) because they are cumulative across rounds.
Only the top-level copies in the design run directory are kept. Per-round
outcome attribution is preserved by each round's
`findings-classification.tsv` joined with `findings.md`.

#### Manifests and voter diagnostics

- `plan-review-slots.ndjson`
- `plan-voter-slots.ndjson`
- `scout-plan-manifest.json`
- `*-vote-output.txt`
- `*-vote-output-first-pass.txt`
- `voter*-diag.txt`
- `plan.txt` (round 1 only; rounds ≥ 2 commit `plan.diff` vs previous round)

#### Loop forensics

- `round-summary.env`

#### Revise sub-tree (`round-<N>/revise/`)

- `codex-output.txt`
- `cursor-output.txt`
- `claude-output.txt`
- `revise.env`
- `prompt.txt`
- `*-candidate.patch`

### code-review `findings-classification.tsv`

`/implement` review rounds publish
`larch-logs/implement/<RUN_ID>/round-<N>/findings-classification.tsv`.
Standalone `/review --diff` publishes flat per-round batches named
`review-findings-classification-round-N.tsv` under
`larch-logs/review/<RUN_ID>/`.

New three-slot code-review TSV writes use this schema:

```text
finding_id\treviewer_slots\tvoting_result\tv1_vote\tv1_correctness\tv1_severity\tv1_quality\tv1_uncertain\tv1_tool\tv2_vote\tv2_correctness\tv2_severity\tv2_quality\tv2_uncertain\tv2_tool\tv3_vote\tv3_correctness\tv3_severity\tv3_quality\tv3_uncertain\tv3_tool\tscope
```

`finding_id` is the ballot id (`FINDING_N` or `OOS_N`), `reviewer_slots` is the
pipe-delimited proposer attribution, `voting_result` is one of `accepted`, `neutral`, or `rejected`, and `scope` is `in_scope` or `oos`. Producers write `scope=oos` for direct `OOS_*` rows, legacy `[OUT_OF_SCOPE]` or `[OOS]` rows, and scope-drift rows. Consumers prefer explicit `scope=oos` over id prefixes; legacy TSVs without `scope` remain readable with flat accepted +1 scoring and `OOS_` prefix fallback. On the three-slot code-review path, `v1` is `codex-validity`, `v2` is `codex-plan-fidelity`, and `v3` is `codex-pragmatism`; `claude` appears in `v1_tool` only on the both-externals-down fallback path, and older logs may still contain `cursor-validity`. Empty or failed slots keep their `vN_tool` label with empty rating cells. Rating cells are enum-only; missing or invalid axis tokens are empty and force `vN_uncertain=true`. Older logs may lack `vN_tool` or use the compact 18-column layout. MAV re-tally rows may also use the legacy single-voter 18-column shape.

The published TSV schemas remain backward compatible for new writes. The
analyzer reads older 21-column design TSVs without `body_severity` and older
18-column compact code-review TSVs through header-driven detection.

For 0-judge degraded rounds (`TALLY_STATUS=main-agent-vote-required`),
`voting_result=rejected` is a placeholder TSV sentinel, not a literal panel
outcome. Those rows intentionally keep empty `vN_*` cells until later
main-agent adjudication; the accepted/rejected/OOS markdown artifacts are the
authoritative operator-facing outcome for that degraded round.

`/review` uses the same `larch-logs/<skill>/<RUN_ID>/` layout when a run ID is provided. Review phase names are encoded in flat batch slugs, not subdirectories: `review-context` for gathered context, `review-panel-manifest` for launched slots, `review-findings` for collected finding records, `review-tally` for vote results, `review-scout-manifest` for dynamic-reviewer scout status, `difficulty-rating` for the standalone run difficulty record, `review-round-summary` for the human-readable round summary, and `review-findings-classification-round-N` for the forensic vote/rating TSV.

## manifest.json

Created by `scripts/larch.sh run-log init` during **Step 0** when the tracking issue is first resolved (tracking adoption / post-resolution). Updated by `scripts/larch.sh run-log manifest` throughout the run. Contains: skill name, run ID, operator CWD, operator repo root, tracking-issue number, PR number (once created), the larch plugin version (`larch_version`), the main-agent model and reasoning effort (`model_roster.main` and `effort`, resolved at init from the active session transcript and `CLAUDE_CODE_*` / `CLAUDE_*` env, falling back to `unknown`), the run status last recorded in that manifest snapshot, and optional routing flags such as `coder_fallback=true` when omitted-`--coder` routing fell past Codex. Authoritative contract: `docs/run-log-cli.md`.

`model_roster.main` is the orchestrator (main-agent) model id, not the implementer coder. It is captured once at run-log init (the newest session transcript at that point is the orchestrator session, before subagents spawn) via `tokens.read_main_model`, then preserved across `run-log manifest` merges. Historical runs predating this capture carry `"unknown"`.

Attribution for the Step 8 CI-fix `larch:ci-fixer` path and the `larch:arch-assessor` assessor is prose-only: their identity is described in `skills/implement/SKILL.md` and `agents/*.md`, and no `MODE=subagent` or `TIER=subagent` tokens are emitted into run-log records for them (#7192, #7193, #7219). The conflict-resolution `larch:ci-fixer` path (`MODE=conflict`) documents attribution as `MODE=subagent` / `TIER=subagent` in `skills/implement/references/conflict-resolution.md` (#7198). The Step 2.4 Claude-fallback implementer path (`larch:claude-implementer`, work-mode `MODE=step2-plan`) records attribution as `MODE=subagent` / `TIER=subagent` via `--rater-tool subagent` and `--producer subagent` on the orchestrator fences (#7195).

Current `/implement` archives are created after terminal reconciliation, so a
merged run records the final `"done"` manifest. Bail, stall, and cancellation
archives retain their terminal status.

## Batch files

### include-probe-evidence.md

**Mode**: replace. **Written**: optional, when a plan's acceptance criteria require Phase 1 empirical subprocess output that otherwise lives only under `$IMPLEMENT_TMPDIR` (for example cross-agent include probes). Holds a redacted, tmpdir-free copy: a `BRANCH=A` / `BRANCH=B` header line plus per-agent transcript sections so post-merge reviewers can verify the probe and branch decision without the operator session tree.

### parent-issue.md

**Mode**: replace. **Written**: **Step 0** materialization tail and refreshed by
local recovery checkpoints and the Step 18 terminal snapshot when present.

Tracking-issue sentinel with the adopted or created issue number and run ID.
This is the session-scope idempotency source for tracking issue recovery.

### pre-review-head.txt and pre-review-untracked.txt

**Mode**: replace. **Written**: Step 5 round 1 initialization.

`pre-review-head.txt` records the HEAD SHA before review starts.
`pre-review-untracked.txt` records the untracked-file snapshot used by the
review-change checks.

### codex-impl-transcript.txt and related Codex setup files

**Mode**: replace. **Written**: local recovery checkpoints and the Step 18
terminal snapshot when present.

`codex-impl-transcript.txt` is the external implementer transcript,
`codex-impl-transcript-prompt.txt` is the prompt sidecar,
`codex-commit-message.txt` is the redacted commit message consumed by the
dispatcher, and `codex-impl-manifest-raw.json` is the pre-sanitized manifest
copy retained for diagnosis. These files are optional because non-Codex or
bailout paths may not produce them.

### plan-review-tally.json

**Mode**: replace (JSON object). **Written**: **Step 0** materialization tail. `/implement` writes this batch on every run: the exported plan-review voting tally when one is present, otherwise a stub recording that plan review ran in `/design`.

One JSON object per `/implement` session. The tally envelope shape is shared with
`code-review-tally.json`: `schema_version` (`2`), `phase`, `batch`, `mode`, `rounds`,
`accepted_count`, `rejected_count`, and `exonerated_count` (always 0; retained for backward compatibility). The `body` field is phase-dependent. `plan-review-tally.json` includes `body`; `code-review-tally.json` omits it. For plan review the extra counters are normally `0`. Plan review voting itself runs during `/design`; this batch is often a stub or summary that references that outcome. The `body` contains
the plan-review voting outcome (accepted count, rejected count, round summaries)
plus any rejected plan-review findings under a `## Rejected Plan Review Findings`
sub-header. When no voting artifact is attached for this run, the body may note that plan review was completed in the `/design` phase instead of duplicating ballots here.

### code-review-tally.json

**Mode**: replace (JSON object). **Written**: Step 5, after the Step 5 review loop completes (via `review-and-fix CLI` / `review core`; standalone `/review` is a separate skill).

One JSON object per `/implement` session with these envelope fields:
`schema_version` (`2`), `phase`, `batch`, `mode`, `rounds`, `accepted_count`,
`rejected_count`, and `exonerated_count`. It does not store a `body` field. The
body file is validation input only when the tally is written. Round markdown,
voting prose, and rejected-finding details live in the per-round artifacts and
`review-findings-full.jsonl`.

`rounds` is the total number of completed code-review rounds for the run. For a
normal multi-round `/implement`, it should match the published `round-*`
directory count. `accepted_count` and `rejected_count` are cumulative across all
code-review rounds and are derived from composed `review-findings-full.jsonl`
code-review rows. `exonerated_count` is an informational sub-count of
`rejected_count` (operator-facing summaries use “`K` accepted, `N` rejected (`P` exonerated)” where `P ≤ N`). `rejected_count` counts every finding that did not meet the acceptance threshold (including split-panel and exonerated vote patterns).

For `mode: self-review`, `rounds` is always `1`. `accepted_count` is the count
of in-scope self-review findings fixed inline during Step 5, recorded as
`### [Code Review] Self-review accepted` headings in
`$IMPLEMENT_TMPDIR/self-review-accepted.md`. `rejected_count` is the count of
self-review findings recorded under exact `### [Code Review] Self-review`
headings in `$IMPLEMENT_TMPDIR/rejected-findings.md`.
`review-and-fix write-self-review-tally` reads those files under
`--implement-tmpdir` and derives `accepted_count` and `rejected_count`
internally; a missing or empty file counts as `0`. Self-review tally counters
are not derived from `review-findings-full.jsonl`; that file may remain an empty
sentinel for self-review runs to show that review ran.

**Note**: Internal tally KV may still emit `NEUTRAL_COUNT` for scoreboard accounting; that key is **not** the same thing as `JUDGE_ERROR`, which is a per-judge-per-finding state (the parser fallback when a
voter's ballot did not contain a parseable vote line for that finding). `JUDGE_ERROR`
appears in the per-finding vote breakdown table under the `JERR` column header but
is not separately enumerated in the tally envelope counters.

### review-findings-full.jsonl

**Mode**: replace (line-delimited JSON). **Written**: Step 5, immediately after the `code-review-tally` batch.

Per-finding payloads for plan-review accepted, plan-review rejected, and code-review entries. One JSON object per line has `id`, `issue_number`, `phase` (`plan-review` | `code-review`), `outcome` (`accepted` | `rejected` | `out_of_scope`), `schema_version` (`2`), `reviewer_slots` (array of redacted reviewer labels), `round_num` (empty outside numbered review rounds), `category` (best-effort, extracted from a leading `## <cat>: ...` body line — may be empty), and `prose_body` (redacted). `crates/larch-cli/src/review_compose_commands.rs` owns the producer contract; invoke it through `scripts/larch.sh review compose-findings`.

**Backward compatibility**: Published `review-findings-full.jsonl` files may
mix envelopes across archives. Normalize each line in three ways: **(1) v2**
when `(has("reviewer_slots") and (.reviewer_slots | type == "array"))`, use
`reviewer_slots` and optional `schema_version` as the canonical slot list.
**(2) Legacy** only when v2 is absent: a string `reviewer` field, often without
`schema_version`. **(3) Unknown or partial**, log and skip sparse rows that omit
both usable shapes. Do not treat `reviewer_slots: null` or a non-array value as
v2. Example `jq` sketch:

```jq
if (has("reviewer_slots") and (.reviewer_slots | type == "array")) then
  .reviewer_slots
elif has("reviewer") and (.reviewer | type == "string") then
  [.reviewer]
else
  empty   # or: log "unknown row" to stderr
end
```

See `crates/larch-cli/src/review_compose_commands.rs` for the same mixed-stream contract (`scripts/larch.sh review compose-findings` is the CLI entrypoint).

### version-bump-reasoning.md

**Mode**: replace. **Written**: **not** on the `/implement` ship path after Phase 1 (#3364). Legacy runs may still carry this batch from pre-Phase-1 implement bumps; new implement runs omit it, and it is no longer listed in `docs/run-logs-required-files.tsv`, so the `required-file-presence` audit no longer expects it on implement runs. `/release` and manual `.claude/skills/release` flows own version reasoning when operators need an auditable bump record.

Markdown explanation of the version bump classification: which bump type was chosen (PATCH / MINOR / MAJOR), which changed files drove the decision, and the reasoning applied. Useful for auditing unexpected version jumps on release-driven paths.

### difficulty-rating.json

**Mode**: replace. **Written**: every design, implement, and standalone review run records a JSON object with `schema_version: 1`, rater identity, predicted and applied tier, confidence, bounded rationale, design and implement tiers when known, floor matches, audit placeholders, escalation placeholders, and `panel_skipped` when the run intentionally skipped review. The rating is model judgment anchored by `docs/difficulty-rating.md`; deterministic floors in `docs/difficulty-floor-globs.tsv` can raise the applied tier but never lower it. Early degraded partial dirs may rely on the existing partial-run tolerance rules.

### final-summary.md

**Mode**: replace. **Written**: [`scripts/larch.sh final-report write`](../skills/implement/scripts/write-final-report.md) owns the `/implement` body, its durable `larch-logs/implement/<RUN_ID>/final-summary.md` write, and its tracking-issue `larch:final-summary` upsert. For `/design`, the #8592 Rust `design log-publish` owner renders the enriched `final-summary.md` in the design tmpdir before copying the published snapshot, with tracking-comment upserts suppressed in that pre-copy render. Step 5c and clarify then run the authoritative follow-up `scripts/larch.sh design render-final-summary` pass that upserts the marker-keyed tracking comment.

Published **rich markdown** projection of the run: outcome, mode flags, token totals (Claude / Codex / Cursor / Claude (subprocess) — the spawned-process Claude reviewer/voter/CI/scout lane, machine name `claude_sub`, priced at Claude rates and summed into the total), optional per-lane USD estimates from `larch_core::report::RATE_TABLE`, duration, plan/code review tallies, OOS and execution-issue counts, log directory pointer, the difficulty bullet, the main-agent model, reasoning effort, and larch plugin version (the `- **Main agent model**:`, `- **Effort**:`, and `- **Larch version**:` bullets, read from the run manifest via `--manifest-path` with live fallbacks), and operator-facing notes (fork dry-run, draft, no-merge, upstream issue, fork OOS stubs). The `/implement` body is produced by Rust `larch_core::report::run_summary`: it begins with a `## /<skill> run <run-id>: <outcome>` heading and a normalized markdown bullet list (including `**PR**:` when a PR is known; `- **Outcome**:` for outcomes matching `bailed*`, `stalled`, `cancelled-*`, `failed-*`, or `publish-skipped`; the other fields follow the renderer contract). A versioned HTML sentinel (`<!-- larch:run-summary v=1 -->`) appears on its own line after that bullet block (and before any optional trailing note lines) so consumers can detect the standardized block while the opening line stays human-readable. The `- **PR**:` bullet is omitted when no PR number is known; otherwise `#<number> — <url>` or `#<number>` when the URL is unknown. When `RUN_LOGS_PATH=N/A`, the renderer must not synthesize a fallback log path for `RUN_ID=unknown`, `failed-publish`, or `publish-skipped` outcomes. The #7680 Rust renderer uses the same marker grammar for its bounded `/design` payload; it is not an `/implement` final-report fallback. The tracking-issue `larch:final-summary` comment is the canonical live projection once upserted.

**GLM-5.2 main-agent cost line**: when the resolved run identity (`model_roster.main`, including the `glm-5.2[1m]` alias) is GLM-5.2, the `- **Cost**:` bullet renders `Claude/GLM-5.2 token $T (estimated $E)` with `E = T / 15`, substitutes `E` for the main Claude component in the displayed `TOTAL`, and inserts a `- **Cost note**:` bullet immediately after Cost. Non-GLM summaries keep the plain `Claude $C` segment with no estimated annotation and no cost-note bullet. `claude_sub` remains token-priced from its recorded model and is never divided by 15; non-GLM `[1m]` model names are not remapped by this path.

For `/implement`, rejected and logged-only OOS rows stay in round artifacts and are not rendered in the final summary. OOS files only when the vote threshold accepts it and a strict majority of YES voters rate it `major`; accepted-but-`minor` OOS remains logged only. Explicit `nit` reviewer rows are dropped before voting and recorded per round in public `oos-dropped-before-vote.md`; security-tagged drops go to the local `security-oos-observations.md` sidecar and are not published. #6028 dropped-OOS surfacing applies only to non-`nit` dropped OOS candidates.

### oos-issues.ndjson

**Mode**: append (NDJSON records). **Written**: Step 9a.1, after out-of-scope disposition evidence is materialized.

Two sub-blocks per record: accepted OOS observations that were filed as GitHub issues (each entry includes the filed issue URL), and rejected / out-of-scope observations that were voted down or not filed (each entry includes the rejection reason). Security findings are never filed via this path. `oos-issues.ndjson` is disposition evidence, not the Step 9a.1 completion signal. A provisional `oos-issues.ndjson` written before a failed disposition checkpoint must not mark Step 9a.1 complete.

### run-statistics.md

**Mode**: replace. **Written**: Step 9a.1, after the OOS disposition checkpoint succeeds.

Summary statistics for the run: number of accepted and rejected OOS items, filed-issue URLs, round counts, and other aggregate metrics. Step 9a.1 completion requires post-checkpoint `run-statistics.md`. Explicit `manifest.json` `steps_ran.step9a1=true` is recorded only together with that file; `step9a1=true` without `run-statistics.md` is a stale or corrupt marker and must fail audit/verify scans.

### vendor-failure-diagnostics.txt

**Mode**: replace. **Written**: Step 18 by the Rust-owned `scripts/larch.sh run-log prepare-terminal-snapshot`, when at least one vendor-agent slot logged a failure diagnostic during the run.

Concatenation of per-slot `*.failure-diag` carriers. Each slot entry is redacted before being staged under `$IMPLEMENT_TMPDIR/vendor-failure-diagnostics.parts/`. The Rust terminal snapshot owner sorts the complete parts set, composes it, and atomically replaces `vendor-failure-diagnostics.txt`. CI launchers (`scripts/larch.sh agent launch-codex-ci`, `scripts/larch.sh agent launch-cursor-ci`, `scripts/larch.sh agent launch-claude-ci`) and implement launchers (`scripts/larch.sh agent launch-codex-implement`, `scripts/larch.sh agent launch-cursor-implement`) feed this batch; reviewer launchers (`scripts/larch.sh agent launch-review`) also contribute when `IMPLEMENT_TMPDIR` is set. Early bail, failure, cancel, and stall paths that reach Step 18 use the same aggregation owner.

### token-report.json

**Mode**: replace. **Written**: mutable recovery checkpoints during ship and rebuilt after the closing Step 18 token mark. The Rust-owned `scripts/larch.sh run-log refresh` stages the durable batch. The agentic Claude CI-fix delegate reconstructs `RunContext` and requires `--repo-root`. `ci-fix-exhausted` pairs with Step 12d operator bail. Stall recovery does not auto-resume the ship step for that token.

Structured per-step Claude and external-vendor token usage for the session. The terminal render includes work after Step 7a plus the closing logs-flush mark.

### timing-report.json

**Mode**: replace. **Written**: same lifecycle as `token-report.json`.

Structured per-step elapsed-time data for the session, measured from the timing ledger marks at each step entry. Useful for identifying slow steps (e.g., long Codex spawns, extended CI waits).

JSON reports may include an additive `rounds` array on a matching per-step row. `/implement` code-review rounds attach only to the `Step 5 — code review` row whose interval fully contains the round start and end; `/design` plan-review rounds attach only to the `design Step 3 — plan review` row under the same containment rule. Rows are de-duplicated by round number with the latest ledger row winning, then sorted by round. Round objects contain `round`, `duration_seconds`, `accepted`, and `rejected`; `/design` plan-review round objects also include `oos` when present.

### Debate durable records

These four batches are archive carriers for the `/debate` engine. The
`debate adjudicate --vote-stalemates` verb writes the local stalemate tally to
`debate-stalemate-tally`; `debate synthesize` writes the redacted proposal body
to `debate-proposal`. The surrounding debate flow owns the participant and
round-ledger producers. Together the carriers let an archive-only reader
reconstruct the debate without the session tmpdir.

| Batch | Extension | Mode | Sanitizer | Reconstruction role |
|---|---|---|---|---|
| `debate-participants` | `.tsv` | replace | `none` | Final participant roster (vendor, slot, live/dropped). |
| `debate-round-ledger` | `.ndjson` | append | `json-lines` | Per-turn ledger rows across negotiation rounds. |
| `debate-stalemate-tally` | `.json` | replace | `json-object` | Optional stalemate ballot outcome when `-s` voting ran. |
| `debate-proposal` | `.md` | replace | `none` | Final synthesized proposal body. |

Together, the roster, round ledger, optional stalemate tally, and final
proposal support archive-only reconstruction of who argued, what each round
recorded, how stalemates resolved, and what proposal emerged.

**Durable-record invariant**: debate-batch writes reject recognized
session-tmpdir pointers before redaction or persistence. Valid stored content
still uses the existing secret and path redaction pipeline. Operator-repository
paths remain accepted input and redact to the usual operator-repo token. See
the canonical
[durable debate-record invariant](security/artifacts-redaction-and-publication.md#durable-debate-record-invariant).

### execution-issues.ndjson

**Mode**: append (NDJSON records). **Written**: Step 2 (Q/A entries, progressive), the Step 7a local checkpoint, later external-implementer and pre-push checkpoints, and the Step 18 terminal tail.

Log of noteworthy events during the run, grouped by category: `Pre-existing Code Issues`, `Tool Failures`, `Permission Prompts`, `External Reviewer Issues`, `CI Issues`, `Warnings`, and `Q/A`. Entries from Step 2's Q/A loop are appended progressively. Intermediate checkpoints append only the unrecorded tail. Step 18 performs the final non-truncating append before publication. This batch is the durable audit trail for follow-up work and operational events.

### session-transcript.jsonl

**Mode**: replace. **Written**: `/implement` Step 18 recaptures from the configured source on every terminal path, including normal green runs that completed Step 7a. `/design` captures once inside the shared `design log-publish` entry point, so Step 5c, clarify, and pause-save publish paths use the same hook. Standalone `/review` captures before cleanup and publishes staged batches so the transcript survives tmpdir removal. Historical archives are not backfilled.

A filtered, machine-readable rendering of the Claude Code session, produced by `scripts/larch.sh run-log render-session-transcript` from the raw session JSONL. **Schema v3.** The first line is a `{"v": 3, "source_basename": ..., "turns": N, ...}` header; subsequent lines are per-turn objects with a `blocks` array. Blocks carry user-typed slash commands and text, assistant prose, errored/warned `tool_result` entries, and sanitized reference `Read` stubs with normalized `file_path` values only. File contents, other `tool_call` blocks, and non-error `tool_result` blocks are omitted. Assistant `thinking` blocks are kept only when at least one `tool_use` in the same turn produced an errored result. Harness-injected SKILL.md expansions, attachments, and housekeeping events are dropped. Redacted for tmpdir paths and secrets before publication.

**Accepted capability loss (v3)**: full tool-sequence reconstruction for clean runs is not possible from the published transcript. The retained reference `Read` stubs support aggregate reference-heatmap measurements, not detailed incident forensics.

The `session-transcript` capture records `SESSION_TRANSCRIPT_STATUS` in the execution-issues `Warnings` section for every capture outcome, including `captured`, `suppressed-no-logs-commit`, `render-failed`, and `render-empty`. A terminal recapture failure retains the prior staged transcript when available, reports the failure, blocks publication, and preserves the session for retry. For `/implement` runs whose manifest records `steps_ran.step18=true`, `session-transcript.jsonl` is part of the required-file completeness manifest. When no source is configured, the execution issue names the missing artifact so I-Flush-1 can waive it. The recovery warning records only the discovered transcript basename, not the full operator-local path. A bounded-input condition adds a separate `status=render-bounded` warning entry without changing the capture status. See `docs/session-transcript-render.md` for the complete schema.

`scripts/larch.sh token measure-references-heatmap` synchronizes once, then starts with a `transcript_coverage` section that reports transcript-bearing runs, total runs, missing transcript runs, and the coverage ratio per skill before the per-reference heatmap rows. A skill with transcripts and zero reference reads is reported as measured zero data, not as missing data.

### round-<N>/

**Mode**: directory replace-by-file. **Written**: first at the end of each
`review core` round during `/implement` Step 5, then optionally refreshed
later in the same round after the coder finishes if `review-and-fix CLI`
produces additional registered artifacts (for example coder-side files).

Contains a curated set of per-round artifacts: the aggregate `findings.md`,
accepted / rejected findings, OOS review markdown, voting tally and summary,
`aggregator-dispatch.stderr` / `aggregator-validate.stderr` when the findings
aggregator fails (so execution issues can point at published paths instead of
`$REVIEW_TMPDIR`),
per-voter outputs (the byte-identical vote prompts and the raw per-specialist
reviewer outputs are excluded by Rust `round_artifact_included` during
`scripts/larch.sh run-log write-round` because the aggregates already cover their content),
panel manifest (with `archetype_ref` for dynamic slots — see below),
code-voter slots, the canonical waterfall `*.dropped-slots` ledger, bounded
`dropped-*-*.txt` diagnostics for dropped reviewer slots, and any later
registered coder artifacts. The `review core`
flush is the first snapshot for the round; `review-and-fix CLI` may run one more
`write-round` after coder application so the staged round directory reflects
the full round state before terminal archive publication. There is no
per-round publication.

**`round-meta.json`** (Phase 3c, issue #3716) — the per-round
sidecar files are consolidated into one JSON object rather than published
individually. Sections:

| Section | Source file |
|---|---|
| `tally` | `review-tally.env` (KV → JSON object) |
| `collector` | `collector-results.env` (raw text) |
| `summary` | `review-summary.json` (JSON passthrough) |
| `coder` | `coder.env` (KV → JSON object) |
| `difficulty` | scout difficulty sidecar, persisted difficulty record, or absent placeholder |
| `wrapper_logs.cursor` | `coder-cursor.wrapper.log` (raw text) |
| `wrapper_logs.codex` | `coder-codex.wrapper.log` (raw text) |

Absent sections are omitted except `difficulty`, which may carry `tier_in_effect`, `ceiling_in_effect`, `applied_tier`, `panel_tier`, `round_cap`, `codex_model_role`, `override_source`, `audit_evaluated`, `audit_upgrade`, `escalations`, empty escalation placeholders, and scout source fields when present. The audit scan `coder-tool` reads `round-meta.json`
as the primary source (`.coder.CODER_TOOL` via jq), falling back to `coder.env`
for rounds predating Phase 3c.

**Archetype pool** (Phase 3c): `reviewer-dyn-*.md` archetype definitions are
stored content-addressed in the owning run at
`round-<N>/archetypes/<sha256-12>.md`.
Entries in `panel-manifest.ndjson` carry `vendor` and `resolved_model` for each slot. Entries for `dyn-*` slots also carry an `archetype_ref`
field containing that round-relative path. Resolve the definition inside the
same immutable run archive. No top-level shared run-log pool exists.

## Tracking issue comments

The tracking issue carries marker-keyed summary comments as the workflow progresses. Most are slim run-scoped projections maintained by `/implement`; full payloads live in the remote archive and unpacked local cache. The exception is `larch:diagrams`: it is issue-scoped, jointly maintained by `/design` and `/implement`, and embeds Mermaid diagram bodies directly.

### `larch:metadata`

Written during **Step 0** when the tracking issue is adopted or created.

Content: storage provider, skill, run ID, agent (implementer coder), and larch plugin version. Public summaries do not expose repository log paths.

### `larch:plan`

Written at **Step 0** materialization tail after the plan is finalized.

Content: current plan-review tally status (voting outcome when present, or a pointer that detailed plan review lives in the `/design` run artifacts). The implementation plan is readable at the tracking issue body (`larch:plan` block via `manifest.json::issue_number`).

### `larch:diagrams`

Architecture is generated by `/design` Step 5b.5 after Gate C approval, then written by `/design` Step 5c via `scripts/larch.sh design step5c`; that orchestration entrypoint calls the `design publish` tail to upsert diagrams after the `larch:plan` block is successfully written. Code Flow is written by `/implement` Step 7a only when code-flow generation succeeds.

Content: the Architecture Diagram from `/design` and Code Flow Diagram generated
at Step 7a from the implementation diff, both embedded as Mermaid fences. The
stable marker is `<!-- larch:diagrams v1 -->` with no `runid=` segment.
Diagrams are embedded directly in this comment rather than written as a
run-log batch. Top-level design diagram body artifacts and diagram-generation
or sanitizer failure captures are excluded from published archives. Implement
code-flow diagram body files and `code-flow-diagram.failure.log` are not copied
into the run tree; bounded `execution-issues.md` warnings are the durable
failure surface.

### `larch:final-summary`

For `/implement`, the tracking summary is first rendered during Step 8+ PR
creation with placeholder PR fields, then refreshed with the live URL and again
during Step 18 terminal snapshot preparation. The terminal archive contains the
final `final-summary.md`. Runs that never reach PR creation still run terminal
snapshot preparation and may refresh the tracking summary with `PR: N/A`.

For `/design`, `scripts/larch.sh design log-publish` renders `final-summary.md`
before publishing the run tree. Step 5c and clarify follow with a post-publish
`scripts/larch.sh design render-final-summary` pass that upserts the same
marker-keyed tracking comment when an issue number is configured.
`failed-publish` summaries keep `Run logs: N/A` and append recovery metadata
when available. `publish-skipped` summaries keep `Run logs: N/A` and append the
skipped-publish note instead of recovery prose.

Content: final run status (`STALL_TRACKING` value), PR URL, and remote run
identity. The tracking-issue comment is the canonical live source for the PR
URL.

## Assessment retirement and manual-merge reconciliation

`/implement` Step 8 architectural assessment is authored by a read-only
`larch:arch-assessor` subagent and persisted fail-closed by
`architectural-assessment submit`; there is no per-kind vendor waterfall, no
`unavailable` state, no `architectural-assessment-unavailable` operator-bail,
and no `ship waive-assessment` verb on this path. A guideline `deviation` is
accepted at the ship gate only when its durable note carries the documented
`Exception:` block the fix ladder records: a non-empty rationale, the
`author: main-agent` tier, and a date that parses as a plausible calendar date.
A first submission that already carries an `Exception:` line is rejected
fail-closed; only the fix-ladder decline re-submission, which passes
`--allow-exception`, may persist one, so a forged block induced by untrusted
guideline or reference content cannot clear the gate (#7216). A bare deviation
fails closed with `architectural-guideline-deviation-unresolved`. A tier-2
invariant `violation` re-judge HARD STOPs with `invariant-violation-unresolved`
and creates no PR. Historical `assessment-operator-waiver.json` artifacts and
`unavailable` outcomes from older runs remain readable but are no longer
produced or waived; a legacy `unavailable` durable note resumed at a matching
HEAD routes back through the assessments route instead of composing an
unassessed PR (#7216).

`ship reconcile-manual-merge` first verifies the nominated PR is merged in the trusted repository. It then converges `ship-pr-state.sh`, `finalize-state.sh`, and `session-env.sh` on terminal `PHASE=done` state. Reconciliation clears `STALL_TRACKING`, `STALL_STEP`, `BAIL_REASON`, `IMPLEMENT_BAIL_REASON`, `BAIL_NEEDS_USER_INPUT`, `BAIL_FAILURE_DETAIL_LOG`, `FAILED_RUN_ID`, and nonzero `EXIT_CODE`. It re-reads all three layers, the post-merge sentinel, and the run manifest before emitting `RECONCILE_STATUS=ok`.

The verb does not render or publish. Corrected run records remain a local handoff for a normal archive retry; reconciliation never creates a post-merge Git commit or repair PR.

## Retention

Cloud retention is append-only. Publishers create one immutable object per run
and never slim, overwrite, or delete an archive. There is no Git retention or
garbage-collection workflow.

## Authoritative sources

- `docs/run-log-cli.md` — `run-log` verb contracts, log-root resolution, redaction rules
- `docs/run-log-batches.md` — canonical batch slug table (extension, mode, sanitizer)
- `docs/summary-comment-template.md` — marker literals and comment contracts
## Concise prune/log audit update

Concise review logs use `round-meta.json` `reviewer_signals[]` for reviewer
output audit scans instead of publishing raw transcripts by default. Implement
rounds include `prune-decision.env` and `prune-nit.env`; design plan-review
rounds default to the four-file concise contract while keeping run-root
`plan.txt`.

## /design failure-report artifacts

`/design` auto-reporting writes `design-failure-*.env` and `design-failure-*.md` artifacts under `$DESIGN_TMPDIR`. Important artifacts include terminal state, terminal report sentinels, escalation-success sentinels, operator-action sentinels, escalation ledgers, fallback chat print, operator-action chat audit, captured helper stdout/stderr sidecars, root-cause files, bounded root-cause files, and sensitive-corpus files.

`design-failure-terminal-state.env` is the terminal-state KV contract. Report helper stdout/stderr captures are retained beside `final-summary.md` so the summary body stays free of helper KVs.

## Rejected-analysis ledger and verdict sidecar

`/rejected-analysis` stores its mutable `rejected-analysis/ledger.tsv` and
`rejected-analysis/verdicts.tsv` below the repository-scoped
[analyzer state](analysis-state.md) root. They are not run archives.
The ledger records deterministic drops, verification outcomes, stale or
already-fixed results, dirty-tree rejects, security-sensitive skips, cap drops,
near-duplicate `alias_of` links, filed issue numbers, and deduplicated issue
mappings. Its primary key is `finding_hash`, computed from normalized
`file_path` plus normalized `concern` only. `line_hint`, `FINDING_N`, run ID,
round, voter slots, and filesystem state do not participate in the hash.

The verdict sidecar carries `finding_hash`, source skill, run ID, round, finding
ID, dissenting slots, verifier verdict, re-checked location, evidence, and
triage time for downstream diagnostics, `/voter-calibration` false-negative
labels, and `/difficulty-calibration` under-rating annotations.

`/difficulty-calibration` reads `difficulty-rating.json`, classification TSVs,
`review-findings-full.jsonl` or `review-findings.ndjson` fallbacks, token and
timing reports from the synchronized cache, and the verdict sidecar from
analyzer state. Missing pre-initiative artifacts degrade to counters.
Non-escalated runs without a parseable classification source report realized
tier `unknown`. The analyzer does not write run-log batches.

The collector reads implement artifacts from `larch-logs/implement/<run>/round-*/review-findings-full.jsonl` with `round-*/findings-classification.tsv`, falling back to the run-root JSONL only when no round-local JSONL exists. It reads standalone review artifacts from `larch-logs/review/<run>/review-findings.ndjson` with `review-findings-classification-round-*.tsv`, using `review-findings-full.jsonl` only as a fallback.

Each run work directory also contains session-private `ingest-status.jsonl`.
One row is appended per verifier launch attempt. `launch-failed` rows stay
retryable and are not ledgered as verification failures. `parse-failed`,
`location-mismatch`, `dirty-tree`, stale, and already-fixed rows are terminal
dispositions. `issue-cluster-map.json` maps `/issue` batch indexes to finding
hashes so record can map created and deduplicated issues without parsing issue
prose.

## Scope-disposition batch

`/implement` writes a `scope-disposition.json` run-log batch when an operator records a partial-scope decision. The JSON payload includes the coverage fingerprint, disposition, follow-up issue reference, untouched count, total firm paths, and bounded `todos_left` count. Final summaries project the same state as a plan-coverage line.
