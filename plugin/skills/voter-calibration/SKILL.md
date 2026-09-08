---

# larch-run-lifecycle: shared-v1 skill=voter-calibration
name: voter-calibration
description: "Use when analyzing voter agreement, severity calibration, and chronic outliers from synchronized larch run logs. Diagnostic only; changes no thresholds or points."
allowed-tools: Bash, Read
---

**MANDATORY: Follow the complete shared lifecycle contract in `${CLAUDE_PLUGIN_ROOT}/skills/shared/run-lifecycle.md` with declared skill `voter-calibration`.**

# voter-calibration

**MANDATORY: READ ENTIRE FILE before composing user-facing prose: `${CLAUDE_PLUGIN_ROOT}/skills/shared/readability-style.md`.**

Analyze **voter agreement**, **YES-vote severity spread**, **severity calibration score**, and chronic outlier voters from the synchronized run-log cache.

The core analyzer measures agreement and severity calibration only. It reports voter-side calibration standing from panel self-agreement, not from realized outcomes, issue fate, or reverts. The optional `--realized-outcomes` flag appends a diagnostic-only ground-truth section based on filed OOS issue fate. Nothing here affects reviewer/proposer points, spawning, thresholds, token allocation, or live panel verdicts.

## Usage

`/voter-calibration [--log-root DIR] [--min-votes N] [--outlier-threshold R] [--high-severity-threshold R] [--era {all,pre,post}] [--era-since-date YYYY-MM-DD] [--realized-outcomes] [--repo OWNER/NAME] [--filed-issue-details-json PATH] [--out FILE]`

## Run the Analysis

Call the analyzer via the Bash tool, forwarding any flags, then relay its markdown stdout to the user:

```bash
"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" voter-calibration analyze [flags]
```

Do not re-tally or re-format the analysis in the main agent. The command owns extraction, shared agreement math, and rendering.

Flags:

- `--log-root DIR` - offline fixture corpus override. By default, sync the current repository cache.
- `--min-votes N` - minimum eligible votes before outlier flagging. Default: `20`.
- `--outlier-threshold R` - flag chronic outliers below this agreement rate. Default: `0.50`.
- `--high-severity-threshold R` - flag voters whose valid YES-vote severities exceed this high-severity rate. Default: `0.90`.
- `--era {all,pre,post}` - segment the corpus before and after the ground-truth incentive boundary.
- `--era-since-date YYYY-MM-DD` - override the boundary with UTC midnight on that date.
- `--realized-outcomes` - append the diagnostic-only `## Ground-truth Voter Calibration` section (or a skipped/degraded note) from filed OOS issue fate. The core report never depends on it.
- `--repo OWNER/NAME` - repository override for the realized-outcome fetch and the automatic era boundary; the default is gix-typed origin resolution of the current checkout.
- `--filed-issue-details-json PATH` - offline targeted-issue details for `--realized-outcomes` harnesses.
- `--out FILE` - write the report to `FILE` instead of stdout (prints `REPORT_FILE=...`).

On success, stdout begins with `# Voter Calibration Report`. A missing resolved log root exits `2` with a diagnostic; surface it rather than inventing results.

## Acceptance Readout

- **Default post-ship:** run `--era all` after incentive #5544 ships. Auto-boundary uses `closedAt` from one typed, repository-scoped GitHub issue read. Compare `High Rate` and `Calibration Score` in segmented `## Pre-incentive era` and `## Post-incentive era` sections. Each section contains `## Agreement Table` and `## Voter Severity Scoreboard`.
- **Override or pre-ship:** run `--era all --era-since-date YYYY-MM-DD` when the incentive is unshipped, auto-boundary degrades, or the operator wants a manual cutoff.

## Implementation

The analyzer is Rust-owned: `crates/larch-core/src/voter_calibration.rs` (TSV parsing, shared agreement and severity math, false-negative rates, and markdown rendering) behind the `crates/larch-cli/src/voter_calibration_commands.rs` CLI shim, reached only through `scripts/larch.sh voter-calibration analyze`. GitHub reads route through the typed service owner; no path shells out to `gh` or raw Git. The inline command tests run through `make test-voter-calibration`.
