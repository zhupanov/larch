# /implement Step 18a stall recovery

**Consumer**: `/implement` Step 18a.

**Contract**: Step 18a reports terminal failures only. It never files or prints at first detection. `"$CLAUDE_PLUGIN_ROOT/scripts/larch.sh" stall-recovery` routes classification, attempts, normalization, escalation recording, report composition, corpus generation, Tier A deduplication, chat output, and contract lint to Rust.

**When to load**: MANDATORY before executing Step 18a active-stall recovery when `STALL_RECOVERY_REQUIRED=true`. Load before changing active-stall recovery report composition, escalation recording, or normalized outcome handling.

**MANDATORY: READ ENTIRE FILE before composing terminal stall reports, fallback print text, or root-cause prose: `${CLAUDE_PLUGIN_ROOT}/skills/shared/readability-style.md`.**

## Canonical artifacts

Use these `$IMPLEMENT_TMPDIR` paths for `/implement`:

- `stall-recovery-attempts.env`
- `stall-recovery-escalation-ledger.tsv`
- `stall-recovery-escalation-fallback.tsv`
- `stall-recovery-escalation-record-failure.env`
- `stall-recovery-terminal-report.env`
- `stall-recovery-classification.env`
- `stall-recovery-sensitive-corpus.env`
- `stall-recovery-issue-input.md`
- `stall-recovery-chat-print.md`
- `stall-recovery-operator-action-record.md`
- `stall-recovery-operator-action.env`
- `stall-recovery-root-cause.md`
- `stall-recovery-bounded-root-cause.md`
- `stall-recovery-title.txt`
- `stall-recovery-tier-a-attempts.md`
- `stall-recovery-tier-a-escalation.md`
- `stall-recovery-tier-a-root-cause.md`
- `stall-recovery-bounded-attempts.md`
- `stall-recovery-bounded-escalation-summary.md`
- `stall-recovery-bounded-root-cause-public.md`

Internal seams remain for a future `/design` profile. Do not add public generic profile flags here.

## Step 18a procedure for active stalls

1. **Resolve stall tracking.** Read in-memory state, then `ship-pr-state.sh`, `finalize-state.sh`, and `session-env.sh`. If every layer is false or empty, skip active-stall recovery.
2. **Initialize attempts.** Run `"$CLAUDE_PLUGIN_ROOT/scripts/larch.sh" stall-recovery init-attempts --implement-tmpdir "$IMPLEMENT_TMPDIR" --attempts-file "$IMPLEMENT_TMPDIR/stall-recovery-attempts.env"`.
3. **Classify.** Run:

```bash
"$CLAUDE_PLUGIN_ROOT/scripts/larch.sh" stall-recovery classify \
  --implement-tmpdir "$IMPLEMENT_TMPDIR" \
  --attempts-file "$IMPLEMENT_TMPDIR/stall-recovery-attempts.env" \
  --in-memory-stall-tracking "${STALL_TRACKING:-false}" \
  --stall-step "${STALL_STEP}" \
  --phase "${PHASE:-checks}" \
  --bail-reason "${IMPLEMENT_BAIL_REASON:-${FINAL_BAIL_REASON:-}}" \
  --exit-code "${EXIT_CODE:-unknown}" \
  [--failure-detail-log "$BAIL_FAILURE_DETAIL_LOG"]
```

Bind `EXIT_CODE` from captured composite stdout before Step 18 when durable stall seeding omits it (for example `checks-commit-route` `checks-child-failed` with no `REDACTED_LOG_FILE`). Pass any validated `BAIL_FAILURE_DETAIL_LOG`. The helper writes `stall-recovery-classification.env`, including `MATCHED_CLASSIFIER_PATTERN` and dispatcher identity when known.
4. **Do not file on first detection.** Classify, then retry or stop; record only retries. Warnings require matching classifier results:
   - `protected-path-edit-required-out-of-scope` warns on `.claude-plugin/plugin.json` and classifies as `FAILURE_CLASS=protected-path`.
   - `submodule-edit-required-out-of-scope` classifies as `FAILURE_CLASS=submodule-restricted` with `RESUME_HINT=none`, then prints `**⚠ /implement: implementer bailed on submodule-restricted path; submodule edits are blocked for Main Claude too. No automatic inline recovery will run.**` Step 2's raw `BAIL_REASON` marker and Step 8's structured `GOVERNANCE_REASONS` state can both produce `migration-governance-block` (`contract-failure` / `none`). For either site, print `**⚠ /implement: migration governance blocked. Re-run /design ISSUE_NUMBER to refresh the plan receipt against current main. No reship will run.**` Substitute the issue number; do not call `record-attempt`.
5. **Retry dispatch.** Follow caps in `docs/stall-recovery-report.md`. Record Main Claude handoffs, not retries or reships. Dispatch by `RESUME_HINT`. For protected-path `step2-impl`, `step2-impl` means record escalation before edits, then Main Claude reads `$IMPLEMENT_TMPDIR/plan.txt` and implements inline; Codex cannot edit the protected path. Resume normal checks through shipping. `submodule-restricted` / `none` never repairs inline. `step8-shippr` is the only retry branch that re-invokes `${CLAUDE_PLUGIN_ROOT}/skills/implement/scripts/step-8-ship.sh`; re-enter it through the foreground bgjob wrapper. The wrapper must rejoin a live identity-valid `implement-step8-ship` registry row with `"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" bgjob wait --step implement-step8-ship --tmpdir "$IMPLEMENT_TMPDIR" --max-wait-s 0`, refuse a second driver start, and clear only stale or dead rows before a fresh start. If the wrapper prints `BGJOB_STATUS=STARTED STEP=implement-step8-ship PGID=<n>`, continue with chunked `bgjob wait` per `skills/shared/bgjob-wait.md`. Treat `BGJOB_STATUS=DEAD`, `BGJOB_RC=timeout`, `BGJOB_RC=orphaned`, non-zero `BGJOB_RC`, or missing required KVs as the existing Step 8 failure or stall branch. `step5-review` resumes Step 5 review and reaches Step 8 only through the normal current-run sequence. `RESUME_HINT=checks-commit-route-retry` (`FAILURE_CLASS=transient-infra`, `MATCHED_CLASSIFIER_PATTERN=checks-leg-abandoned` or `MATCHED_CLASSIFIER_PATTERN=checks-child-sigterm`) means a `checks-commit-route` process died before writing `STALL_TRACKING`, left an identity-checked dead bgjob registry row, or the checks child exited by signal or an unresolvable exit code. For Step 3 (`implement-step3-checks`), re-invoke `${CLAUDE_PLUGIN_ROOT}/skills/implement/scripts/run-step-checks.sh --site step3 --commit-site step4 --rebase-checkpoint-4r`, then wait on `implement-step3-checks` with chunked `bgjob wait`. Step 6 classifies `checks-child-sigterm` accurately but does not get automatic retry dispatch. For `--self-review` Step 5 (`implement-checks-step5-self-review`), re-invoke `${CLAUDE_PLUGIN_ROOT}/skills/implement/scripts/run-step-checks.sh --site step5-self-review --commit-site step5-self-review`, then wait on `implement-checks-step5-self-review` with chunked `bgjob wait`. Genuine checks-content failures with a positive composite `EXIT_CODE` forwarded through `--exit-code` still classify as `contract-failure` / `RESUME_HINT=none`. Signal-killed or unresolvable `EXIT_CODE` values on `checks-child-failed` classify as `transient-infra` per the `checks-child-sigterm` pattern above.
6. **Record prompt-side Main Claude handoffs before edits.** Call `"$CLAUDE_PLUGIN_ROOT/scripts/larch.sh" stall-recovery record-escalation` before Step 18a inline `step2-impl` repair, and before inline `step8-shippr` repair **only when Step 18a itself performs Main Claude code edits**. Examples include protected-path inline implementation or CI edits after the Rust ship driver emitted `ledger_ready=true`. A reship with no Main Claude code edits is ordinary and must not record escalation: a `FAILURE_CLASS=transient-infra` / `RESUME_HINT=step8-shippr` reship clears stale handoff and re-invokes `step-8-ship.sh` directly. Pass the literal absolute `IMPLEMENT_TMPDIR` value parsed from Step 0 bootstrap output, not `$IMPLEMENT_TMPDIR` expanded in a new prompt-side Bash call. Stable owner tokens are `step2-impl` and `step8-shippr`; pass one of those `_COMMON_TRIGGERS` owner tokens as `--trigger`, never stall detail, vendor bail reasons (`cursor-runtime-failure`, `quota`, …), or classifier output such as `no-ci-checks-observed`. For Step 2 `step2-impl` handoffs pass exactly `--site step2 --trigger step2-impl --step 2 --phase implementation` plus `--dispatcher` from the stall classification (or the failed vendor binary: `codex` / `cursor` / `claude`) and `--exit-code` when known. Do not invent alternate sites such as bare `STALL_STEP` digits.
7. **Success after recovery.** Run `"$CLAUDE_PLUGIN_ROOT/scripts/larch.sh" stall-recovery clear-stall --implement-tmpdir "$IMPLEMENT_TMPDIR"`. Require `CLEARED=true`, then clear prompt-side stall tracking before the next normalization call.
8. **Terminal failure.** Seed durable terminal stall state with `"$CLAUDE_PLUGIN_ROOT/scripts/larch.sh" stall-recovery seed-terminal-state --implement-tmpdir "$IMPLEMENT_TMPDIR" --stall-step "$STALL_STEP" --phase "$PHASE"`. Before composing, Main Claude investigates and writes `stall-recovery-root-cause.md`. For possible Tier B, also write `stall-recovery-bounded-root-cause.md`, `stall-recovery-title.txt`, and `stall-recovery-sensitive-corpus.env`. Call `compose-report --report-kind terminal-failure` exactly once. Tier A uses `--surface issue-input`, then `dedup-tier-a-report --create-after-dedup true` snapshots, deduplicates, and files it. Tier B uses `--surface chat-print`; the helper resolves upstream larch, dedups, and files or comments unless dry-run is active. Write `stall-recovery-terminal-report.env` atomically after filing, commenting, fallback printing, dry run, or operator-action skip.
9. **Operator action.** If the root-cause verdict is `operator-action`, compose-report writes the non-filing record and sentinel. Do not file or print a public report.

**Prompt-side investigation probes.** During Step 18a.5-8, follow `BASH_AUTHORING.md` bounded-root guidance for every probe. Use `$CLAUDE_PLUGIN_ROOT`-anchored paths. Validate discovered tokens with `"$CLAUDE_PLUGIN_ROOT/scripts/larch.sh" stall-recovery validate-token --token-kind trigger --value CANDIDATE`; owner tokens are `step2-impl` and `step8-shippr`. Never search via `../` from `$IMPLEMENT_TMPDIR`.

## Tier policy

Tier A applies only when `is-larch-dev-clone` is true and `FORKED_TARGET=false`. It bypasses TSV allowlists and redacts the full public issue input, including headings. It may include run linkage, branch, PR URL, validated logs, run-log pointer, full attempts, escalation ledger, root-cause finding, and verbatim bail reason after redaction.

Tier B covers consumer repos and forked runs. It files or comments in the resolved upstream larch repository on success, and prints through chat only on fallback or dry-run. It uses allowlisted machine fields plus bounded root-cause prose. Bounded prose and title validation reject client repo names, branches, paths, PR URLs, plan or issue text, state or evidence values, attempts, ledger, fallback evidence, record-failure markers, run-log pointers, and prompt-state supplements. Before lookup, title derivation, or transport, the helper makes a bounded no-follow Rust snapshot in an unlinked descriptor; only its `/dev/fd` payload may publish. Source change or substitution during snapshot fails closed. Allowlisted larch operational terms are exempt.

## Root-cause finding schema

```text
verdict=larch-defect|environment|operator-action
confidence=low|medium|high
summary=<single-line>

<finding prose with durable evidence citations>
```

The finding must distinguish observation from inference and cite evidence by path or artifact name.

## Ship-pr and script handoff ownership

- `review-and-fix step5` records `coder-main-agent-required` directly.
- Step 5 `main-agent-vote-required` is emitted as `STEP5_REVIEW_LEDGER_*` for the prompt side to record once.
- `scripts/larch.sh checks lint-fix` emits `LINT_FIX_LEDGER_*` only for `main-agent-required` paths.
- The Rust ship driver emits ledger-ready data for handoffs and returns before recovery-waterfall edits on ship-pr-internal lint-fix `main-agent-required`.
- Clean retries, reships, and health-only paths do not record escalation events.

## Filing flow

Tier A and B share the descriptor-owning helper:

1. Compose `issue-input` and treat compose output as artifact metadata only.
2. In dry-run, skip filing and write `STALL_RECOVERY_REPORT_STATUS=dry-run`.
3. Run `"$CLAUDE_PLUGIN_ROOT/scripts/larch.sh" stall-recovery dedup-tier-a-report --create-after-dedup true`. It defaults its mutation context to `$IMPLEMENT_TMPDIR/session-env.sh`; an explicit `--context-file` remains available for a trusted file in that same session directory.
4. It snapshots the body, derives marker/title there, exact-signature deduplicates, then comments or creates from that descriptor. Lookup failure uses its fail-open-create route.
5. On `dedup-comment` or `filed`, record normalized URL fields; do not call another publisher.
6. On `fallback-print-required`, print the sanitized artifact instead of creating a duplicate.

Tier B files public reports in the resolved upstream larch repository:

1. Call `compose-report --surface chat-print`.
2. On `filed` or `dedup-comment`, print only a short notice using `STALL_RECOVERY_REPORT_URL`.
3. On `fallback-print-required`, print `stall-recovery-chat-print.md` for manual filing.
4. On `dry-run`, keep local artifact-only behavior.
5. On `skipped_operator_action`, keep the local sentinel and do not file.

`is-larch-dev-clone` selects content tier only. It no longer decides whether a public report is filed. Tier B passes only bounded public comment payload files to the cross-repo helper: bounded attempts, allowlisted escalation site/trigger summaries, and bounded root-cause prose. It must not pass raw root-cause files, raw ledgers, full report bodies, raw logs, paths, branches, or run IDs to the Tier B comment path.

Public report dedup uses the `REPORT_DEDUP_SIGNATURE` marker, not retry `FAILURE_SIGNATURE`. The marker is exact `<!-- larch-stall:signature=<64-hex> -->`. Terminal signatures include only `report_kind`, `failure_class`, `step`, `phase`, and `safe_bail_token`. Escalation-success signatures add sanitized `escalation_site` and `escalation_trigger`. Dispatcher, matched classifier, evidence digests, paths, branches, run IDs, raw state, raw logs, and `skill=implement` stay out of the Part 2 seed.
