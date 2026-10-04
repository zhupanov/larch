# scripts/external-tool-registry.sh - contract

## Purpose

The shell file is the repository-side golden for external-tool and
implementer-coder names. Production dispatch is Rust-owned. This contract also
records the final released child-process inventory after #7686.

## Consumers and parity

`scripts/test-external-tool-registry.sh` sources the shell file. It checks the
same ordered names against these Rust-owned commands:

- `scripts/larch.sh agent model-args`
- `scripts/larch.sh agent check-reviewers`
- `scripts/larch.sh agent collect-results`
- `scripts/larch.sh implement step2-dispatch`

Update the shell golden, Rust owners, and tests together when the taxonomy
changes.

## Related

`scripts/larch.sh agent run-external-agent` is NOT sourced from this registry and still does not validate `--tool` against it, per DECISION_1 of #1099. The human-facing log keeps the raw label, while the `.meta` `TOOL=` sidecar field is sanitized at write time through a label-safe allowlist (alphanumerics, `.`, `_`, `-`); disallowed bytes are translated to `_` (length-preserved), and an empty sanitized result falls back to `sanitized-empty`. See `crates/larch-core/src/vendor/external_agent.rs` for the full sanitization contract.

## Public API

- `LARCH_EXTERNAL_TOOLS`
- `LARCH_IMPLEMENTER_CODERS`
- `larch_is_external_tool`
- `larch_is_implementer_coder`
- `larch_external_tools_braced`
- `larch_implementer_coders_braced`

## Released child-process inventory

`crates/larch-core/src/process.rs` owns the normal product allowlist.
`ExternalProgram` has no arbitrary-executable variant. The
`ExternalProcessRunner` port and `crates/larch-adapters/src/process.rs` own the
normal spawn, clean environment, bounded output, timeout, cancellation,
termination, and reap path.

| Executable | Approved operation | Rust owner and approval |
| --- | --- | --- |
| `claude`, `codex`, `cursor` | Typed vendor probes, reviewers, implementers, fixers, and other declared agent roles. | `VendorProgram` and `crates/larch-core/src/vendor/`; true external products approved by #7687. |
| Verified Gitleaks path | Secret scanning through one checksum-pinned executable. | `ScannerProgram`; the scanner bootstrap verifies the selected path before launch. |
| `git` | Exact diff plus the closed installed-Git mutation and network operation set. | `GitCliOperation`, `crates/larch-adapters/src/git/`, and `docs/git-operation-inventory.md`; approved by #7671. |
| `gh` | Exactly `gh auth token --hostname github.com`. | `GitHubCliOperation::AuthToken` and `crates/larch-core/src/github_auth.rs`; approved by #7672. GitHub API work stays in the Octocrab adapter. |
| `lsof` | Check whether a validated Git index lock has a live holder. | `HostUtilityProgram::Lsof` and `crates/larch-adapters/src/process.rs`. |
| `ps`, `pgrep` | Capture process identity and enumerate validated descendants or process groups. | `HostUtilityProgram::{Ps,Pgrep}` and `crates/larch-adapters/src/process_identity.rs`. |
| `security` | Read the fixed Cursor keychain item on macOS. | `HostUtilityProgram::Security` and `crates/larch-adapters/src/vendor_auth.rs`. |
| `python3` | Probe an interpreter version for fixed read-only compatibility gates, or run fixed `-m pytest` verification for changed Python tests in a consumer repository. | `HostUtilityProgram::{Python3,Pytest}`. These cases do not execute larch code and do not make Python a larch install prerequisite. |
| `make` | Run a repository-defined analyze-bugs runtime verification target selected from changed paths. | `HostUtilityProgram::Make` and `crates/larch-cli/src/analyze_bugs_commands.rs`. |
| `pre-commit` | Run repository hooks over the changed-file selection. | `HostUtilityProgram::PreCommit` and `crates/larch-cli/src/checks_run_relevant_commands.rs`. |
| Validated `scripts/larch.sh` or `bin/larch` | Bootstrap a missing binary or self-check a release-matched binary. | `LarchProgram::{bootstrap,binary}`. Roots are absolute, lexically safe, and plugin-owned. The boundary was approved by #7670. |

### Reviewed direct-spawn seams

The `subprocess-via-runner` lint rejects direct Rust process construction
outside the shared adapter unless the call carries a reason-bearing
`lint-subprocess-via-runner` approval. The remaining production approvals are:

| Executable class | Bounded use | Owner |
| --- | --- | --- |
| Current `larch` executable and caller-supplied bgjob program | Start the detached bgjob supervisor and worker, then run the command that the caller explicitly passed to `bgjob start`. A bare program must resolve to an executable on `PATH`; a path-bearing program is checked when the worker spawns it. | `crates/larch-cli/src/bgjob_commands.rs`; this lifecycle cannot use a runner-owned child because the daemon must outlive its caller. |
| `cargo` | Read locked workspace metadata and stream changed-path Clippy output. | `crates/larch-cli/src/checks_rust_clippy_commands.rs` and `crates/larch-cli/src/ci_selection.rs`; source-checkout validation only. |
| Validated `scripts/larch.sh` or `larch` path | Compose nested design and run-log commands, inspect a candidate bundle version, and smoke a staged release executable. | Design command modules, `crates/larch-cli/src/ci_policy_candidate_commands.rs`, and `crates/larch-cli/src/release_assets.rs`. |
| `bash` | Encode Bash `%q` compatibility output and run fixed shipped design scripts. | `crates/larch-cli/src/design_step0_commands.rs` and `crates/larch-cli/src/plan_review_commands.rs`. |
| `which` | Probe only an already selected vendor executable name. | `crates/larch-cli/src/plan_quality_revise_commands.rs`. |
| Validated consumer-repository script | Run `--help` and a declared dry-run or validation hook during plan-command review. | `crates/larch-cli/src/plan_quality_commands.rs`; paths must resolve inside the consumer repository or validated plugin root. |
| Explicit test or operator override | Run deterministic reviewer, scout, diagram, or plan-revision harnesses. | The owning command module validates the supplied path. Each direct call has a reason-bearing lint approval. |
| Fixed stall-recovery helper scripts | Resolve the upstream larch repository and file a bounded cross-repository failure report. | `crates/larch-cli/src/stall_recovery_reporting.rs`; helper paths come from the validated plugin root. |
| `git` in `larch-lint` | Discover the repository and list tracked paths before product crates are available. | `crates/larch-lint/src/repository.rs`; repository-policy bootstrap only. |
| Arbitrary developer harness child | Wrap a CI or developer harness and forward its streams and exit status. | `crates/larch-harness-mark/src/harness_mark.rs`; not a plugin runtime entrypoint or release artifact. |

Test-only child processes are outside the released inventory. Their call sites
carry the same reason-bearing lint approval.

### Pre-binary bootstrap boundary

The clean-install path cannot use the Rust process port until the executable
exists. `scripts/larch.sh` therefore owns this #7670 exception. Its closed
bootstrap prerequisite set is `awk`, `chmod`, `cmp`, `dd`, `gh`, `gzip`,
`kill`, `ln`, `mkdir`, `mktemp`, `mv`, `rm`, `rmdir`, `sed`, `sleep`, `sort`,
`tar`, `tr`, `uname`, and `wc`, plus either `sha256sum` or `shasum`. The script
uses fixed GitHub release and attestation operations, verifies the complete
artifact identity, and executes only the staged or installed `larch` binary.

Other shipped Bash stays confined to `scripts/residual-bash-paths.txt` and the
thin skill wrappers allowed by `AGENTS.md`. Those files may call the verified
`scripts/larch.sh` entrypoint, fixed repository helpers, the Git operations in
`docs/git-operation-inventory.md`, and standard host utilities required by
their local contract. Adding another external product, dynamic executable, or
service CLI requires an entry in this inventory and a mechanical allowlist.

## Invariants

- `# shellcheck shell=bash` is the first content line.
- The library has no stdout/stderr while the file is being sourced. Formatter functions (`larch_external_tools_braced`, `larch_implementer_coders_braced`) intentionally print to stdout when called by consumers.
- No `set -e`, `set -u`, or `set -o pipefail` mutation; no `exit`; no I/O on source.
- Bash 3.2-compatible: no associative arrays, namerefs, mapfile/readarray, or eval.
- `claude` is an implementer-only coder; it MUST NOT appear in `LARCH_EXTERNAL_TOOLS`. Step2's `TOOL=` envelope-line contract continues to mean external implementer only.
- Re-source is idempotent via the `LARCH_EXTERNAL_TOOL_REGISTRY_LOADED` sentinel, set as the final line of the library so "loaded" implies "fully initialized."

## Failure symptoms

## Non-goals

Per-tool model defaults and plugin `userConfig` environment variables stay in `crates/larch-core/src/vendor_model.rs`; this shell registry only names tool taxonomy.

## Adding a new external tool

1. Append the new id to `LARCH_EXTERNAL_TOOLS` and to `LARCH_IMPLEMENTER_CODERS` if it is also an implementer.
2. Add the per-tool branch in `scripts/larch.sh agent model-args` and its Rust owner.
3. Add the per-tool branch in `agent check-reviewers` presence detection and in any dispatcher fallback helpers; decide opt-in vs. default and update `--include-*` policy accordingly.
4. If the new tool is also an implementer, add the launcher branch in `implement step2-dispatch`.
5. No change is required for `scripts/larch.sh agent run-external-agent`'s raw `--tool` label: it sanitizes `.meta` `TOOL=` for any input. Prefer a label-safe id (alphanumerics, `.`, `_`, `-`) so `.meta` `TOOL=` matches the registry id verbatim; non-label-safe ids may still collide after sanitization (e.g. `tool/a` and `tool?a` both become `tool_a`), so `.meta` `TOOL=` is not a bijection from arbitrary labels. Direct execution remains closed to approved typed vendor programs; add an explicit process-port variant before a new vendor executable can launch.
6. If the new tool produces output collected by `scripts/larch.sh agent collect-results`, ensure its tool derivation can classify the new id from metadata and filenames so dispatcher fallback can attribute results.
7. Update the relevant sibling `.md` contracts.
8. Run `make lint` and `scripts/larch.sh checks run-relevant --site local --tmpdir "${TMPDIR:-/tmp}"`.

## Collector integration

`scripts/larch.sh agent collect-results` uses the same external-tool allowlist exposed by `scripts/larch.sh agent external-tool-registry --kind external-tools` for both `.meta` `TOOL=` validation and basename inference. The collector deliberately keeps an `unknown` fallback for observational classification of partial or malformed launches, which is semantically different from dispatch validation and is not a registry member.

## Tests

`scripts/test-external-tool-registry.sh` covers registry contents, predicates, brace formatting, source-time side effects, consumer consistency, and nested-cwd step2 path resolution.

## CI wiring

Target: `make test-external-tool-registry`. A `make lint` prerequisite via `the test-harnesses-N shard partition`. Also documented in `docs/linting.md`.
