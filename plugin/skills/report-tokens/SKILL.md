---

# larch-run-lifecycle: shared-v1 skill=report-tokens
name: report-tokens
description: "Use when analyzing token costs from synchronized larch run logs for `--skill=design|implement|debate`: price token reports, optionally plot trends, and print cost-reduction suggestions."
allowed-tools: Bash, Read
---

**MANDATORY: Follow the complete shared lifecycle contract in `${CLAUDE_PLUGIN_ROOT}/skills/shared/run-lifecycle.md` with declared skill `report-tokens`.**

# Report Tokens

**MANDATORY: READ ENTIRE FILE before composing user-facing prose: `${CLAUDE_PLUGIN_ROOT}/skills/shared/readability-style.md`.**

Analyze token costs from synchronized larch run logs for the selected skill (`--skill=design|implement|debate`) in the current Git repository. The CLI syncs once, scans the unpacked cache, reads the skill-specific token report JSON files, prices each run through `larch_core::report`, prints a markdown analysis, writes a durable NDJSON cache snapshot, renders the trend chart, and optionally posts a GitHub `[Implement Analysis Report]`, `[Design Analysis Report]`, or `[Debate Analysis Report]` issue.

For `--skill=implement`, reports carry no workflow dimension and graph/per-day trend output aggregates all runs into one `All runs` series/table set. For `--skill=design` and `--skill=debate`, one aggregate report is generated. Debate run logs may record no vendor token legs; the report prices what is present and otherwise surfaces the existing empty-report gap. The filed issue intentionally omits raw per-issue JSON and actual-spend reconciliation unless `LARCH_REPORT_TOKENS_POST_ACTUAL_SPEND=1` is set.

Rate overrides: set environment variables documented in `docs/configuration-and-permissions.md` before invoking.

## Flags

Pass any of these after the skill name (for example, `/report-tokens --skill implement --no-issue`):

- `--skill <name>` (**required**): `design`, `implement`, or `debate`. Enum-validate before invoking the CLI; pass through to the module.
- `--no-issue` — skip posting the analysis report GitHub issue. `LARCH_REPORT_TOKENS_NO_ISSUE=1` has the same effect.
- `--no-plot` — skip chart generation; text analysis is still printed. `LARCH_REPORT_TOKENS_NO_PLOT=1` has the same effect.
- `--run-id <ID>` — flag reference: `${CLAUDE_PLUGIN_ROOT}/skills/shared/run-id-flag.md`.

<!-- step:1 — Run analysis -->

Parse and validate `--skill` first. Reject missing or out-of-enum values (`design`, `implement`, `debate`) before calling the CLI. Parse any `--no-issue`, `--no-plot`, or `--run-id <ID>` flags. The `--run-id` flag is consumed by the orchestrator and NOT forwarded to the CLI. Then:

```bash
"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" report-tokens analyze --skill "<name>" --operator-invoked [FLAGS]
```

where `[FLAGS]` are only `--no-issue` and/or `--no-plot`; `--run-id` is never included. `--operator-invoked` authorizes the analysis-report issue write, because `/report-tokens` is a direct operator-requested command; omit it with `--no-issue`, which posts nothing.

Verify the CLI exited successfully. On a normal run, stdout includes `## Report Tokens Analysis` plus `Cache JSON: <path>`. If it exits non-zero, stop and surface the error; do not invent partial cost results.

The CLI renders the trend chart itself. Unless `--no-plot` was passed, stdout ends with `Plots written to:` and one absolute PNG path per line; report those paths to the operator. `Plot generation disabled.` or `No plots generated.` means there is no chart to report, which is not an error: the text analysis is complete either way.

Advertised `Cache JSON:` and plot paths remain on disk after CLI exit and expire through automatic SessionStart `cleanup run` age sweeps for `larch-*` paths, rather than growing without bound.

## NEVER

1. **NEVER treat dollar output as billing truth.** The CLI uses transparent default rates and prints them with the analysis because vendor pricing and model routing can drift outside larch's control.
2. **NEVER forward removed replot flags.** Re-run against the synchronized cache instead.
