# Stall-recovery runtime

`scripts/larch.sh stall-recovery` owns every runtime command: classification, attempt history, retry policy, normalization, escalation recording, state mutation and validation, reporting, corpus generation, Tier A deduplication, Tier B validation, chat rendering, development-clone detection, and contract lint. Each command has one owner and no fallback.

## Canonical `/implement` artifacts

The `/implement` runtime uses pinned files under `$IMPLEMENT_TMPDIR`:

- canonical ledger: `stall-recovery-escalation-ledger.tsv`
- ledger fallback: `stall-recovery-escalation-fallback.tsv`
- ledger write failure marker: `stall-recovery-escalation-record-failure.env`
- terminal sentinel: `stall-recovery-terminal-report.env`
- escalation-success sentinel: `stall-recovery-escalation-success.env`
- classification state: `stall-recovery-classification.env`
- pending clear transaction: `stall-recovery-clear.transaction.env` (private and present only while recovery is incomplete)
- prompt-state sensitive supplement: `stall-recovery-sensitive-corpus.env`
- issue input artifact: `stall-recovery-issue-input.md`
- chat-print artifact: `stall-recovery-chat-print.md`
- operator-action record: `stall-recovery-operator-action-record.md`
- operator-action sentinel: `stall-recovery-operator-action.env`
- root-cause finding: `stall-recovery-root-cause.md`
- bounded root-cause finding: `stall-recovery-bounded-root-cause.md`
- root-caused title: `stall-recovery-title.txt`
- Tier A dedup slices: `stall-recovery-tier-a-attempts.md`, `stall-recovery-tier-a-escalation.md`, `stall-recovery-tier-a-root-cause.md`
- Tier B dedup slices: `stall-recovery-bounded-attempts.md`, `stall-recovery-bounded-escalation-summary.md`, `stall-recovery-bounded-root-cause-public.md`

`/implement` defaults remain unchanged. Generic callers may opt into prefixed artifacts with `--profile generic --artifact-prefix <prefix>`. The `/design` port uses `--artifact-prefix design-failure`, so default `stall-recovery-*` filenames remain byte-compatible for `/implement` while `/design` writes `design-failure-*`.

## Generic profile

Generic mode is selected with `--profile generic`. Supported generic flags are `--artifact-prefix`, `--implement-tmpdir`, `--primary-state-file`, `--finalize-state-file`, and `--session-env-file`. Flags may appear before the subcommand, and state override flags are honored by `classify` and `compose-report`.

The generic profile exposes two Rust-owned validation APIs through `scripts/larch.sh stall-recovery`:

- `validate-token --token-kind outcome|step|phase|site|trigger|bail|source-script|root-cause --value <token>` validates safe vocab without sourcing private helper internals.
- `validate-terminal-state --primary-state-file <file>` validates required terminal-state keys, design vocab, path confinement, symlink rejection, and redaction rules.

Generic `/design` callers map `design-failure-terminal-state.env` into the classification model. Required keys include `FAILURE_OUTCOME`, `STALL_STEP`, `PHASE`, `SITE`, `TRIGGER`, `BAIL_REASON`, `EXIT_CODE`, `FAILURE_DETAIL_LOG`, and `SOURCE_SCRIPT`; optional keys include `ROOT_CAUSE_HINT`, `SUMMARY_OUTCOME`, `OCCURRED_AT`, and `EVIDENCE_REF`.

Generic `/design` examples include step `judge-panel`, phase `judge-panel`, site `decompose-panel`, trigger `decompose-panel-retry-exhausted`, and source script `split-path`. These tokens live in the safe-value helpers for the generic profile, not in the Tier B field allowlist TSV unless a new public field is exposed.

Escalation-success uses durable ledger evidence with `compose-report --report-kind escalation-success`; it does not run terminal `classify`. Generic public dedup uses a skill/profile-aware seed version so `/design` signatures cannot collide with `/implement` signatures.

## Subcommands

- `init-attempts --implement-tmpdir <path> --attempts-file <path>` initializes retry history.
- `classify --implement-tmpdir <path> ...` emits sanitized KVs and writes `stall-recovery-classification.env`. It refuses a pending clear transaction rather than publishing a classification from mixed state. For `/implement`, either Step 2's shared marker in raw `BAIL_REASON` or a nonempty `GOVERNANCE_REASONS` value other than `no-verdict` in `ship-pr-state.sh` maps to `contract-failure` / `none` / `migration-governance-block`. Evidence text cannot trigger it.
- `record-attempt ...` appends retry attempt history under a stable private companion lock, so atomic ledger replacement preserves every successful concurrent append. It refuses a malformed existing ledger rather than rewriting it.
- `retry-policy --class <class>` emits the retry-cap table projection. Retry caps are part of the public recovery policy.
- `record-escalation --implement-tmpdir <path> --site <token> --trigger <token> --step <token> --phase <token> [--dispatcher <token>] [--exit-code <n>] [--failure-detail-log <path>]` appends one canonical ledger row. The canonical ledger path is always `stall-recovery-escalation-ledger.tsv`. `--failure-detail-log` is optional evidence and never blocks ledger recording. Structural detail-log misses are skipped with `detail_log_skipped=failure-detail-log-*` on the ledger row while escalation still succeeds. Oversize detail logs are truncated to the optional-evidence cap and attached through a tmpdir-local sidecar when materialization succeeds; truncation failure still succeeds with `detail_log_skipped=failure-detail-log-truncate-failed`. `hard_fail` and `Tool Failure: record-escalation` are reserved for token validation failures, unsafe ledger, fallback, or marker paths, and total recording failure when canonical append fails and fallback or marker evidence cannot be written. Token-validation failures name the failing kind and a sanitized value in both stderr and the Tool Failure `reason` field (`token-validation-failed kind=<kind> value=<sanitized>`). Canonical-ledger append failure alone is not a Tool Failure when fallback evidence is written successfully; it returns `0` and emits `ESCALATION_FALLBACK_WRITTEN=true`. Append failures can still write fallback evidence or the record-failure marker.
- `normalize-outcome --implement-tmpdir <path>` is the shared final-outcome API used by `write-final-report.sh` and Step 18a.5. It refuses a pending clear transaction and otherwise emits `IMPLEMENT_NORMALIZED_OUTCOME=<token>`, `IMPLEMENT_OUTCOME_SUCCEEDED=true|false`, stall-tracking layer diagnostics, and the state fields used in the decision.
- `compose-report --report-kind terminal-failure|escalation-success --surface issue-input|chat-print ...` is the single public report-rendering API. It writes Tier A issue input or Tier B chat-print output and emits normalized report env fields.
- `dedup-tier-a-report --implement-tmpdir <path>` runs normalized Tier A exact-signature dedup in the current repository. With `--create-after-dedup true`, it keeps the prior lookup-failure-create behavior but performs lookup and creation through one in-process `file-report` operation, so the created body is the approved snapshot.
- `normalize-file-failure-report-env ...` maps `file-report` `FILE_FAILURE_REPORT_*` output to canonical `STALL_RECOVERY_REPORT_*` output.
- `validate-tier-b-public-file ... --snapshot-fd FD` rebuilds the effective sensitive corpus, reads a bounded no-follow regular-file snapshot with identity checks across the read, and writes the approved Tier B body or comment to a caller-owned unlinked descriptor for transport. Its `--publication-tier tier-a` transport mode snapshots already-redacted Tier A inputs with the same file-safety checks. `--public-fd FD` permits Tier B to revalidate an already-approved unlinked descriptor without reopening its original source path.
- `normalize-issue-env ...` persists canonical issue number and URL after an issue-creation result returns. It collapses embedded carriage returns and newlines in accepted issue metadata before validation so the persisted env file remains one `KEY=value` row per line.
- `chat-print ...` is a convenience wrapper for `compose-report --surface chat-print`.
- `is-larch-dev-clone`, `clear-stall`, `seed-terminal-state`, and `lint` keep their existing operational roles through the Rust runtime.

`clear-stall` preflights every durable stall layer plus `stall-recovery-classification.env` and `stall-recovery-issue.env` before any destructive mutation. It also finishes required abandoned-bgjob recovery before publishing a private pending transaction marker. The marker records the start inventory, remains through every atomic state rewrite and derived-artifact removal, and is removed only after read-back verifies the completed clear. An interrupted clear therefore remains recognizable to `classify` and `normalize-outcome`; retry resumes its incomplete phases, preserves unrelated state keys, and emits `CLEARED=true` only after all expected layers have `STALL_TRACKING=false` and an empty `STALL_STEP`. It never writes `IMPLEMENT_STALL_TRACKING`.

`bug-body`, `bug-comment`, and `issue-input-file` are retired report surfaces.

## Outcome normalization

`normalize-outcome` preserves the final-summary precedence:

1. Any observed `STALL_TRACKING=true` in `ship-pr-state.sh`, `finalize-state.sh`, or `session-env.sh` maps to `stalled`.
2. `FORKED_TARGET=true` maps to `forked-dry-run`.
3. `DESIGN_ONLY_DONE=true` maps to `design-only`.
4. `MERGE_RESULT=merged` or `admin_merged` maps to `merged`.
5. `MERGE_RESULT=already_merged` maps to `force-merged-externally`.
6. A non-zero draft PR maps to `pr-created-draft`.
7. A non-zero non-draft PR with `MERGE=false` maps to `pr-created`.
8. Otherwise the outcome is `bailed`, except `BAIL_NEEDS_USER_INPUT=true` remaps only that fallthrough to `bailed-needs-user-input`.

Step 18a.5 treats only `merged`, `force-merged-externally`, `pr-created`, `pr-created-draft`, and `forked-dry-run` as success. Unknown, partial, failed, invented, or missing outcomes do not succeed. Every observed `STALL_TRACKING` layer must be false.

## Escalation-success evidence

Step 18a.5 counts only these evidence sources:

- non-empty canonical ledger
- non-empty fallback ledger
- non-empty record-failure marker
- uniquely tagged `record-escalation` Tool Failure entries

Generic Tool Failures do not trigger escalation-success reporting. Terminal failure absorbs ledger, fallback, and marker evidence into the terminal report. A run publishes at most one report.

## Root-cause artifacts

Main Claude must investigate before composing. The root-cause file schema is:

```text
verdict=larch-defect|environment|operator-action
confidence=low|medium|high
summary=<single-line safe summary>

<finding prose with durable evidence citations>
```

`operator-action` writes the local non-filing record and sentinel, then skips public filing or printing. This also applies after successful merge so cleanup has a durable non-filing record.

## Tier behavior

Tier A is a larch dev clone with `FORKED_TARGET=false`. Tier A uses `issue-input`, bypasses TSV field allowlists, and redacts secrets from the complete public heading and body. It may include run linkage, branch, PR URL, validated logs, run-log pointer, full attempts, escalation ledger, root-cause finding, and verbatim bail reason after secret redaction. Tier A snapshots, exact-signature deduplicates, and creates in the current larch repository through `dedup-tier-a-report --create-after-dedup true`, which calls the Rust-owned `file-report` path in process.

Tier B covers consumer repos and forked runs. Tier B writes the sanitized `chat-print` artifact, resolves the upstream larch repository from `.claude-plugin/plugin.json`, then files or comments in that upstream repository through `scripts/larch.sh stall-recovery file-report`. It renders allowlisted machine fields plus validated bounded root-cause prose. `compose-report` requires `stall-recovery-sensitive-corpus.env` for Tier B. `file-report` applies the same Rust-owned validation as `scripts/larch.sh stall-recovery validate-tier-b-public-file --snapshot-fd`, which remains the descriptor publication boundary for external callers. The verb holds the approved body and comment bytes in process. Marker lookup, create-title derivation, and GitHub transport consume those frozen bytes without reopening the caller path. Allowlisted larch operational enums and machine fields are exempt, including step tokens, phase tokens, site tokens, trigger tokens, bail tokens, dispatcher names, `lint-fix-loop`, `ship-pr`, and `main-agent-required`.

Consumer and forked runs file Tier B reports on the public upstream larch repository under the operator's GitHub identity. If repo resolution, lookup, auth, network, create, or comment posting fails, the verb emits `STALL_RECOVERY_REPORT_STATUS=fallback-print-required` and preserves `stall-recovery-chat-print.md` for manual filing.

Tier B bail-token rendering derives from `larch_core::stall_recovery::implementation_bail_tokens`. Tier B sensitive-token sources include plan text, feature description, execution issues, validated failure-detail logs, raw attempt values, canonical ledger, fallback evidence, record-failure marker text, run-log pointer text, `finalize-state.sh`, `ship-pr-state.sh`, `session-env.sh`, prompt-state supplement values, repo names, branch names, PR URLs, issue text, plan text, and client paths.

## Titles

Terminal reports use the canonical bug prefix:

```text
[BUG] /implement terminal: <safe-root-cause-summary> (<class> at <step>)
```

Escalation-success reports use:

```text
[BUG] /implement escalation: <safe-root-cause-summary> (<site>:<trigger>)
```

Mixed-case `[Bug]` is accepted only as historical input for matching and title stripping; generated titles always use `[BUG]`.

Explicit title text comes from `stall-recovery-title.txt`. If it is unsafe, composition falls back to the validated root-cause summary. If neither is safe, composition fails closed and requires a rewrite. The full heading and body are redacted after composition.

## Public report dedup signature

Retry failure signatures and public report dedup signatures are separate. `FAILURE_SIGNATURE` remains private retry state. Public issue dedup uses `REPORT_DEDUP_SIGNATURE` only in this marker:

```text
<!-- larch-stall:signature=<64-hex> -->
```

The canonical seed grammar is:

- version line `larch-stall-report-dedup-v1`
- UTF-8 text with LF endings and a final newline
- fixed field order by report kind
- each field line encoded as `key<TAB>byte_length<TAB>value`
- SHA-256, lowercase 64-hex output

Terminal-failure seeds include only `report_kind`, `failure_class`, `step`, `phase`, and `safe_bail_token`. Escalation-success seeds include those fields plus sanitized `escalation_site` and `escalation_trigger` from the same first ledger or fallback row used in the title. The seed excludes dispatcher, matched classifier, evidence digests, paths, branches, run IDs, raw state, raw logs, and `skill=implement`. Generic-profile hashing includes the skill label and artifact prefix. `/implement` keeps the original seed unchanged.

Tier A places the marker immediately after the `###` title line so `file-report` derives its marker and title from the same approved snapshot. Tier B places the marker near the top of `stall-recovery-chat-print.md`. The final Tier B body is validated after marker insertion and before cross-repo filing.

## Filing and status normalization

`stall-recovery file-report` fetches one newest-first page of at most 100 open GitHub issue-list records with bodies through the typed GitHub service. It ignores pull requests, exact-matches the public marker, and comments `+1 occurrence` on duplicates. Older pages are outside the dedup window. Tier A and Tier B use the same Rust owner for create and comment paths. All issue and comment writes pass through `IssueMutationOwner`, including live-mutation authorization, redaction, identity checks, and exact read-back. Tier A calls `dedup-tier-a-report --create-after-dedup true`; a lookup failure proceeds to the same approved-byte create path, preserving fail-open creation without reopening `issue-input`.

Tier A and Tier B callers pass the authoritative session tmpdir with `--trusted-root`, the matching live run ID, and the mutation context file. `file-report` validates canonical-root containment and run identity before GitHub setup. It freezes approved public inputs before deduplication or transport. A source that changes while it is read, or is missing, oversized, non-regular, or symlinked, reaches no GitHub mutation. Later source replacement cannot alter the approved transport bytes. External descriptor consumers continue to use `validate-tier-b-public-file --snapshot-fd`; that command returns only a private, unlinked descriptor through `/dev/fd`.

Helper output maps to canonical status:

- `filed` to `STALL_RECOVERY_REPORT_STATUS=filed`
- `dry-run` to `STALL_RECOVERY_REPORT_STATUS=dry-run`
- `dedup-comment`, `no-match`, `fallback-print-required`, and `lookup-failed-open` pass through

`STALL_RECOVERY_REPORT_URL` is the canonical URL for notices. `STALL_RECOVERY_REPORT_ISSUE_URL` and `STALL_RECOVERY_REPORT_ISSUE_NUMBER` are compatibility aliases only for issue URLs. Dedup-comment URLs do not populate issue URL aliases.

Dry-run is local-only. It skips Tier A filing, upstream resolution, and cross-repo filing, then emits `STALL_RECOVERY_REPORT_STATUS=dry-run`.

Tier B dedup comments may include only bounded public slices: bounded attempts, allowlisted escalation site/trigger summaries, and bounded root-cause prose. Tier B callers must not pass raw ledgers, raw root-cause files, full report bodies, raw logs, paths, branches, or run IDs to the comment path.

## Surface Allowlists

Lint parity covers Tier B only. The committed TSV, Rust policy, and this table must remain byte-equivalent at the `surface + field_key + source + transform` level.

<!-- stall-recovery-allowlist:begin -->
| surface | field_key | source | transform |
|---|---|---|---|
| chat-print | report_kind | REPORT_KIND | enum |
| chat-print | failing_step | STALL_STEP | enum |
| chat-print | failing_phase | PHASE | enum |
| chat-print | failure_class | FAILURE_CLASS | enum |
| chat-print | bail_reason | BAIL_REASON | expanded-bail-token-union |
| chat-print | exit_code | EXIT_CODE | integer-or-unknown |
| chat-print | dispatcher | DISPATCHER | enum |
| chat-print | matched_classifier_pattern | MATCHED_CLASSIFIER_PATTERN | enum |
| chat-print | larch_version | larch-version | token |
| chat-print | run_id | RUN_ID | token-or-unknown |
| chat-print | attempt_table | attempts-file | allowlisted-attempt-fields |
| chat-print | escalation_site | escalation-ledger | enum |
| chat-print | escalation_trigger | escalation-ledger | enum |
| chat-print | fallback_escalation_marker | escalation-fallback | present-marker |
| chat-print | record_failure_marker | record-failure-marker | present-marker |
| chat-print | record_escalation_tool_failure | execution-issues | present-marker |
| chat-print | bounded_root_cause | bounded-root-cause-file | validated-larch-internal-prose |
<!-- stall-recovery-allowlist:end -->

## Retry Caps

| failure_class | attempts | delay |
|---|---:|---|
| transient-infra | 4 | `sleep-seconds.sh 5` |
| test-failure | 8 | none |
| lint-failure | 8 | none |
| dispatch-failure | 3 | none |
| protected-path | 1 | none |
| submodule-restricted | 0 | none |
| ci-fix-exhausted | 0 | none |
| same-cause-repeat | 2 | none |
| contract-failure | 0 | none |
| recoverable | 0 | none |
| unrecoverable | 0 | none |

For `same-cause-repeat`, the orchestrator uses the alternate strategy immediately. For `transient-infra`, the emitted retry delay means `sleep-seconds.sh 5` between attempts. `protected-path` means Codex hit a permanent protected-path sandbox policy; Main Claude resumes Step 2 inline; for `protected-path-edit-required-out-of-scope`, the operator warning names `.claude-plugin/plugin.json`. `submodule-restricted` means the external implementer hit a permanent submodule-edit restriction. It does not route to inline Step 2 recovery because Main Claude can also be blocked by the submodule-edit guard. `recoverable` means a `/design` Step 5c publish-tail failed after the plan was written; the run is salvageable by completing the remaining post-plan publish work, so it gets no automatic retry.

## Dry run

`LARCH_STALL_RECOVERY_DRY_RUN=1` makes report composition write local artifacts and emit `DRY_RUN_DECISION=true`. Callers must skip `/larch:issue`, Tier A dedup, upstream resolution, and cross-repo filing when dry-run is true.
