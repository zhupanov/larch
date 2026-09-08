# Step 9a.1 OOS Filing Contract

**Consumer**: `/implement` policy readers that need the Step 9a.1 combine, cap, conflict, filing, recovery, and evidence contract.

**Contract**: `${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh oos file` is the live Rust filing driver. It runs before `step-8-ship.sh`. Do not reproduce this pipeline in prompt-side Bash, invoke `/issue` as a fallback, or run its internal batch verbs in sequence. The Rust #7681 Step 8 workflow routes the private-security continuation and post-checkpoint bookkeeping.

**When to load**: Read this file when composing accepted OOS or interpreting Step 9a.1 evidence. It is a behavioral reference, not an executable procedure.

## Ownership

All six OOS commands migrated by #8178 and #8179 enter through `${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh`.

| Command | Rust driver | Rust policy owner |
| --- | --- | --- |
| `oos materialize-manifest` | `crates/larch-cli/src/oos_commands.rs` | `crates/larch-core/src/issue/oos_batch.rs` and `oos_record.rs` |
| `oos issue-cap` | `crates/larch-cli/src/oos_commands.rs` | `crates/larch-core/src/issue/oos_batch.rs` |
| `oos file-conflict-deps` | `crates/larch-cli/src/oos_commands.rs` | `crates/larch-core/src/issue/oos_conflict.rs` |
| `oos disposition-gate` | `crates/larch-cli/src/oos_commands.rs` | `crates/larch-core/src/issue/oos_disposition.rs` and `oos_record.rs` |
| `oos disposition-checkpoint` | `crates/larch-cli/src/oos_commands.rs` | `crates/larch-core/src/issue/oos_disposition.rs` and `oos_record.rs` |
| `oos file` | `crates/larch-cli/src/oos_file_commands.rs` | `crates/larch-core/src/issue/oos_filing.rs` plus the four policy modules above |

The `oos file` driver composes the cap, conflict planner, issue creation, dependency writes, and checkpoint in process. It does not spawn the five standalone OOS verbs or call `/issue`. The standalone verbs expose the same policy owners to distinct callers and tests.

## Live route

The #7681 Step 8 workflow router in
`crates/larch-cli/src/ship_pre_driver_commands.rs` composes the verified Rust
entrypoint. `crates/larch-cli/src/main.rs` dispatches the filing command to
`oos_file_commands.rs`.

The route has one Rust command implementation and enters through the verified wrapper.

## Filing stages

### 1. Resolve accepted inputs

The driver reads accepted blocks in this fixed order:

1. `$IMPLEMENT_TMPDIR/oos-accepted-main-agent.md`
2. the design artifact
3. `$IMPLEMENT_TMPDIR/oos-accepted-review.md`

The design artifact resolves from `$DESIGN_TMPDIR/oos-accepted-design.md` when that file exists, then `$IMPLEMENT_TMPDIR/design-export/oos-accepted-design.md`, then `$IMPLEMENT_TMPDIR/oos-accepted-design.md`. Missing files are empty inputs.

External implementer `oos_observations[]` reach the main-agent artifact through the Rust `oos materialize-manifest` owner. Prompt-side code must not parse the manifest JSON.

Security-routed observations remain in `$IMPLEMENT_TMPDIR/security-oos-observations.md`. They never enter public filing. A dedicated `- **focus-area**:` field whose value begins with `security`, including a suffix such as `security-hardening`, selects that route. Description prose that merely mentions `focus-area = security` does not. Private disposition follows `${CLAUDE_PLUGIN_ROOT}/docs/security/workflow-trust-and-mutations.md`.

### 2. Recover durable evidence

Before creating anything, the driver reads `$IMPLEMENT_TMPDIR/oos-issues-created.md`, the run-scoped `oos-issues.ndjson`, and structured `- **Filed URL**:` fields in accepted blocks.

It matches persisted evidence by filed URL or source-scoped stable ID first, then by normalized title. Combining and capping may bind several source IDs to one filed issue. A retry files only unmatched blocks. When only sentinel evidence survives, the driver materializes strict filed-URL evidence before the checkpoint.

This recovery is deterministic. The Rust filer does not invoke `/issue` semantic dedup.

### 3. Combine and cap the pending batch

The driver writes `$IMPLEMENT_TMPDIR/oos-combined.md`. For two or more pending blocks, it asks the bounded shared Codex launcher to combine related observations. Missing Codex, a failed launch, invalid output, or output that increases the item count keeps the original batch and records a warning.

The Rust issue-cap owner then applies `OOS_ISSUES_PER_RUN_CAP` in process. A cap refusal fails closed before issue creation. Rollups preserve every source block and inherit `oos-correctness` priority when any source has `focus-area: correctness` or `focus-area: regression`.

### 4. Plan file-conflict edges

The shared Rust conflict owner parses the post-cap batch and writes `$IMPLEMENT_TMPDIR/oos-intra-batch-deps.tsv`. A lower 1-based batch index blocks a higher index when their file ranges overlap under the inclusive rule.

A non-empty plan controls typed creation order and blocker writes. An empty plan adds no intra-batch edge. If conflict-cap validation, conflict parsing, or dependency-file output fails, the driver records a warning and a `Tool Failures` entry, removes unusable output when required, and continues without deterministic conflict edges. The separate per-run issue cap fails closed before issue creation. The driver does not invoke an LLM dependency fallback.

### 5. Create and wire issues

The Rust `FilingGateway` probes the tracking issue before filing. Each item is sanitized through `larch_core::issue::oos_batch::sanitize_public_text`, parsed through the shared issue-input owner, and created through the typed Rust issue-mutation path. The driver then verifies the tracking-issue and intra-batch blocked-by relationships.

The process is automatic. Do not ask the operator for confirmation, call `/issue`, or reconstruct `--intra-batch-deps-file` arguments in prompt prose.

Partial failures preserve verified prior evidence. An issue created during the current pass is closed when its required dependency write or priority-label application cannot be completed. Successful items from an earlier pass remain intact. Mixed priority-label failures retain verified non-priority results and return a non-zero status with exact counters.

### 6. Persist evidence and checkpoint

The driver records verified filed results in the sentinel and run-scoped `oos-issues.ndjson` before it evaluates disposition. A failed or interrupted checkpoint may therefore leave provisional filing evidence for a safe retry. That evidence alone does not prove Step 9a.1 completion.

The driver composes the Rust disposition checkpoint in process. On a normal zero result, it writes `run-statistics.md` and stamps `steps_ran.step9a1`. A private security sidecar returns its distinct pending status and leaves the workflow on the #7681 continuation. The Rust Step 8 router clears `OOS_PENDING` only after the later checkpoint and its bookkeeping succeed.

## Carve-outs

- `forked_target=true`: create no public OOS issue. Record synthetic `skipped://oos/<N>` disposition evidence in the run batch, run the checkpoint, and preserve the accepted observations for final reporting.
- `repo_unavailable=true`: create no public OOS issue. Record the skipped disposition in the run evidence.
- Non-empty `security-oos-observations.md`: use the private security process. Never file its content through this pipeline.

## Sentinel format

`oos-issues-created.md` is a Markdown table consumed as loose disposition evidence:

| OOS title | Issue | URL |
| --- | --- | --- |
| Example OOS title | #123 | https://…/issues/&lt;n&gt; |

- **Filed**: &lt;N&gt;

Each persisted row carries one verified created or recovered issue URL. The URL must contain a literal `https://…/issues/<n>` token so the Rust gate can count it.

## Focused tests

```text
cargo test --locked --package larch-cli --bin larch oos_file_commands::tests
cargo test --locked --package larch-cli --bin larch oos_commands::tests
cargo test --locked --package larch-core --lib issue::oos_
make oos-disposition-gate-bash-harness
```
