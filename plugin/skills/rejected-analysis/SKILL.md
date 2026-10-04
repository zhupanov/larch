---

# larch-run-lifecycle: shared-v1 skill=rejected-analysis
name: rejected-analysis
description: "Use when recovering verified real rejected code-review findings from synchronized larch run logs and filing GitHub issues for findings that remain unfixed."
argument-hint: "--n DAYS"
allowed-tools: Bash, Read, Write, Agent, Skill
---

**MANDATORY: Follow the complete shared lifecycle contract in `${CLAUDE_PLUGIN_ROOT}/skills/shared/run-lifecycle.md` with declared skill `rejected-analysis`.**

# rejected-analysis

**MANDATORY: READ ENTIRE FILE before composing user-facing prose: `${CLAUDE_PLUGIN_ROOT}/skills/shared/readability-style.md`.**

Recover **verified real** rejected code-review findings from synchronized run logs, then file the smallest safe set of issues.

This is **mutating**. It files by default after verification.

## Usage

`/rejected-analysis --n DAYS`

- Accept exactly two tokens: `--n` and a positive integer day count.
- Reject missing, non-integer, zero, negative, or extra arguments.
- No other public flags exist.

## Command KV binding contract

Claude Code Bash fences do **not** preserve shell state.

After **every** `scripts/larch.sh rejected-analysis` fence, parse whole-line `KEY=value` rows from stdout before any later Bash, Agent, or Skill step.

Required bindings after **prepare**:

- `CORPUS_ROOT`
- `WORK_DIR`
- `VERIFY_COUNT`
- `VERDICTS_FILE`
- `INGEST_STATUS_FILE`
- `LEDGER_PENDING_FILE`
- `ISSUE_SENTINEL`
- `REPO_ROOT`
- each `VERIFY_PROMPT_<candidate-id>` row

Required bindings after each **ingest-verdict** call:

- `INGEST_STATUS`
- optional `INGEST_DISPOSITION`

Required bindings after **finalize**:

- `CONFIRMED_COUNT`
- `ISSUE_BATCH_FILE`
- `ISSUE_CLUSTER_MAP_FILE`
- `ISSUE_SENTINEL`
- `LEDGER_PENDING_FILE`
- `INGEST_STATUS_FILE`
- `ISSUE_OUTPUT_STUB`
- `LAUNCH_FAILURES`

Required bindings after **record**:

- `LEDGER_APPENDED`
- `UNMAPPED_CONFIRMED`
- `RECORD_EXIT_RC`
- issue and dismissed counters when present

Never expand unbound `$WORK_DIR`, `$VERDICTS_FILE`, `$INGEST_STATUS_FILE`, or `$ISSUE_SENTINEL` literals in later fences. Use only the parsed values retained in the orchestrator context.

## Workflow

### Step 1: Validate arguments

Parse `$ARGUMENTS` mentally.

- It must match `--n DAYS` exactly.
- `DAYS` must be a positive base-10 integer.
- If validation fails, print a concise `/rejected-analysis` usage error and stop before creating any work dir.

### Step 2: Prepare candidates

Synchronize immutable inputs once, parse one whole-line `CORPUS_ROOT`, then run:

```bash
SYNC_OUT=$("${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" run-log sync)
"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" rejected-analysis prepare \
  --days "$DAYS" --log-root "<parsed CORPUS_ROOT>"
```

Parse and retain `WORK_DIR`, `VERIFY_COUNT`, `VERDICTS_FILE`, `INGEST_STATUS_FILE`, `LEDGER_PENDING_FILE`, `ISSUE_SENTINEL`, `REPO_ROOT`, and every `VERIFY_PROMPT_<candidate-id>=<path>` row.

The Rust prepare command owns local cache discovery, vote joins, 1-YES inclusion, 0-YES drops, OOS-deferred drops, security-sensitive drops, near-duplicate collapse, overlap against at most the 100 most recent open issues, cap accounting, prompt rendering, `ledger-pending.tsv`, empty `verdicts.jsonl`, and empty durable `ingest-status.jsonl`. Later steps read ordinary files below the parsed corpus root and perform no more cloud operations.

The frozen `finding_hash` excludes run-local `FINDING_N`. It hashes only normalized `file_path` and normalized `concern`. It never uses live filesystem existence to choose the hash path.

### Step 3: Skip only verification when there are no candidates

When parsed `VERIFY_COUNT=0`, skip only Steps 4 and 5.

Still run Step 6 `finalize` and Step 8 `record` so prepare-owned
`ledger-pending.tsv` rows merge into repository-scoped analyzer state.

### Step 4: Launch read-only verification

For each parsed `VERIFY_PROMPT_<candidate-id>` path, launch one external verifier:

```bash
"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" agent launch-review \
  --tool <cursor|codex per availability> \
  --output "<parsed WORK_DIR>/verdict-<candidate-id>.txt" \
  --stderr-sink "<parsed WORK_DIR>/verdict-<candidate-id>.failure.log" \
  --timeout 600 \
  --timing-task-kind rejected-analysis-verify \
  --site rejected-analysis verify \
  --prompt-file "<parsed prompt path>"
```

`agent launch-review` enforces read-only internally. Codex uses its read-only sandbox. Cursor uses ask mode plus dirty-tree baseline checks.

Do **not** pass `--sandbox` or other unsupported launcher flags.

Parse launcher stdout for:

- `OUTPUT`
- `LAUNCHER_EXIT`

When `LAUNCHER_EXIT!=0`, increment the retained `LAUNCH_FAILURES` counter. Do not treat the output file as authoritative. Do not ledger that candidate yet.

When `LAUNCHER_EXIT=0`, read `${OUTPUT}.dirty-tree`. If that sidecar has `STATUS=dirty` or `STATUS=unknown`, reject the verdict for filing purposes. The ingest path records `dismissed:dirty-tree`.

### Step 5: Ingest each launch attempt

For every launch attempt, call the Rust command. Do not prompt-side parse JSON from the launcher output file.

```bash
"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" rejected-analysis ingest-verdict \
  --work-dir "<parsed WORK_DIR>" \
  --candidate-id "<candidate-id>" \
  --output "<parsed OUTPUT>" \
  --launcher-exit "<parsed LAUNCHER_EXIT>"
```

Parse `INGEST_STATUS`.

Valid values are:

- `ingested`
- `launch-failed`
- `dirty-tree`
- `parse-failed`
- `location-mismatch`

Every ingest call appends exactly one durable row to `ingest-status.jsonl`, including `launch-failed` and `parse-failed`. `finalize` treats `ingest-status.jsonl` as authoritative. It never ledgers `launch-failed` as `verification-failed`.

### Step 6: Finalize outcomes

If `LAUNCH_FAILURES>0`, still continue through `finalize` and `record`. Exit non-zero only after `record` persists safe rows.

Run:

```bash
"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" rejected-analysis finalize --work-dir "<parsed WORK_DIR>"
```

Parse `CONFIRMED_COUNT`, `ISSUE_BATCH_FILE`, `ISSUE_CLUSTER_MAP_FILE`, `ISSUE_SENTINEL`, `LEDGER_PENDING_FILE`, `INGEST_STATUS_FILE`, `ISSUE_OUTPUT_STUB`, and `LAUNCH_FAILURES`.

`finalize` re-runs the security-sensitive classifier before issue rendering. Confirmed security-sensitive findings are not public-filed. They are ledgered locally as `dismissed:security-sensitive`, and the operator must follow `SECURITY.md` responsible disclosure guidance.

### Step 7: File confirmed non-security clusters via `/issue`

Run this step only when parsed `CONFIRMED_COUNT>0` and parsed `ISSUE_BATCH_FILE` is non-empty.

First clear the sentinel defensively:

```bash
rm -f "<parsed ISSUE_SENTINEL>"
```

Invoke `/issue` via the Skill tool:

`/issue --input-file "<parsed ISSUE_BATCH_FILE>" --sentinel-file "<parsed ISSUE_SENTINEL>"`

> **Continue after child returns.** When the child Skill returns, immediately continue with this skill's next step. Do not end the turn, summarize, or hand off.

Capture exact `/issue` stdout immediately after the Skill returns. Write it to `"<parsed WORK_DIR>/issue.stdout.txt"`, or to parsed `ISSUE_OUTPUT_STUB` when provided.

Parse captured stdout for:

- `ISSUES_CREATED`
- `ISSUES_FAILED`
- `ISSUES_DEDUPLICATED`
- `ISSUE_<i>_NUMBER`
- `ISSUE_<i>_URL`
- `ISSUE_<i>_DUPLICATE=true`
- `ISSUE_<i>_DUPLICATE_OF_NUMBER`
- `ISSUE_<i>_DUPLICATE_OF_URL`

Verify the sentinel:

```bash
"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" verify skill-called --sentinel-file "<parsed ISSUE_SENTINEL>"
```

Set retained `ISSUE_VERIFIED=true|false` and `ISSUES_FAILED=<n>` from parsed stdout plus sentinel verification.

Do not exit before `record`. Partial `/issue` failures and sentinel failures are surfaced to `record`.

### Step 8: Record the ledger

When `/issue` was skipped, skip sentinel verification and pass no issue output, or pass the empty parsed `ISSUE_OUTPUT_STUB`.

When Step 7 ran, pass `--issue-verified true` only after sentinel verification succeeds. Pass `--issue-verified false` when sentinel verification fails. Omitting `--issue-verified` after a non-empty `/issue` stdout is an error surfaced by `record` as `UNMAPPED_CONFIRMED=true` with non-zero `RECORD_EXIT_RC`.

Always invoke `record` after `finalize`:

```bash
"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" rejected-analysis record \
  --work-dir "<parsed WORK_DIR>" \
  [--issue-output "<parsed WORK_DIR>/issue.stdout.txt"] \
  [--issue-verified true|false] \
  [--issues-failed <n>] \
  [--launch-failures <n>]
```

`record` uses `issue-cluster-map.json` as the authoritative cluster-to-hash map. It writes safe dismissed rows even when `/issue` partially fails. It maps resolved clusters to `filed-as` or `deduped-as` when `ISSUE_VERIFIED=true` even if `ISSUES_FAILED>0`, withholding only unmapped clusters. It never marks unmapped confirmed hashes as `filed-as` when `ISSUE_VERIFIED=false`.

### Step 9: Exit

After `record` returns, exit non-zero when any condition holds:

- `record` reports a ledger write or readback failure.
- `record` reports `UNMAPPED_CONFIRMED=true`.
- parsed `ISSUES_FAILED>0`.
- `ISSUE_VERIFIED=false` after `/issue` ran.
- `LAUNCH_FAILURES>0`.
- `RECORD_EXIT_RC` is non-zero.

Report success counters only when all conditions are clear and `record` reports `LEDGER_APPENDED` with readback OK.

## Implementation files

- `scripts/test-rejected-analysis.sh` (contract: `scripts/test-rejected-analysis.md`) is the offline structural harness for this skill.

## NEVER

- Never file 0-YES rejected findings.
- Never file stale, already-fixed, dirty-tree, or verification-failed findings.
- Never file security-sensitive confirmed findings as public GitHub issues.
- Never file `scope=oos`, `scope=out_of_scope`, or `OOS_*` deferred findings.
- Never bypass the ledger because `/issue` dedup is fuzzy.
- Never treat this as voter scoring.
- Never include `/design` plan-review findings in v1.
- Never skip `record` because `/issue` or sentinel verification failed.
- Never mark unmapped confirmed hashes as `filed-as` when `ISSUES_FAILED>0`.
- Never reuse a stale `/issue` sentinel without `rm -f` immediately before invocation.
- Never use unbound `$WORK_DIR`, `$VERDICTS_FILE`, `$INGEST_STATUS_FILE`, or `$ISSUE_SENTINEL` across Bash fences.
- Never pass unsupported `agent launch-review` flags, including `--sandbox`.
- Never prompt-side `json.loads` launcher output files. Use `ingest-verdict`.
- Never ledger launcher or infra failures as `verification-failed`. They remain retryable.
- Never infer verification failure from absent `verdicts.jsonl` rows when `ingest-status.jsonl` records `launch-failed`.
- Never use live filesystem existence to choose hash `file_path`.
- Never claim success when `ISSUES_FAILED>0`, `ISSUE_VERIFIED=false`, or `LAUNCH_FAILURES>0`.
- Never include run-local `FINDING_N` in `finding_hash`.
- Never accept verifier `current_location` that does not bind to the candidate's normalized `file_path` and optional `line_hint`.
