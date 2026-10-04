# Final Summary Emit Contract

Shared orchestrator-side contract for publishing `final-summary.md` bodies to top chat. Call sites supply the profile, the source binding when the profile needs one, the after-action, and any cleanup that must run before terminal emission.

## Shared rules

- Emit only the body as plain orchestrator chat markdown.
- At the terminal placement point, emit its full body verbatim as plain chat markdown.
- The emitted final-summary body must be the final assistant text message of the turn; no tool call, warning relay, cleanup, footer, recap, or other prose may follow it.
- When a `Read` path is used, write the Read result directly into an in-context cache. Only the terminal placement rule authorizes plain-chat emission.
- Never use Bash, Python, or another tool call to extract or print the final-summary body.
- Do NOT paraphrase, summarize, reorder, or add prose between bullets.
- Do NOT condense, collapse, or omit any part of the body (including `### Round N reviewer timing` ASCII Gantt blocks). Do NOT wrap any section in `<details>` or equivalent HTML.
- A `/design` final summary can begin with `## Review Phase Detail` before its later `## /design run ...` block. Start terminal emission at byte 1 of the cached file; `Review Phase Detail` is required output, not optional context.
- Never begin a `/design` terminal emission at `## /design run ...`, even when that block looks like the structured summary. Doing so omits the preceding review timing and violates this contract.
- Do not add prose around the block.
- Do not add post-emit recap prose, artifact bullet recaps, or parenthetical cost paraphrases such as approximate no-cost restatements.
- Preserve the full structured block, including title, mode, duration, cost line with per-agent breakdown, tokens, and bullets.
- The caller supplies the profile, source binding when applicable, after-action, and all pre-terminal cleanup or footer work.

## Caller profile parameters

Callers that use the marker-first or `/design` Read-always readiness profile must bind these values at the call site:

- begin marker token
- end marker token
- source description: task-output, wrapper stdout, or bgjob `DONE` stdout plus result env
- whether extraction is in-context-only, including any required bgjob result-env read
- Read policy: marker fallback `allowed` with a named path, marker fallback `forbidden`, or `/design` required Read from `FINAL_SUMMARY_PATH`
- sidecar follow-on policy: `allowed` via `REPORT_GATE_SIDECARS_FILE`, or `forbidden`
- after-action

## Deferred emit procedure

Use this procedure when any cleanup, warning replay, operator line, footer, sentinel write, or tail relay must happen after the source is still available:

1. Read/cache or extract/cache the final-summary body from the call-site source while that source is still available.
2. If sidecar follow-on is allowed, Read/cache allowed sidecar bodies while their files still exist.
3. Do not emit the cached body yet.
4. Run all required cleanup, warning replay, operator lines, sentinels, footers, teardown, and tail relay.
5. Choose the terminal body by the call-site precedence rules.
6. Emit the selected cached final-summary body, followed immediately by any cached sidecar bodies, as the sole terminal plain-chat output. No tool call or recap may follow.

If a user message interrupts after finalize returns but before terminal emission, carry the in-context cached-body obligation into the next turn. Emit the cached body verbatim as the first text, before answering the intervening message. Do not use Read or a disk cache to reconstruct a lost body. If the in-context cache is unavailable, point to the tracking-issue comment and stop.

## `/design` Read-always readiness profile

Use this profile for `/design` final `bgjob wait` `DONE` stdout and the matching bgjob result env.

1. Parse `FINAL_SUMMARY_PATH=<path>` from final `bgjob wait` `DONE` stdout already in the orchestrator context window, or from the matching `$DESIGN_TMPDIR/bgjob/<step>.result.env` after `BGJOB_RC=0` and required-KV validation.
2. Confirm readiness from the same DONE stdout or matching result env: either whole-line `LARCH_FINAL_SUMMARY_BEGIN` and `LARCH_FINAL_SUMMARY_END` markers (marker body expected empty), or `FINAL_SUMMARY_READY=true`. Treat either form as a readiness signal only. Bgjob merge/result envs surface the KV form because contract-stream marker lines are not merged into DONE stdout.
3. Do not extract or emit summary bodies from marker pairs on `/design` paths.
4. When `FINAL_SUMMARY_PATH` is non-empty and the path names a non-empty file, use the Read tool on that path and cache the full file body verbatim, including all subsections such as `### Round N reviewer timing` ASCII bar charts and the `**Top reviewers**` list. If the file begins with `## Review Phase Detail`, retain that heading and every following byte through the later `## /design run ...` block. Do NOT collapse, wrap in `<details>`, omit any part of the file body, or start terminal emission at the later run-summary heading.
5. The Read/cache may happen before cleanup, Step 6, cancellation routing, partition routing, warning replay, or footer text. Plain-chat emission must wait until the terminal placement point after those required actions.
6. Do not re-read task-output files, stdout captures, unrelated result env files, or tmpdir logs to recover markers. Do not re-read those files to recover summary bodies. The only result-env read is the caller's required bgjob result env used for `BGJOB_RC=0` and `FINAL_SUMMARY_PATH` validation.
7. Do not scrape markers via Bash or Python.
8. Only when the caller sidecar policy is `allowed`, and the caller source or matching result env includes non-empty `REPORT_GATE_SIDECARS_FILE=<path>`, Read/cache that file before cleanup and emit its full body verbatim with the terminal final-summary body. When the caller sidecar policy is `forbidden`, skip sidecar follow-on entirely.

## Marker-first profile

Use this profile when the caller names a source that can emit markers with a non-empty body. `/implement` binds captured foreground Bash wrapper stdout, not asynchronous notification output.

1. Locate the first balanced whole-line caller begin/end marker pair in the caller-named source already in the orchestrator context window.
2. Extract/cache the marker body — including all subsections such as `### Round N reviewer timing` ASCII bar charts and the `**Top reviewers**` list. Do NOT collapse, wrap in `<details>`, or omit any part of the marker body.
3. Plain-chat emission happens only at the terminal placement point after caller-required cleanup, warning replay, sentinel, teardown, and tail-relay work.
4. Do not re-read task-output files, stdout captures, result env files, or tmpdir logs to recover markers.
5. Do not scrape markers via Bash or Python.
6. Only when steps 1–2 yield no valid marker body and the caller Read fallback policy is `allowed`, Read/cache the caller-named fallback path when non-empty. When the caller Read fallback policy is `forbidden`, skip Read fallback entirely.
7. Only when the caller sidecar policy is `allowed`, and the caller source includes non-empty `REPORT_GATE_SIDECARS_FILE=<path>`, Read/cache that file and emit its full body verbatim with the terminal final-summary body. When the caller sidecar policy is `forbidden`, skip sidecar follow-on entirely.

## Callsite bindings

| Call site | Markers | Source | In-context-only | Read fallback | Sidecar follow-on | After-action |
| --- | --- | --- | --- | --- | --- | --- |
| `/design` Read-always readiness | `LARCH_FINAL_SUMMARY_BEGIN` / `LARCH_FINAL_SUMMARY_END` readiness only (empty body), or `FINAL_SUMMARY_READY=true` in DONE/result env | final `bgjob wait` `DONE` stdout plus matching `$DESIGN_TMPDIR/bgjob/<step>.result.env` after `BGJOB_RC=0` and required-KV validation | `true` after the caller's required result-env read | required Read/cache of parsed `FINAL_SUMMARY_PATH=<path>` when non-empty | `allowed` via `REPORT_GATE_SIDECARS_FILE`; Read/cache before cleanup | caller-specific continuation, then terminal emit |
| `/implement` Step 17 marker-first | `---LARCH-SUMMARY-FINAL-BEGIN---` / `---LARCH-SUMMARY-FINAL-END---` | captured foreground `scripts/larch.sh implement step-16-17` Bash wrapper stdout | `true` | `forbidden` | `forbidden` | cache marker body; wrapper writes `.step17-emitted` via `--step17-emitted true` before teardown when a body is pending |
| `/implement` Step 18b marker-first | `---LARCH-SUMMARY-FINAL-BEGIN---` / `---LARCH-SUMMARY-FINAL-END---` | green path: captured foreground `scripts/larch.sh implement step-18-gate-logs-flush` stdout when `NEXT_ACTION=logs-flush-done`; non-green path: captured foreground `step-18.sh --phase logs-flush` stdout on stall-recovery and escalation-filing branches | `true` | `forbidden` | `forbidden` | cache through Step 19; do not write `.step17-emitted` after logs-flush returns |

## `/implement` terminal-emit precedence

This subsection is authoritative for orchestrator chat emit on `/implement`.

1. After Step 18 warnings, closing marks, and publication plus Step 19 restore, teardown, and tail relay complete, choose exactly one body for terminal plain-chat emit.
2. **Precedence A — refreshed Step 18 body:** when captured composite stdout (`NEXT_ACTION=logs-flush-done`) or captured standalone logs-flush stdout has `EMIT_BODY=true`, `WFR_RC=0`, and a valid marker pair with non-empty body, terminal emit must use that post-Step-18b marker body even if a Step 17 cache exists.
3. **Precedence B — Step 17 cache:** when Precedence A does not apply and a non-empty Step 17 marker body was cached during Step 17, terminal emit uses the Step 17 cache.
4. **Precedence C — missing body:** when neither applies, emit only the existing missing-marker warning; do not Read `summary-final.md` after teardown.
5. **Precedence D — render failure (`WFR_RC!=0`):** when `WFR_RC` is non-zero, the final report render failed (summary write, manifest reconcile, tracking upsert, or a composition exception) and no body is available. The finalize wrapper has already printed `**⚠ Step 18: final report render failed (WFR_RC=<n>): <reason>**` (reason from the `ERROR=` KV) to composite stderr; treat that warning as the accounted-for outcome. Do not emit a body, do not Read `summary-final.md`, and do not add free-form recap prose. This closes the #6979 silence gap where an rc-failure path (`EMIT_BODY=false`) printed nothing because the missing-marker warning was gated on `EMIT_BODY=true` and `WFR_RC=0`.
6. `STEP17_EMITTED_PRESENT` and `--step17-emitted true` are sentinel/suppression inputs for Step 18b refresh logic (`should_emit_updated_body`), not overrides of orchestrator terminal-emit precedence.

## File-only profile

Use this profile when the caller has no source path.

1. Skip marker extraction entirely; do not scan prior tool output for markers.
2. When `[ -s "${FINAL_SUMMARY_PATH:-$DESIGN_TMPDIR/final-summary.md}" ]`, Read/cache that file and defer emission until the terminal placement point.
3. No `REPORT_GATE_SIDECARS_FILE` follow-on unless a caller explicitly names a sidecar source outside this profile.

## Update Triggers

Update this file when final-summary marker names, `FINAL_SUMMARY_READY` readiness KVs, bgjob `DONE` stdout or result-env source bindings, Read fallback policy, sidecar policy, preamble wording, post-emit recap/no-cost paraphrase rules, orchestrator-text emit rules, or terminal-emit precedence changes.
