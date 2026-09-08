---
# larch-run-lifecycle: shared-v1 skill=implement
# Referenced implement script files:
# skills/implement/scripts/step-architectural-invariants-write-compose.md
# skills/implement/scripts/step-architectural-invariants-write-compose.sh
# skills/implement/scripts/step-architectural-guidelines-write-compose.md
# skills/implement/scripts/step-architectural-guidelines-write-compose.sh
# skills/implement/scripts/test-architectural-guidelines-step.sh
# skills/implement/scripts/test-architectural-guidelines-step.md
name: implement
description: "Use when implementing from a GitHub issue with a vetted in-body plan (run /design first). Materialize, implement, validate, review, PR, CI. See /research, /design, /im, /f, /implement --merge."
argument-hint: "[--merge] [--forked] [--draft] [--no-admin-fallback] [--no-logs-commit] [--coder <claude|codex|cursor>] [--run-id <ID>] [--force|-f] [--self-review] [--self-implement] [--difficulty <TRIVIAL|MODERATE|HARD>] <issue-N>"
allowed-tools: AskUserQuestion, Bash, Read, Edit, Write, Grep, Glob, Agent, Task, WebFetch, WebSearch, Skill
---

**MANDATORY: `implement`: Rust owns lifecycle start/finish (`skills/shared/run-lifecycle-ownership.tsv`). Never run the generic lifecycle (`${CLAUDE_PLUGIN_ROOT}/skills/shared/run-lifecycle.md`). Send `--lifecycle-parent-context` only to Step 0.**
# Implement Skill

**MANDATORY: READ ENTIRE FILE before composing user-facing prose: `${CLAUDE_PLUGIN_ROOT}/skills/shared/readability-style.md`.**

End-to-end: fetch the vetted `larch:plan`, materialize artifacts, implement, validate, commit, review, ship the PR, monitor CI, and clean up. With `--merge`: also run CI+rebase+merge, delete the local branch, verify main, and have the active Step 8+ driver checkpoint the run-log manifest plus `scripts/larch.sh final-report write` before exit. The tmpdir/tracking summary may reflect `MERGE_RESULT` without any post-merge `git commit` (NEVER #16). Step 18 owns the complete terminal snapshot and publication. Step 19 owns cleanup.

**Protocol Execution Directive.** You are the `/implement` orchestrator. After flag parsing and mutual-exclusion checks, your FIRST external actions MUST be: (1) when `forked_target=true`, run `"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" admission fork-env` once and parse `UPSTREAM_REPO` plus sibling fork KV lines from stdout; (2) run exactly one `"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" implement preflight` call as the sole mechanical surface for Preflight items 1-3, passing `--repo "$UPSTREAM_REPO"` when forked; (3) after prompt-side Preflight judgment, run Step 0 unchanged through `${CLAUDE_PLUGIN_ROOT}/skills/implement/scripts/step-0-bootstrap.sh --mode initial`. Prompt-side judgment starts only after helper exit `0`. Item 4 is the main-agent plan-adequacy audit when `force_requested=false`; force skips it and proceeds to item 6. Item 6 remains the semantic materiality judgment after `AUDIT=pass` or force skip. When `forked_target=true` and `UPSTREAM_REPO` is already set from (1), **do not** re-run `scripts/larch.sh admission fork-env`; reuse the fork metadata to avoid a second bootstrap tmpdir.

**Anti-halt continuation reminder.** After each child `Skill` call (`/review`, `/issue`, `/implement`) and each numbered or sub-step `Bash` helper, including `scripts/larch.sh checks run-relevant`, IMMEDIATELY continue to this skill's NEXT numbered step. Do NOT stop on cleanup output, Bash stdout, status, summary, handoff, recap, or "returning to parent" prose. For bgjob-migrated steps, `WAIT` means the next action is another identical `bgjob wait`; after final `DONE`, parse required KVs from the last `DONE` stdout and `$IMPLEMENT_TMPDIR/bgjob/<step>.result.env`. Applies from Preflight through Step 19 except explicit non-sequential directives in THIS file (`skip to Step N`, `bail to cleanup`, `jump back`, `loop back`, `fall through`, `break out`). Every relevant-checks helper call is covered. **Critical boundary: Step 9b PR creation → Step 10 CI monitor immediately; PR creation is NOT the end.** **Critical boundary: when the active Rust Step 8+ `ship pr` driver exits, route only from process exit code + JSON stdout; do not parse `ship-pr-state.sh` or the retired bash exit matrix.** **Critical boundary: after `route-exit` emits `NEXT_ACTION=ci-fix`, do NOT end the turn; run the ci-fix repair procedure in the same turn.** **Critical boundary: after preflight audit passes (`AUDIT=pass`), continue through Preflight items 6-7, then run Step 0 `${CLAUDE_PLUGIN_ROOT}/skills/implement/scripts/step-0-bootstrap.sh --mode initial`; do NOT end the turn on the audit-pass envelope. Critical boundary: after the force plan-adequacy audit skip breadcrumb prints, continue through Preflight items 6–7, then run Step 0; do NOT halt waiting for an `AUDIT=pass` envelope on the force skip path.** **Terminal boundary: the combined Step 16-17 wrapper only captures a pending final-report body; Step 18 terminalizes and publishes logs, Step 19 cleans up and relays the teardown tail, then the selected final-report body is the last plain-chat text with no following tool call.** → shared/subskill-invocation.md#anti-halt

**Skill-name fallback reminder.** When invoking a child skill via the Skill tool from this file, ALWAYS try the bare name first (`"design"`, `"review"`, `"issue"`, `"implement"`). Use the fully qualified `larch:` form (`"larch:design"`, etc.) only after bare-name lookup returns `Unknown skill`; in a consumer repo with a different plugin namespace, use that namespace as the fallback. `/implement` does not invoke relevant-checks through the Skill tool on the green path; it uses the captured Rust checks helper so success returns one bounded machine line, or `RELEVANT_CHECKS_SKIPPED=true` only on explicit `--allow-skip` test paths. Phase 1 (#3364) does not invoke `/release`; versioning moves to `/release` (Phase 3). Do NOT mirror this skill's own namespaced invocation (`larch:implement`) onto child Skill calls. → shared/subskill-invocation.md#bare-name-fallback

## Load-Bearing Invariants

Two invariants enforced across multiple steps. Anchor cross-step questions here; do not re-derive inline.

1. **Step 9a.1 OOS Sentinel Idempotency** — re-running `/implement` in the same session MUST NOT double-file vote-accepted non-security OOS. **Enforcement**: Rust-owned `${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh oos file` reads accepted inputs, the sentinel, and run NDJSON; binds prior URLs by stable ID/title; and files only unmatched blocks. **Why**: durable evidence makes retries deterministic.

**Fork-mode carve-out for Invariant #1**: when `forked_target=true`, Rust creates no public OOS issue, records `skipped://oos/<N>` evidence, checkpoints, and preserves accepted items in final-report text. CI comparison uses `upstream/main` via `scripts/larch.sh push rebase --base-remote upstream --base-ref main` and `scripts/larch.sh ci status --base-remote upstream --base-ref main`.

2. **Tracking-Issue Sentinel Idempotency** (umbrella #348) — re-running `/implement` in the same session MUST NOT double-adopt the wrong issue or corrupt `RUN_ID`. **Enforcement**: Step 0 checks `$IMPLEMENT_TMPDIR/parent-issue.md`; on retry it recovers prior `ISSUE_NUMBER` and `RUN_ID`, skipping Branch 2 adoption, `run-log init`, and `scripts/larch.sh tracking post-issue`. Write the sentinel ONLY after `ISSUE_NUMBER`, `RUN_ID`, and the metadata summary comment resolve. If `run-log init` fails, set `IMPLEMENT_BAIL_REASON=tracking-init-failed`, `STALL_TRACKING=true`, skip the sentinel, skip to Step 18, and **preserve `$ISSUE_NUMBER`** so Step 18 can rename the issue to `[STALLED]` when applicable. Reserve `DEFERRED=true` for the non-stalled metadata-publication defer path (`POSTED=false` / no sentinel, then continue within Step 0). **Why**: `"$CLAUDE_PLUGIN_ROOT/scripts/larch.sh" tracking-issue upsert-summary` uses marker literals for the four slim comments, but the local sentinel remains the byte-exact session guard, parallel to Invariant #1.

## NEVER List

Each rule states WHY; per-site reminders reference by anchor name.

1. **NEVER simply "log and return" on push failure in the Step 12 merge loop inside the active Step 8+ driver.** **Why**: `scripts/larch.sh ci wait` and `scripts/larch.sh merge pr` operate on remote PR state only; a log-and-return would let the merge loop proceed to `ACTION=merge` on a remote branch that never received the fix push. **How to apply**: Step 10 CI-fix paths may degrade gracefully; Step 12 family MUST bail to 12d.

2. **(removed in Phase 1 #3364 — bump verification on the ship path; see `${CLAUDE_PLUGIN_ROOT}/skills/implement/references/conflict-resolution.md` retirement stub.)**

3. **NEVER use the `ours`/`theirs` git labels when describing conflict sides during rebase.** **Why**: during rebase their semantics are inverted vs. merge (`--ours` = base being rebased onto = upstream main); labels cause silent resolution errors. **How to apply**: always use "upstream (main)" and "feature branch commit" in Phase 1 commentary and user prompts.

4. **NEVER skip the code-review step regardless of the nature of changes.** **Why**: code, skills, docs, data, and config all require reviewer-panel vetting. **How to apply**: on the standard path, Step 5 invokes `skills/implement/scripts/step-5-review.sh` once per Step 5 entry; that launcher prints the banner, forwards session-env and tmpdir context, and launches the file-backed `review-and-fix CLI` review loop **without** any `--panel` token (see `crates/larch-cli/tests/review_and_fix_commands.rs`). `review-and-fix step5` uses `$IMPLEMENT_TMPDIR/plan.txt` and a fixed cap of 2 for every tier; degraded rounds consume the active budget, and escalated rounds skip pruning. The review panel is applied only inside `review-and-fix CLI` → `review core`. **`--self-review` exception**: when `self_review=true`, Step 5 skips `review-and-fix step5` and a Claude Agent-tool subagent (`larch:claude-self-reviewer`) performs thorough self-review; review still runs.

5. **NEVER let the Step 9a.1 sentinel short-circuit silently skip the larch-log OOS update.** **Why**: recovery MUST write accepted-OOS evidence; silent skip breaks durable logs. **How to apply**: let `${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh oos file` re-read sentinel/NDJSON evidence, materialize recovery rows, checkpoint, then write statistics. Never add a prompt-side sentinel branch. On the security continuation, retained #7681 routing writes statistics only after the Rust disposition checkpoint succeeds (NEVER #14). Fork mode follows Invariant #1.

6. **NEVER let the focus-area enum drift out of checked review prompt surfaces.** **Why**: `.github/workflows/ci.yaml` inspects the canonical review/design prompt files for the unquoted focus-area enum; Step 5 now delegates prompt construction to review scripts instead of embedding prompt strings here. **How to apply**: when moving review prompt text between scripts or skill files, update the CI file list in the same PR so the surface containing `code-quality / risk-integration / correctness / architecture / security` remains checked.

7. **NEVER bail mid-run on orchestrator-judgment "scope" or "capacity" concerns without a mechanical justification.** **Why**: `/implement` is designed for long autonomous runs. Subjective remaining-work judgments are NOT valid bail reasons. The only sanctioned non-error halt paths between Step 2 and Step 19 are: (a) Step 12d under documented judgment conditions; (b) explicit user halt in a fresh interactive turn; (c) hard tool failure. **How to apply**: follow the next explicit control-flow directive unless a sanctioned halt path applies. **Post-merge sub-clause (highest-stakes halt boundary)**: the `✅ 12: CI+merge loop status=complete outcome=merged pr=<N> elapsed=<elapsed>` line at Step 12b, and the analogous `✅ 12: CI+merge loop status=complete outcome=force-merged-externally pr=<N> elapsed=<elapsed>` line at Step 12a's `already_merged` branch, is the most halt-prone point. The run is not done: Steps 14, 15, 16, 17, 18, and 19 still must run. Ending the turn, posting a recap, or writing a handoff between that breadcrumb and Step 14's first action violates NEVER #7. `pr_closed=true` and `DONE_RENAME_APPLIED=true` are PRE-conditions for Steps 14-19, not POST-conditions of a finished run.

8. **NEVER call `ScheduleWakeup` anywhere in the `/implement` orchestrator.** **Why:** improvised wakeups re-fire as `/loop` input and can extend turns past Step 19. **How to apply:** do not call `ScheduleWakeup` at any step. Do not spawn a Monitor or a Bash polling loop (`for`/`while`/`until` + `sleep`) to watch another helper finish. Long helpers use bgjob start/wait (`run-step-checks.sh`, Step 5 `step-5-review.sh`, Step 5 `step-5-resume.sh`, Step 6 `step-6-entry.sh`, Step 7a, Step 8 `step-8-ship.sh`, and `checks repair-loop --bgjob-launch true`): after `BGJOB_STATUS=WAIT`, run the identical `bgjob wait` again with no intervening prose or tools. See `${CLAUDE_PLUGIN_ROOT}/skills/shared/bgjob-wait.md` for the normal wait contract. **NEVER use the `Monitor` tool anywhere within the `/implement` orchestrator.** During bgjob waits, do not use Monitor, TaskOutput, task output reads, sleep, or alternate progress probes between identical `bgjob wait` calls. Step 8 reads the merged ship outcome KVs from its result env on `DONE`; it does not infer success from `BGJOB_RC=0` alone. Do not use `ps`, Monitor, TaskOutput, or background recovery waiters. Do NOT spawn multiple Monitor calls watching logs or PID exits.

9. **NEVER branch Step 2 on `STATUS` before completing §2.1.5 envelope validation.** **Why**: the dispatcher emits `ORCHESTRATOR_EDIT_AUTHORITY=allowed|forbidden`, with `allowed` iff `STATUS=claude_fallback`; any illegal pairing or malformed envelope lets Claude-fallback plan edits start while an external implementer still owns commits (issue #1058). **How to apply**: after parsing §2.1 KV stdout, always run all §2.1.5 checks before §2.2 branches on `STATUS`. On failure, synthesize `orchestrator-envelope-invalid`; do not enter Step 3 or consume `MANIFEST`.

10. **(removed — see issues #2485 / #2487; the post-/design boundary halt rule and its archival hook scripts were deleted after the issue-anchored cutover.)**

11. **NEVER write, recreate, or modify `$IMPLEMENT_TMPDIR/finalize-state.sh` from prompt-side orchestrator code.** **Why**: the ship lifecycle and Rust `implement-finalize` owner write it on terminal outcomes before returning JSON; a prompt-side subset triggers `state-file missing required key` teardown cascades and stale session tmpdirs. **How to apply**: do NOT write it by `cat`, `printf`, `echo`, Write, `sed -i`, `tee`, or any other means. The only pre-teardown reconstructor is conditional `"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" session restore-finalize-state`, per Step 19. If `scripts/larch.sh implement-finalize teardown` reports `state-file missing required key` and `ship-pr-state.sh` is absent, surface the error and stop; do NOT compose the file from prompt-side shell variables. See Step 19 cleanup.

12. **NEVER write, append to, or recreate `$IMPLEMENT_TMPDIR/session-env.sh` from prompt-side orchestrator code.** **Why**: child scripts read it on each invocation; prompt-side `>>`, heredoc rewrites, or `printf` fixups bypass the writer's anchored filter and post-condition assertion, reproducing issue #2326's incomplete Step 1 materialization. **How to apply**: sanctioned writers are `"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" session write-env`, `"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" session setup`, `"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" session persist-run-flags`, and the Rust-owned `bootstrap invoke` flow. The plan file is always `$IMPLEMENT_TMPDIR/plan.txt`; child scripts do not read `PLAN_FILE` from `session-env.sh`. If plan logging or Step 5 fails because that path is missing, repair Step 1 materialization. The orchestrator may only READ via `CLAUDE_PLUGIN_ROOT="$CLAUDE_PLUGIN_ROOT" "${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" session read-key` or invoke the sanctioned writers.

13. **(removed — see issue #3111 Stage 4; Family-B background+monitor pairs are deleted.)**

14. **NEVER silently drop a voted-in OOS finding.** **Why**: accepted OOS blocks are the durable contract between reviewers, manifests, and Step 9a.1 disposition. Losing them breaks auditability and follow-up tracking. **How to apply**: non-security accepted OOS is filed by the pre-driver `${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh oos file` path before `step-8-ship.sh`; that Rust path owns disposition-checkpoint, run-statistics, and manifest `steps_ran.step9a1` stamping via `crates/larch-cli/src/oos_file_commands.rs:after_checkpoint`. `references/oos-pipeline.md` documents the Rust pipeline and must not become a prompt-side `/issue` fallback. On `NEXT_ACTION=oos-pipeline`, read `$IMPLEMENT_TMPDIR/security-oos-observations.md`, follow `${CLAUDE_PLUGIN_ROOT}/docs/security/workflow-trust-and-mutations.md` `## Security Findings in OOS Workflows` privately with no public `/issue`, clear the sidecar only after private disposition completes, then run the Step 8 checkpoint wrapper with no `/issue` call. Do not run prompt-side direct `oos disposition-checkpoint`, compose run statistics, or patch `OOS_PENDING=false` outside that wrapper.

15. **NEVER set `OOS_PENDING=false` outside retained #7681 `scripts/larch.sh implement step-8-oos-checkpoint` workflow-router success** (fork-mode and `repo_unavailable=true` skip this gate intentionally). **Why**: `OOS_PENDING` gates ship-pr progress until accepted OOS blocks have filed issue URLs, `Inline-triage rule N:` breadcrumbs, rejection markers, or private security disposition. **How to apply**: invoke the checkpoint wrapper after security-sidecar disposition when applicable and before or at the Step 8 OOS checkpoint wrapper on the `oos-pipeline` branch, or after pre-driver `${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh oos file` on the normal path. Only checkpoint `NEXT_ACTION=reship` may write run statistics, stamp the manifest, and clear `OOS_PENDING=false` through the allowed-key patch helper.

16. **NEVER make any git commit after the PR has merged**, regardless of branch or path. **Why**: `/implement` MUST NOT advance repo history after merge, especially on `main`. Commits after `$IMPLEMENT_TMPDIR/post-merge-sentinel` strand on local main and can break later cleanup or pulls. Past regressions: #2120, #2128, #2140, #2182, #2552. **How to apply**: all post-merge git commits are policy violations. Step 18 validates, sanitizes, and publishes the staging tree through the archive publisher. Do NOT add callers that commit run logs to Git. Do NOT re-render and Git-commit the final summary; re-render in tmpdir only. `scripts/larch.sh final-report write --comment-only` must remain API-only. See `docs/run-log-cli.md` and the Rust ship lifecycle docs.
17. **NEVER write a free-form natural-language recap summary at end of turn after Step 17, Step 18, or Step 19**: including but not limited to a "Run complete." / "Implementation merged." prose line, a bullet list summarizing PR / Version / Changes / Code review / CI / Tracking issue, a parenthetical cost paraphrase (for example `~$10.46`, `~$X total`), or any natural-language replacement for the structured `## /implement run ...: <outcome>` block rendered into `summary-final.md` by `scripts/larch.sh implement step-16-17` through `scripts/larch.sh implement step-17 --no-print-stdout` and marker extraction. **Why**: free-form summaries either omit the canonical `- **Cost**:` line or paraphrase it as a TOTAL-only figure, dropping the per-agent breakdown (`Claude $X, Codex $X, Cursor $X`) users depend on. **How to apply**: Step 17 and Step 18 capture candidate marker bodies until Step 19 finishes cleanup. Follow the marker-first profile in `${CLAUDE_PLUGIN_ROOT}/skills/shared/final-summary-emit.md` with `/implement` markers `---LARCH-SUMMARY-FINAL-BEGIN---` / `---LARCH-SUMMARY-FINAL-END---`. Step 17 binds the source to captured foreground `scripts/larch.sh implement step-16-17` Bash wrapper stdout, not asynchronous notification output, and caches any non-empty body for deferred terminal emit. Step 18b binds the source to captured foreground `scripts/larch.sh implement step-18-gate-logs-flush` Bash wrapper stdout on `NEXT_ACTION=logs-flush-done`, or captured foreground `step-18.sh --phase logs-flush` Bash wrapper stdout on the stall-recovery path, not asynchronous notification output. Read fallback is `forbidden` for Step 17 and Step 18b. Sidecar follow-on is `forbidden`. Do not emit plain-chat final report text until Step 18 terminalization and Step 19 restore, teardown, and tail relay are complete. The Step 18 wrapper writes `.step17-emitted` before final-report refresh when `--step17-emitted true`, and again before it emits markers; the orchestrator does not write `.step17-emitted` after logs-flush returns. Apply `/implement` terminal-emit precedence from the shared contract: refreshed Step 18 marker body wins when `EMIT_BODY=true`, `WFR_RC=0`, and valid markers are present; otherwise use a non-empty Step 17 cache; otherwise emit only the existing missing-marker warning. If a user message interrupts after logs-flush returns, emit the surviving in-context cached body first in the next turn. Never Read or use a disk cache to reconstruct it. Do NOT add a closing recap, do NOT echo the structured block in your own words, and do NOT mention costs in your own prose. The only final orchestrator-text addition permitted is one verbatim full-body emission from the selected cached Step 18 or Step 17 source at terminal text position. **Verbatim means the entire marker body without omission or condensing.** Do NOT wrap any section in `<details>`, collapse or omit `### Round N reviewer timing` ASCII bar charts, or drop the `**Top reviewers**` list. Every part of the marker body, including all Gantt timing sections, must appear as plain chat markdown exactly as it appears between the markers. The missing-marker warning is printed only when `EMIT_BODY=true` and `WFR_RC=0`. When `WFR_RC!=0`, the final report render genuinely failed: the logs-flush wrapper has already printed `**⚠ Step 18: final report render failed (WFR_RC=<n>): <reason>**` (reason from the `ERROR=` KV) to composite stderr, and that warning is the accounted-for outcome. Do not emit a body, do not Read `summary-final.md`, and do not add free-form recap prose (#6979: an rc-failure path must never be fully silent). No prose or tool call may follow the final verbatim report emission.

18. **NEVER spawn Agent-tool subagents for code-writing work during Step 18a stall recovery.** **Why**: recovery is a single-runner continuation; Agent-tool code edits bypass stall classification, retry caps, and atomic `STALL_TRACKING` clear ordering. **How to apply**: for `step2-impl`, main Claude reads `$IMPLEMENT_TMPDIR/plan.txt`, edits inline, checks, commits, and continues in the current run. Review and ship wrappers may still use their documented external lanes.

19. **NEVER print code-flow diagram bodies to chat.** **Why**: diagram content belongs only in the issue-scoped `larch:diagrams` comment and PR body, and printing it bloats context. **How to apply**: do not print `$IMPLEMENT_TMPDIR/code-flow-diagram.md`, `$IMPLEMENT_TMPDIR/code-flow-section.md`, or any `## Code Flow Diagram` section body. Step 7a emits breadcrumbs and KVs only.

20. **NEVER copy diagram failure captures into published implement run logs.** **Why**: generator or sanitizer captures may contain partial Mermaid. **How to apply**: do not copy or flush `code-flow-diagram.failure.log`, code-flow diagram body files, or generator/sanitizer stdout containing Mermaid into `larch-logs/implement/<RUN_ID>/`; durable diagnostics are bounded `execution-issues.md` warnings only.

21. **NEVER make Edit, Write, or repo-mutating Bash calls on git-tracked paths between Preflight item 6 and `BOOTSTRAP_NEXT=step2`.** **Why**: pre-bootstrap edits bypass the dirty-tree checkpoint; partial exits remain gated. **How to apply**: item 6 is a **read-only bounded probe** (`test -f`, `test -e`, targeted `rg`/`grep`) except `$PREFLIGHT_TMPDIR/**`, the stale-notice comment, and `plan-receipt refresh` after a sole `stale-plan-base-scope` finding clears. Deeper work and tracked edits wait for exit 0 plus `BOOTSTRAP_NEXT=step2`. **Carve-out — rebase-routing**: on `BOOTSTRAP_NEXT=rebase-routing`, follow `rebase-checkpoint-routing.md` and `conflict-resolution.md`; only `larch:ci-fixer` `MODE=conflict` edits feature-branch conflicts. Repeat the gate before every `step-0-bootstrap.sh` fence.

**Single-runner assumption**: run one `/implement` per repository at a time. Concurrent sessions can interleave working-tree mutations, corrupt dirty-tree probes, or attribute one runner's edits to another. The inverse hazard — mistaking one of your OWN spawned subprocesses' edits (the `checks repair-loop` lint-fix tiers, the Step 3 pre-commit ruff autofix) for a concurrent runner — is guarded by the self-edit attribution log at `$IMPLEMENT_TMPDIR/self-edit-log.tsv`; consult it (see `references/checks-repair-loop.md` §6) before halting on a suspected parallel session. Dirty-tree guards reduce blast radius but do not serialize writes. Between Step 0 and documented checkpoint probes, `/implement` and child skills write only to session tmpdirs (`$IMPLEMENT_TMPDIR`, `$DESIGN_TMPDIR`, `$REVIEW_TMPDIR`) until implementation intentionally edits the repo.

**Mode matrix**:

| Mode | PR target | Tracking issue lifecycle | Version bump | CI base comparison | Merge |
|---|---|---|---|---|---|
| Default | `$REPO` from session setup | enabled | skipped (Phase 1) | `origin/main` | skipped |
| `--merge` | `$REPO` from session setup | enabled | skipped (Phase 1) | `origin/main` | enabled |
| `--forked` | `$FORK_REPO` from origin | disabled | disabled | `upstream/main` | disabled |

## Progress Reporting

Every step MUST print breadcrumb status lines per shared/progress-reporting.md: start lines on entry, bounded progress lines for long work, and completion lines from wrappers.

**MANDATORY at session start**: Read `${CLAUDE_PLUGIN_ROOT}/skills/implement/scripts/step-name-registry.tsv` to get the Step Name Registry (step number → short name mapping for progress breadcrumbs).

**Phase 1 (#3364)**: Do not print orchestrator `🔶` / `⏩` / `✅` breadcrumbs for ship-pr substeps **8**. The Rust dispatcher owns launch and checkpoint routing, while the Rust `ship pr` owner emits the internal state-machine contract.

**Generated-file rebase conflicts:** the ship driver treats every non-run-log conflict as an existing conflict-route handoff. At Step 8b, the driver persists `RESUME_PHASE=ship-pr-rrr-phase14`, `CALLER_KIND=ship_pr_pre_push`, and `CONFLICT_FILES`, then lets `ship route-exit` emit `NEXT_ACTION=conflict-fix`. Do not invent conflict metadata prompt-side beyond the driver-provided `CONFLICT_FILES`.

## Extracted Script Registry

Load `${CLAUDE_PLUGIN_ROOT}/skills/implement/references/extracted-script-registry.md` only when editing or auditing extracted `/implement` script contracts.

## Bash block prelude

The Claude Code Bash tool does NOT preserve shell state between calls. Step 0 emits `$IMPLEMENT_TMPDIR/larch-run.sh` and the PID-keyed stable launcher, using the top-level Bash-tool `$PPID` captured by the Step 0 fence. Every post-Step-0 Bash fence that calls a plugin script MUST delegate through that stable launcher:

```text
"$HOME/.cache/larch/sessions/implement-run-$PPID.sh" <relative-script-path> ...
```

Post-Step-0 fences have exactly one nonblank, noncomment physical line. Do not source `plugin-root.env` inline, source session pointers, export variables, use continuations, or add inline shell logic. The `LARCH_CLAUDE_PID="$PPID"` prefix on the Step 0 fence is a plain environment-variable-prefix assignment, not post-Step-0 shell logic. Put foreground markers, anti-halt reminders, and rationale in prose outside fences. Each fence is a thin launcher invocation.

Pre-bootstrap fences keep their existing shapes. Step 0 initial bootstrap may keep the source guard plus the one-line `larch-run.sh --print-plugin-root` fallback (the generated launcher owns the `session-env.sh` parse). The single Preflight helper fence may keep its inline `preflight_args` assembly. Do not generalize those old shapes to post-Step-0 fences.

Sourcing full `session-env.sh` remains forbidden because it imports the whole namespace and can shadow caller state. Rust-owned `bootstrap invoke` emits the tmpdir-local launcher only after Step 0 `session write-env` succeeds, then `session write-implement-env` writes the PID-keyed stable launcher. All later script argv assembly belongs inside wrappers.

## Verbosity Control

Follow shared/verbosity-control.md rules.

**Preserved:** step breadcrumbs (`🔶`, `⏩`, `⏭️`), warning/error lines, structured summaries, plans, design decisions, code-review findings, and the final report.

**Suppressed:** explanatory prose, script paths, inter-call rationale, and per-reviewer completion messages. Rebase-skip cases at Steps 1.r, 4.r, 7.r, and 7a.r silently continue unless the referenced routing file says otherwise.

## Rebase Checkpoint Macro

Standardizes post-step rebase checkpoints 1.r, 4.r, 7.r, and 7a.r. Step 4.r is folded into Step 3 `checks-commit-route`; 7.r into Step 6 `step-6-entry`; 7a.r into `step-7a`. Each site routes through `rebase-checkpoint-routing.md` only when its composite emits `CHECKPOINT_NEXT=load-routing` or a malformed/missing routing KV.

**Thin implementation**: `${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh push checkpoint-probe` owns full argv, exit codes, and KV grammar in `skills/implement/references/rebase-checkpoint-routing.md`. Checkpoint **4.r** is folded into Step 3, **7.r** into Step 6, and **7a.r** into Step 7a. The absorbed **1.r** checkpoint is inside Step 0 bootstrap; route only on `BOOTSTRAP_NEXT=rebase-routing` and load the routing reference then.

**Registry identifiers:** `1.r` / `1.m` remain stable macro `<step-prefix>` tokens listed in `skills/implement/scripts/step-name-registry.tsv`; they label internal rebase checkpoints, not standalone orchestrator steps after plan materialization folded into Step 0.

**Conditional routing reference**: Absorbed `1.r`: branch only on `BOOTSTRAP_NEXT=rebase-routing` from the Step 0 bootstrap stdout envelope. Parse `ROUTE=`, `REBASE_RC=`, conflict detail KVs, and advisory `PHANTOM_*`. If `ROUTE=conflict` but no conflict files are present because the rebase auto-committed after earlier conflict resolution, follow `rebase-checkpoint-routing.md` phantom-probe instructions. When `DEGRADED_PROMPT_REQUIRED=true` on the absorbed `1.r` path, **MANDATORY: READ ENTIRE FILE** `${CLAUDE_PLUGIN_ROOT}/skills/implement/references/bootstrap-recovery.md` for degraded-prompt handling before treating absent routing keys as rebase failure. Folded `4.r`, `7.r`, and `7a.r`: parse `CHECKPOINT_NEXT=continue|load-routing` from captured stdout. `CHECKPOINT_NEXT=continue` is the only macro no-op predicate (skip the routing reference). Missing or malformed `CHECKPOINT_NEXT` fails closed: **MANDATORY: READ ENTIRE FILE** `${CLAUDE_PLUGIN_ROOT}/skills/implement/references/rebase-checkpoint-routing.md`. On `CHECKPOINT_NEXT=load-routing`, load that reference and branch on `ROUTE=`, `REBASE_RC=`, `REBASE_OUTCOME=`, and related KVs inside it. Do not use `ROUTE=continue` alone as the skip predicate when `CHECKPOINT_NEXT` is missing or malformed. The `7.r` macro skip is `CHECKPOINT_NEXT`-only. The `7a.r` macro skip is `CHECKPOINT_NEXT`-only.

## Checks Failure Entry Macro

Use this macro after Step 3 emits `STATUS=fail` or a folded composite emits `NEXT_ACTION=checks-failed`; the failure path remains in-step. Call sites should invoke **Checks Failure Entry Macro** by name with their pinned `--site` / `--checks-site` arguments instead of restating these read steps.
1. At folded sites, key-scan the full composite stdout for both `DIGEST_FILE` and `REDACTED_LOG_FILE`, not only the first physical composite line. Do not Read either file on the checks-repair path; never read raw `LOG_FILE`. `REDACTED_LOG_FILE` remains the input passed to `checks repair-loop`, and a later `NEXT_ACTION=main-agent-edit` handoff materializes bounded subagent evidence from it.
2. **MANDATORY: READ ENTIRE FILE**: `${CLAUDE_PLUGIN_ROOT}/skills/implement/references/checks-repair-loop.md`.
3. Follow that reference's pinned site split for the call site, including re-entry and folded-site recapture rules.

## Durable Bail to Step 18 Macro

**MANDATORY: READ ENTIRE FILE**: `${CLAUDE_PLUGIN_ROOT}/skills/implement/references/step5-review-branches.md`; follow its **Durable Bail** section with pinned `STALL_STEP=5`.

## Flags

**Invocation contract**: Accept one leading `--lifecycle-parent-context <absolute-context-path>`; bind `LIFECYCLE_PARENT_CONTEXT` and remove it. Reject other forms before Preflight.

**Flags**: Parse flags from the start of `$ARGUMENTS` before the positional issue. Flags may appear in any order. **All boolean flags default to `false`.** Set a mental flag to `true` only when its listed token appears. `--force` and `-f` both set `force_requested=true`. Strip recognized flags before binding the issue.

| Flag | Default | Purpose |
|------|---------|---------|
| `--merge` | `false` | Enable CI+rebase+merge loop (Steps 12–15) and related merge surfaces |
| `--no-admin-fallback` | `false` | No-queue Step 12b: skip admin and try a plain merge after the eligibility gate |
| `--no-logs-commit` | `false` | Suppress terminal implement archive publication |
| `--forked` | `false` | Fork-CI dry-run against `origin` / `upstream/main`; disables tracking-issue lifecycle, merge |
| `--draft` | `false` | Create PR as draft; implies no merge loop |
| `--force` / `-f` | `false` | Skip the item 4 plan-adequacy audit entirely (no `AUDIT=refuse` result exists to downgrade). Downgrade the three remaining fail-closed Preflight gates — missing plan, malformed plan, and `missing-designed-prefix` — to warn-and-proceed; warn loudly on each triggered bypass. Keeps the helper-side plan-block fallback. Does not affect coder selection. |
| `--self-review` | `false` | Skip the external review panel; a Claude Agent-tool subagent (`larch:claude-self-reviewer`, same model family as the orchestrator) performs thorough self-review at Step 5 instead of the main agent |
| `--self-implement` | `false` | Force `coder=claude` (skip external implementers). Step 2.4 Claude-fallback plan work always runs via `larch:claude-implementer` whether or not this flag is set; the flag only forces the Claude coder selection. Independent of `--force`. |
| `--difficulty <TRIVIAL\|MODERATE\|HARD>` | empty | Set the starting Step 5 review tier. The override beats rating and floors, logs `override_source=operator`, and the 1:30 audit can still upgrade a below-HARD run while preserving both fields. |
| `--coder` | unset | Pin external implementer to claude, codex, or cursor when set; otherwise availability waterfall. Ignored when `--self-implement` is active (always forces claude subagent). |
| `--run-id <ID>` | empty | Optional stable run id |

**Mutual exclusion**: reject `--forked` with `--merge`, `--draft` with `--merge`, and `--force` / `-f` with `--draft`, printing the exact warning named by the pair and exiting before Preflight. (`--force` / `-f` and `--merge` are **compatible**: use both for a forced fix through CI and automatic merge.) The `--force` / `-f` and `--draft` together case uses the third warning. Exact warnings: `**⚠ --forked and --merge are mutually exclusive. Aborting.**`; `**⚠ --draft and --merge are mutually exclusive. Aborting.**`; `**⚠ --force and --draft are mutually exclusive. Aborting.**`.

**Positional `<issue-N>` (required)**:

1. After flag parse, **exactly one** positional token must remain and MUST match `^[0-9]+$`. Bind it as `TARGET_ISSUE_NUMBER` for Preflight and Step 0 tracking adoption (authoritative subject issue for the run).
2. If any **non-flag** token remains that is **not** all digits (a verbal feature description or extra args), print verbatim:

`**❌ /implement no longer accepts a verbal feature description. Run /design <issue-N> first to write a plan to the issue body, then re-run /implement <issue-N>.**`

and exit **2** (orchestrator stop — do not start Preflight or Step 0).

3. Removed argv surfaces (must not be accepted as flags here): `--auto`, `--quick`, `--inline`, `--design-only`, `--no-issues`, `--hard`, `--issue`, `--session-env`, `--subagent`, `--design-classification`, `--branch-info`, `--step-prefix`, `--full`, `--dynamic-archetypes`, `--no-dynamic-archetypes`, `--emergency` (replaced by `--force` / `-f`; when `--emergency` is present print `**⚠ /implement --emergency is removed. Use --force or -f instead. Aborting.**` and exit **2** before Preflight).

**`--forked`**: compatible with `--draft`, `--no-logs-commit`, and `--coder`, subject to `--merge` / `--draft` exclusions above. Disable tracking-issue lifecycle. Treat `TARGET_ISSUE_NUMBER`, when set, only as **`UPSTREAM_DESIGN_ISSUE`** context in Step 0 fork tracking resolution, not as a local tracking issue.

## Preflight — issue-anchored plan

Run **before Step 0** after `TARGET_ISSUE_NUMBER` is known and flag mutex checks pass. Use a shell `mktemp -d` preflight tmpdir, not `$IMPLEMENT_TMPDIR` (not created until Step 0). Keep `PLAN_TMP="$PREFLIGHT_TMPDIR/plan-from-issue.txt"` through Step 0 materialization. When `forked_target=true`, `UPSTREAM_REPO` MUST already come from Protocol `scripts/larch.sh admission fork-env`. Run `admission fork-env`, then the preflight helper, then Step 0 bootstrap.

**Force mode (`--force`)**: when `force_requested=true`, read `${CLAUDE_PLUGIN_ROOT}/skills/implement/references/force-mode.md` completely before applying force behavior. Inline item 4 remains authoritative for the skip breadcrumb and no-read / no-audit-file / no-bypass-log contract.

1. **Mechanical Preflight helper (items 1-3)** — `"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" implement preflight` is the sole mechanical Preflight surface for admission, issue fetch, plan extraction, force missing/malformed fallback composition, and zero-review provenance refusal (`panel-init-failed`, `panel-skipped`, `rounds_completed: 0`). Invoke it through the verified larch entrypoint:
   ```bash
   [ -z "${CLAUDE_PLUGIN_ROOT:-}" ] && [ -n "${IMPLEMENT_TMPDIR:-}" ] && [ -f "$IMPLEMENT_TMPDIR/plugin-root.env" ] && . "$IMPLEMENT_TMPDIR/plugin-root.env"
   export IMPLEMENT_TMPDIR

   preflight_args=(--issue "$TARGET_ISSUE_NUMBER" --preflight-tmpdir "$PREFLIGHT_TMPDIR")
   if [ -n "${UPSTREAM_REPO:-}" ]; then
     preflight_args=("${preflight_args[@]}" --repo "$UPSTREAM_REPO")
   fi
   if [ "${force_requested:-false}" = true ]; then
     preflight_args=("${preflight_args[@]}" --force)
   fi

   "${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" implement preflight "${preflight_args[@]}"
   ```
   The helper writes `$PREFLIGHT_TMPDIR/issue.json`, `$PREFLIGHT_TMPDIR/plan-from-issue.txt`, and `$PREFLIGHT_TMPDIR/force-bypass.log` only for bypasses.

   After the helper returns:
   - Capture stdout from the Bash tool result.
   - On non-zero exit, abort before item 4 and preserve the helper's exit semantics.
   - Do not parse or require an envelope on non-zero exit.
   - On exit `0`, parse the validated success envelope; `scripts/larch.sh implement preflight` self-validates the success envelope and exits `2` before success parsing when malformed.
   - Parse one `KEY=value` record per line.
   - Split each envelope line at the first `=` only and preserve the remaining value verbatim.
   - Ignore non-envelope warning or prose lines that do not begin with an allowed envelope key plus `=`.
   - Parse only exact allowed preflight envelope keys: `ADMISSION_RESULT`, `RESUME`, `TITLE`, `BLOCK_PRESENT`, `PLAN_PATH`, `ISSUE_JSON_PATH`, `BYPASS_COUNT`, `PLAN_RECEIPT_SCOPE_REVALIDATION`, `PLAN_RECEIPT_PREVIOUS_BASE_SHA`, `PLAN_RECEIPT_TARGET_BASE_SHA`, `DESIGN_DIFFICULTY`, `MAIN_CI_STATUS`, `MAIN_FAILED_RUN_ID`, `MAIN_HEALTH_HEAD_SHA`, and `MAIN_HEALTH_DETAIL`.
   - Bind `PLAN_TMP` from `PLAN_PATH`, bind the three `PLAN_RECEIPT_*` values, and bind the four `MAIN_*` values from the envelope and/or durable `$IMPLEMENT_TMPDIR/main-health.env` before routing. `PLAN_RECEIPT_SCOPE_REVALIDATION=true` is the sole authority for the conditional refresh below; do not route from a prose warning. After `BOOTSTRAP_NEXT=step2`: `fail` without a matching repair marker loads `${CLAUDE_PLUGIN_ROOT}/skills/implement/references/step2-main-health-fix.md`; a matching marker dispatches; `pending` runs bounded `scripts/larch.sh ci main-health --wait`; `error` operator-bails with detail; `pass` continues; `skip` prints `**⚠ /implement: default-branch push CI is unavailable; continuing without main-health verification.**` and continues because the target repo has no configured main-health workflow or no default-branch push workflow runs.

4. **Plan-adequacy audit (main agent, in-prompt only)** — **When `force_requested=true`, skip this audit entirely.** This force audit-skip branch is the first control-flow instruction in item 4 and runs before any mandatory read below: print one skip breadcrumb `⏭️ /implement --force: skipping plan-adequacy audit for issue #<N>; continuing to semantic materiality.`, then jump directly to item 6. On the force audit-skip branch, do **not** read `${CLAUDE_PLUGIN_ROOT}/skills/implement/references/preflight-plan-audit.md`, do **not** create or overwrite `$PREFLIGHT_TMPDIR/audit.txt`, and do **not** append to `$PREFLIGHT_TMPDIR/force-bypass.log` — the audit skip is not a downgraded gate and writes no bypass-log entry.

   **When `force_requested=false` (only)** — **MANDATORY: READ ENTIRE FILE** at Preflight item 4: `${CLAUDE_PLUGIN_ROOT}/skills/implement/references/preflight-plan-audit.md`. Read issue title/body from `$PREFLIGHT_TMPDIR/issue.json` and plan text from `$PLAN_TMP`. Do not fetch the issue live or rerun plan-block extraction. On `AUDIT=pass`, return the pass envelope in chat only and do not write `$PREFLIGHT_TMPDIR/audit.txt`. On `AUDIT=refuse`, write that file. Do **not** delegate to a subagent or external audit CLI.

5. **On `AUDIT=refuse`** — read `audit.txt` only on refuse. This non-force-only path follows `${CLAUDE_PLUGIN_ROOT}/skills/implement/references/preflight-plan-audit.md` for clarify state, comment, and label flow, then exits **3** before Step 0.
6. **On `AUDIT=pass` or the force audit skip — semantic materiality (read-only bounded probe — see NEVER #21)** — run one bounded read-only probe of cited paths and symbols; before `BOOTSTRAP_NEXT=step2`, only `$PREFLIGHT_TMPDIR/**`, the stale notice, and conditional refresh may write. On clear staleness, write and post `$PREFLIGHT_TMPDIR/stale-notice.md` (retry once; add `--repo "$UPSTREAM_REPO"` when forked), exit **2**, and never close or rename. For `PLAN_RECEIPT_SCOPE_REVALIDATION=true`, resolve `origin/main` (or `upstream/main`) before and after probing its exact object; both SHAs must equal `PLAN_RECEIPT_TARGET_BASE_SHA`.
   If current, invoke `"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" plan-receipt refresh --issue "$TARGET_ISSUE_NUMBER"` (add `--repo "$UPSTREAM_REPO"` when set) with `--repo-root "$CLAUDE_PROJECT_DIR" --preflight-tmpdir "$PREFLIGHT_TMPDIR" --base-ref <resolved-ref> --previous-base-sha "$PLAN_RECEIPT_PREVIOUS_BASE_SHA" --base-sha "$PLAN_RECEIPT_TARGET_BASE_SHA"`; require `PLAN_RECEIPT_REFRESHED=true`, matching `PLAN_RECEIPT_BASE_SHA`, `PLAN_RECEIPT_SNAPSHOT_UPDATED=true`, and `PLAN_RECEIPT_SCOPE_DRIFT_LOGGED=true`, else exit **2**. The command alone renews and reads back the issue snapshot and writes one scope-drift `Warnings` record; otherwise continue to Step 0.

7. **Preflight pass gate**: retain `PREFLIGHT_TMPDIR` and `plan-from-issue.txt`; proceed to Step 0.

**Preflight — admission gate known limitation (D3)**: `scripts/larch.sh admission gate` inherits blocker discovery's historical **fail-open** posture on GitHub API failures. API outages can yield zero detected blockers (`ADMISSION_RESULT=pass`) even when blockers are unknown. Operators needing strict fail-closed blocker reads must pause runs during outages; see `crates/larch-cli/src/admission_commands.rs`. **Native-first short-circuit**: native dependency API blockers skip the prose scan for speed, so operator-visible lists may omit prose-only blockers until native blockers clear.

### `/implement` orchestrator exit codes (Preflight + argv)

| Code | When |
|------|------|
| **0** | Normal completion of the scripted skill path. |
| **2** | Flag mutual-exclusion, verbal/non-numeric argv tail, missing/malformed `larch:plan` when not bypassed by `--force`, empty issue body and empty title under `--force` (nothing to implement), `gh` / `scripts/larch.sh plan-block read` / admission hard failures (except `missing-designed-prefix` when bypassed by `--force`), semantic stale notice posted at Preflight item 6, `persist-implement-run-flags` validation failures, and other operator-visible hard errors where this file specifies exit **2**. |
| **3** | **Preflight audit refused** — `AUDIT=refuse` exits **3**. Follow `${CLAUDE_PLUGIN_ROOT}/skills/implement/references/preflight-plan-audit.md` `## Clarify-request flow after AUDIT=refuse` for post, label, `STATE=ambiguous`, and `STATE=awaiting-response` behavior. **Force note**: `--force` skips the item 4 plan-adequacy audit before any `AUDIT=refuse` result exists, so this exit-**3** refuse path is unreachable under `--force`. |

<!-- step:0 — Session Setup -->
## Step 0 — Session Setup

Print: `> **🔶 /implement 0: setup**`

Step 0 is owned by `scripts/larch.sh bootstrap invoke` (`--mode initial` / `--mode resume`). Rust owns session setup/rehydration, the entry gate, session env, source snapshot, reviewer refresh, live pointer, tracking, plan materialization, dirty-tree checkpointing, branch capture, plan logging, implementer selection (`phase_coder_select`), and routing handoff. The wrapper forwards `/implement --force`, `/implement --self-review`, and `/implement --self-implement` via `case "${force_requested:-}" in` / `case "${self_review:-}" in` / `case "${self_implement:-}" in` so omitted flags stay omitted from bootstrap argv. Do not duplicate absorbed helper calls prompt-side. When `self_implement_requested=true`, `phase_coder_select` forces `coder=claude` regardless of `--coder` or tool availability; `--force` alone no longer affects coder selection. Every ordinary Step 2.4 Claude-fallback path (vendor missing, explicit `coder=claude`, or `--self-implement`) implements via the `larch:claude-implementer` Agent-tool subagent — the main agent never Edit/Write plan-scoped files on this branch. Use `SELF_REVIEW_REQUESTED` from the routing envelope to set `self_review` after parse when flag parsing did not already set it.

Wrapper reachability: `${CLAUDE_PLUGIN_ROOT}/skills/implement/scripts/step-0-bootstrap.sh` forwards `--difficulty` when set and delegates to `scripts/larch.sh bootstrap invoke`; the prompt-side entry remains the Step 0 wrapper below. The Rust bootstrap uses the typed Git reader and captures `BRANCH_NAME` after branch creation.

**Bootstrap edit gate (NEVER #21)**: do not call Edit, Write, or repo-mutating Bash on git-tracked paths until bootstrap exits 0 with `BOOTSTRAP_NEXT=step2`. The feature branch is created inside `step-0-bootstrap.sh`. On `dirty-recovery` or `degraded-prompt`, repo edits remain forbidden until resume yields `step2`. Repeat this gate before every `step-0-bootstrap.sh` fence (initial and `--mode resume`) until `BOOTSTRAP_NEXT=step2`. **Carve-out — rebase-routing**: when bootstrap returns `BOOTSTRAP_NEXT=rebase-routing`, follow `rebase-checkpoint-routing.md` and `conflict-resolution.md` (spawn `larch:ci-fixer` `MODE=conflict`; main agent does not edit conflicted files).

**⚠ Foreground required — do not use Claude background mode.**

```bash
[ -z "${CLAUDE_PLUGIN_ROOT:-}" ] && [ -n "${IMPLEMENT_TMPDIR:-}" ] && [ -f "$IMPLEMENT_TMPDIR/plugin-root.env" ] && . "$IMPLEMENT_TMPDIR/plugin-root.env"
export IMPLEMENT_TMPDIR
[ -z "${CLAUDE_PLUGIN_ROOT:-}" ] && [ -n "${IMPLEMENT_TMPDIR:-}" ] && [ -x "$IMPLEMENT_TMPDIR/larch-run.sh" ] && CLAUDE_PLUGIN_ROOT=$("$IMPLEMENT_TMPDIR/larch-run.sh" --print-plugin-root 2>/dev/null || true)
export CLAUDE_PLUGIN_ROOT
# Foreground required
LARCH_CLAUDE_PID="$PPID" "${CLAUDE_PLUGIN_ROOT}/skills/implement/scripts/step-0-bootstrap.sh" --mode initial --issue-number "$TARGET_ISSUE_NUMBER" --preflight-tmpdir "$PREFLIGHT_TMPDIR" --force-requested "${force_requested:-false}" --self-review-requested "${self_review:-false}" --self-implement-requested "${self_implement:-false}" --forked-target "${forked_target:-false}" --merge-requested "${merge:-false}" --draft-requested "${draft:-false}" --no-admin-fallback "${no_admin_fallback:-false}" --no-logs-commit "${no_logs_commit:-false}" --upstream-repo "${UPSTREAM_REPO:-}" --run-id "${RUN_ID:-}" --caller-env "${CALLER_ENV_PATH:-}" --session-env "${SESSION_ENV_PATH:-}" --coder "${coder:-}" --difficulty "${difficulty:-}" --lifecycle-parent-context "${LIFECYCLE_PARENT_CONTEXT:-}"
```

Parse wrapper stdout as a routing envelope; `$IMPLEMENT_TMPDIR/bootstrap-routing.env` is durable. On `--mode resume`, restore persisted coder and parent-context state from `$IMPLEMENT_TMPDIR/lifecycle-parent-context.env`. Rust owns Step 0; `step-0-bootstrap.sh` owns the wrapper. Offline coverage: `crates/larch-cli/src/implement_bootstrap_continuation.rs` unit tests plus Rust parity. Preserve `CONTEXT_FILE` for child skills. If `/implement` would replace the target with 2+ issues, **MANDATORY: READ ENTIRE FILE**: Read `${CLAUDE_PLUGIN_ROOT}/skills/implement/references/umbrella-partition.md` completely and execute it. On exit `0`, require `BOOTSTRAP_NEXT` in `step2|dirty-recovery|degraded-prompt|rebase-routing|cleanup`; if `BOOTSTRAP_NEXT` is absent or any other value, treat the bootstrap envelope as malformed and abort with exit `2`. Routing after parsing:

| `BOOTSTRAP_NEXT` | Routing |
|---|---|
| `BOOTSTRAP_NEXT=step2` | Proceed directly to Step 2 with `--coder "$coder"`. |
| `BOOTSTRAP_NEXT=degraded-prompt` | **MANDATORY: READ ENTIRE FILE**: Read `${CLAUDE_PLUGIN_ROOT}/skills/implement/references/bootstrap-recovery.md` completely. Execute the degraded-prompt branch. |
| `BOOTSTRAP_NEXT=rebase-routing` | **MANDATORY: READ ENTIRE FILE**: `${CLAUDE_PLUGIN_ROOT}/skills/implement/references/rebase-checkpoint-routing.md`. Parse `ROUTE`, `REBASE_RC`, conflict detail KVs, and advisory `PHANTOM_*` KVs from the Step 0 envelope; the Rust owner already selected conflict, bail, or malformed/absent post-1.r `ROUTE` details. |
| `BOOTSTRAP_NEXT=dirty-recovery` | **MANDATORY: READ ENTIRE FILE**: Read `${CLAUDE_PLUGIN_ROOT}/skills/implement/references/bootstrap-recovery.md` completely. Execute the dirty-recovery branch. |
| `BOOTSTRAP_NEXT=cleanup` | Do not enter Step 2; skip to Step 18 terminalization, then Step 19 cleanup, after required local-only cleanup. |

**Absorbed continue tail.** On the continue path (`IMPLEMENT_BAIL_REASON` empty, `STALL_TRACKING=false`, readable `PLAN_FILE`, non-empty `coder`), `scripts/larch.sh bootstrap invoke` runs the degraded-tools gate and checkpoint `1.r` internally and folds KVs into Step 0 stdout. `step-0-bootstrap.sh` forwards `--non-interactive true|false` from the canonical predicate in `${CLAUDE_PLUGIN_ROOT}/skills/shared/external-reviewers.md`; do not rely on `LARCH_SKILL_NON_INTERACTIVE` alone. One-down bootstrap emits `DEGRADED_PROMPT_REQUIRED=true` and stops before 1.r until the explicit Continue sentinel exists. Both-down emits `DEGRADED_HARD_FAIL=true` and stops in every mode. Advisory `PHANTOM_*` KVs appear only on Step 0 stdout, not `$IMPLEMENT_TMPDIR/bootstrap-routing.env`. Do not use `CODEX_STATE` or `CURSOR_STATE` as the operator explanation when stderr relayed the full degraded block.

**Step 1.r routing.** For checkpoint `1.r`, enter rebase handling only when `BOOTSTRAP_NEXT=rebase-routing` appears in the Step 0 bootstrap envelope. In that branch, use `ROUTE=`, `REBASE_RC=`, conflict detail KVs, and advisory `PHANTOM_*` from the same envelope. Step `4.r` is folded into the Step 3 `checks-commit-route` composite; `7.r` is folded into the Step 6 `step-6-entry` composite and `7a.r` into `step-7a`, each relaying `CHECKPOINT_NEXT=continue|load-routing` for the same **Rebase Checkpoint Macro** routing (`continue` skips the reference; `load-routing` or missing/malformed values load it).

`phase_coder_select` is the only omitted-`--coder` authority for Step 0. Explicit `--coder=claude` does not set `coder_fallback=true`; only the implicit waterfall Codex → Cursor → Claude emits that flag when it reaches Claude. `diff_lines: <N>` in `plan.txt` is informational sizing context and does not route the implementer.

`session-env.sh` reaches `review-and-fix CLI` in Step 5 via `--session-env-path`. Bash fences delegate through `$IMPLEMENT_TMPDIR/larch-run.sh`; wrappers read token, timing, stall, and run-id keys from `$IMPLEMENT_TMPDIR/session-env.sh` via `scripts/larch.sh session read-key`. `LARCH_RUN_ID` is written by `_write_base_session_env()` after `_phase_tracking()` resolves `RUN_ID`, not by the initial Step 0 `session write-env` call.

### Cross-Skill Presence Propagation

No cross-skill presence propagation action is required; this anchor preserves the post-review boundary chain.

## Phantom Untracked Probe

Read `${CLAUDE_PLUGIN_ROOT}/skills/implement/references/phantom-probe.md` only when changing probe call sites. Trailing `PHANTOM_*` KVs are advisory telemetry; do not act on them.

## Execution Issues Tracking

Progressive disclosure: do not load `${CLAUDE_PLUGIN_ROOT}/skills/implement/references/execution-issues-tracking.md` at section entry. Load it only for active OOS triage, `Pre-existing Code Issues` dual-write, self-review step 3, or Step 8 `oos-pipeline` call sites.

<!-- step:2 — Implement the Feature -->

Print: `> **🔶 /implement 2: implementation**`

`scripts/larch.sh implement run-dispatch` marks Step 2 token and timing telemetry internally on the first dispatch only. The mark happens after `dispatch.lock` acquisition and is skipped on `--answers` redispatch.

<!-- step:2 entry preconditions — legal next-actions matrix -->

This matrix is authoritative for Step 2. After parsing dispatcher stdout in 2.1 and completing 2.1.5 envelope validation, the orchestrator may take only the rows below. **If 2.2 / 2.4 prose appears to disagree, the matrix wins.** See NEVER #9.

| Resolved `STATUS` | `ORCHESTRATOR_EDIT_AUTHORITY` | Permitted next-actions | Forbidden |
|---|---|---|---|
| `complete` | `forbidden` (required) | Set `MANIFEST_PATH=$MANIFEST`; proceed to Step 3 | Edit, Write, repo-mutating Bash against the **git working tree**; `git diff`-based reconstruction; transcript inspection for diff replay |
| `needs_qa` | `forbidden` (required) | Run Q/A loop in 2.3 (read `$QA_PENDING`, ask via `AskUserQuestion`, **write answers JSON to `$IMPLEMENT_TMPDIR/codex-answers-$RESUME_N.json` — permitted**, re-invoke dispatcher with `--answers`) | Edit, Write, repo-mutating Bash against the **git working tree** unrelated to redispatch |
| `bailed` | `forbidden` (required) | Log `Step 2 — $TOOL_LABEL bailed: $REASON` to `Warnings`; bail per 2.2's REASON-set routing (Step 12d) | Edit, Write, repo-mutating Bash against the **git working tree**; do NOT attempt to "recover" by editing |
| `claude_fallback` + `RECOVERY_FROM=manifest-schema-invalid` | `allowed` (required) | Run Step 2.4 recovery sub-branch only: plan-scope alignment, commit-message synthesis, no implementation edits | Opportunistic Q/A, main-agent re-implementation, Edit/Write against recovered files, `git add -A`, destructive git cleanup |
| `claude_fallback` | `allowed` (required) | Run Step 2.4: spawn `larch:claude-implementer` (`MODE=step2-plan`) for plan edits; relay `FALLBACK_QUESTIONS` via `AskUserQuestion` + `SendMessage`/fresh-spawn; orchestrator owns difficulty/scout/commit fences | Main-agent Edit/Write of plan-scoped files; main-agent reads of plan body, feature body, or `ARCHITECTURAL_*.md` |
| any envelope failure (validation in 2.1.5) | n/a | Synthesize orchestrator-local bail with `REASON=orchestrator-envelope-invalid` (see 2.1.5); route as Step 2 → Step 12d hard-bail | Setting `MANIFEST_PATH`; entering 2.3 / 2.4 / Step 3 |

**Always-permitted writes regardless of row**: `$IMPLEMENT_TMPDIR/**` (Q/A artifacts, larch-log records, execution-issues), larch-log and summary publication calls in 2.5, captured `scripts/larch.sh checks run-relevant` helpers, and reads of `TRANSCRIPT` / `SIDECAR_LOG` for warning text extraction (NOT diff reconstruction). The forbidden column scopes to the **git working tree**.

**No mid-run scope re-litigation.** Once Step 2 begins with a plan, the orchestrator does not ask whether to stop for scope, capacity, or effort. Oversize plans should have failed `/design` or Preflight audit. Mid-run, the dispatcher or Claude-fallback subagent executes the plan or hits a concrete Step 12d bail. This does NOT suppress Codex Q/A loop questions or Claude-fallback `FALLBACK_QUESTIONS` ambiguity questions relayed by the orchestrator. See NEVER #7.

<!-- step:2 dispatch — coder selection -->

Regression coverage for this dispatcher surface lives in the inline tests in `crates/larch-cli/src/implement_step2_commands.rs`. The launcher and dispatcher contract is `skills/implement/references/step2-dispatch.md`.

**2.1 — First dispatch invocation**:

```bash
"$HOME/.cache/larch/sessions/implement-run-$PPID.sh" scripts/larch.sh implement run-dispatch --coder "$coder"
```

**Do NOT poll or print sidecar output while dispatching.** The Rust adapter starts or reattaches `implement-step2-dispatch`; its only launcher output is `BGJOB_STATUS=STARTED STEP=implement-step2-dispatch PGID=<n>`. Wait with the shared bgjob contract below. On `BGJOB_STATUS=WAIT`, repeat the exact wait fence with no intervening prose or tools. On `DONE`, read `$IMPLEMENT_TMPDIR/bgjob/implement-step2-dispatch.result.env`; continue only with `BGJOB_RC=0` and the complete dispatcher envelope from that file. Do not use sidecars, daemon stdout, or launcher stdout as completion evidence.

```bash
"$HOME/.cache/larch/sessions/implement-run-$PPID.sh" scripts/larch.sh bgjob wait --step implement-step2-dispatch --tmpdir "$IMPLEMENT_TMPDIR" --max-wait-s 270
```

The child `scripts/larch.sh implement run-dispatch` always passes `--plan-file "$IMPLEMENT_TMPDIR/plan.txt"` and no workflow flag; it does **not** assemble paths from `PLAN_FILE` keys in `session-env.sh`. It reads `CURSOR_BINARY_FOUND` / `CODEX_BINARY_FOUND` from `$IMPLEMENT_TMPDIR/session-env.sh` or fresh executable checks, uses `$IMPLEMENT_TMPDIR/feature-description.txt`, and if the Step 0 selected binary is missing, relays `STATUS=claude_fallback` with edit authority instead of hard-failing. Before relaying stdout, it resolves repo root and captures `step2-prelaunch-porcelain.nul` plus prelaunch digests for Step 2.4. Its full envelope is atomically published into the bgjob result env. Parse `STATUS`, `TOOL`, `MANIFEST`, `QA_PENDING`, `REASON`, `TRANSCRIPT`, `SIDECAR_LOG`, `ORCHESTRATOR_EDIT_AUTHORITY`, additive plan-coverage KVs, and optional recovery triplet `RECOVERY_FROM`, `RECOVERY_PRIOR_TOOL`, `RECOVERY_PATHS_FILE` only from that result env. Coverage applies only to firm `### NEW:` / `### UPDATED:` / `### REWRITTEN:` headings from `$IMPLEMENT_TMPDIR/plan.txt`, not `### MAY_UPDATE:`. Malformed coverage on a complete path fails closed in the Rust dispatcher. Then run 2.1.5 before branching on `STATUS`. Derive:

Set `TOOL_LABEL` to `Codex` for `TOOL=codex`, `Cursor` for `TOOL=cursor`, and `external implementer` for any other tool token.

**2.1.5 — Envelope validation (fail-closed)**:

After parsing 2.1's KV envelope and BEFORE the 2.2 `STATUS` switch, validate:

1. `STATUS` is exactly one of `complete`, `needs_qa`, `bailed`, `claude_fallback`.
2. `ORCHESTRATOR_EDIT_AUTHORITY` is exactly one of `allowed` or `forbidden`, and appears **exactly once** on stdout. Zero or duplicate `ORCHESTRATOR_EDIT_AUTHORITY=` lines are illegal and trigger `orchestrator-envelope-invalid`. The Rust integration suite pins the dispatcher-side value.
3. The pair is **legal**: `ORCHESTRATOR_EDIT_AUTHORITY=allowed` iff `STATUS=claude_fallback`. Any other combination is illegal.
4. Recovery triplet integrity: if any of `RECOVERY_FROM`, `RECOVERY_PRIOR_TOOL`, or `RECOVERY_PATHS_FILE` is present, all three must be present; `RECOVERY_FROM` must equal `manifest-schema-invalid`; `RECOVERY_PRIOR_TOOL` must be `codex` or `cursor`; `RECOVERY_PATHS_FILE` must point to a readable non-empty file; and `STATUS` must be `claude_fallback`.
5. Status-keyed manifest readability (mirrors the dispatcher contract in `skills/implement/references/step2-dispatch.md` stdout grammar):
   - If `STATUS=complete`: `MANIFEST` is non-empty and points to a readable file. `QA_PENDING` MUST be absent.
   - If `STATUS=needs_qa`: `QA_PENDING` is non-empty and points to a readable file, AND `MANIFEST` is non-empty and points to a readable file.
   - If `STATUS=bailed` or `STATUS=claude_fallback`: this check does not apply (no required manifest path on these branches).

If any check fails, synthesize an orchestrator-local bail: set `STATUS=bailed`, `REASON=orchestrator-envelope-invalid`, log `Step 2 — orchestrator-envelope-invalid: STATUS=<raw> AUTH=<raw> reason=<which-check-failed>` to `$IMPLEMENT_TMPDIR/execution-issues.md` `Warnings`, set `FINAL_BAIL_REASON=orchestrator-envelope-invalid`, `IMPLEMENT_BAIL_REASON=orchestrator-envelope-invalid`, `STALL_STEP=2`, `PHASE=implementation`, `STALL_TRACKING=true`, do NOT consume `MANIFEST`, do NOT enter 2.3 or Step 3, and bail to Step 12d. **`orchestrator-envelope-invalid` is orchestrator-local**, not a dispatcher REASON token.

**2.2 — Branch on `STATUS`**:

- `STATUS=complete` → set `$MANIFEST_PATH=$MANIFEST`, then run the Step 2 post-dispatch wrapper as one foreground Bash invocation:

**⚠ Foreground required — do not use Claude background mode.**

```bash
"$HOME/.cache/larch/sessions/implement-run-$PPID.sh" skills/implement/scripts/step-2-post-dispatch.sh --expected-branch "$BRANCH_NAME"
```

From the combined wrapper stdout capture, first token-scan all `PHANTOM_*` KVs per **Phantom Untracked Probe** (advisory), regardless of wrapper exit code. Optionally bind `BRANCH=` and `COMMIT_SHA=` for degraded display persistence. Then parse exactly one `POST_DISPATCH_NEXT=continue|bail`. Missing, duplicated, malformed, or `bail` output prints `**⚠ /implement Step 2: post-dispatch branch mismatch (expected $BRANCH_NAME).**`, appends a sanitized `main-branch-post-dispatch` warning via `"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" run-log append-entry`, sets `FINAL_BAIL_REASON=main-branch-post-dispatch`, `IMPLEMENT_BAIL_REASON=main-branch-post-dispatch`, `STALL_STEP=2`, `PHASE=implementation`, `STALL_TRACKING=true`, and bails to Step 12d without Step 3. `BAIL_REASON=main-branch-post-dispatch` is required. Missing `COMMIT_SHA=` is not failure. Only after `POST_DISPATCH_NEXT=continue`, parse the Step 2 plan-coverage KVs. When `PLAN_COVERAGE_DISPOSITION_REQUIRED=true`, ask the operator to choose `proceed-partial` or `bail-rescope`. Wait indefinitely; if the platform returns a no-response fallback, ask the same prompt again. For `proceed-partial`, run `"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" implement scope-disposition record --tmpdir "$IMPLEMENT_TMPDIR" --disposition proceed-partial --repo "$REPO" --tracking-issue "$ISSUE_NUMBER" --run-id "$RUN_ID"` and continue only after it records successfully. For `bail-rescope`, record `--disposition bail-rescope`, set `FINAL_BAIL_REASON=plan-scope-disposition-bail-rescope`, and route to Step 12d. Otherwise proceed to Step 3. Steps 4 / 9a / 9a.1 read this manifest; the orchestrator does not reconstruct changes with `git diff`. The probe runs only inside `skills/implement/scripts/step-2-post-dispatch.sh` on external `STATUS=complete`, after dispatcher commit; do not run it on `STATUS=claude_fallback`.
- `STATUS=needs_qa` → run the Q/A loop in 2.3. Note: the dispatcher may have repaired a non-standard `qa-pending.json` (e.g., `items[]` → `questions[]`) before emitting this status; the Q/A loop always reads canonical `questions[]` format from `$QA_PENDING`.
- `STATUS=bailed` → print and append protected-path or submodule warnings first when `REASON=protected-path-edit-required-out-of-scope` or `REASON=submodule-edit-required-out-of-scope`, using the exact warning strings in this bullet. If `REASON=prior-attempt-unfinalized`, do not launch or re-launch an external implementer: `step2-recovery-paths.nul` is the authoritative list of stranded edits relative to the preserved prelaunch snapshot, and the run must bail for recovery rather than re-baselining those edits. Then log `Step 2 — $TOOL_LABEL bailed: $REASON` to `Warnings`, mirror `REASON` into `FINAL_BAIL_REASON` and `IMPLEMENT_BAIL_REASON`, set `STALL_STEP=2`, `PHASE=implementation`, `STALL_TRACKING=true`, and bail to Step 12d. Exact warnings: `**⚠ /implement: Codex bailed on protected path .claude-plugin/plugin.json; Claude subagent will implement.**`; `**⚠ /implement: implementer bailed on submodule-restricted path; submodule edits are blocked for the Claude-fallback subagent too. No automatic inline recovery will run.**` Step 18a passes the in-memory step/phase/bail triplet to `scripts/larch.sh stall-recovery classify`; that classifier sanitizes public bail rendering and prevents compound tokens such as `dirty-state-after-timeout` from substring-matching transient-infra.
- `STATUS=claude_fallback` with `RECOVERY_FROM=manifest-schema-invalid` (with `ORCHESTRATOR_EDIT_AUTHORITY=allowed`, validated mechanically in 2.1.5) → enter the Step 2.4 recovery sub-branch, not the ordinary Claude-fallback implementation branch.
- `STATUS=claude_fallback` without `RECOVERY_FROM` (with `ORCHESTRATOR_EDIT_AUTHORITY=allowed`, validated mechanically in 2.1.5) → run the ordinary Claude-fallback branch in 2.4. If `ORCHESTRATOR_EDIT_AUTHORITY != allowed`, treat as envelope failure per 2.1.5 (do NOT enter 2.4).

**Step 12d hard-bail routing**: when Step 2 bails to Step 12d, mirror `FINAL_BAIL_REASON` / `IMPLEMENT_BAIL_REASON` from dispatcher `REASON` or the synthesized source, set `STALL_TRACKING=true`, set `STALL_STEP` and `PHASE`, and skip Steps 3-15. Execution continues at Step 18, where Step 18a stall recovery runs **before** the Step 16/17 final report per the recover-then-report contract. **Step 12d bail is not terminal.** Step 18a classifies and gates recovery; Step 16/17 renders once at Step 18b for terminal stall or during the natural post-recovery pass, then Step 18b tears down.

**Branch enforcement on `claude_fallback`**: the checked-out symbolic branch vs `BRANCH_NAME` assertion in the `STATUS=complete` bullet is scoped to `STATUS=complete` only (NEVER #9). On `claude_fallback`, the later Rust `ship pr` branch guard compares state `BRANCH_NAME` to the checked-out symbolic branch and refuses `main` or `master` unless `FORKED_TARGET=true` in `ship-pr-state.sh` and checkout still matches. Forked upstream-target flows may use the default branch name in state; every other run stalls before PR prep.

**2.3 — Q/A loop** (when `STATUS=needs_qa`):

1. Read `$QA_PENDING` (a JSON file containing `{"questions": [{"id": "q1", "text": "..."}, ...]}`).
2. Pose the questions to the operator via `AskUserQuestion` in a single batched call (one prompt per question, preserving the `id`). Log every Q/A pair to `$IMPLEMENT_TMPDIR/execution-issues.md` under `### Q/A` per the schema in 2.5 below.
3. Compose an answers file `$IMPLEMENT_TMPDIR/codex-answers-$RESUME_N.json` with shape `{"answers": [{"id": "q1", "text": "<answer>"}, ...]}` (`$RESUME_N` is the 1-indexed resume cycle counter the orchestrator tracks locally). The filename retains `codex-` for historical compatibility; the dispatcher accepts it for Cursor resumes too.
4. Re-invoke the Step 2 adapter with §2.1 flags plus `--answers "$IMPLEMENT_TMPDIR/codex-answers-$RESUME_N.json"`. The adapter replaces only the completed `needs_qa` result for this explicit redispatch; it still derives `$PLAN_FILE`, `$FEATURE_FILE`, and cursor presence from `$IMPLEMENT_TMPDIR/session-env.sh` and conventional paths. `--answers` is the only redispatch addition. **On every dispatcher return, including `--answers` redispatch, re-parse KVs and run §2.1.5 envelope validation in full BEFORE §2.2 branching.** Malformed or AUTH-illegal resume envelopes fail closed as `orchestrator-envelope-invalid`. The dispatcher enforces the 5-cycle cap; the 6th `--answers` invocation returns `STATUS=bailed REASON=qa-loop-exceeded`.

> **Continue to Step 3 IMMEDIATELY after re-dispatch returns.** The Q/A loop re-dispatch is not a halting point — proceed to Step 3 checks as soon as the dispatcher exits. → shared/subskill-invocation.md#step-boundary

**Recovery sub-branch**: when `RECOVERY_FROM=manifest-schema-invalid`, do not ask opportunistic questions and do not re-implement. Preserve external implementer working-tree edits. Run `"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" dirty-tree scope-check --plan-file "$IMPLEMENT_TMPDIR/plan.txt" --paths-file "$RECOVERY_PATHS_FILE"`; on non-zero, set `FINAL_BAIL_REASON=recovery-out-of-scope`, `IMPLEMENT_BAIL_REASON=recovery-out-of-scope`, `STALL_STEP=2`, `PHASE=implementation`, `STALL_TRACKING=true`, and bail to Step 12d. Synthesize a concise redacted commit message into `$IMPLEMENT_TMPDIR/recovery-commit-message.txt`. The Step 3 composite owns fresh postlaunch capture, `step2-recovery-paths-final.nul`, and final plan-scope validation before commit. NEVER use `git reset --hard`, `git restore`, `git checkout -- <path>`, or `git add -A` against recovered edits.

Print one of the following based on which path landed here, evaluated **in this exact order** (first match wins):
- When `self_implement=true`: `**ℹ Implementing with Claude subagent (--self-implement; larch:claude-implementer).**`
- When `coder=claude` AND `coder_fallback=true`: `**⚠ Cursor and Codex unavailable — implementing with Claude subagent (larch:claude-implementer).**`
- When `coder=codex`: print `**⚠ Codex selection drifted after Step 0; Step 2 fell back to Claude subagent (larch:claude-implementer).**` If `REASON` is the classified CLI-upgrade message, append `REASON` to that banner and log `Step 2 — codex selection drift: $REASON` to the `Warnings` section of `$IMPLEMENT_TMPDIR/execution-issues.md`. Otherwise, log `Step 2 — codex selection drift: session-env no longer permits codex, dispatcher returned claude_fallback` as before.
- When `coder=claude`: `**ℹ Implementing with Claude subagent (coder=claude; larch:claude-implementer).**`

If `coder=cursor` and Step 2 returned `STATUS=claude_fallback`, that is **not** a Step 2.4 messaging branch. Step 2 must already have failed closed before entering 2.4 because the bootstrap-selected Cursor path is not allowed to silently drift into Claude fallback.

**Claude-fallback subagent branch** (ordinary `STATUS=claude_fallback` without `RECOVERY_FROM`, including `--self-implement`, explicit `coder=claude`, and vendor-missing fallback): the main agent is the orchestrator only. Do **not** read `ARCHITECTURAL_INVARIANTS.md` / `ARCHITECTURAL_GUIDELINES.md`, do **not** read the plan or feature-description bodies, and do **not** Edit/Write plan-scoped files. Attribution for this path is recorded as `MODE=subagent` / `TIER=subagent` on the difficulty and scout fences (`--rater-tool subagent`, `--producer subagent`); the implementer work-mode token remains solely `MODE=step2-plan`. Spawn one Agent-tool subagent with `subagent_type` `larch:claude-implementer`. Prompt contains only: `MODE=step2-plan`, repository root, working branch `$BRANCH_NAME`, plan path `$IMPLEMENT_TMPDIR/plan.txt`, feature-description path `$IMPLEMENT_TMPDIR/feature-description.txt`, implement tmpdir `$IMPLEMENT_TMPDIR`, optional answers-file path when resuming after questions, and the contract reminders from `agents/claude-implementer.md` Step 2.4 mode. No plan body, feature body, or architectural-file content is inlined. Parse the three trailing `CODER_*` lines:

- `CODER_RESULT=complete`: working-tree edits are ready; continue with the shared post-edit fences below (difficulty with `--rater-tool subagent`, scout normalize with `--producer subagent`, recovery-paths, commit-message). The subagent must have written `$IMPLEMENT_TMPDIR/scout-coder-manifest.raw.json` and `$IMPLEMENT_TMPDIR/implementation-commit-message.txt` when it made edits.
- `CODER_RESULT=needs_qa`: the subagent cannot ask the operator. Parse any `FALLBACK_QUESTIONS` block from its return, run `AskUserQuestion`, write `$IMPLEMENT_TMPDIR/codex-answers-$RESUME_N.json`, and continue the same subagent via `SendMessage` with the answers path (or fresh-spawn with the answers path when `SendMessage` is unavailable). Cap at 5 answer cycles; on the 6th, bail with `REASON=qa-loop-exceeded` to Step 12d.
- `CODER_RESULT=bail` or `no-progress`: log `Step 2 — claude-implementer subagent: $CODER_SUMMARY` to `Warnings`, set `FINAL_BAIL_REASON=claude-implementer-subagent-$CODER_RESULT`, `IMPLEMENT_BAIL_REASON` to the same, `STALL_STEP=2`, `PHASE=implementation`, `STALL_TRACKING=true`, and bail to Step 12d.
- Missing or malformed `CODER_*` trailer: treat as `bail` with summary `orchestrator-envelope-invalid`.

After `CODER_RESULT=complete`, jump to **Claude-fallback difficulty contract** (orchestrator still rates difficulty from the resulting tree; the subagent owns plan-scoped edits and architectural acknowledgment). There is no main-agent Edit/Write implementation branch on ordinary Claude fallback.

Claude-fallback subagent implementation is not complete until the difficulty rating is recorded and the coder-produced scout manifest is normalized; skipping the fence drops coder-produced dynamics and Step 5 runs static reviewers only; it does not relaunch scout dynamic-archetypes on /implement.

**Claude-fallback difficulty contract**: after the subagent returns `complete` and before Step 3, rate the implementation with `"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" difficulty render-rubric` as the anchor. Write `$IMPLEMENT_TMPDIR/implement-difficulty-rating.raw.json` with `predicted_tier`, `confidence`, and bounded `rationale`, then call `"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" difficulty write-record --output "$IMPLEMENT_TMPDIR/difficulty-rating.json" --rater implement --rater-tool subagent --raw-rating-file "$IMPLEMENT_TMPDIR/implement-difficulty-rating.raw.json" --implement-raw-rating-file "$IMPLEMENT_TMPDIR/implement-difficulty-rating.raw.json" --design-tier "${DESIGN_DIFFICULTY:-}"` when a design prior is present. Then run `"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" implement scope-disposition compute --tmpdir "$IMPLEMENT_TMPDIR" --repo-root "$REPO_ROOT" --manifest-path "$IMPLEMENT_TMPDIR/manifest.json"`. If `PLAN_COVERAGE_DISPOSITION_REQUIRED=true`, use the same operator prompt and recording flow as the external-complete path before Step 3. For `--self-review`, keep Step 5 skipping unchanged and pass `--panel-skipped self-review`. Difficulty now selects the Step 5 panel tier, round cap, Codex reviewer model role, audit-upgrade state, and escalation state; `--self-review` still records `panel_skipped=self-review` instead of launching the external panel.

**Claude-fallback scout manifest contract**: after the subagent returns `complete` and before Step 3, the subagent must already have written `$IMPLEMENT_TMPDIR/scout-coder-manifest.raw.json` (use `{"archetypes":[]}` when no dynamic specialists are useful). For non-empty manifests, follow `agents/_implementer-base.md` scout selection rules: short lowercase slugs, prefer `dyn-<topic>`, avoid static/reserved slugs (`correctness`, `edge-cases`, `testing`, `generic`, `structure`, `plan-fidelity`, `security`, and `REVIEW_RESERVED` / `crates/larch-core/src/design/plan_scout.rs`), keep `rationale` single-line, and keep `prompt_body` 2-6 sentences focused on changed code. Use this compact schema:

```json
{"archetypes":[{"name":"slug","focus_area":"code-quality|risk-integration|correctness|architecture|security","weight":1,"rationale":"single-line reason","prompt_body":"2-6 sentence focus directive"}]}
```

**Pinned normalization fence (required, nonblocking)**: immediately after implementation and before Step 3, run exactly this one-line launcher fence:

```bash
"$HOME/.cache/larch/sessions/implement-run-$PPID.sh" scripts/larch.sh implement normalize-coder-scout --tmpdir "$IMPLEMENT_TMPDIR" --input "$IMPLEMENT_TMPDIR/scout-coder-manifest.raw.json" --producer subagent
```

If `scout-coder-manifest.raw.json` is absent, still run the helper with that expected path so it writes `missing-or-invalid` and an empty manifest. Invalid manifest output is nonblocking but loud. This fence is mandatory on every Claude-fallback path, including `--force`, `--self-implement`, explicit `--coder claude`, and both-tools-unavailable fallback. External `STATUS=complete` is unchanged; the dispatcher normalizes after a complete manifest.

After implementation and `normalize-coder-scout`, ensure redacted Step 4 commit text exists at `$IMPLEMENT_TMPDIR/implementation-commit-message.txt` (the Claude-fallback subagent writes it on `complete`). Derive `$IMPLEMENT_TMPDIR/implementation-commit-paths.nul` from a fresh postlaunch capture with:

```bash
"$HOME/.cache/larch/sessions/implement-run-$PPID.sh" scripts/larch.sh implement recovery-paths --repo-root "$REPO_ROOT" --tmpdir "$IMPLEMENT_TMPDIR" --capture-postlaunch --prelaunch-porcelain "$IMPLEMENT_TMPDIR/step2-prelaunch-porcelain.nul" --postlaunch-porcelain "$IMPLEMENT_TMPDIR/step2-postlaunch-porcelain.nul" --prelaunch-digests "$IMPLEMENT_TMPDIR/step2-prelaunch-content-digests.txt" --out-file "$IMPLEMENT_TMPDIR/implementation-commit-paths.nul"
```

Before re-launching the checks-repair composite after repair edits, refresh the postlaunch porcelain, pathspec, and commit message.

After the implementation commit (Step 4), the orchestrator constructs an in-memory manifest equivalent (computed from `git diff --name-only $BASELINE..HEAD` and the commit message) for Steps 9a / 9a.1 to consume. `$MANIFEST_PATH` is left empty on this branch.

### 2.5 — Q/A logging + larch-log append

After each `AskUserQuestion` return (Codex Q/A loop, Claude-fallback `FALLBACK_QUESTIONS` relay, or mid-coding ambiguity) and each chosen ambiguity resolution, append to `$IMPLEMENT_TMPDIR/execution-issues.md` under `### Q/A` using:

```markdown
- **Step 2 (<question|ambiguity>)**: <question or ambiguity description>
  **A**: <user answer OR chosen interpretation + one-sentence rationale>
```

**Sanitize Q/A at compose time** (secrets → `<REDACTED-TOKEN>`, internal URLs → `<INTERNAL-URL>`, PII → `<REDACTED-PII>`) because answers can contain sensitive content and `execution-issues.md` is published in the run archive.

**Progressive log append**:
1. Compose an NDJSON record with `phase="implement"`, `step="2"`, `category="Q/A"`, and a sanitized markdown `body`.
2. Append it with:
   ```bash
   "$HOME/.cache/larch/sessions/implement-run-$PPID.sh" scripts/larch.sh run-log append --log-root "$IMPLEMENT_TMPDIR/larch-logs" --skill implement --run-id "$RUN_ID" --batch execution-issues --record-file "$IMPLEMENT_TMPDIR/execution-issue-record.ndjson"
   ```
3. On `LOG_WRITTEN=false` with `ERROR=`, log `Step 2 — Q/A larch-log append failed: $ERROR` to `Warnings` and continue. Non-fatal.

If `RUN_ID` is unavailable on a degraded local-only path, keep the `$IMPLEMENT_TMPDIR/execution-issues.md` append. Step 18 fails closed and preserves the session unless `--no-logs-commit` explicitly suppresses publication.

Material answers that change scope or approach also log here (same `Q/A` category).

> **Continue to Step 3 IMMEDIATELY after the raw-manifest write and normalize-coder-scout fence complete.** Implementation is not the end of the run — checks, commit, review, PR, CI, and merge still must run.

<!-- step:3 — Relevant Checks (first pass) -->

Print: `> **🔶 /implement 3: checks (1)**`

> **Continue after bgjob `DONE`.** Follow `${CLAUDE_PLUGIN_ROOT}/skills/shared/bgjob-wait.md`. On `DONE` with `BGJOB_RC=0` and required Step 3 composite KVs in `$IMPLEMENT_TMPDIR/bgjob/implement-step3-checks.result.env`: if the folded rebase probe emits `CHECKPOINT_NEXT=load-routing` with a non-zero child rc, branch through the rebase macro before judging the rc. On `NEXT_ACTION=checks-failed`, apply **Checks Failure Entry Macro** with pinned `--site step3`. On `NEXT_ACTION=stall`, bail through Step 12d with the composite Step 4 stall state. On `NEXT_ACTION=continue`, parse `CHECKPOINT_NEXT=continue|load-routing` for folded `4.r` routing before Step 5. Failure path stays inside Step 3. Do NOT end the turn, summarize, or hand off.

**⚠ Bgjob foreground launch required — do not use Claude background mode. Expected launcher stdout is exactly `BGJOB_STATUS=STARTED STEP=implement-step3-checks PGID=<n>`.**

```bash
"$HOME/.cache/larch/sessions/implement-run-$PPID.sh" skills/implement/scripts/run-step-checks.sh --site step3 --commit-site step4 --rebase-checkpoint-4r --forked-target "${forked_target:-false}" # lint-consecutive-bash: ok step3 checks bgjob launch precedes the repeated wait fence
```

Wait with the shared bgjob contract. Repeat this exact fence on `BGJOB_STATUS=WAIT`.

```bash
"$HOME/.cache/larch/sessions/implement-run-$PPID.sh" scripts/larch.sh bgjob wait --step implement-step3-checks --tmpdir "$IMPLEMENT_TMPDIR" --max-wait-s 270
```

<!-- step:4 — First Commit (implementation) -->

Print: `> **🔶 /implement 4: commit (impl)**`

Step 4 is owned by the Step 3 composite. On external implementer path (`$MANIFEST_PATH` non-empty), the composite Step 4 leg returns `noop` because the dispatcher already committed `$TOOL_LABEL` edits using `manifest.commit_message`. Skip the `implement commit` invocation. Keep the skip breadcrumb: print `⏩ 4: commit (impl) status=skip reason=dispatcher-committed sha=$COMMIT_SHA elapsed=<elapsed>`. On Claude fallback, the composite invokes `scripts/larch.sh implement commit` with the redacted message and NUL pathspec from Step 2.4. On recovery paths, it refreshes `step2-recovery-paths-final.nul` after checks and commits that pathspec. Commit messages describe WHAT and WHY, not HOW.

### Rebase onto latest main (after implementation commit)

Checkpoint `4.r` is folded into Step 3 composite stdout. Parse `CHECKPOINT_NEXT` and apply **Rebase Checkpoint Macro** with `<step-prefix>=4.r` and `<short-name>=commit (impl)`. The wrapper already performs the `4.r-post-rebase` phantom probe, so parse advisory `PHANTOM_*` from the same stdout.

> **Continue to Step 5 IMMEDIATELY.** The implementation commit is not the end of the run — code review, checks (2), commit, code flow diagram, and PR still must run.

<!-- step:5 — Code Review: review-and-fix step5 → review-and-fix CLI (dynamic-archetypes default=1 in implement tmpdir mode; maximum allowed cap=1) -->
## Step 5 — Code Review

### Self-review mode (`--self-review`)

When `self_review=true`, skip the scripted review loop below.

**MANDATORY: READ ENTIRE FILE**: Read `${CLAUDE_PLUGIN_ROOT}/skills/implement/references/self-review.md` completely.

The reference owns Claude-subagent self-review dispatch (`larch:claude-self-reviewer`), the composite checks-commit route, `NEXT_ACTION=main-agent-edit` re-entry, tally write, and post-Step-5 continuation.

### Scripted review loop

**IMPORTANT: Code review must ALWAYS run.** Never skip for any change type. Step 5 invokes **one** `skills/implement/scripts/step-5-review.sh` launcher per Step 5 entry. The thin wrapper delegates to the Rust implement verb, which calls Rust `bgjob adapt` for lifecycle handling. The child marks telemetry, resolves `dynamic_archetypes_cap`, logs the Step 5 banner (hard ceiling of 2 for every tier; TRIVIAL Cursor Composer 2.5 singles with Codex gpt-5.6-luna only when Cursor is down; MODERATE Cursor Composer 2.5 plus Codex gpt-5.6-terra pairs; HARD Cursor Composer 2.5 plus Codex gpt-5.6-terra pairs; escalated rounds skip pruning; prune-to-empty converges; no round-5 re-probe; specialists per vendor plus at most one dynamic archetype pair) to bgjob stderr, and runs the file-backed `review-and-fix step5 --mode loop --starting-round 1` review loop. Owner death, orphan handling, timeout, stdout/stderr logs, and process-group cleanup belong to bgjob. `/implement` Step 5 does not launch a separate dynamic scout; it consumes the coder manifest when eligible, otherwise static reviewers only. The absorbed loop owns rounds, captured checks, lint-fix repair, substantiality, and bulk-skip gates. The Rust verb reads `$IMPLEMENT_TMPDIR/plan.txt`, uses the persisted difficulty override and resolved tier state, and does **not** forward `--panel`. The review panel is applied only inside `review-and-fix CLI` → `review core` with specialists per vendor plus at most one dynamic archetype pair; round 2 may launch a mechanically reduced reviewer panel from round-1 productivity, and an all-pruned round converges immediately.

Nested review token-context propagation through `review-and-fix CLI` is pinned by `${CLAUDE_PLUGIN_ROOT}/crates/larch-cli/tests/review_and_fix_commands.rs` and `${CLAUDE_PLUGIN_ROOT}/skills/implement/scripts/test-implement-review-token-propagation.md`.

The Step 5 adapter contract — bgjob start stdout, live-registry rejoin, canonical review and resume classification, atomic merge publication, and absence of detach sidecars — is pinned by the inline tests in `${CLAUDE_PLUGIN_ROOT}/crates/larch-cli/src/implement_review_commands.rs` and `${CLAUDE_PLUGIN_ROOT}/skills/implement/scripts/test-step-5-review.md`.

> **Continue after bgjob `DONE`.** Follow `${CLAUDE_PLUGIN_ROOT}/skills/shared/bgjob-wait.md`. On `DONE` with `BGJOB_RC=0` and required Step 5 review KVs in `$IMPLEMENT_TMPDIR/bgjob/implement-step5-review.result.env`: if the result env carries a valid `STEP5_REVIEW_STATUS=stall` envelope with required KVs, route through the Step 5 stall branch before any generic non-zero `BGJOB_RC` failure gate so `STALL_TRACKING` and Step 18 seeding are preserved. Treat `BGJOB_RC=timeout`, `BGJOB_RC=orphaned`, any other non-zero `BGJOB_RC` without that envelope, or missing required KVs as the existing Step 5 failure/stall branch.

**⚠ Bgjob foreground launch required — do not use Claude background mode. Expected fresh-launch stdout is exactly `BGJOB_STATUS=STARTED STEP=implement-step5-review PGID=<n>`.**

```bash
"$HOME/.cache/larch/sessions/implement-run-$PPID.sh" skills/implement/scripts/step-5-review.sh # lint-consecutive-bash: ok step5 review bgjob launch precedes the repeated wait fence
```

Wait with the shared bgjob contract. Repeat this exact fence on `BGJOB_STATUS=WAIT`.

```bash
"$HOME/.cache/larch/sessions/implement-run-$PPID.sh" scripts/larch.sh bgjob wait --step implement-step5-review --tmpdir "$IMPLEMENT_TMPDIR" --max-wait-s 270
```

Only when stdout contains `STEP5_REVIEW_STATUS`, parse child stdout with **token-aware** extraction: each line may contain multiple `KEY=value` tokens. Extract at least `STEP5_REVIEW_STATUS`, `STALL_TRACKING`, `STALL_REASON`, `ROUNDS_COMPLETED`, `FINAL_ROUND_NUM`, `FINAL_REVIEW_AND_FIX_STATUS`, `CODER_STATUS`, `FILES_CHANGED_HINT`, and `EFFECTIVE_ROUND_CAP`.

**Branch order override**: when `STEP5_REVIEW_STATUS=self-review-required`, run the self-review procedure below to completion first. Only after self-review completes may you continue through the same post-self-review chain as `--self-review`. This branch overrides the generic non-stall continuation line.

> **Continue after the loop returns.** On any non-stall `STEP5_REVIEW_STATUS`, execute the Cross-Skill Presence Propagation + Track Rejected Code Review Findings + Step 6 breadcrumb in order — do NOT end the turn, summarize, or write a handoff message before reaching Step 6. → shared/subskill-invocation.md#anti-halt

For `stall`, `main-agent-vote-required`, `coder-main-agent-required`, and `mav-resume-past-cap`, **MANDATORY: READ ENTIRE FILE** before executing the branch: `${CLAUDE_PLUGIN_ROOT}/skills/implement/references/step5-review-branches.md`.

Branch on `STEP5_REVIEW_STATUS` (only when present — preflight failures without it terminate at Step 18 per above):

- **`complete`**: proceed with Cross-Skill Presence Propagation, then Track Rejected Code Review Findings, then the Step 6 breadcrumb (the absorbed loop already ran `scripts/larch.sh checks run-relevant`, `scripts/larch.sh checks lint-fix` when needed, and the substantiality / bulk-skip gates inside Bash).
- **`cap-hit`**: print `**⚠ 5: code review hit $EFFECTIVE_ROUND_CAP-round cap without converging. Proceeding.**`, log to `Warnings`, then run the same post-Step-5 chain as `complete`.
- **`self-review-required`**: print `**⚠ /implement Step 5: all external reviewers failed at runtime; falling back to Claude-subagent self-review.**`, log a `Warnings` entry in `$IMPLEMENT_TMPDIR/execution-issues.md`, then **MANDATORY: READ ENTIRE FILE**: read `${CLAUDE_PLUGIN_ROOT}/skills/implement/references/self-review.md` and execute it exactly as the `--self-review` branch does. Do not call `review-and-fix step5` again. Do not stall as `panel-failed`. Do not reach Step 6 before self-review finishes.
<!-- # intentionally non-stable: step-5-resume.sh captures wall-clock time for round duration -->
- **`stall`**: follow the `stall` branch body in the Step 5 review-branches reference. Skip to Step 18 (stall recovery runs before the final report).
- **`main-agent-vote-required`**: follow the MAV branch body in the Step 5 review-branches reference, then run the composite checks/resume handoff against the MAV-applied fixes.

- **`coder-main-agent-required`**: follow the coder waterfall branch body in the Step 5 review-branches reference, then run the composite checks/resume handoff against the applied fixes.

> **Continue after bgjob `DONE`.** Follow `${CLAUDE_PLUGIN_ROOT}/skills/shared/bgjob-wait.md`. On `DONE` with `BGJOB_RC=0` and required resume KVs in `$IMPLEMENT_TMPDIR/bgjob/implement-step5-resume.result.env`: on composite `NEXT_ACTION=checks-failed`, apply **Checks Failure Entry Macro** with pinned `--site step5-mav --checks-site step5-review-fixes`. On checks pass, apply the composite stdout parsing slice and full resume envelope contract below. On `NEXT_ACTION=main-agent-edit`, delegate through the macro/reference. Only explicit structural failures use this route. Named pre-ship lint-fix exhaustion reasons stall with `FAILURE_REASON` and `LINT_FIX_TIER_LEDGER_PATH`; never repair them inline. Terminal `NEXT_ACTION=stall` from the repair loop is a routing summary only: do not skip to Step 18 here. First run the main-agent handoff terminal-stall timing capture and durable bail, then skip to Step 18. Do **not** re-invoke the Step 5 loop wrapper.

**⚠ Bgjob foreground launch required — do not use Claude background mode. Expected launcher stdout is exactly `BGJOB_STATUS=STARTED STEP=implement-step5-resume PGID=<n>`.**

```bash
"$HOME/.cache/larch/sessions/implement-run-$PPID.sh" skills/implement/scripts/step-5-resume.sh --checks-site step5-review-fixes --final-round-num "$FINAL_ROUND_NUM" # lint-consecutive-bash: ok step5 resume bgjob launch precedes the repeated wait fence
```

Wait with the shared bgjob contract. Repeat this exact fence on `BGJOB_STATUS=WAIT`.

```bash
"$HOME/.cache/larch/sessions/implement-run-$PPID.sh" scripts/larch.sh bgjob wait --step implement-step5-resume --tmpdir "$IMPLEMENT_TMPDIR" --max-wait-s 270
```

<!-- # intentionally non-stable: step-5-resume.sh captures wall-clock time for round duration -->
Before leaving the main-agent handoff terminal-stall path, record timing exactly once through `${CLAUDE_PLUGIN_ROOT}/skills/implement/scripts/step-5-resume.sh`. If checks/lint end in terminal stall, invoke the wrapper with `--final-round-num "$FINAL_ROUND_NUM" --record-only`, set `STALL_TRACKING=true` defensively, run **Durable Bail to Step 18 Macro** with pinned `STALL_STEP=5`, skip to Step 18, and do not continue to the composite resume success path or Step 6/16:

```bash
"$HOME/.cache/larch/sessions/implement-run-$PPID.sh" skills/implement/scripts/step-5-resume.sh --final-round-num "$FINAL_ROUND_NUM" --record-only
```

After the Step 5 resume bgjob returns `DONE` with `BGJOB_RC=0`, capture the full final `DONE` stdout and/or `$IMPLEMENT_TMPDIR/bgjob/implement-step5-resume.result.env`. Whitespace-token-scan only the first physical line for checks keys: `FAILURE_REASON`, `RELEVANT_CHECKS_OK`, `RELEVANT_CHECKS_SKIPPED`, `STATUS`, `EXIT_CODE`, and `PHASE`. Key-scan the full composite stdout for `DIGEST_FILE` and `REDACTED_LOG_FILE` so folded failure keys are not lost behind leading output. Parse exactly one line-anchored composite `NEXT_ACTION=` anywhere in the capture for `checks-failed` only. Ignore leading-line `NEXT_ACTION` tokens for resume authorization.

On resume, the loop evaluates substantiality and bulk-skip against the round-`FINAL_ROUND_NUM` artifacts before scheduling additional rounds. If `FINAL_ROUND_NUM == EFFECTIVE_ROUND_CAP`, the wrapper returns `STEP5_REVIEW_STATUS=mav-resume-past-cap`.

On checks pass, parse `BGJOB_RC=0`, the relayed resume child exit code, and the full composite stdout/result env. Use token-aware extraction for review-loop keys that may share a line, and parse line-anchored `NEXT_ACTION=`, `COMMITTED=`, `ERROR=`, `SHA=`, `COMMIT_OUTCOME=`, and `COMMIT_ROUTE_OUTCOME=` for diagnostics. Step 6 continuation requires `STEP5_REVIEW_STATUS`; without it, NEVER #4 is unsatisfied. When stdout contains `STEP5_REVIEW_STATUS=`, route by the Step 5 status table only. Do not map a normal Step 5 loop stall to `resume-handoff-commit-failed` because rc is non-zero or commit-route emitted `NEXT_ACTION=stall`.

When composite stdout lacks `STEP5_REVIEW_STATUS=` and lacks `NEXT_ACTION=checks-failed`, evaluate in order. First, `NEXT_ACTION=stall` means durable stall state is already seeded by commit-route; skip to Step 18. `NEXT_ACTION=continue` without `STEP5_REVIEW_STATUS=` is not Step 6 continuation. `NEXT_ACTION=continue` without `STEP5_REVIEW_STATUS=` is a preflight/resume failure: log, set `STALL_TRACKING=true` and `STALL_STEP=5`, and skip to Step 18. missing, duplicated, malformed, or non-zero-without-`NEXT_ACTION` output is an invalid composite envelope and follows the same failure path. Do not proceed to Cross-Skill Presence Propagation, rejected-findings tracking, Step 6, or Step 8 on lacks-envelope paths. A non-zero resume child rc with parsed `NEXT_ACTION=continue` is also a preflight failure. `STEP5_REVIEW_STATUS=` is the only Step 6 authorization; commit-phase success (`NEXT_ACTION=continue`, `COMMIT_ROUTE_OUTCOME=continue`, or `COMMIT_OUTCOME=ok|noop`) alone does not satisfy NEVER #4.

<!-- # intentionally non-stable: step-5-resume.sh captures wall-clock time for round duration -->
- **`mav-resume-past-cap`**: follow the `mav-resume-past-cap` branch body in the Step 5 review-branches reference, then follow the same post-Step-5 chain as `complete`.

Note: `review-and-fix CLI` runs `flush_review_batches` after each successful `_implement_round_body` round, and best-effort once on many stall paths, writing `code-review-tally` and `review-findings-full`. `compose_review_findings_output` passes `--issue 0` as the contract; consumers join by `RUN_ID`. Step 5 needs no extra main-agent `scripts/larch.sh voting write-tally` or `review compose-findings` call.

### Track Rejected Code Review Findings

**MANDATORY: READ ENTIRE FILE before composing rejected finding text or reasons not implemented: `${CLAUDE_PLUGIN_ROOT}/skills/shared/readability-style.md`.**

`review-and-fix CLI` copies rejected in-scope findings from the latest round to `$IMPLEMENT_TMPDIR/rejected-findings.md`. When coder output marks `SKIPPED:` or the round fails, reviewers can still reject findings; log them there for Step 16 instead of reprinting full findings inline.

```markdown
### [Code Review] <Reviewer Name>
**Finding**: <actionable description of the finding — include the specific file(s) and line(s) affected, what the reviewer identified as the issue, and what change they suggested. Use short sentences and bullets when helpful. Detail means enough content for a reader who never saw the original review to understand and act on the issue, not extra length.>
**Reason not implemented**: <clear justification for why this finding was not addressed — include the specific technical reasoning, relevant project conventions or design decisions, and why the current code is acceptable despite the finding. Preserve important details, but keep sentences short.>
```

<!-- step:6 — Relevant Checks (second pass) -->

Print: `> **🔶 /implement 6: checks (2)**`

The Step 6 thin wrapper delegates lifecycle ownership to `bgjob adapt`. The Rust child validates the seeded repository identity before checks and again before atomic result publication. The composite writes `.review-boundary-passed` at entry after Cross-Skill Presence Propagation, rejected-findings tracking, and the Step 6 breadcrumb. That releases `hook-stop-fail-close.sh`'s post-review guard.

> **Continue after bgjob `DONE`.** Follow `${CLAUDE_PLUGIN_ROOT}/skills/shared/bgjob-wait.md`. On `DONE` with `BGJOB_RC=0` and required Step 6 composite KVs in `$IMPLEMENT_TMPDIR/bgjob/implement-step6-checks.result.env`: if `CHECKPOINT_NEXT=load-routing` arrives with a non-zero child rc, treat the probe as a routed conflict and continue through the rebase macro instead of failing closed on rc alone. On `NEXT_ACTION=skip-to-7a`, print `⏩ 6: checks (2) status=skip reason=no-review-changes elapsed=<elapsed>` and proceed to Step 7a immediately. Do NOT end the turn, summarize, or hand off. On `NEXT_ACTION=checks-failed`, apply **Checks Failure Entry Macro** with pinned `--site step6`. On `NEXT_ACTION=stall`, bail through Step 12d. On `NEXT_ACTION=continue`, parse `CHECKPOINT_NEXT=continue|load-routing` for folded `7.r` routing before Step 7a.

**⚠ Bgjob foreground launch required — do not use Claude background mode. Expected launcher stdout is exactly `BGJOB_STATUS=STARTED STEP=implement-step6-checks PGID=<n>`.**

```bash
"$HOME/.cache/larch/sessions/implement-run-$PPID.sh" skills/implement/scripts/step-6-entry.sh --forked-target "${forked_target:-false}" # lint-consecutive-bash: ok step6 checks bgjob launch precedes the repeated wait fence
```

Wait with the shared bgjob contract. Repeat this exact fence on `BGJOB_STATUS=WAIT`.

```bash
"$HOME/.cache/larch/sessions/implement-run-$PPID.sh" scripts/larch.sh bgjob wait --step implement-step6-checks --tmpdir "$IMPLEMENT_TMPDIR" --max-wait-s 270
```

Parse `FILES_CHANGED`, `UNTRACKED_BASELINE`, `GIT_PROBE_FAILED`, and exactly one line-anchored composite `NEXT_ACTION=` record from the final `DONE` stdout and/or bgjob result env. Do NOT `eval` or `source` stdout. If `UNTRACKED_BASELINE` is present, treat it as the pre-Step-6 untracked set. If `GIT_PROBE_FAILED=true`, continue with warning semantics already embedded by the wrapper; do not reconstruct paths prompt-side.

Route `NEXT_ACTION=skip-to-7a` directly to Step 7a. Route `NEXT_ACTION=continue` through folded `7.r` `CHECKPOINT_NEXT=continue|load-routing` handling from **Rebase Checkpoint Macro** using `<step-prefix>=7.r` and `<short-name>=commit (review)`. Missing or malformed `NEXT_ACTION` is Tool Failure.

<!-- step:7 — Second Commit (review fixes) -->

The `FILES_CHANGED=true` path runs Step 7's commit route inside the Step 6 composite fence above. The composite's `--emit-step7-breadcrumb` flag emits the Step 7 breadcrumb before the commit leg.

If no files changed, skip. `review-and-fix CLI` commits accepted fixes inline each round, so the common path is already clean. If `FILES_CHANGED=true`, the Step 6 composite owns Step 7 commit routing and emits the breadcrumb. On `NEXT_ACTION=stall`, skip to Step 18 (stall recovery runs before the final report; durable bail is already seeded by commit-route). If the Step 7 commit route lacks durable seed, set prompt-side `STALL_TRACKING=true` and `STALL_STEP=7` when durable seed is absent, and skip to Step 18.

<!-- step:7a — Code Flow Diagram -->

Print: `> **🔶 /implement 7a: pre-ship**`

Runs unconditionally after Step 7 (regardless of Steps 6-7 skip).

Step 7a composes no prompt-side public summary and never emits diagram fences. The helper owns silent `larch:diagrams` upsert through `"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" diagrams upsert`; the orchestrator parses only KVs and breadcrumbs.

`scripts/larch.sh implement step-7a` consolidates small/non-runtime classification, the Rust `implement code-flow-diagram` owner, Code Flow section composition, shared `larch:diagrams` upsert, the 7a.r checkpoint, and a local execution-issues checkpoint. It emits a KV tail; do not duplicate those calls prompt-side.
The helper upserts the stable issue-scoped `<!-- larch:diagrams v1 -->` comment only when `$IMPLEMENT_TMPDIR/code-flow-section.md` exists after successful generation. Contract: `skills/implement/scripts/step-7a.md`; `skills/implement/scripts/test-step-7a.sh` (`skills/implement/scripts/test-step-7a.md`).

**⚠ Bgjob foreground launch required — do not use Claude background mode. Expected launcher stdout is exactly `BGJOB_STATUS=STARTED STEP=implement-step7a PGID=<n>`.**

```bash
"$HOME/.cache/larch/sessions/implement-run-$PPID.sh" scripts/larch.sh implement step-7a --bgjob-launch true --implement-tmpdir "$IMPLEMENT_TMPDIR" --issue-number "${ISSUE_NUMBER:-}" --run-id "$RUN_ID" --no-logs-commit "${no_logs_commit:-false}" --forked-target "${forked_target:-false}" # lint-consecutive-bash: ok step7a bgjob launch precedes the repeated wait fence
```

Wait with the shared bgjob contract. Repeat this exact fence on `BGJOB_STATUS=WAIT`.

```bash
"$HOME/.cache/larch/sessions/implement-run-$PPID.sh" scripts/larch.sh bgjob wait --step implement-step7a --tmpdir "$IMPLEMENT_TMPDIR" --max-wait-s 270
```

Treat the final `DONE` stdout and `$IMPLEMENT_TMPDIR/bgjob/implement-step7a.result.env` as one KV stream. Continue only when `BGJOB_RC=0` and required Step 7a KVs are present; if `CHECKPOINT_NEXT=load-routing` is present, route the non-zero probe rc through the rebase macro before treating the step as failed. Scan `REBASE_OUTCOME` only for stream ordering, then read `CHECKPOINT_NEXT=continue|load-routing` and final KV tail for diagram/log status. The `7a.r` macro skip is `CHECKPOINT_NEXT`-only. Route `load-routing` via the **Rebase Checkpoint Macro** using `<step-prefix>=7a.r` and `<short-name>=pre-ship`.

> **Continue to Step 8 IMMEDIATELY.** Step 7a no longer authors or stages architectural assessments. Step 8 owns the subagent assessment route. After later `HEAD` movement, `architectural-assessment materialize` re-runs its deterministic pre-filter against incremental scope, reuses valid coverage for nonintersecting changes, and re-assesses (via a fresh `larch:arch-assessor`) only when a later code change newly intersects architectural scope. PR creation, CI monitoring, and merge still must run.

<!-- step:8+ — Ship PR State Machine -->
## Step 8+ — Ship PR State Machine

Steps 8-14 are driven by the **Rust ship dispatcher** behind `step-8-ship.sh`. The wrapper enters through `scripts/larch.sh`; the Rust parent composes the shared bgjob adapter, and its child rehydrates state, runs the advisory phantom probe, invokes Rust `ship pr` in process, and writes ship outcome KVs directly to the bgjob merge-result env.

Run `"$HOME/.cache/larch/sessions/implement-run-$PPID.sh" scripts/larch.sh ship pre-driver` before reading the Step 8+ matrix. The pre-driver recomputes plan coverage and refuses to ship when a required scope disposition is missing, stale, or invalid. It emits `NEXT_ACTION=halt-scope-disposition` for a readable missing/stale disposition; malformed or tampered artifacts stay Tool Failure.
**MANDATORY: READ ENTIRE FILE**: Read `${CLAUDE_PLUGIN_ROOT}/skills/implement/references/ship-pr-exit-matrix.md` completely.

**Post-ship durable handoff.** Step 8 runs under bgjob using step slug `implement-step8-ship`. If the launcher prints `BGJOB_STATUS=WAIT`, the next action is the identical wait fence with no intervening prose or tools. On final `DONE`, read the full wait KV block and `$IMPLEMENT_TMPDIR/bgjob/implement-step8-ship.result.env`. Continue to `ship route-exit` only when `BGJOB_RC` is not `timeout` or `orphaned` and the result env has the required ship outcome KVs. Do not require `BGJOB_RC=0`; the numeric driver rc in the result env is authoritative for route-exit. Treat `BGJOB_STATUS=DEAD`, terminal bgjob rc tokens, or a missing/malformed ship outcome as the existing setup failure or stall branch. Do not poll, sleep, use Monitor, inspect process state, or treat launcher stdout / `DONE` alone as sufficient. The handoff is durable across turn breaks; after an unexpected turn end, resume through the wrapper, which rejoins a live identity-valid registry row or deliberately replaces a completed result for a reship.

```bash
"$HOME/.cache/larch/sessions/implement-run-$PPID.sh" scripts/larch.sh ship route-exit --implement-tmpdir "$IMPLEMENT_TMPDIR"
```

**Pre-driver predicate** (evaluate before choosing fences; read `$IMPLEMENT_TMPDIR/ship-pr-state.sh` when present): state file absent/empty, or `PHASE=checks` and `PR_NUMBER` is empty/absent. Seeded-but-no-PR state is still pre-driver. Run `ship pre-driver` only for this prompt-side predicate.

```bash
"$HOME/.cache/larch/sessions/implement-run-$PPID.sh" scripts/larch.sh ship pre-driver
```

**Seeder authority.** The Rust-owned `ship seed-initial-state` command owns the canonical initial state write contract. Rust also owns durable input resolution and canonical argv assembly behind `step-8-seed-initial.sh`.

Branch on pre-driver `NEXT_ACTION`:

- **`stall`**: the Rust runtime guard failed. Set `STALL_TRACKING=true`, skip `step-8-ship.sh`, and go directly to Step 18 (stall recovery runs before the final report). Pre-driver `stall` never routes through post-driver Step 16 prose.
- **`halt-seed`**: initial seeding failed. Stop before `${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh oos file` and `step-8-ship.sh`; the child output is already on stderr for Tool Failures logging.
- **`halt-oos`**: pre-driver OOS filing failed. Stop before `step-8-ship.sh`, log the failure under Tool Failures, and route to Step 18 per the normal stall path.
- **`oos-pipeline`**: security sidecar present before ship. Do not invoke `step-8-ship.sh` yet. Follow the same private-disposition flow as post-driver `oos-pipeline` below (read `$IMPLEMENT_TMPDIR/security-oos-observations.md`, follow `${CLAUDE_PLUGIN_ROOT}/docs/security/workflow-trust-and-mutations.md` `## Security Findings in OOS Workflows`, then run the OOS checkpoint fence).
- **`ship`**: proceed to `step-8-ship.sh`. On `NEXT_ACTION=ship`, proceed to `step-8-ship.sh` (the wrapper runs the internal guard and advisory phantom probe before the driver). A pre-driver retry reruns guard and `${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh oos file` while skipping the seeder when `ship-pr-state.sh` already has shell KV entries.

Invoke `step-8-ship.sh` as a bgjob foreground adapter. The wrapper delegates identity-valid live-job reattachment and completed-result replacement to `bgjob adapt`; the ship child writes its outcome directly to the per-step merge-result env.

**Post-driver Step 8+ continuations:** when the pre-driver predicate no longer matches, invoke only `step-8-ship.sh`; do not rerun pre-driver. The wrapper still runs its guard and advisory phantom probe inside the bgjob child before the driver.

> **Continue after bgjob `DONE`.** Follow `${CLAUDE_PLUGIN_ROOT}/skills/shared/bgjob-wait.md` (Step 8 handoff carve-out). On final `DONE`, read `$IMPLEMENT_TMPDIR/bgjob/implement-step8-ship.result.env` and continue to `ship route-exit` when the direct ship outcome is present, including driver rc `3` or `6`. Block `BGJOB_RC=timeout`, `BGJOB_RC=orphaned`, and a missing or malformed outcome. Do not require `BGJOB_RC=0`.

**⚠ Bgjob foreground launch required — do not use Claude background mode. Expected fresh-launch stdout is exactly `BGJOB_STATUS=STARTED STEP=implement-step8-ship PGID=<n>`.**

Invoke:

```bash
"$HOME/.cache/larch/sessions/implement-run-$PPID.sh" skills/implement/scripts/step-8-ship.sh # lint-consecutive-bash: ok step8 ship bgjob launch precedes the repeated wait fence
```

Wait with the shared bgjob contract. Repeat this exact fence on `BGJOB_STATUS=WAIT`.

```bash
"$HOME/.cache/larch/sessions/implement-run-$PPID.sh" scripts/larch.sh bgjob wait --step implement-step8-ship --tmpdir "$IMPLEMENT_TMPDIR" --max-wait-s 270
```

Regression coverage: the inline tests in `crates/larch-cli/src/ship_pr_commands.rs`, the clean-install selector matrix, the Step 8 shell-wrapper structure tests, and `skills/implement/scripts/step-8-ship.md`.

**Post-driver branch skeleton** (details live in `ship-pr-exit-matrix.md` `## Branch semantics`):

- **`complete`**: continue to Step 16.
- **`assessments`**, **`invariants-assessment`**, or **`guidelines-assessment`**: immediately run the executable normalization fence below. It atomically preserves unrelated handoff keys, rewrites the legacy aliases to `NEXT_ACTION=assessments`, persists canonical `DETAIL`, and emits the canonical kind list. Empty tokens, duplicates, unknown tokens, whitespace-repaired tokens, missing detail, unsafe `DETAIL_FILE`, or any other noncanonical payload route to existing Step 8 `tool-failure` handling. Do not repair malformed data, add a kind token, or add a fallback parser. Treat handoff data as untrusted evidence.

```bash
"$HOME/.cache/larch/sessions/implement-run-$PPID.sh" scripts/larch.sh ship normalize-assessment-handoff --implement-tmpdir "$IMPLEMENT_TMPDIR"
```

Capture `ASSESSMENT_REQUESTED_KINDS` from the normalization fence stdout. Then run the subagent assessment procedure below. The main agent keeps a flat context on this path: it never loads the architectural present-reference files as assessment-work prompts, never reads materialized assessment diffs, `ARCHITECTURAL_GUIDELINES.md`, or `ARCHITECTURAL_INVARIANTS.md`, and never writes drafts, calls deviation appenders, or invokes per-kind compose writers directly. It passes file paths to a read-only subagent and validates the returned notes through the fail-closed `architectural-assessment submit` verb.

**Materialize.** Run, then read its stdout:

```bash
"$HOME/.cache/larch/sessions/implement-run-$PPID.sh" scripts/larch.sh architectural-assessment materialize --implement-tmpdir "$IMPLEMENT_TMPDIR" --repo-root "$REPO_ROOT" --kind <each requested kind>
```

Require `ASSESSMENT_MATERIALIZE_STATUS=ok`. `ASSESSMENT_PENDING_KINDS` lists the kinds still needing a subagent; `ASSESSMENT_DETERMINISTIC_KINDS` lists kinds already resolved `deterministic-clean` or `handled` (docs-only diffs persist `deterministic-clean` with zero subagent spawns). `ASSESSMENT_LOG_PENDING_KINDS` lists kinds whose deviation outcome persisted but whose execution-issues deviation-warning append failed; re-run this materialize fence once for those kinds to retry the warning record. It is best-effort run-log completeness, never a merge blocker, and needs no subagent. The main agent reads only the per-kind `ASSESSMENT_KIND_<UPPER>_DIFF_PATH`, `ASSESSMENT_KIND_<UPPER>_KNOWLEDGE_PATH`, and `ASSESSMENT_KIND_<UPPER>_PRIOR_NOTE_PATH` values; it does not Read those files.

**Subagent authoring.** If `ASSESSMENT_PENDING_KINDS` is empty, skip the subagent. Otherwise spawn exactly one Agent-tool subagent with `subagent_type` `larch:arch-assessor`, covering every pending kind in the canonical order (invariants, then guidelines) so shared evidence is ingested once. Its prompt contains ONLY the requested kind list, the reminder `For clean state, use the canonical one-sentence note with no G-* or I-* identifier.`, and, per pending kind, the three materialize paths above. No evidence content is inlined. Follow `agents/arch-assessor.md`.

**Parse and submit.** Parse the subagent's final message: per pending kind it must contain one `ASSESSMENT_KIND=<kind>` line, one `ASSESSMENT_STATE=<state>` line, and one fenced note block, per `agents/arch-assessor.md`. For each pending kind, write that kind's fenced note body to `$IMPLEMENT_TMPDIR/assessment-note-<kind>.md`, then run:

```bash
"$HOME/.cache/larch/sessions/implement-run-$PPID.sh" scripts/larch.sh architectural-assessment submit --implement-tmpdir "$IMPLEMENT_TMPDIR" --repo-root "$REPO_ROOT" --kind <kind> --state <state> --note-file "$IMPLEMENT_TMPDIR/assessment-note-<kind>.md"
```

`submit` revalidates identity fail-closed (HEAD unchanged since materialize, fingerprint match), validates the state token and note shape/size, redacts, atomic-writes the same durable surfaces Step 16-17 read, and emits `ASSESSMENT_STATUS=complete` plus `ASSESSMENT_RESULTS=<kind>:<state>` on success. Require `ASSESSMENT_STATUS=complete` for every pending kind. A `submit` exit code `10` means HEAD drifted between materialize and submit: re-run materialize and spawn a fresh `larch:arch-assessor` for that kind, bounded at two attempts per kind, then existing Step 8 `tool-failure` handling. An unparseable final message, or `submit` `ASSESSMENT_STATUS=invalid-note`, gets exactly one fresh `larch:arch-assessor` respawn; if that also fails, route to existing Step 8 `tool-failure` handling. Nothing persists from a bad message: `submit` revalidates everything fail-closed. Do not reinterpret stale, partial, or fail-closed output as success.

**Fix ladder for adverse outcomes.** The ci-fixer subagent never fixes architectural violations or deviations; the coder does, then the main agent. Never route invariant or guideline fixes to the ci-fixer subagent. Per kind, independently, after `submit` persists `violation` (invariants) or `deviation` (guidelines):
- Tier 1, coder. Spawn one Agent-tool subagent with `subagent_type` `larch:claude-implementer` carrying paths to the plan (`$IMPLEMENT_TMPDIR/plan.txt`), the assessor note (`$IMPLEMENT_TMPDIR/assessment-note-<kind>.md`), and the materialized evidence, plus a scoped instruction: fix the named `violation`/`deviation` and nothing else. It edits, commits, and pushes via `"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" push branch`. Parse its three `CODER_*` result lines (`CODER_RESULT=pushed|no-progress|bail`). One round per kind.
- Re-judge. After any fix attempt: re-run materialize, spawn a FRESH `larch:arch-assessor`, submit the new verdict. The judge never evaluates its own fix and the fixer never judges.
- Tier 2, main agent (a deliberate, documented exception to the context-flat rule; last resort). If the re-judge still reports an adverse state, the main agent MAY read the materialized evidence and the note for that kind only. For invariants it MUST attempt the fix inline (edit, commit, push via `"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" push branch`, then re-materialize and re-judge via a fresh assessor). For guidelines it chooses one of fix inline (same mechanics) or decline to fix.
- Terminal outcomes. Invariants: if the tier-2 re-judge still reports `violation`, HARD STOP — record operator-bail reason `invariant-violation-unresolved` and follow the existing Step 12d bail flow; there is no waiver and the run must not create or merge the PR. Guidelines: if the main agent declines to fix, append a documented-exception block to the note before re-submitting — one line `Exception: <rationale> (author: main-agent, date: YYYY-MM-DD)` — then re-run submit with state `deviation` and the `--allow-exception` flag (only this decline re-submission may persist an `Exception:` line; a first submission carrying one is rejected fail-closed); the merge gate accepts that deviation only with this block.
- Bounds: one tier-1 round and at most one tier-2 round per kind, orthogonal to the HEAD-drift retry bound above.

After all requested results persist and validate (pending kinds `complete`; deterministic/handled kinds already resolved), return to the Step 8 ship launcher above exactly once. Do not relaunch once per kind. Continue to Step 8, not Step 16.
- **`halt-scope-disposition`**: Re-read `$IMPLEMENT_TMPDIR/plan-coverage.json` and ask the same `proceed-partial` / `bail-rescope` prompt. Record through `"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" implement scope-disposition record ...`, then relaunch Step 8. Do not create or update a PR before the record succeeds.
- **`reship`**: If `.ship-route-exit-handoff.env` has `RESUME_PHASE=ship-pr-rrr-phase14` and `CALLER_KIND=ship_pr_pre_push`, skip the pre-fix rebase. This is an existing conflict-resolution continuation. Proceed to the Step 8 bgjob relaunch, preserving those keys until conflict-resolution Phase 4 completes. For every other `reship`, run the foreground pre-fix rebase before the bgjob relaunch. Do not sleep.

```bash
"$HOME/.cache/larch/sessions/implement-run-$PPID.sh" scripts/larch.sh ship pre-fix-rebase --implement-tmpdir "$IMPLEMENT_TMPDIR"
```

Branch on its stdout: When `PRE_FIX_REBASE_REQUIRED=true` is set in `.ship-route-exit-handoff.env` and `$IMPLEMENT_TMPDIR/.ship-pre-fix-rebase-ok` is absent (regular, non-symlink), route to Step 16 with `STALL_TRACKING`, then Step 18. Otherwise `NEXT_ACTION=continue` proceeds to the Step 8 bgjob `step-8-ship.sh` relaunch. `NEXT_ACTION=conflict-fix` loads `conflict-resolution.md`; `NEXT_ACTION=stall` routes like post-driver stall.

- **`oos-pipeline`**: security sidecar disposition only. Do not load `execution-issues-tracking.md`, do not load or run `oos-pipeline.md`, and do not call `/issue` on this branch. Read `$IMPLEMENT_TMPDIR/security-oos-observations.md`, follow `${CLAUDE_PLUGIN_ROOT}/docs/security/workflow-trust-and-mutations.md` `## Security Findings in OOS Workflows` privately, and clear the sidecar only after private disposition completes. **MANDATORY: READ ENTIRE FILE**: Read `${CLAUDE_PLUGIN_ROOT}/skills/implement/references/ship-pr-oos-checkpoint-router.md` completely before the `step-8-oos-checkpoint.sh` fence. Expect the checkpoint to stall while `security-oos-observations.md` remains non-empty or private security disposition is pending.
- **`ci-fix`**: Handles PR failures, `main-ci-fail`, and `flaky-defect-unfixed`. If `FORKED_TARGET=true` or `REPO_UNAVAILABLE=true`, skip autonomous edits and route to **operator-bail**. Otherwise, run the foreground pre-fix rebase before the subagent CI-fix loop below; `FAILED_RUN_ID` may name a default-branch push run. Branch on stdout: `NEXT_ACTION=continue` runs the loop; `NEXT_ACTION=conflict-fix` loads `conflict-resolution.md`; `NEXT_ACTION=stall` routes like post-driver stall. `.ship-route-exit-handoff.env` carries `CI_ERRORS_FILE=<absolute path>` on distill success, or `CI_ERRORS_FILE=` plus `CI_ERRORS_DISTILL_CLASS=<class>` on distill failure. A recorded main-health repair marker covering the same `MAIN_FAILED_RUN_ID` / base SHA lets Step 8+ merge without waiting for default-branch green; new or different default-branch failures block and route here. The subagent CI-fix loop is specified below after the rebase fence.
- **`postmerge-repair`**: Load only `${CLAUDE_PLUGIN_ROOT}/skills/implement/references/postmerge-emergency-repair.md`. This path handles `postmerge-main-ci-fail` after the merged-SHA push watch and must not fall through to generic `ci-fix`.

```bash
"$HOME/.cache/larch/sessions/implement-run-$PPID.sh" scripts/larch.sh ship pre-fix-rebase --implement-tmpdir "$IMPLEMENT_TMPDIR"
```

When `NEXT_ACTION=continue`, first verify that `.ship-route-exit-handoff.env` does not have `PRE_FIX_REBASE_REQUIRED=true` without a regular, non-symlink `$IMPLEMENT_TMPDIR/.ship-pre-fix-rebase-ok`; if the proof is missing, continue to Step 16 with `STALL_TRACKING`, then Step 18. Then run the subagent CI-fix loop specified below.

The Step 8 `ci-fix` route is a CI-fixer subagent round loop. The main agent never reads `CI_ERRORS_FILE`, never runs `gh run`, and never edits repository files on this path; its only evidence is the handoff KVs (`FAILED_JOBS_COUNT`, `CI_ERRORS_DISTILL_CLASS`) and the subagent's three `FIXER_*` result lines.

**Evidence escalation.** If `CI_ERRORS_FILE` is empty or missing, re-run `"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" ci distill-log --run-id "$FAILED_RUN_ID" --repo <owner/name> --output "$IMPLEMENT_TMPDIR/ci-errors-$FAILED_RUN_ID.md"` once (run the command; do not read its output). If it fails again, route operator-bail with reason `ci-evidence-unavailable`.

**Round loop.** Keep the `$IMPLEMENT_TMPDIR/main-agent-ci-fix.count` counter (name retained for reader continuity; it now counts fixer rounds), rounds 1 through 30. Exhaustion routes operator-bail with reason `ci-fix-exhausted`. The main agent appends each complete `FIXER_SUMMARY` value to `$IMPLEMENT_TMPDIR/ci-fixer-rounds.md`; it must begin `failure_signature=<value>`, so the fixer can detect a non-consecutive repeated failure. Pass that path on every spawn.

- **Round 1**: spawn the Agent tool with `subagent_type` `larch:ci-fixer`. Its prompt contains only the repository root, the working branch, the PR URL, the `CI_ERRORS_FILE` path, the rounds-file path, the round number, and the contract reminders from `agents/ci-fixer.md`. Do not pass `MODE`; the ci-fixer defaults to `MODE=ci`, which commits and pushes the fix, and passing `MODE=checks` returns `FIXER_RESULT=committed` instead of `pushed` and breaks the round-loop routing below. No log content is inlined.
- **Rounds 2..30**: continue the same subagent via `SendMessage` with `Round <N>. New digest: <CI_ERRORS_FILE path>.`. When `SendMessage` is unavailable in the session, spawn a fresh `larch:ci-fixer` per round instead (same gating pattern as `/review --subagent`); the rounds file carries the history either way.
- **Parse the final message's three `FIXER_*` lines only.**
  - `FIXER_RESULT=pushed`: relaunch `step-8-ship.sh` through the Step 8 bgjob start/wait pair; CI adjudicates the fix.
  - `FIXER_RESULT=committed`: the fixer committed without pushing (accidental `MODE=checks` spawn). Require a non-empty `FIXER_COMMIT` SHA, push it via `"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" push branch`, append `FIXER_SUMMARY` to the rounds file, then relaunch `step-8-ship.sh` through the Step 8 bgjob start/wait pair; CI adjudicates the fix. With an empty `FIXER_COMMIT`, treat as an unparseable final message and use the respawn path below.
  - `FIXER_RESULT=no-progress`: if the prior round was also `no-progress` with the same failure signature, route operator-bail with reason `ci-fix-no-progress`; otherwise relaunch `step-8-ship.sh` for a fresh digest and another round.
  - `FIXER_RESULT=bail` with `status=oscillation-detected` in `FIXER_SUMMARY`: route operator-bail with reason `ci-fix-oscillation`; do not respawn the fixer against the same loop.
  - Other `FIXER_RESULT=bail` values or an unparseable final message: give the subagent one fresh respawn; if that also fails, route operator-bail with the existing tool-failure contract.

**Salvage rule.** After every subagent return or death, run `git status --porcelain`. A dirty tree is committed as `CI fix round <N> salvage` and pushed via `"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" push branch`; CI adjudicates. Never reset away subagent work; never relaunch `step-8-ship.sh` with a dirty tree.

Any path that ends in ship re-entry uses the Step 8 bgjob start/wait pair, including exact `local-unfixable` routing via the Exit 3 table.
- **`conflict-fix`** (post-driver only): Read `RESUME_PHASE`, `CALLER_KIND`, and `CONFLICT_FILES` from `.ship-route-exit-handoff.env`. When `RESUME_PHASE=ship-pr-rrr-phase14` and `CALLER_KIND=ship_pr_pre_push`, **MANDATORY: READ ENTIRE FILE**: Read `${CLAUDE_PLUGIN_ROOT}/skills/implement/references/conflict-resolution.md` completely, then run conflict-resolution first (spawn `larch:ci-fixer` with `MODE=conflict`; parse `FIXER_RESULT=resolved|needs-operator|bail` only; never Read conflicted hunks in the main agent; attribution `MODE=subagent` / `TIER=subagent`). Otherwise treat it as a malformed handoff and continue to Step 16 with `STALL_TRACKING`, then Step 18.
- **`governance-refresh`** (post-driver only): a sole `stale-plan-base-scope` at the ship governance gate. Read `GOVERNANCE_REASONS`, `GOVERNANCE_RECEIPT_BASE_SHA`, and `GOVERNANCE_TARGET_BASE_SHA` from `.ship-route-exit-handoff.env`; anything other than exactly `GOVERNANCE_REASONS=stale-plan-base-scope` is a malformed handoff: continue to Step 16 with `STALL_TRACKING`, then Step 18. Re-run the Preflight item 6 bounded read-only semantic-materiality probe against the base target tree at `GOVERNANCE_TARGET_BASE_SHA` (`git show <sha>:<path>`), never the feature branch. On clear staleness, route **operator-bail** with the handoff's `NEEDS_USER_REASON`; do not post a stale notice or rename the issue. When current, run the refresh fence and branch on its stdout `NEXT_ACTION`: `reship` relaunches `step-8-ship.sh` through the Step 8 bgjob start/wait pair (no `ship pre-fix-rebase`; the driver rebases itself); `operator-bail` follows the operator-bail branch with the fence's `DETAIL`. A non-zero exit or missing `NEXT_ACTION` is a Tool Failure.

```bash
"$HOME/.cache/larch/sessions/implement-run-$PPID.sh" scripts/larch.sh ship governance-refresh --implement-tmpdir "$IMPLEMENT_TMPDIR"
```

- **`operator-bail`**: follow the reason-specific contract in `ship-pr-exit-matrix.md` after its required ledger recording. For `NEEDS_USER_REASON=invariant-violation-unresolved` (Step 8 fix-ladder hard stop after the coder and main-agent fixes both left an invariant `violation`), HARD STOP: there is no waiver and no override — follow the existing Step 12d bail flow without creating or merging the PR. For `NEEDS_USER_REASON=architectural-guideline-deviation-unresolved` (a guideline `deviation` reached the ship gate without the documented-exception block the fix ladder records), re-run the assessments route so the fix ladder either fixes the deviation or records the `Exception:` block; do not proceed to PR without a valid exception block. For any approved manual recovery, do not start Steps 16, 16a, 17, or 18 until `ship reconcile-manual-merge` returns verified `RECONCILE_STATUS=ok`, including the bail-overlay post-read. Then set in-memory `STALL_TRACKING=false` and pass `--stall-tracking-memory false`. Publish corrected post-merge run-log records only through a normal reviewable repair PR. Never commit them to the already-merged implementation PR.
- **`stall`** (post-driver only): continue to Step 16 with `STALL_TRACKING`, then Step 18. Do not reuse pre-driver stall bullets.
- **`tool-failure`**: append Tool Failures and stop hard. Do not run Step 18 stall rename.

**OOS checkpoint fence.** After `NEXT_ACTION=oos-pipeline`, complete security-sidecar private disposition when applicable, then invoke the checkpoint wrapper. **MANDATORY: READ ENTIRE FILE**: Read `${CLAUDE_PLUGIN_ROOT}/skills/implement/references/ship-pr-oos-checkpoint-router.md` completely before invoking the fence. Parse stdout for `NEXT_ACTION=`. Halt with Tool Failures only when `NEXT_ACTION` is missing after invoke. Do not halt merely because rc is non-zero when stdout contains `NEXT_ACTION=`.

```bash
"$HOME/.cache/larch/sessions/implement-run-$PPID.sh" skills/implement/scripts/step-8-oos-checkpoint.sh
```

- **`NEXT_ACTION=reship`**: re-invoke ship through the Step 8 bgjob start/wait pair with the same `RESUME_PHASE` carve-out. Do not sleep.
- **`NEXT_ACTION=stall`** (OOS-checkpoint stall): halt Step 8+ until resolved. Do not write stats, do not clear `OOS_PENDING=false`, and do not route to the post-driver Step 16 stall path.

When `ship-pr-exit-matrix.md` requires tracking metadata projection refresh, run this fence; skip it when `ISSUE_NUMBER` is empty or `0`.

```bash
"$HOME/.cache/larch/sessions/implement-run-$PPID.sh" scripts/larch.sh execution-issues refresh --implement-tmpdir "$IMPLEMENT_TMPDIR" --best-effort
```

> **Continue to Step 15.** The active Rust ship driver owns this transition after postmerge cleanup.

> **Continue to Step 16.** Do NOT stop after PR creation, merge, local cleanup, or teardown output. `ship-pr` reaching `PHASE=done` is not the run end; Steps 16, 18, and 19 still own rejected-findings replay and final token/timing caps.

<!-- step:16 — Rejected Code Review Findings Report -->

Print: `> **🔶 /implement 16: rejected findings**`

`implement step-16-17` reads the compose-time durable architectural-guidelines note only when it is already current for `HEAD`. It performs no semantic reassessment.

Report unimplemented code review suggestions without reprinting the full findings inline.

**Recover-then-report contract (issue #5011).** Steps 16, 16a, and 17 prepare the final report on the green terminal path and after successful stall recovery re-enters the normal sequence. Stall paths and Step 12d bails set `STALL_TRACKING=true` and **skip to Step 18** so Step 18a recovery runs first. Terminal marker emission happens only after Step 18 warnings and publication plus Step 19 teardown and tail relay complete. The final report renders exactly once at terminal text position. This avoids premature `— stalled` reports and duplicate renders.

> **Continue to Step 16a.** The composed wrapper handles this transition; do NOT end the turn after rejected findings.

<!-- step:16a — Slack Issue Announce -->

Print: `> **🔶 /implement 16a: notify**`

> **Continue to Step 17.** The composed wrapper handles this transition; do NOT end the turn after Slack notification.

<!-- step:17 — Final Report -->

Print: `> **🔶 /implement 17: final report**`

Run the composed wrapper for rejected findings, best-effort Slack notification, and terminal `larch:final-summary` projection. Do not branch around it on early bailouts that still have a tracking issue. On terminal stalls that skip here via recover-then-report, `scripts/larch.sh final-report step18b` runs Step 16/16a side effects before emitting the final body.

```bash
"$HOME/.cache/larch/sessions/implement-run-$PPID.sh" scripts/larch.sh implement step-16-17 --implement-tmpdir "$IMPLEMENT_TMPDIR"
```

The markdown body comes from the Rust `render run-summary` owner rendered in process by `final-report write`; optional per-lane USD comes from `larch_core::report::RATE_TABLE`. The dollar-primary cost line lives in the `larch:final-summary` block written to `summary-final.md` by `final-report write` without `--print-stdout` on the active `scripts/larch.sh implement step-16-17` path.

After the combined Step 16-17 fence returns, follow `${CLAUDE_PLUGIN_ROOT}/skills/shared/final-summary-emit.md` marker-first profile to cache, not emit. Binding: markers `---LARCH-SUMMARY-FINAL-BEGIN---` / `---LARCH-SUMMARY-FINAL-END---`; source captured foreground `scripts/larch.sh implement step-16-17` Bash wrapper stdout already in context, not asynchronous notification output; in-context-only `true`; Read fallback `forbidden`; sidecar follow-on `forbidden`. When the shared profile caches a non-empty marker body, retain it as the Step 17 cache for deferred terminal emit. If markers are absent or body empty, cache no Step 17 body. Continue to Step 18 so Step 18b can decide via `EMIT_BODY`.

Internal Step 16, Slack, and Step 17 failures are logged inside the composed wrapper and `scripts/larch.sh implement step-17`; the outer fence still continues to Step 18. Stale-summary guard: absent markers after failed Step 17 render are expected even when an old `summary-final.md` remains. do not Read that file on the Step 17 primary path. Marker emission is gated on captured Step 17 render success and a non-empty `summary-final.md`, not `summary-final.md` presence alone.

Step 18 status KVs and optional final summary body use branch-qualified sources. Green path (`NEXT_ACTION=logs-flush-done`): use captured composite stdout from `scripts/larch.sh implement step-18-gate-logs-flush`. Stall-recovery breakout: use captured standalone logs-flush stdout from `step-18.sh --phase logs-flush`. Step 18b uses the same shared marker-first profile with `/implement` markers, Read fallback `forbidden`, and sidecar follow-on `forbidden`, but terminal emission waits until Step 19 restore, teardown, and tail relay finish. Use `EMIT_BODY` and `WFR_RC` for refreshed-body precedence and missing-marker warnings, not direct `summary-final.md` emission. Closing token/timing data enters the terminal archive.

> **Continue to Step 18.** Do NOT end the turn after caching the final report.

<!-- step:18 — Stall Recovery, Logs Flush, and Final Warnings -->

Print: `> **🔶 /implement 18: logs flush**`

**MANDATORY: READ ENTIRE FILE**: Read `${CLAUDE_PLUGIN_ROOT}/skills/implement/references/step18-logs-flush.md` completely.

### Step 18a — Stall recovery gate

Step 18a runs first on every Step 18 entry, before terminal snapshot preparation. Per recover-then-report, stall paths and Step 12d bails skip directly here, so recovery runs **before** the Step 16/17 final report. The composite fence reads stall layers, emits `STALL_TRACKING_*` plus `STALL_RECOVERY_REQUIRED`, runs `normalize-outcome`, evaluates green-path Step 18a.5 skips, and flushes logs internally when no prompt-side branch is needed. Do not create `current-implement-env-$PPID.sh`.

Bind `STEP17_EMITTED_FOR_STEP18` before the composite fence because the no-stall green path finalizes inside it. Use `true` only when a non-empty Step 17 marker body was cached for deferred terminal emit; otherwise use `false`. Do not set it merely because a stale `$IMPLEMENT_TMPDIR/.step17-emitted` exists without a current Step 17 cache.

```bash
"$HOME/.cache/larch/sessions/implement-run-$PPID.sh" scripts/larch.sh implement step-18-gate-logs-flush --implement-tmpdir "$IMPLEMENT_TMPDIR" --stall-tracking-memory "${STALL_TRACKING:-false}" --step17-emitted "${STEP17_EMITTED_FOR_STEP18:-false}"
```

Always retain captured composite stdout. Parse line-anchored `NEXT_ACTION`, `STALL_RECOVERY_REQUIRED`, four `STALL_TRACKING_*` KVs, `SESSION_TRANSCRIPT_STATUS`, `TERMINAL_SNAPSHOT_STATUS`, `RUN_LOG_FINAL_FLUSH_OK`, `RUN_LOG_PUBLISH_OK`, Step 18b markers, and status KVs from it even when rc is non-zero. Missing `NEXT_ACTION` is Tool Failure. A false final-flush or publication KV with a non-zero rc is terminal: report it, retain recovery material, and do not emit a success body or run Step 19.

Parse `STALL_RECOVERY_REQUIRED` and the four `STALL_TRACKING_*` KVs from captured composite stdout immediately after the composite fence returns. Branch primarily on `NEXT_ACTION=stall-recovery`; treat `STALL_RECOVERY_REQUIRED=true` as diagnostic confirmation. Layer interpretation lives in `step18-logs-flush.md`.

Branch by the composite `NEXT_ACTION`:

- **`logs-flush-done`**: parse `EMIT_BODY`, `WFR_RC`, `ERROR` (final-report render failure reason), `SESSION_TRANSCRIPT_STATUS`, `TERMINAL_SNAPSHOT_STATUS`, `RUN_LOG_FINAL_FLUSH_OK`, `RUN_LOG_PUBLISH_OK`, final summary markers, and status KVs from captured composite stdout. Cache the selected marker body, then continue to Step 19.
- **`logs-flush-failed`**: report the terminal snapshot or publication failure and stop with recovery material intact. Do not run Step 19.
- **`stall-recovery`**: **MANDATORY: READ ENTIRE FILE** `${CLAUDE_PLUGIN_ROOT}/skills/implement/references/stall-recovery.md`, then execute its 9-sub-step active-stall procedure. During active recovery before `CLEARED=true`, do not run the standalone `--phase logs-flush` fence. After successful recovery (`CLEARED=true`), run the standalone `step-18.sh --phase logs-flush` fence. Proceed without re-running `scripts/larch.sh implement step-18-gate-logs-flush` after terminal recovery completes.
- Missing `NEXT_ACTION`: treat as Tool Failure.

Step 18a surface under `${CLAUDE_PLUGIN_ROOT}`: `scripts/larch.sh stall-recovery` owns every command, including `file-report` and the contract lint. Other contracts: `docs/stall-recovery-report.md`, `scripts/resolve-upstream-larch-repo.sh`, `skills/implement/scripts/step-18.sh`, `skills/implement/scripts/step-18.md`, and the Step 18 tests in `crates/larch-cli/src/implement_terminal_commands.rs`. Terminal title-prefix handling happens in Step 19 cleanup.

**Escalation recording owners.** Prompt-side call sites record before Main Claude edits for Step 3 lint `main-agent-required`, Step 5 self-review lint `main-agent-required`, Step 5 `main-agent-vote-required`, Step 5 MAV/check lint `main-agent-required`, Step 6 lint `main-agent-required`, Step 8+ Rust ship-pr CI handoffs, Step 18a `step2-impl`, and Step 18a `step8-shippr` code-editing repairs, but only when the ship driver emitted `ledger_ready=true` or Main Claude is editing code. Pure reship such as `transient-infra` records nothing. Parse exact `LINT_FIX_LEDGER_*`, `STEP5_REVIEW_LEDGER_*`, and ship driver JSON `ledger_ready` / `ledger_site` / `ledger_trigger` / `ledger_step` / `ledger_phase` / `ledger_dispatcher` / `ledger_exit_code` / `ledger_failure_detail_log` fields. For each prompt-side call, pass the literal absolute `IMPLEMENT_TMPDIR` parsed from Step 0 bootstrap output; do not expand `$IMPLEMENT_TMPDIR` in a later Bash invocation. Do not duplicate records owned by `review-and-fix step5` or child scripts. Preserve protected-path and submodule warning strings before Main Claude edits or terminal no-recovery routing.

Anti-halt continuation: after `init-attempts`, continue to classify; after classify, continue to retry or terminal routing; after each dispatch, continue to retry accounting; after success or terminal failure on the recovery branch, continue to Step 18b. Do not recurse into Step 18 from recovery, call `ScheduleWakeup`, write `$IMPLEMENT_TMPDIR/session-env.sh`, mutate `$IMPLEMENT_TMPDIR/finalize-state.sh`, or spawn Agent-tool subagents for code-writing recovery.

### Step 18b — Final snapshot and publication

Repeat any external reviewer warnings from earlier from Step 5 review or runtime-fallback flips, e.g., `**⚠ Codex not available: <reason>**` or `**⚠ Cursor review failed: <reason>**`. See `step18-logs-flush.md` for mode-specific warning and snapshot behavior.

Use the standalone logs-flush fence only on the stall-recovery breakout path.

```bash
"$HOME/.cache/larch/sessions/implement-run-$PPID.sh" skills/implement/scripts/step-18.sh --phase logs-flush --step17-emitted "${STEP17_EMITTED_FOR_STEP18:-false}"
```

On the green path (`NEXT_ACTION=logs-flush-done`), parse captured composite stdout only. On stall recovery, parse standalone logs-flush stdout only. Follow `${CLAUDE_PLUGIN_ROOT}/skills/shared/final-summary-emit.md` marker-first profile to cache any Step 18 body only after terminalization succeeds. Binding: markers `---LARCH-SUMMARY-FINAL-BEGIN---` / `---LARCH-SUMMARY-FINAL-END---`; source captured foreground `scripts/larch.sh implement step-18-gate-logs-flush` stdout on the green path, or captured foreground `step-18.sh --phase logs-flush` stdout on breakout; not asynchronous notification output; in-context-only `true`; Read fallback `forbidden`; sidecar follow-on `forbidden`. When terminalization did not fail, `EMIT_BODY=true`, `WFR_RC=0`, and markers are absent or invalid, print `**⚠ Step 18: EMIT_BODY=true but marker pair missing from composite stdout.**` on the green path or `**⚠ Step 18: EMIT_BODY=true but marker pair missing from logs-flush stdout.**` on breakout.

`STEP17_EMITTED_PRESENT` is informational only. Terminal-emit precedence is: a valid non-empty Step 18 marker body wins when `EMIT_BODY=true` and `WFR_RC=0`; otherwise a non-empty Step 17 cache wins when `EMIT_BODY=false`; otherwise emit only the existing missing-marker warning. When `WFR_RC!=0`, rely on the Step 18 render-failure warning and do not emit a body or add recap prose. Cap the token and timing ledgers before terminal snapshot rendering. `RUN_LOG_FINAL_FLUSH_OK=true` and `RUN_LOG_PUBLISH_OK=true` are required before Step 19 for enabled, disabled, and suppressed terminal states.

> **Continue to Step 19.** Do not emit the cached final report yet.

<!-- step:19 — Cleanup -->

Print: `> **🔶 /implement 19: cleanup**` **MANDATORY: READ ENTIRE FILE**: Read `${CLAUDE_PLUGIN_ROOT}/skills/implement/references/step19-cleanup.md` completely.

Run cleanup only after Step 18 returned `logs-flush-done` or the standalone logs-flush fence returned zero with both success KVs. Run `"$HOME/.cache/larch/sessions/implement-run-$PPID.sh" skills/implement/scripts/step-19.sh --implement-tmpdir "$IMPLEMENT_TMPDIR"`. Always retain captured Step 19 stdout. A non-zero exit or `CLEANUP_BLOCKED=run-log-not-terminalized` is terminal and leaves recovery material intact.
Relay teardown tail records verbatim from captured Step 19 stdout. Tail records document the mechanical outcome through the `FINALIZE_*`, `RENAME_*`, `ISSUE_URL`, `STASH_REF`, and `SENTINEL_WRITTEN` KVs. If a user message interrupts after Step 18 returns, the in-context cached-body obligation survives into the next turn; finish Step 19 before emitting that body. Never Read or use a disk cache to rebuild it. Tail relay precedes terminal marker emit. The selected marker body is the final text with no following tool call.

<!-- larch:step19-teardown-tail-relay: Step 19 teardown tail relay is distinct from the Step 18 final report source. -->
