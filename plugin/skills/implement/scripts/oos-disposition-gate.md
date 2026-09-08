# oos disposition-gate

Rust-owned mechanical gate invoked from `/implement` Step 8+ after the Step 9a.1 filing pass finishes and **before** `OOS_PENDING` is cleared. It prevents silent loss of voted-in, non-security OOS items: accepted `### OOS_` blocks must produce at least one filed GitHub issue URL for the run (possibly one combined issue), have enough `Inline-triage rule N:` occurrences in commit messages on the supplied `--commit-range`, or appear under an explicit `oos-issues` NDJSON Rejected sub-block with enough distinct structured `OOS_<n>` markers (see Counting rules).

## Ownership trace

`skills/implement/scripts/oos-disposition-gate.sh` is a thin compatibility wrapper. It selects the active plugin root and executes `${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh oos disposition-gate`. `crates/larch-cli/src/main.rs` dispatches that verb to `crates/larch-cli/src/oos_commands.rs`; `crates/larch-core/src/issue/oos_disposition.rs` and `crates/larch-core/src/issue/oos_record.rs` own its counters, state, and block grammar.

## Invocation

```text
scripts/larch.sh oos disposition-gate [--fork-mode] [--repo-unavailable] \
  --accepted-files CSV \
  (--filed-urls-file PATH | --filed-urls-strict-file PATH)... \
  [--oos-issues-ndjson PATH] --commit-range RANGE
```

Legacy direct-wrapper invocation accepts the same arguments:

```text
oos-disposition-gate.sh [arguments...]
```

- `--accepted-files` — Non-empty comma-separated list of Markdown paths (typically `$IMPLEMENT_TMPDIR/oos-accepted-main-agent.md`, `oos-accepted-design.md`, `oos-accepted-review.md`). Missing paths are ignored; an aggregate containing no non-security OOS blocks passes. Outside the two skip modes, at least one loose or strict filed-URL input is required even when NDJSON is supplied.
- `--filed-urls-file` — Repeatable **loose** counter. Each path is a Step 9a.1 sentinel, `/design` `oos-issues-created.md`, or any sidecar where issue URLs appear as plain tokens (the Rust scanner reads the whole file). De-duplicated `https://…/issues/<n>` tokens are counted across the **union** of all `--filed-urls-file` arguments and **unioned** with URL tokens from `--oos-issues-ndjson` when that path is supplied.
- `--filed-urls-strict-file` — Repeatable **strict** counter. Counts only lines matching a dedicated `- **Filed URL**` markdown list item with optional whitespace before the colon whose value is an `https://.../issues/<n>` token. Unlike the loose counter, this structured-field regex does not restrict the host to `github.com` or `GH_HOST`. Incidental issue URLs elsewhere in the file (for example inside a reviewer `**Description**`) are ignored. Unique strict URLs are de-duplicated across all strict-file arguments. The gate’s `filed_urls` total is **`count_filed_urls_union_files(loose…, ndjson)` + `count_filed_urls_strict_files(strict…)`** (double-counting a URL that appears in both a loose and a strict input is allowed — the pass criterion is disjunctive; see Exit codes, not `filed_urls >= non_security_oos`).
- `--oos-issues-ndjson` — Optional path to the staged `oos-issues.ndjson` batch for the run. When present, unique issue URLs from this file participate in the filed-URL count, and rejected-sub-block bodies contribute `rejected_oos_markers` (see below).
- `--commit-range` — Git revision range passed to `git log` (e.g. `$(git merge-base HEAD origin/main)..HEAD`, or `origin/main..HEAD` when merge-base is empty but `origin/main` resolves). Used only when the gate is not skipped.
- `--fork-mode` / `--repo-unavailable` — When either is set, the gate **exits 0 immediately** (no file reads, no `git log`).

## Exit codes

| Code | Meaning |
|------|--------|
| 0 | Skipped (fork / repo-unavailable), or nothing to check (`non_security_oos == 0`), or disposition satisfied: **`filed_urls > 0`** *or* **`inline_triage_lines >= non_security_oos`** *or* **`rejected_oos_markers >= non_security_oos`** (implemented by Rust `DispositionCounters::cleared`; the first branch is *not* `filed_urls >= non_security_oos`). |
| 1 | Disposition gap: `non_security_oos > 0` and **`filed_urls == 0`** (the Rust counter sums loose and strict URL counts) and `inline < non_security_oos` and `rejected_oos_markers < non_security_oos`. |
| 2 | Bad arguments, invalid `commit-range`, not inside a git work tree when a scan is required, an `--accepted-files` path exists but is not a regular file, or `oos-issues.ndjson` lists filed issue URLs while no CSV path resolves to a regular file (misconfiguration). |

## Counting rules

- **non_security_oos**: parsed by `larch_core::issue::oos_record::count_non_security_blocks` across all accepted files. It counts `### OOS_` blocks, plus legacy tagged `### FINDING_N: [OUT_OF_SCOPE]` headers where the `[OUT_OF_SCOPE]` literal is required for `FINDING` headers. Bare `### FINDING_N:` stays uncounted. Coder-skipped OOS is normalized to canonical `### OOS_` at append time in `review-and-fix CLI`, so the legacy match is defense-in-depth; #3550. Counted blocks must not contain a dedicated `- **focus-area**:` field line whose value begins with `security`. Security-routed entries are excluded. Prose such as `focus-area = security` inside a `**Description**` line does **not** mark a block as security-routed.
- **filed_urls** — Sum of (a) unique `https://…/issues/<digits>` substrings from the loose union of every `--filed-urls-file` path plus `--oos-issues-ndjson` (when provided), and (b) unique URLs read only from `- **Filed URL**` field lines (optional whitespace before `:`) in every `--filed-urls-strict-file` path. `larch_core::issue::oos_disposition` owns both counters.
- **inline** — Count of literal `Inline-triage rule` substring occurrences in commit messages returned by the Rust Git adapter for `RANGE`. Multiple occurrences on one line count separately; there is no strict per-OOS index linkage.
- **rejected_oos_markers** — Count of distinct `OOS_<digits>` tokens in `--oos-issues-ndjson` record bodies under a Rejected section (`## Rejected` or text containing `Rejected / Out-of-Scope`). The section ends at the next second-level heading. Repeated markers across records count once.

## Consumer

The Rust `oos disposition-checkpoint` verb composes the same `gate_counters` evaluator in process; it does not spawn this wrapper or a second command. The Rust #7681 Step 8 router invokes that checkpoint through `scripts/larch.sh`, then owns exit-code mapping, a de-duplicated Tool Failures fallback when the command could not record one, and post-pass bookkeeping. Orchestrator readers should use `oos-disposition-checkpoint.md` for the checkpoint exit contract and logging sites. After checkpoint exit **0**, `run-statistics`, `OOS_PENDING=false`, and re-invocation of `step-8-ship.sh` without resume-phase remain router-owned per `skills/implement/SKILL.md`; on checkpoint non-zero, the router must not perform those post-pass steps.

## Test authority

- **Behavioral authority**: unit tests in `crates/larch-cli/src/oos_commands.rs` cover command parsing, gate input validation, Git history, and checkpoint resolution. Unit tests in `crates/larch-core/src/issue/oos_disposition.rs` and `crates/larch-core/src/issue/oos_record.rs` cover counters, state, URL evidence, rejected markers, inline-triage evidence, and the block grammar.
- **Delegation smoke**: `skills/implement/scripts/test-oos-disposition-gate.sh` covers only thin-wrapper plugin-root selection, CLI routing, argv forwarding, exit-status forwarding, and stdout/stderr passthrough for both `oos-disposition-gate.sh` and `oos-disposition-checkpoint.sh`.

Focused commands:

```text
cargo test --locked --package larch-cli --bin larch oos_commands::tests
cargo test --locked --package larch-core --lib issue::oos_disposition::tests
cargo test --locked --package larch-core --lib issue::oos_record::tests
make oos-disposition-gate-bash-harness
```
