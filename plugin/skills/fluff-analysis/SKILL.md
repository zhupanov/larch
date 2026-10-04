---

# larch-run-lifecycle: shared-v1 skill=fluff-analysis
name: fluff-analysis
description: "Use when analyzing review fluff in synchronized larch run logs: rejected, OOS, or accepted-low-value findings, plus tuning recommendations."
allowed-tools: Bash, Read
---

**MANDATORY: Follow the complete shared lifecycle contract in `${CLAUDE_PLUGIN_ROOT}/skills/shared/run-lifecycle.md` with declared skill `fluff-analysis`.**

# fluff-analysis

**MANDATORY: READ ENTIRE FILE before composing user-facing prose: `${CLAUDE_PLUGIN_ROOT}/skills/shared/readability-style.md`.**

Characterize review **fluff** from the synchronized run-log cache in the current repository. This includes suggestions that are *not accepted* (rejected or deferred to Out-of-Scope) or *accepted-but-low-value*. The analyzer normalizes every review finding (outcome, reviewer/voter severity, semantic tags), then prints a markdown report and recommendations for tightening the reviewer self-filter and judge (voter) instructions.

This is the standing tool behind the kind of analysis filed as a `[Analysis Report]` issue: re-run it as the corpus grows to track whether necessity-gate changes (see `skills/shared/review-acceptance-rubric.md`) move acceptance and findings-per-run.

## Usage

`/fluff-analysis [--include-in-progress] [--cutoff ISO8601] [--since-version X.Y.Z] [--min-group N] [--log-root DIR] [--out FILE]`

## Run the Analysis

Call the analyzer via the Bash tool, forwarding any flags, then relay its markdown stdout to the user:

```bash
"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" fluff-analysis analyze [flags]
```

Do not re-derive, re-tally, or re-format the analysis in the main agent — the command owns extraction, classification, and rendering.

Flags:

- `--include-in-progress`: also read in-progress `/design` session temp dirs under the session cache (a racy snapshot of unflushed runs). Off by default.
- `--cutoff ISO8601` — enable a pre/post comparison section split at this timestamp (e.g. the date a reviewer-instruction change landed). Omitted by default.
- `--since-version X.Y.Z` — enable a pre/post comparison split by `manifest.json.larch_version`; preferred for release-gated behavior changes.
- `--min-group N` — minimum findings for a semantic group to appear in the tables. Default: `20`.
- `--sessions-dir DIR` — session cache dir for `--include-in-progress`. Default: `~/.cache/larch/sessions`.
- `--inprogress-since ISO8601` — lower bound on in-progress session mtime (skips stale temp dirs).
- `--log-root DIR`: offline fixture corpus override. By default, sync the current repository cache.
- `--out FILE` — write the report to `FILE` instead of stdout.

On success, stdout begins with `# Review Fluff Analysis`. If the log root is missing the command exits non-zero with a diagnostic; surface it rather than inventing results.

## Implementation

The analyzer is Rust-owned: `crates/larch-core/src/fluff_analysis.rs` (extraction, the multi-label semantic classifier, acceptance aggregation, and markdown report rendering) behind the `crates/larch-cli/src/fluff_analysis_commands.rs` CLI shim, reached only through `scripts/larch.sh fluff-analysis analyze`. The inline command tests run through `make test-fluff-analysis`.

- `scripts/test-fluff-analysis-corpus.sh` (contract: `scripts/test-fluff-analysis-corpus.md`) — optional synchronized-cache smoke for post-version low-value acceptance.

## NEVER

1. **NEVER present the tag-group rates as exact classifications.** Keyword tags are directional; the severity and outcome cuts are exact. The report says so — keep that framing.
2. **NEVER treat a low in-scope acceptance rate as "worthless."** OOS-heavy groups (security, scope reductions) are valid-but-deferred, not fluff; only reject-heavy groups are true fluff.
