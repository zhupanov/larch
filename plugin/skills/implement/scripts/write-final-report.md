# write-final-report.sh (`/implement`)

**MANDATORY: READ ENTIRE FILE before composing user-facing prose: `${CLAUDE_PLUGIN_ROOT}/skills/shared/readability-style.md`.**

Builds the **rich markdown** final run summary, writes the staged `final-summary.md` for terminal archive publication (unless `--comment-only`), upserts the tracking-issue `larch:final-summary` comment, and optionally mirrors the body to the renderer print stream via `--print-stdout`. Top-chat visibility is owned by the `/implement` orchestrator, which emits the persisted `summary-final.md` body verbatim after the Bash call per `skills/implement/SKILL.md`.

The markdown body is produced by the Rust `render run-summary` owner (`crates/larch-cli/src/rendering_commands.rs`): a `## /<skill> run <run-id>: <outcome>` heading, the normalized bullet list, then the `<!-- larch:run-summary v=1 -->` sentinel (see that script’s contract). The renderer always emits `- **Outcome**:` as the first bullet. Successful outcomes display as `✅ DONE`, `stalled` displays as `❌ STALLED`, and other outcomes display raw. It no longer emits `- **Mode**:`. It emits `- Force: true` when `run-flags.sh` has `FORCE_REQUESTED=true`, and omits `- **PR**:` when the normalized display would be `N/A`. Optional per-lane USD lines use [`larch_core::report::RATE_TABLE`](../../../crates/larch-core/src/report/token_cost.rs) and the env vars documented under **Per-vendor rates** in [`docs/configuration-and-permissions.md`](../../../docs/configuration-and-permissions.md). The cost line includes the spawned-process Claude lane (`Claude (subprocess)` / machine name `claude_sub`, issue #3637): this script reads `.claude_sub.totals.total` and `BUCKETS_claude_sub` from `token-report.json` and forwards `--claude-sub-*` token flags to the renderer.

## Implement outcome enum (`--outcome` raw values)

These values are emitted by the shared `scripts/larch.sh stall-recovery normalize-outcome` helper. `scripts/larch.sh final-report write` (via the thin `write-final-report.sh` wrapper) consumes that helper, and Step 18a.5 uses the same API for escalation-success reporting. Rust coverage in `crates/larch-cli/tests/final_report.rs` stays aligned with the helper.

1. `stalled`: any observed `STALL_TRACKING=true` in ship-pr state, finalize state, or session env.
2. `forked-dry-run`: `FORKED_TARGET=true`.
3. `design-only`: `DESIGN_ONLY_DONE=true`.
4. `merged`: `MERGE_RESULT` is `merged` or `admin_merged`.
5. `force-merged-externally`: `MERGE_RESULT=already_merged`.
6. `pr-created-draft`: non-zero `PR_NUMBER` and `DRAFT=true`.
7. `pr-created`: non-zero `PR_NUMBER`, `DRAFT=false`, `MERGE=false`.
8. `bailed`: none of the above success/partial paths matched; assigned only after the explicit if/elif chain as a fallthrough default.
9. `bailed-needs-user-input`: `BAIL_NEEDS_USER_INPUT=true` on finalize state **and** the outcome would otherwise be `bailed` (distinct bail class for operator follow-up).

## Bail-time `steps_ran` invariant

If the run ends before Step 9a.1 or before `${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh oos file` succeeds, the terminal manifest MUST NOT leave `steps_ran` as an ambiguous empty object for downstream audit tooling. Step 9a.1 completion requires post-checkpoint `run-statistics.md`; explicit `manifest.json` `steps_ran.step9a1=true` is valid only together with that file. `step9a1=true` without `run-statistics.md` is a stale or corrupt marker and must fail audit/verify scans. `oos-issues.ndjson` without `run-statistics.md` is provisional disposition evidence and must not suppress `steps_ran.step9a1=false`.

`scripts/larch.sh final-report write` records explicit `steps_ran.step9a1=false` (and `step8` / `step7a` when their on-disk artifacts are absent) for terminal non-merge outcomes (`bailed`, `stalled`, `design-only`, fork dry-run, PR-created-without-merge, etc.); a non-zero exit from that `run-log manifest` call fails finalization. `scripts/larch.sh run-log verify-completeness` treats missing/null `steps_ran` like `jq '.steps_ran // {}'` for the empty-object bail path, matching `scripts/larch.sh audit-runs scan-run`.

## Usage

```bash
write-final-report.sh --implement-tmpdir PATH [--comment-only] [--print-stdout]
```

## Inputs (files under `IMPLEMENT_TMPDIR`)

| File | Keys / role |
|------|----------------|
| `parent-issue.md` | `ISSUE_NUMBER`, `RUN_ID`, optional `ISSUE_URL` |
| `session-env.sh` | `REPO`, `REPO_UNAVAILABLE`, `UPSTREAM_DESIGN_ISSUE` |
| `ship-pr-state.sh` | `PR_URL`, `PR_NUMBER`, `STALL_TRACKING`, `MERGE_RESULT`, `MERGE`, `DRAFT`, `FORKED_TARGET` |
| `finalize-state.sh` | `DESIGN_ONLY_DONE`, `BAIL_NEEDS_USER_INPUT`, optional `STALL_TRACKING` |
| `run-flags.sh` | `NO_ISSUES`, `FORCE_REQUESTED` (from `persist-implement-run-flags.sh`); legacy `QUICK_MODE` line may exist but is ignored |
| `larch-logs/implement/<RUN_ID>/` | `token-report.json`, `timing-report.json`, review tallies, OOS / execution-issues batches |

## Outputs

| Artifact | When |
|----------|------|
| `$IMPLEMENT_TMPDIR/summary-final.md` | Always (upsert payload) |
| `larch-logs/implement/<RUN_ID>/final-summary.md` | Unless `--comment-only` |
| KV lines | Always (see below) |

### `--print-stdout`

When set, the writer prints the rendered markdown body to **stdout** before the
status KV lines. Top-chat visibility is still owned by the orchestrator, which
emits the persisted `$IMPLEMENT_TMPDIR/summary-final.md` body verbatim after the
wrapper returns (per `skills/implement/SKILL.md` Step 17 / Step 18 prose). The
canonical tmpdir basename is `summary-final.md`, distinct from the archived
`larch-logs/implement/<RUN_ID>/final-summary.md` run-log artifact.

### Key-value contract

| Key | Values |
|-----|--------|
| `COMMENT_URL` | Upserted comment URL, or empty on skip/failure |
| `STATUS` | `ok` \| `failed` |
| `ERROR` | On `failed`: short message |

When `ISSUE_NUMBER=0` or `REPO_UNAVAILABLE=true`, tracking upsert is skipped and
the writer still returns `STATUS=ok` with an empty `COMMENT_URL`. GitHub upsert
failure → `STATUS=failed`, non-zero exit.

## `RUN_ID` validation

After resolving `RUN_ID` from `parent-issue.md` or `session-id`, the script rejects values that contain `/` or `..`, matching the traversal checks in the Rust-owned `run-log refresh` command (`*/*` / `*'..'*)`. Rust additionally applies the shared ASCII slug allowlist. Treat `run-log refresh` as a pattern reference only. Here, a rejected `RUN_ID` fails closed: it emits `COMMENT_URL=` (empty), `STATUS=failed`, and `ERROR="invalid RUN_ID (path-traversal characters rejected)"`, and exits non-zero without creating or modifying anything under the run log directory tree (`larch-logs/implement/<RUN_ID>/`). By contrast, `run-log refresh` treats invalid `RUN_ID` as a non-fatal skip (`REFRESH_SKIPPED=true`, `REASON=invalid-run-id`) and exits `0`.

## `--comment-only`

Still refreshes `summary-final.md` for the upsert but **does not** overwrite `larch-logs/.../final-summary.md`. Used after PR creation so the tracking comment picks up the live URL without dirtying the run-log tree before the next flush.

## PR line counts

After `REPO` and `PR_NUMBER` resolve, when `REPO_UNAVAILABLE=true` the writer
skips the typed pull-request file helper entirely and treats line data as
unavailable. Otherwise it first reuses cached `LINES_*` values from
`ship-pr-state.sh` when they match the current `PR_NUMBER` and
`LINES_STATUS=ok`. On cache miss it calls the helper in process, merges the
four counters back into `ship-pr-state.sh` when writable (replacing prior
`LINES_*` keys), and never aborts the report on helper failure. This avoids
repeated live GitHub file-list calls during `--comment-only` refreshes.

When status is `ok` and all four counters are non-empty integers, the writer
forwards them into the Rust `render run-summary` owner. Otherwise the renderer
omits those values and the bullet shows `N/A`.

## Review phase detail (per-round, issue #3774)

Before writing `summary-final.md`, the writer calls
`review_phase_detail.render_implement_review_detail` in process (rounds root
under `larch-logs/implement/<RUN_ID>/`, findings under that run dir, timing and
token ledgers from the implement tmpdir). The rendered **Review Phase Detail**
markdown is prefixed ahead of the run-summary body (together with exec-issue and
architectural detail sections), so it appears before the
`<!-- larch:run-summary v=1 -->` sentinel.

The section is a per-round table (suggestions made/accepted, OOS proposed/accepted,
time, cost, reviewers launched), a Total row, optional reviewer timing ASCII
Gantt charts, the top reviewers by suggestions accepted (`vendor/archetype`),
and a failed-reviewer-slot breakdown. Final reports do not pass `--no-gantt`.
Reviewer timing charts are included when timing data is available. The
`--no-gantt` flag is reserved for terminal progress output so live progress stays
plain text.

The Cost column is the per-round **vendor** cost (Codex + Cursor + Claude subprocess),
attributed by token-ledger timestamp window and priced via `larch_core::report::RATE_TABLE`.

The helper is best-effort. For a valid selected rounds root with zero completed
rounds (for example `--self-review` runs, where Step 5 does no panel review), it
renders `## Review Phase Detail` plus `No review rounds completed.`. A completed
`round-meta.json` only outside the selected rounds root (for example under the
live `IMPLEMENT_TMPDIR/round-N/` working dirs, issue #3794) is still not counted
as a completed round; the final report shows the no-completed-round message for
that selected valid root. The Rust-owned `progress render-phase-detail` command skips
the shared renderer when every discovered round dir under the selected root lacks
`round-meta.json`, so in-flight-only reviews do not append
`No review rounds completed.` during live Step 5 or design plan review. A render
failure is swallowed and never blocks the report. `/design`'s plan review uses
the same shared renderer through its final summary helper; see the helper's `.md`
for the `/design` contract.

## Token-data-missing primary path

When no usable token JSON exists, the JSON is unparseable, `.claude.totals` is
absent, or every available token bucket is zero, the in-process writer passes
`cost_unavailable` to the renderer. The rendered body therefore uses
`- **Cost**: N/A` instead of a misleading all-zero dollar line.

## Render failure behavior

The wrapper delegates to `scripts/larch.sh final-report write`, which renders the
summary in process via the Rust `render run-summary` owner. There is no
separate Bash self-composed renderer fallback. Tracking-comment failures still
return `STATUS=failed` after writing `summary-final.md`; repo-unavailable runs
skip the tracking upsert and return `STATUS=ok` with an empty `COMMENT_URL`.

## Test authority

- **Behavioral authority**: `crates/larch-cli/tests/final_report.rs` (`make test-write-final-report` → `write-final-report-rust-harness`), covering outcome matrix, comment-only, manifest stamp/failure, cost unavailable variants, force flags, line-count cache, and review-phase injection.
- **Delegation smoke**: `skills/implement/scripts/test-write-final-report.sh` (`write-final-report-bash-harness`) covers only thin-wrapper plugin-root selection, exact `final-report write` CLI routing, argv forwarding, exit-status forwarding, and stdout/stderr passthrough.
