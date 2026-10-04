# /design Step 5 green-path finalization body

**Consumer**: `/design` Step 5 happy path only.

**Contract**: Normative Step 5 prose for OOS filing, post-approval diagrams, plan compose/publish decisions, warning replay, and footer. `SKILL.md` keeps routing skeleton, Bash fences, background params, and final-summary marker bindings.

**When to load**: **MANDATORY READ ENTIRE FILE** at Step 5 entry, after the Step 5 banner/invariant and before the Step 5b skeleton.

## Ordering contract

Step 5 order: prepare emits `NEXT_ACTION`; `SKILL.md` branches; Step 5b.5 writes skip marker or candidate; Step 5c sanitizes diagrams before publish.

**MANDATORY: READ ENTIRE FILE before Step 5 diagram, final plan, summary, or Gate C prose composition: `${CLAUDE_PLUGIN_ROOT}/skills/shared/readability-style.md`.**

## Step 5b OOS filing body

**Privacy guardrail.** OOS Descriptions and reviewer `path:line` hints become **public** GitHub issues through `/larch:issue`. Reviewers must follow `${CLAUDE_PLUGIN_ROOT}/docs/security/artifacts-redaction-and-publication.md` and avoid high-risk paths or secret-adjacent material. The outbound redaction inside `issue create-one` is only a mechanical backstop.

**Session-backed authorization.** Step 5b `/larch:issue` OOS filing is session-backed. When constructing nested `issue create-one` args for session-backed filing, pass `--context-file "$DESIGN_TMPDIR/source-env.sh" --run-id "$LARCH_RUN_ID" --trusted-root "$DESIGN_TMPDIR"`. The `source-env.sh` contains `LARCH_LIVE_MUTATION_OK=true` and `LARCH_RUN_ID` set by the real `/design` Step 0 driver. Manual OOS recovery via direct `issue create-one` must pass `--operator-invoked` instead of a context file. Dry-run paths are authorization-free and require neither flag.

Stages conflicts; prompt calls `/larch:issue`. Helpers: `${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh design file-oos-prepare|file-oos-annotate` (sibling `file-design-oos.md`). Harness: Makefile `test-file-design-oos` (`test-file-design-oos.md`).

Cross-session idempotency: after successful `annotate` with `ISSUES_FAILED=0`, the helper best-effort atomically caches `$DESIGN_TMPDIR/oos-issues-created.md` at `~/.cache/larch/design-oos-filed/<ISSUE_NUMBER>.md`. Later `/design` restores those URLs only when the in-session sentinel is missing or empty and the cache is non-empty; a non-empty in-session sentinel wins. `--clear-cross-session-cache` deletes the issue cache and priority-label sidecars. `ISSUE_NUMBER` comes from the environment, or `--issue-number` for tests.

Priority labels: after `/larch:issue` succeeds, `scripts/larch.sh design file-oos-annotate` writes `oos-issues-created.md`, ensures `oos-correctness`, and applies it only to filed OOS with `focus-area: correctness` or `focus-area: regression`. The wrapper passes the same session-backed context, run ID, and trusted root described above; direct operator recovery must pass `--operator-invoked`. The typed GitHub service and issue-mutation owner use `--repo <REPO>` from prepare or session state and fail closed when authorization or `REPO` is missing.

When a priority label is outstanding, annotate writes `.oos-priority-label-pending` and durable cache sidecars before the first typed label provision or issue-label mutation. Sidecars hold sentinel URLs, post-cap combined text, and filing order. Later `NEXT_ACTION=label-only` labels from them without calling `/larch:issue`; `oos-accepted-design.md` and `oos-issue.stdout.txt` are not required.

If the prepare wrapper exits non-zero, parse only `NEXT_ACTION=` and `STEP5B_STATUS=` from `$DESIGN_TMPDIR/oos-filing-prepare.env`. For `NEXT_ACTION=unknown-oos-status` or `STEP5B_STATUS=unknown-oos-status`, preserve the warning and stop for repair. Otherwise append captured stderr with `"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" run-log append-failure` to `$DESIGN_TMPDIR/execution-issues.md` under `Tool Failures` with site `design Step 5b`, warn that OOS filing was skipped due to helper failure, and continue to Step 5b.5 without invoking `/larch:issue`.

When prepare output has `STEP5B_STATUS=prepare-failed-continue`, preserve the warning and continue to Step 5b.5 without invoking `/larch:issue`.

### `NEXT_ACTION=skip-pipeline`

Do not call `/larch:issue`.

- Re-emit `OOS_SKIP_BREADCRUMB` when non-empty.
- For `FILE_DESIGN_OOS_STATUS=skip-already-filed-sentinel`, or prepare stdout / `oos-filing-prepare.env` still carrying `WARN=` for that status, parse `WARN=`. If non-empty, append a `Warnings` entry to `$DESIGN_TMPDIR/execution-issues.md` via `run-log append-failure` with site `design Step 5b`, tool `scripts/larch.sh design file-oos-prepare`, category `Warnings`, exit code 0.
- Check `STEP5B_NEEDS_ANNOTATE=true` after warning handling. If annotate is needed, call `design-step5b-annotate.sh` only when `$DESIGN_TMPDIR/oos-issue.stdout.txt` exists and is non-empty. Treat annotate as best-effort on this skip path: append non-zero annotate exits as `Tool Failures`, then continue to Step 5b.5.
- When annotate is not needed, continue to Step 5b.5 without the file-issues annotate sequence. Prepare already wrote `.completed/step-5b` for `skip-already-filed-sentinel` without annotate.
- Do not route `skip-already-filed-sentinel` through the annotate-before-issue manual recovery path.

### `NEXT_ACTION=file-issues`

Parse `FILE_DESIGN_OOS_COMBINED=`, `FILE_DESIGN_OOS_DEPS_TSV=`, and `FILE_DESIGN_OOS_DEPS_AVAILABLE=` from `oos-filing-prepare.env`. Accepted non-security OOS plus Gate C approval authorizes `/larch:issue`; no confirmation or `AskUserQuestion`, including retry.

If `FILE_DESIGN_OOS_DEPS_AVAILABLE=true` **and** `FILE_DESIGN_OOS_DEPS_TSV` points at a non-empty readable file, invoke **`/larch:issue`** in batch mode with `--input-file` set to `FILE_DESIGN_OOS_COMBINED`, `--title-prefix "[OOS]"`, `--blocked-by-issue "$ISSUE_NUMBER"`, `--sentinel-file "$DESIGN_TMPDIR/oos-issue-sentinel"`, **`--intra-batch-deps-file`** set to `FILE_DESIGN_OOS_DEPS_TSV`, and **`--no-dep-llm`** because caller-supplied serialization edges are authoritative. Otherwise invoke the same Skill call **without** `--intra-batch-deps-file` / `--no-dep-llm`, log a `Warnings` entry for the degraded path, and mirror the `/implement` Step 9a.1 warning.

Capture **stdout only** from the Skill tool to `$DESIGN_TMPDIR/oos-issue.stdout.txt`. **This write is MANDATORY** for every `/issue` invocation. If the Skill tool returns output inline, use the Write tool to write the exact captured `/larch:issue` stdout to that file before `annotate`. Never skip or reorder annotate relative to this write: `cmd_annotate` is the only writer of `oos-issues-created.md`, and `scripts/larch.sh design render-final-summary` reads OOS count only from that file.

Run annotate and capture stdout to `$DESIGN_TMPDIR/oos-filing-annotate.stdout.txt`. On `FILE_DESIGN_OOS_STATUS=annotate-failed-empty-stdout` with `NEXT_ACTION=retry-file-and-annotate`, retry the file-and-annotate sequence once. Use `$DESIGN_TMPDIR/.oos-issue-retry-used` as the once-only sentinel. If the sentinel already exists, append `Tool Failures`, print a non-retryable failure, and do not write `.completed/step-5b`.

For the retry, re-run `/larch:issue` with the same arguments used for `NEXT_ACTION=file-issues`, capture stdout to `$DESIGN_TMPDIR/oos-issue.stdout.txt`, then re-run `"$HOME/.cache/larch/sessions/design-run-$PPID.sh" design-step5b-annotate.sh`. If the second annotate returns `annotate-failed-empty-stdout`, stop before Step 5b.5. Do not loop.

On non-zero `_oos_ann_rc` with `FILE_DESIGN_OOS_STATUS=annotate-label-failed` or `.oos-priority-label-pending`, append under `Tool Failures`, print the label-failure status, and stop before Step 5b.5. Do not write `.completed/step-5b`. The next retry must run label-only annotate or re-prepare to get `NEXT_ACTION=label-only`.

On non-zero `_oos_ann_rc` when `ISSUES_FAILED>0` in `$DESIGN_TMPDIR/oos-issue.stdout.txt`, append under `Tool Failures` via `run-log append-failure`, including stderr. Print `**⚠ /design: OOS filing completed with ISSUES_FAILED>0; see execution-issues and oos-issue.stdout.txt**`, then continue to Step 5b.5. Per-block `Filed URL` lines are written only for successful items.

On non-zero `_oos_ann_rc` without the retry, label, or partial-failure contract, treat it as annotate or parse failure: append `Tool Failures` and continue to Step 5b.5.

**Manual OOS recovery when annotate ran before `/larch:issue`** (`STEP5B_STATUS=annotate-failed`, rc=1, `oos-issue.stdout.txt` empty or missing): the Step 5b sentinel was not written; re-run the `/larch:issue` + annotate sequence manually before continuing to Step 5b.5. Manual recovery files accepted non-security OOS; no confirmation/`AskUserQuestion`. Never file security-routed OOS here:

1. `/larch:issue --no-dedup --input-file <oos-combined.md> --title-prefix "[OOS]" --label "enhancement"`; do **not** use `--blocked-by-issue` (mutually exclusive with `--no-dedup`).
2. Capture stdout to `$DESIGN_TMPDIR/oos-issue.stdout.txt`.
3. Apply the blocker edge: `"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" issue add-blocked-by --client-issue <OOS_NUM> --blocker-issue <TRACKING_NUM> --repo <REPO> --context-file "$DESIGN_TMPDIR/source-env.sh" --run-id "$LARCH_RUN_ID" --trusted-root "$DESIGN_TMPDIR"`.
4. Re-run annotate: `"$HOME/.cache/larch/sessions/design-run-$PPID.sh" design-step5b-annotate.sh`.

### `NEXT_ACTION=label-only`

Do not call `/larch:issue`. Run `design-step5b-annotate.sh` in label-only mode. It reads `oos-issues-created.md`, `oos-combined.md`, optional `oos-design-filing-order.txt`, and `REPO`; it skips empty-stdout and missing-accepted sequencing errors. URL mapping is 1-based by original `ISSUE_n` filing slot, including failed-slot gaps. Cap rollup labels the sole surviving URL when any post-cap combined block is high-risk.

`.completed/step-5b` is written by the Step 5b prepare/annotate wrappers on successful annotate paths: `annotate-complete`, `annotate-label-complete`, and prepare skip paths with no pending labels. Non-zero annotate exits write `.completed/step-5b` only for the documented partial `/larch:issue` carve-out without a label-retry subset. Exclude `annotate-label-failed`, `.oos-priority-label-pending`, and `STEP5B_STATUS=annotate-label-failed` from any non-zero stdout completion rule.

## Step 5b.5 diagram composition

If `DIAGRAM_REQUIRED=true`, generate Mermaid from the approved plan under `${CLAUDE_PLUGIN_ROOT}/skills/shared/mermaid-safe-content.md`; quietly write `$DESIGN_TMPDIR/architecture-diagram.candidate.md` with the required heading and fence. Emit no Claude-authored composition, safe-content reading, content/write/validation, success, or transition narration, and no diagram body. Harness-rendered `Write(...)`, `Wrote N lines`, and command counts are outside this contract.

On pre-write generation failure, print only `**⚠ 5b.5: arch diagram: generation failed, proceeding without diagram (<elapsed>)**`; an optional local failure capture is allowed. Log only a bounded generation warning through `crates/larch-core/src/report/diagram_log.rs::write_bounded_diagram_failure_log`. Never log raw output or bodies. Step 5b.5 must not warn or log sanitizer rejection.

Do not invoke `scripts/larch.sh mermaid sanitize` or another sanitizer; promote/reject, move/delete the candidate; or write `.completed/step-5b.5`, `architecture-diagram.md`, or `architecture-diagram.skipped`. Continue with `> **Continue to Step 5c IMMEDIATELY.**` without a pre-check or free-form recap.

Step 5c alone sanitizes the unchanged candidate, promotes or skips it, logs sanitizer rejection, and writes Step-5c-owned artifacts.

## Step 5c compose and publish

Compose `$DESIGN_TMPDIR/composed-plan.md` containing `## Plan`, `## Acceptance`, and a trailing `diff_lines: <N>` line from `$DESIGN_TMPDIR/diff-lines.txt` or a best-effort estimate.

The Step 5c driver delegates to `scripts/larch.sh design step5c`, which calls the publish tail. Before redaction, the tail writes review provenance, re-runs `plan check-size --design-tmpdir "$DESIGN_TMPDIR" --plan-file "$DESIGN_TMPDIR/plan.txt"`, and refuses incomplete review, size failures, missing `architectural-invariant-assessment.md`, or missing `architectural-guideline-assessment.md`. It exits 4 with `.design-publish-result.env` for composed-plan defects. On that exit, execute **### Plan command validator failure (shared)** with `--site` `design Step 5c`: preserve `$DESIGN_TMPDIR`; skip cleanup, publish, rename, and redact.

When log publish fails after plan write, preserve `$DESIGN_TMPDIR` and direct the operator to `design-publish-tail.failure.log` and `execution-issues.md`. The Step 5c driver copies the bounded first stderr line into its child stderr and the execution-issues ledger.

A missing or empty `$DESIGN_TMPDIR/composed-plan.md` exits 4 with `VALIDATE_STATUS=defects-found`; Fix-and-retry composes first, then re-runs `"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" design step5c --session-env-path "$HOME/.cache/larch/sessions/current-design-env-$PPID.sh" --claude-pid "$PPID" --fresh-attempt`. Override is not offered. For ordinary composed-plan validator defects, Fix-and-retry re-runs the same command; Override also adds `--skip-validate`.

When `_publish_rc=4`, enter the shared branch with parsed `VALIDATE_*` / `PUBLISH_REFUSE_REASON`. Before review-provenance: missing `composed-plan.md` → Fix-and-retry / Cancel; size refusal → the unified inline Split-path; `PUBLISH_REFUSE_REASON=missing-invariant-assessment` → publish precondition; `PUBLISH_REFUSE_REASON=missing-guideline-assessment` → publish precondition. Missing invariant assessment is evaluated and surfaced before missing guideline assessment when both artifacts are missing. For invariant refusal, skip autofix/Override, preserve `$DESIGN_TMPDIR`, and offer **Return to Gate C** / **Cancel** only. Warning: `**⚠ 5c: publish refused: missing architectural-invariant-assessment.md; return to Gate C to persist the architectural-invariant assessment before publish.**`. Return: Step 4b (`resume@4b`) → `scripts/larch.sh architectural-invariants present-note --repo-root "$REPO_ROOT"` → `scripts/larch.sh architectural-invariants persist-design-assessment --repo-root "$REPO_ROOT" --design-tmpdir "$DESIGN_TMPDIR"` (clean, sidecar, or no-flags branch as appropriate) → `"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" design step5c --session-env-path "$HOME/.cache/larch/sessions/current-design-env-$PPID.sh" --claude-pid "$PPID" --fresh-attempt`; Cancel skips redact, plan write, publish, rename, cleanup. Keep `PUBLISH_REFUSE_REASON=missing-guideline-assessment` after the invariant branch: skip autofix/Override, preserve `$DESIGN_TMPDIR`, and offer **Return to Gate C** / **Cancel**. Warning: `**⚠ 5c: publish refused: missing architectural-guideline-assessment.md; return to Gate C to persist the architectural-guideline assessment before publish.**`. Return: Step 4b (`resume@4b`) → `architectural-guidelines present-note` → `persist-design-assessment` → the same Step 5c retry command; Cancel skips redact, plan write, publish, rename, cleanup. Then handle the two Gate C content refusals, mirroring the missing-assessment recovery. `PUBLISH_REFUSE_REASON=invariant-violation` → publish precondition: skip autofix/Override, preserve `$DESIGN_TMPDIR`, and offer **Return to Gate C** / **Cancel** only. Warning: `**⚠ 5c: publish refused: architectural-invariant-assessment.md records a violation; return to Gate C to resolve the invariant violation before publish.**`. Return: Step 4b (`resume@4b`) re-runs the full Gate C presentation and adverse-outcome ladder, and only then retries the same Step 5c command; Cancel skips redact, plan write, publish, rename, cleanup. `PUBLISH_REFUSE_REASON=invalid-guideline-deviation` → publish precondition: skip autofix/Override, preserve `$DESIGN_TMPDIR`, and offer **Return to Gate C** / **Cancel** only. Warning: `**⚠ 5c: publish refused: architectural-guideline-assessment.md records a guideline deviation without a documented exception; return to Gate C to fix the plan or record an exception before publish.**`. Return: Step 4b (`resume@4b`) re-runs the full Gate C presentation and adverse-outcome ladder, and only then retries the same Step 5c command; Cancel skips redact, plan write, publish, rename, cleanup. Empty `VALIDATE_LOG_FILE` with zero missing scripts is review-provenance refusal: Fix-and-retry re-runs `/design`, or Cancel.

When `_publish_rc=3`, the publish tail may have completed but `.design-publish-result.env` could not be written. Parse the captured stdout fallback (`_publish_stdout_file`) and continue Step 5c items 5-7 with the warning above. Do not treat exit 3 as publish-tail incomplete.

When `_publish_rc` is in `{0, 1, 3, 4}`, parse through `scripts/larch.sh design read-result-env --input "$DESIGN_TMPDIR/.design-step5c-status.env"` after bgjob `DONE`; the helper prefers `$DESIGN_TMPDIR/bgjob/design-step5c.result.env` and falls back to the legacy status env only when absent. Gate success on `BGJOB_RC=0`. Exit 1 is the normal plan-block-write failure path. Do not abort solely because `_publish_rc=1`.

On a failed plan-block or receipt write, `design log-publish --reason retryable-failure` redacts and stages the attempt's artifacts without running a lifecycle terminal verb. It leaves `PUBLISH_OK=false`. A later Step 5c retry with `--fresh-attempt` reuses the open run and may terminalize it as success. Cleanup remains ineligible until the plan write and terminal log publish both succeed.

**Driver WARN replay (top chat):** After the Bash block, when `_publish_rc` ∈ {0, 1, 3} and driver WARN bodies were parsed, emit each distinct WARN `_value` verbatim to top chat before terminal final-summary emission. Do not leave them only as `WARN=` machine lines inside Bash output.

Only when `_publish_rc` is 0, 1, or 3 and driver output was parsed from file and/or stdout: on `PLAN_WRITE_OK=true`, print `⏩ 5c.5: status=${UPSERT_STATUS:-unknown} arch=${ARCHITECTURE_SOURCE:-unknown}`. The `scripts/larch.sh design step5c` fence already wrote `step-5c` under the `PLAN_WRITE_OK=true` gate before leaving the fence. Rename (`RENAMED`) and Step 6 cleanup remain gated on `PUBLISH_OK` separately.

Only when `_publish_rc` is 0, 1, or 3 and driver output was parsed, or stdout fallback populated `PLAN_WRITE_OK`: when `PLAN_WRITE_OK=false`, print `**⚠ 5: plan-block-write failed: preserving $DESIGN_TMPDIR**` and skip Step 6 cleanup. Do not write `step-5c`.

## Step 5d warning replay and footer

Repeat any external reviewer warnings from earlier steps, including Step 0 reviewer-availability checks via `session setup`, Step 3 runtime failures, Step 5b.5 diagram generation failure, and driver WARN bodies replayed from Step 5c, so they are visible at the end of the workflow. Examples:

- `**⚠ Codex not available: <reason>**`
- `**⚠ 5b.5: arch diagram: generation failed, proceeding without diagram (<elapsed>)**`

The rigid `larch:final-summary` body is produced by `scripts/larch.sh design render-final-summary` inside `scripts/larch.sh design step5c` after the publish outcome is known. Parse `FINAL_SUMMARY_PATH` from final bgjob `DONE` stdout or result env, then use the shared Read-always readiness profile to Read/cache the body before cleanup. Do not add token/timing chat tails, extra recap prose, or farewell wording outside that rendered block and the machine footer.

When `PLAN_WRITE_OK=true`, repeat the external-reviewer warnings, then emit exactly one machine footer as the last human-visible output line of Step 5 before Step 6 cleanup. When `PLAN_WRITE_OK=false`, Step 5c already cached the summary before the `**⚠ 5: plan-block-write failed**` line. Do not invoke `scripts/larch.sh design render-final-summary` again. Step 6 is still skipped when plan write fails. In all `_publish_rc` 0, 1, or 3 paths, prompt-side emission waits until all cleanup or failure routing is complete, then emits the cached final summary as the final assistant text.
