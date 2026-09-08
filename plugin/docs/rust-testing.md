# Rust Testing

Rust tests must be deterministic, offline by default, and safe under Cargo's
parallel test runner. Use `larch-test-support` from integration tests and from
crate-local tests where the dependency graph permits it.

## Shared fixtures

- `TestWorkspace` owns a temporary root. It rejects absolute paths and dot
  segments, rejects symlink traversal, creates parents, and removes the root
  on drop.
- `TestEnvironment` builds a complete child environment without changing the
  test process. Call `env_clear` before applying its iterator to a command.
- `TestClock` implements the core wall, monotonic, and async clock ports.
  Sleeps advance injected time without waiting.
- `FakeProcessRunner` records typed requests and returns queued outcomes.
  `ProcessOutputBuilder` creates byte-exact success and failure results.
- `VendorProcessHarness` installs the Cargo-built fake `claude`, `codex`, and
  `cursor` binaries on a private complete `PATH`. `VendorScript` replays ordered
  stdout and stderr chunks, bounded inter-chunk delays, an exit code, or a
  never-exit process. A never-exit script may spawn a chain of at most two
  descendants in separate process groups and record their PIDs so timeout tests
  can prove the complete process tree stopped. Recorded contracts load through
  `VendorContractFixture`.
- `HttpResponseBuilder` creates in-memory responses and rejects invalid status
  codes, header names, and header line injection.
- `GitRepository::builder` creates an owned repository through installed Git.
  `GitFixture` names the shared unborn, detached, refs, changes, conflict,
  non-UTF-8 path, special-file, attributes and filters, sparse-checkout,
  submodule, linked-worktree, hooks and signing, remotes, and corruption states.
  Select `GitObjectFormat::Sha1` or `GitObjectFormat::Sha256`. Match
  `GitFixtureError::Skip` and print its `FixtureCapability` and reason when the
  host lacks a feature. Never turn a capability skip into an unreported early
  return.
- `RunLogTree::builder` creates isolated run-log staging, cache, pending, and
  object-store doubles. `RunLogFixture` names the historical and durability
  corpora used by reporting parity tests.
- `DesignSession::builder` creates isolated design sessions with env files,
  plan wire, clarify/pause bodies, and design run-log staging.
  `DesignFixture` names absent, partial, conflicting, and committed states for
  the #7680 design migration.

Never call `set_current_dir`, `set_var`, or `remove_var` in a test. Do not use a
shared fixed path, port, clock, response queue, or mutable static. Give each
test its own fixture and let Cargo run it in parallel. Cursor config isolation
uses `CursorConfigContext`, which returns a private directory and a
`ChildEnvironment::CursorConfigDir` override for the child request; it must not
mutate `CURSOR_CONFIG_DIR` in the test process.

## Git oracle and semantic snapshots

Git fixtures may invoke installed Git as a test oracle. `GitRepository` finds
the executable once, then clears the child environment and supplies an owned
home, temp directory, config, identity, dates, locale, and path. The path
contains the installed Git directory, the fixture helper directory, and only
the discovered system directories required by Git's shell helpers:
`basename`, `sed`, and `uname`. This keeps Homebrew and `/usr/local` Git
portable without inheriting the full ambient path. Commands set only the child
working directory. Fixture code must not change the test process environment
or working directory. Product crates must still use the closed Git interfaces
described in `ARCHITECTURE.md`; the fixture API does not authorize a production
arbitrary-argument Git runner.

Repository read differential tests exercise the public `RepositoryRead` port
through `GixRepository`. Compare typed IDs, ref targets, config provenance,
URLs, worktree records, and error classes with parsed Git oracle results. Path
comparisons may normalize filesystem aliases such as macOS `/var` and
`/private/var`, but the adapter result must retain its original path bytes.
For status and change sets, compare staged, unstaged, untracked, ignored, and
unmerged paths plus change kinds, modes, IDs, conflict stages, and flags. Cover
pathspec and case configuration, filters and CRLF, sparse indexes, submodules,
non-UTF-8 paths, symlinks, executable bits, and configured rewrite behavior.
An unsupported semantic must return its typed error instead of dropping data.

Use `SemanticSnapshot::capture` after each implementation runs against its own
equivalent repository. Supply the operation's public result through
`ExecutionSnapshot`. Compare the typed snapshots first. Use
`SemanticSnapshot::render` only for a checked-in review artifact. The
`larch-git-snapshot-v1` format captures:

- exit class and bounded public stdout and stderr;
- object IDs and types, refs, reflogs, index stages and modes, index flags,
  and byte-preserving untracked and ignored paths;
- worktree bytes, modes, symlinks, linked-worktree records, operation-state
  files, relevant config, and hook or helper transcripts;
- an independent `git fsck --full --no-dangling` result.

Each byte field stores at most 1 MiB plus its full length, truncation state,
and deterministic checksum. Each filesystem section stores at most 4,096
entries and records section truncation. Rendered paths and bytes use lowercase
hex. Capture replaces the owned temporary root with `<ROOT>` and redacts
credential-bearing config and URL user information. Do not add normalization
for object IDs, modes, paths, repository state, or other semantic values.

Snapshot reviews must explain every changed semantic field. Update a checked-in
render only after both implementations produce the intended state. Test
success, injected failure, interruption, and corruption separately. A failed
Git probe remains snapshot data so corrupt repositories stay comparable.

Normal Git tests remain offline. Use local repositories for remotes and
submodules. Use fixture scripts for hooks, filters, credential helpers,
askpass, signing programs, and other child tools. Store their bounded
transcripts under `GitRepository::transcript_root`; never use live credentials
or a remote URL that can resolve to a service. Run the named fixture matrix on
macOS and Linux. Capability skips must name the missing feature and host error
in test output.

The final ownership gate runs in the same cross-platform Rust lane. Focused
coverage is `cargo test --locked --package larch-lint --test integration git_ownership::`.
It injects direct process creation, arbitrary Git arguments, `gix` bypasses,
duplicate owners, a new CLI exception, non-atomic command state, and inventory
drift. The adapter and CLI suites supply the SHA-1/SHA-256, case, path, filter,
hook, credential, worktree, interruption, recovery, and `git fsck` fixtures.

## Run-log fixtures and reporting parity

`larch-test-support` owns offline run-log corpora for the #7683 reporting
migration. `RunLogTree::builder` creates an isolated temporary root with
staging (`larch-logs/<skill>/<run-id>/`), cache, pending-publication, and a
local object-store double. Named `RunLogFixture` values cover absent, partial,
corrupt, checkpoint, interrupted, committed, archive-pending, batch-corpus,
token/timing/progress, credential-bearing transcript, and historical shapes
the tolerant reader still accepts: manifest v1, lifecycle schema v1, and the
legacy panel-prompt-sizes TSV header.

`RunLogSnapshot::capture` builds one bounded semantic snapshot of a run-log
tree: relative paths, modes, byte content or digests, ordering, durability
markers, and the supplied `ExecutionSnapshot`. Capture replaces the temporary
root with `<ROOT>` and redacts credential-bearing lines and URL userinfo.
`ReportSnapshot::capture` records exact machine fields from JSON reports plus
normalized prose from final-summary, final-report, and run-statistics files,
including RFC3339 timestamp substitution.

Use `ReportingParityOracle` to compare two run-log or report snapshots and
report only differing channels. Prefer typed snapshot equality in tests; use
`render` only for checked-in review artifacts (`larch-run-log-snapshot-v1`,
`larch-report-snapshot-v1`). Snapshot reviews must explain every changed
semantic field. Test success, injected failure, interruption, and corruption
separately so those states stay distinguishable.

Token extraction has a second, narrower oracle. `crates/larch-core/tests/
fixtures/token_scan/` holds ledger and transcript inputs beside the recorded
output of the retired owner over exactly those inputs, and
`tests/token_scan.rs` asserts full JSON equality against them. Treat those
records as reviewed contract fixtures: an intentional contract change updates
the inputs and expected reports together. Scanning stays streaming: the scan
reads one run at a time and both ledgers and transcripts line by line, so peak
memory is bounded by the largest single run and not by corpus size, as
`larch_core::report`'s token-scan module documents.

Token pricing uses the same differential shape. `crates/larch-core/tests/
fixtures/token_cost/` holds two recorded case files: `argv-cases.json` pairs
pricing flags and a rate environment with the reviewed `KEY=value` block
and cost line, and `record-cases.json` pairs a raw token report with the flags
`token_cost_argv` derived from it, the resulting block, and the `price_run` cost
fields. `tests/token_cost.rs` asserts string and value equality against both,
including the blended fallback a negative bucket forces. Every case stores its
own inputs. Update both files together for an intentional pricing-contract
change, and review every changed number. `flags.json` records the closed
count-flag set so the Rust grammar cannot drift wider or narrower.

`LocalObjectStore` is a filesystem double for the documented object-store
operations (`preflight_prefix`, `list`, `upload_create`, `metadata`,
`download`). It stays offline, rejects unsafe keys, and never contacts a
network endpoint. Fixture code must not call `set_current_dir`, `set_var`, or
`remove_var`.

## Issue fixtures and parity

`larch-test-support` owns the offline issue-domain fixtures for #7682.
`IssueGraph::builder` creates an owned graph under a private temporary root;
the named `Absent`, `Partial`, `Conflicting`, and `Committed` cases cover
plain and wire-bearing bodies, comments, labels, sub-issue trees, blocked-by
edges, OOS manifests, tracking records, and umbrella proposals.

`IssueGraphSnapshot::capture` records bounded, stable issue semantics:
numbers, titles, bodies, states, labels, comments, and both edge sets. It
redacts credentials and replaces owned roots and repository slugs with stable
markers. `IssueStdoutSnapshot::capture` preserves ordered `KEY=value` fields
while separately normalizing prose; both channels receive the same identity
normalization and credential redaction. Use `IssueParityOracle` to compare
either kind of snapshot and report only changed channels. Test absent, partial,
conflicting, committed, failure, and interruption shapes independently. Review
every changed semantic field before accepting a parity result.

`IssueServiceStub` replays a bounded queue of recorded HTTP exchanges on an
ephemeral loopback listener. It supports exact-route pagination, 429
rate-limit responses, 409 conflicts, interrupted connections, and mixed batch
outcomes while recording redacted requests. It is an offline adapter fixture,
not a production GitHub client or a permission to make network calls. Always
finish the stub so unconsumed exchanges fail the test.

`larch-core` reaches these fixtures through a dev-dependency on
`larch-test-support`, which itself depends on `larch-core`. Cargo permits that
cycle because it exists only in the dev graph, and no release build links the
fixture crate. Add issue-domain golden bytes to the graph fixtures rather than
re-declaring wire bodies in each crate's tests.

## Design fixtures and parity

`larch-test-support` owns the offline design-domain fixtures for #7680.
`DesignSession::builder` creates an owned session under a private temporary
root. Named `DesignFixture` values cover absent, partial, conflicting, and
committed shapes with PID-keyed `current-design-env-$PPID.sh` files, step result
envs, issue bodies carrying plan blocks, named blocks, clarify threads, and
pause pointers per `docs/issue-anchored-plan.md`, plan-grammar documents, and
design run-log staging trees under `larch-logs/design/<run-id>/`.

`DesignSessionSnapshot::capture` records bounded, stable session semantics:
state, ppid, issue number, repository slug, run id, and redacted file entries.
It replaces owned roots, design tmpdirs, repository slugs, run ids, and issue
numbers with stable markers, and redacts credential-bearing lines and GitHub
tokens. `DesignStdoutSnapshot::capture` preserves ordered `KEY=value` fields
while separately normalizing prose. Use `DesignParityOracle` to compare either
kind of snapshot and report only changed channels. Review every changed
semantic field before accepting a parity result.

`DesignGithubScenario` builds recorded `IssueServiceExchange` queues for the
design clarify round-trip, pause save/load path, and label mutation-conflict
retry. It starts the existing loopback `IssueServiceStub`; it is not a second
GitHub client and never contacts a non-loopback address. Always finish the stub
so unconsumed exchanges fail the test. Fixture code must not call
`set_current_dir`, `set_var`, or `remove_var`.

`larch_core::design` owns the plan-grammar and plan-quality analysis core
ported in #8575: heading and trailer parsing, M1/M2 validation with injected
tracked paths, optional metadata, size-trigger assessment, and plan-command
extraction. Golden parity for command extraction reuses
`fixtures/plan-commands/parse-plan-commands`. Later command leaves
register CLI verbs; this core stays network-free and filesystem-bounded.

`larch_core::debate` owns the debate protocol vocabulary ported in #8597:
wire constants and enums, lexical validators, slot-ledger row grammar with
exact rejection-reason tokens, concession citation classification, and reason
normalization plus fingerprints. Leaf #8598 adds the state half: round-state
assembly, point resolution, stalemate detection, adjudication records, and the
proposal transition machine. Inline tests cover the executable-contract cases,
including the transition and stalemate rejection tokens, and pin golden
fingerprint fixtures for byte parity. `larch_core::debate` is the sole owner.
This core is network-free and
filesystem-free.

Leaf #8599 ports the state store. `larch_core::debate::state` owns canonical
JSON, strict duplicate-rejecting parsing, payload fingerprints, schema
versioning, every encode and decode codec, and the pure `decode_state` and
`encode_state` pair with an exit-code-bearing `StateError`. The effectful
`load_state`, `write_state`, and the `O_NOFOLLOW` flock live in
`larch_cli::debate_state`, reusing the `larch-adapters` trusted-root
confinement and the `analysis_state` lock precedent. Integration tests in
`crates/larch-cli/tests/debate_state.rs` load recorded legacy state fixtures
under `crates/larch-cli/tests/fixtures/debate_state/` (`state-v2.json`,
`state-v2-active.json`, and `state-v1.json`), assert a byte-identical schema-2
round trip and the schema-1 migration to the current schema, and cover the lock
refusing a non-regular or symlinked path plus the stale-fingerprint exit code.
Run `cargo test -p larch-core debate` and `cargo test -p larch-cli
debate_state`.

## Test boundaries

- Unit tests live in a crate-local `#[cfg(test)]` module. They cover private
  logic through injected ports. Use an owned workspace if filesystem state is
  part of the behavior; never use the ambient working directory.
- Integration tests live directly under a crate's `tests/` directory. They
  cover the public crate boundary and may use owned filesystem fixtures.
- `larch-cli`, `larch-lint`, `larch-core`, and `larch-adapters` each expose one
  Cargo integration target named `integration`. Their `tests/main.rs` roots use
  `automod` to compile every other top-level Rust file as a module. Add new test
  modules beside the existing files. Run a whole module without suffix-name
  collisions with `cargo nextest run --locked --package <package> --test
  integration -E 'test(/^<module>::/)'`.
- Golden tests compare complete user-facing or wire-format bytes under
  `fixtures/`. Update goldens only for an intentional, reviewed contract
  change. Keep the update switch opt-in and disabled in CI.
- Property tests cover parsers, codecs, path rules, and other broad input
  spaces. Fix the generator seed in failure output and keep shrinking enabled.
  A property test complements named boundary examples; it does not replace
  them.
- Live-smoke tests exercise a real remote service or vendor executable. Mark
  them ignored by default, require an explicit opt-in, and never run them in
  normal pull-request CI.

Normal unit, integration, golden, property, and coverage runs cannot access an
external network. Use injected HTTP responses. A transport adapter test may
bind an ephemeral loopback port when it must exercise socket framing; it cannot
resolve DNS or contact a non-loopback address.

Do not depend on an installed executable unless the test covers an approved
executable compatibility boundary. The process adapter may invoke Git.
`VendorProcessHarness` may invoke its Cargo-built fake vendors. Everything else
uses `FakeProcessRunner`. Real Claude, Codex, Cursor, service credentials, and
remote endpoints belong only in explicit live-smoke runs.
Tests that need vendor process timing use `VendorProcessHarness` with
`TokioProcessRunner`. Never append the ambient `PATH`; a missing fake must fail
with `ProcessErrorKind::Spawn` even when a real vendor executable is installed.

## Coverage and CI

Coverage is CI-only. The required Rust jobs currently divide ownership as
follows:

- `rust-lint` runs format and Clippy with incremental compilation and dev/test
  debug output disabled.
- `rust-deny` runs the locked all-feature dependency policy in parallel.
- Together, `rust-full-shards` and `rust-full-policy` are the full-mode
  producers; `rust-partial` and `rust-skip` are the mutually exclusive
  alternatives. `rust-coverage` is the stable required aggregate and
  full-mode LCOV gate. Under `if: always()`, it validates the selected mode and
  every mode result, then passes only when the selected mode succeeds and the
  alternatives are skipped. Full mode also requires successful
  `rust-full-lcov-tool` preparation before it performs the merge and line gate
  described below. `rust-gate` independently validates `rust-lint`,
  `rust-deny`, and the raw producer-result shape without waiting for
  `rust-coverage`; both stable checks are required, so a red producer or LCOV
  threshold failure blocks the merge queue. The selected producer also runs
  the bootstrap integration consumer before `rust-coverage` reports success.
- The `rust-full-shards` matrix owns the full locked-workspace test coverage.
  Four cells run `cargo llvm-cov nextest` with disjoint
  `--partition hash:N/4` partitions and upload distinct LCOV reports. Shard 1
  alone runs workspace doctests and plugin projection validation and stages
  cache candidates. `rust-full-policy`
  runs beside the four cells. It builds only the instrumented `larch` binary,
  runs the single `larch lint all` scan, and uploads its LCOV and per-rule
  timing artifacts. `rust-full-lcov-tool` starts beside those producers. It
  installs the exact pinned Ubuntu LCOV package, archives only the package-owned
  runtime files and dependencies installed or updated by that transaction,
  verifies the extracted LCOV 2.0 runtime, and uploads a checksum-bound
  same-run tool artifact.
  `rust-coverage` downloads that artifact and exactly five same-run LCOV reports
  in one artifact operation. It verifies the tool checksum, archive paths,
  runtime version, exact report paths, and report count. The prepared LCOV 2.0
  merger combines the reports with five-way parallelism, then the job applies
  the unchanged 88% line threshold to the canonical `LF`/`LH` totals in that
  merged report without parsing the input set a second time.
  After coverage-target pruning, an exact cache miss in a successful
  `merge_group` full lane stages and verifies a policy-cache candidate from
  the locally prepared policy bundle. The trusted main publisher may promote
  it only after that candidate's SHA becomes the current `main` SHA, then
  rewrites and revalidates its `refs/heads/main` provenance. This does not
  build a second executable.
  The composed full mode is the only path that enforces full-workspace
  coverage. The
  `merge_group` `checks_requested` trigger runs the same full, read-only path
  for a merge-queue candidate. Manual dispatches and merge-queue runs use the
  full shard and policy path; a normal `main` push runs only trusted cache
  publication. Pull requests use that full path when selection is `full` or
  selection cannot prove a narrower path.
- `rust-partial` and `rust-skip` may be the selected producer only for pull
  requests. `rust-partial` runs the selector-proven package closure without a
  misleading full-workspace coverage threshold. `rust-skip` runs no
  pull-request Rust binary; it validates and uses the exact trusted-main policy
  executable instead. Both producers retain repository policy, plugin
  projection validation, and direct bootstrap integration. Full, partial, and
  skip each run `scripts/test-rust-integration-consumer.sh` with the selected
  executable and then run the findings-classification contract. The harness
  requires an explicit mode and rejects default profile files in the temporary
  client repository.
- `rust-coverage-benchmark` runs only when a manual dispatch sets
  `coverage_profile_benchmark=true`. Its matrix keeps the profile sweep out of
  the protected production path and does not publish production integration
  output.
- `rust-coverage-target-cache-benchmark` runs only when a manual dispatch on
  `main` sets `coverage_target_cache_benchmark=true`. It runs beside the normal
  cache-off full shard and policy path as the control, uses the same coverage
  action and profile, and uploads a uniquely named verification artifact.

### Rust coverage shard count

The production matrix uses four hash partitions. This is a measured tradeoff,
not a claim that hash partitioning balances test runtime exactly. The policy
job runs in parallel with every test shard. A resize changes only the test
matrix; the stable merger expects one policy report in addition to the
configured shard count.

[Merge-group run 32695855775](https://github.com/character-ai/larch/actions/runs/32695855775)
measured shard 1 at 470 seconds and the other three shards at 286 to 331
seconds. Shard 1 spent 131 seconds on repository policy after its 202-second
compilation. Moving that scan to the dedicated binary-only job removes the
serial policy phase without duplicating the full workspace test build. New
production timing can revise the shard count through
`/rebalance-tests --kind rust --n-rust-shards N`; the command verifies complete
post-change shard cohorts before it accepts the result.

### Bash-shard Cargo target ownership

The Bash-harness matrix deliberately excludes Cargo-backed Make targets. The
following focused local targets are covered by the stronger full-workspace
`cargo llvm-cov nextest --no-report --workspace --all-features --locked`
execution in `rust-full-shards`:

Each listed Make recipe is the shared `$(HARNESS_MARK) --label $@ --` prefix
followed by its table command; it has no other recipe lines.

| Focused local Make target | Complete Cargo recipe | `rust-full-shards` nextest surface |
| --- | --- | --- |
| `test-collect-agent-results` | `cargo test --locked --package larch-cli --test integration collector_commands::` | `collector_commands` |
| `test-analyze` | `cargo test --locked --package larch-cli --bin larch analyze_issues_commands` | `analyze_issues_commands` |
| `test-fetch-combinable-issues-filter` | `cargo test --locked --package larch-cli combine_issues_commands --bin larch` | `combine_issues_commands` |
| `test-blocker` | `cargo test --locked --package larch-core --lib prose_blockers` | `prose_blockers` |
| `test-check-clean-tree` | `cargo test --locked --package larch-cli --test integration cli::clean_tree_reports_clean_and_tracked_or_untracked_dirty_state` | `cli` |
| `test-check-scope-reduction-marker` | `cargo test --locked --package larch-cli --test integration dirty_tree::scope_` | `dirty_tree` |
| `test-phantom-probe-with-warn` | `cargo test --locked --package larch-cli --test integration cli::phantom_probe` | `cli` |
| `test-run-step2-dispatch` | `cargo test --locked --package larch-cli --bin larch implement_step2_commands::commands_tests::run_dispatch` | `larch` binary unit tests |
| `test-step2-dispatch` | `cargo test --locked --package larch-cli --test integration implement_step2_dispatch_parity::` | `implement_step2_dispatch_parity` |
| `test-git-commit-only` | `cargo test --locked -p larch-cli --test integration git_commands::nul_pathspec_only_commit_preserves_unrelated_staged_content` | `git_commands` |
| `test-dispatch-code-voters` | `cargo test --locked --package larch-cli --test integration voter_dispatch_commands::` | `voter_dispatch_commands` |
| `test-check-mid-run-dirty-tree` | `cargo test --locked --package larch-cli --test integration dirty_tree::` | `dirty_tree` |
| `test-check-phantom-dirty` | `cargo test --locked --package larch-cli --test integration cli::check_phantom_dirty` | `cli` |
| `test-no-grouped-reuse-guard` | `cargo test --locked --package larch-cli --test integration waterfall_commands::dispatcher_carries_no_grouped_reuse_machinery` | `waterfall_commands` |
| `test-launch-claude-subprocess` | `cargo test --locked --package larch-cli --test integration -- claude_commands::subprocess_` | `claude_commands` |
| `test-launch-claude-review` | `cargo test --locked --package larch-cli --test integration -- claude_commands::review_` | `claude_commands` |
| `test-dispatch-with-waterfall` | `cargo test --locked --package larch-cli --test integration waterfall_commands::` | `waterfall_commands` |

The focused targets remain available for local debugging. They are not
`test-harnesses-N` prerequisites, so a fresh Bash-harness runner does not
duplicate the workspace test compilation.

`rust-full-shards` and the dispatch-only combined coverage jobs install
checksum-verified pinned `cargo-nextest` and `cargo-llvm-cov` binaries without
a source-install fallback. `rust-full-policy` installs only the pinned
`cargo-llvm-cov` binary. Normal local checks use changed-path Clippy and do not
install coverage tooling or create instrumented artifacts.

`rust-full-shards` sets `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_TEST_DEBUG=0`,
`CARGO_PROFILE_TEST_OPT_LEVEL=0`, and runs nextest with
`NEXTEST_TEST_THREADS=16`. It cleans prior coverage state, builds one
coverage-instrumented artifact set with `cargo nextest run --no-run` under the
environment exported by `cargo llvm-cov show-env`. The same environment then
builds the `larch` CLI with Cargo's `--profile test`, which resolves to
`target/llvm-cov-target/debug/larch`; it does not compile a second CLI with the
dev profile or build an uninstrumented `target/debug/larch`. Each cell runs its
`cargo llvm-cov nextest --no-report --partition hash:N/4` partition. Shard 1
also runs a separate `cargo test --doc --workspace --all-features --locked
--target-dir target/llvm-cov-target` command under the same
`cargo llvm-cov show-env` environment. `--no-report` preserves the coverage
artifact set between normal test phases; the stable toolchain runs doctests
without cargo-llvm-cov's nightly-only doctest instrumentation. Each shard
report retains the existing filename exclusions. The stable `rust-coverage`
job verifies and extracts the LCOV runtime prepared in parallel, merges the four
test reports and the policy report through LCOV's parallel add-tracefile path,
then applies the line threshold to the merged report's generated line totals.
After shard 1 produces its report, it uses the executable for the single
plugin-runtime generate-and-validate step before uploading it for bootstrap
integration tests. The separate doctest command stays
required even when the workspace currently has no doctests. Nextest's
slow-test status and final status output remain visible in the job log.

`rust-full-policy` exports the same coverage environment, then runs
`cargo build --locked --package larch-cli --bin larch --all-features --profile test`.
It does not install nextest or build the workspace test executables. It fails
closed unless the coverage-target executable is runnable and reports its
version, then uses it for exactly one `larch lint all` invocation. That scan
writes a sorted per-rule timing TSV. Its LCOV report contributes to the merged
line gate before the stable `rust-coverage` job applies the threshold.

Repository-policy execution has a fixed four-worker bound. Measured
whole-repository scans may start before ordinary rules to avoid a worker-tail,
but that dispatch preference cannot change selected rules, their ownership, or
the stable findings, warnings, and name-sorted timing rows. A priority change
requires current-tree and triggering-fixture output equivalence plus comparable
full-policy timing evidence.

### Pull-request Rust selection

`rust-selection` runs only for pull requests. The checkout action supplies the
tested merge candidate with full history, and a second checkout action supplies
the pull-request base in an isolated directory. The trusted base copy of the
cache-key action derives both the exact `trusted-main-rust-policy` key and its
Rust-input digest from that isolated base, never from candidate files. The job
restores and validates that exact entry, then invokes `ci rust-select` through
the base's `scripts/larch.sh` wrapper with the validated executable. The base
checkout therefore owns the command surface and expected content identity,
while the candidate remains diff data. The typed command proves the base and
candidate commit identities and ancestry before it reads that diff.

A missing trusted checkout, empty base-input digest, cache miss, invalid
executable, malformed identity, unavailable history proof, or command failure
selects `full`. A candidate Rust-input change does not invent a cache key that
only the unmerged candidate could publish; the trusted-base classifier decides
whether that change is a global or package-scoped input. A new or unpublished
trusted-base Rust-input identity still misses safely. The first pull request
carrying a new selector command can likewise fall back when its base predates
that command. Selection never compiles or executes pull-request code.

The selector inspects the candidate checkout only as data. A pull request
cannot change selector code and use that change to choose a narrower path;
changes to the selector, CI workflow, coverage action, or its redaction and
workspace-metadata dependencies are global inputs and run `full`.

The selector verifies both commits, the candidate checkout, and base ancestry
before it reads the diff. An unavailable history proof, missing commit,
non-ancestor base, empty or malformed diff, unsupported status, metadata parse
failure, unknown path, unsupported workspace shape, or internal error selects
`full`. It emits one redacted deterministic JSON result as the
`rust-ci-selection` artifact and renders a concise step summary. Every dynamic
field crosses the Rust core redaction boundary and a residual-secret rescan; a
scrub failure emits a static `full` result with no changed-path data. The
summary HTML-escapes the redacted data. The artifact preserves the classifier's
`mode` as `proposed_mode` and records the lane's `effective_mode`, reason,
`rollout_state`, and `observation_only` value after trusted-cache validation
and any `full-rust-ci` override; the summary shows both proposed and effective
execution decisions.

`RUST_CI_PARTIAL_ENFORCEMENT` and `RUST_CI_SKIP_ENFORCEMENT` are `true` after
their recorded independent pull-request windows had successful full backstops
and zero false-safe results. A proposed non-full mode executes only when
trusted-main policy verification succeeds; a cache miss or verification
failure remains `full`.

The workflow enforces these modes after both live observation windows completed:

- `full` runs format, full Clippy, dependency policy, full coverage, doctests,
  repository policy, plugin projection validation, and the Linux artifact for
  bootstrap integration tests. Manual dispatches and merge-queue runs always take
  this path. A normal push to `main` runs only trusted cache publication.
- `partial` accepts only Rust-source changes whose Cargo-metadata package
  closure is a strict subset of the workspace and contains `larch-cli`. The
  closure includes every transitive normal, build, and dev reverse dependency edge,
  has deterministic ordering, and every changed path must belong to exactly one
  Cargo target source root. `rust-lint` runs workspace format plus selected
  locked all-feature Clippy. `rust-partial` runs selected locked all-feature
  tests and applicable library doctests, builds the candidate `larch` binary,
  runs repository policy and plugin projection validation with it, and uploads
  it for bootstrap integration. It does not claim or enforce the full-workspace
  coverage threshold. Dependency policy is skipped only because manifests,
  lockfile, Cargo configuration, and deny inputs are all global `full` inputs.
- `skip` accepts only supplementary paths with explicit owners: root
  documentation/configuration files, `.claude/`, `agents/`, `docs/`, and
  `skills/`. The selector records every applicable owner. The normal lint,
  agent, and generated-plugin checks still validate their owned content. Rust
  repository policy and plugin generation run through a verified
  trusted-main executable; bootstrap integration receives that same verified
  executable. No pull-request Rust binary runs in this path. The `rust-skip`
  job's elapsed duration is the selected execution-path measurement;
  `rust-coverage` and `rust-gate` prove required status coverage but do not
  replace that duration.

For `skip`, the trusted main publisher promotes an immutable
`trusted-main-rust-policy` cache entry only from a successful merge-group
full-mode artifact whose SHA exactly became `main`. Its key and metadata bind
the Linux binary to tracked crate Rust sources (never generated target output),
root and crate manifests, root or crate build scripts, lockfile, toolchain,
and `.cargo/` inputs. The cache has no broad fallback. For pull requests, both
the lookup key and expected input digest come from the isolated trusted base;
candidate Rust files cannot choose either value. The selection job verifies
regular-file shape, content checksum, input identity,
`refs/heads/main` provenance, source-SHA shape, and executable version before
it permits an enforced `skip` or uses the executable to calculate any non-full
proposal. The selection job uploads the verified handoff only when `skip`
is the effective mode. The `rust-skip` job verifies the downloaded files again.
Full and partial decisions do not pay for that artifact transfer. A cache miss
or any validation failure selects `full` before another lane can rely on it.

`Cargo.lock`, any Cargo manifest, `rust-toolchain.toml`, `.cargo/`, build
scripts, Makefiles, `deny.toml`, Rust CI/profile files, and selector machinery
are global inputs and always select `full`. `rust-coverage` remains the stable
required status: it accepts either both successful full-mode producers or one
successful alternative (`rust-partial` or `rust-skip`), with every unselected
producer skipped. Full mode additionally requires the parallel LCOV runtime
preparation to succeed. An unavailable selector defaults to `full`, and the
aggregate passes only when that full path succeeds. The
merge group is the per-merge full-run backstop; manual dispatch provides a
full rerun. To force the full
path while debugging a pull request, apply the `full-rust-ci` label; label and
unlabel events rerun CI, and that label can only narrow toward the safer
`full` mode.

Promotion is intentionally manual and class-specific. Keep an unproven class's
enforcement value `false` until the live record has at least three independent
pull requests with proposed non-full decisions, every retained full backstop is
successful, and that class has no false-safe result. A reviewed workflow change
may then set only that proven class to `true`; a selector result, cache result,
label, or pull-request-controlled input can never flip the value. The promoting
change is global and therefore cannot supply its own selected-path timing; the
next eligible pull request must record it against the comparable full control.
The other class remains in observation until its evidence meets the same rule.

The independent historical classifier replays are recorded in
[`rust-ci-selection-observation.md`](rust-ci-selection-observation.md). They
show useful decisions and successful historical full backstops, but do not
substitute for live selected-path results. Future changes to a selector,
ownership rule, policy-binary identity, or cache schema start fresh classifier
and live selected-path comparisons rather than inheriting this record.

### Coverage-profile measurement contract

A profile comparison uses the same commit, `ubuntu-24.04` runner, pinned
toolchain, runner-provided linker, `CARGO_INCREMENTAL=0`,
`CARGO_PROFILE_TEST_DEBUG=0`, and the same cache class. A manual dispatch with
`coverage_profile_benchmark=true` runs the dispatch-only
`rust-coverage-benchmark` job with three samples of both test optimization
levels and every nextest thread count from 4 through 16. It compiles one
coverage-instrumented artifact set per profile, clears only `profraw` data
between thread counts, runs one complete repository-policy scan after each
nextest pass and before that path's coverage report, and reports the raw
end-to-end coverage-phase totals (common profile cleanup, compilation,
doctests, plus that thread count's cleanup, nextest, policy, and report) and
their median. A candidate that varies by more than 10% across its first two
samples is rerun before comparison. The coverage line gate is unchanged; a
profile whose report fails it is not eligible.

### Rust test-policy overlap measurement contract

A manual dispatch with `rust_phase_overlap_benchmark=true` runs the
`rust-phase-overlap-benchmark` matrix: three `sequential` control samples and
three `parallel` candidate samples at one commit on `ubuntu-24.04`. Every cell
uses the same pinned coverage tools, dependency-cache policy, test profile,
`NEXTEST_TEST_THREADS=16`, and 88.000% line threshold. The matrix is
observational and cannot mutate the protected full-mode producers. A reviewed
workflow change is required to promote a proven phase mode.

After compilation and the required doctests, the candidate starts nextest and
the one repository-policy scan together. Each branch requires the
`LLVM_PROFILE_FILE` process placeholder, writes its timing row and complete
stdout/stderr to a distinct runner-temp file, and is explicitly awaited. The
action emits both labeled logs and timing rows before deciding success. A
failure from either branch fails the job before `cargo llvm-cov report`, so the
coverage threshold and LCOV run only after every coverage-producing process
has exited successfully. The coverage-built executable, plugin projection
validation, LCOV artifact, policy timing artifact, and uniquely named
integration artifact remain required for every eligible sample.

Compare the raw nextest, policy, report, end-to-end, and job durations only
within one cache class. Retain parallel production execution only when all
three paired candidate samples pass every preserved check and improve the
median end-to-end job time without worsening the Rust gate or total workflow
median. Otherwise retain the sequential producer and record the measured
result.

### Rust test-policy overlap evidence

[Benchmark run 31415431053](https://github.com/character-ai/larch/actions/runs/31415431053)
ran the paired matrix at `386ad7dfadd4630a1d5ab1de9860dba261ec8b77` on
`ubuntu-24.04`. Every cell was an exact coverage-target-cache hit with warm
Cargo-input and coverage-tool caches, the same pinned tools and profile, and
16 nextest threads. The six target-cache restores reported 1,348,853,760 to
1,348,861,952 allocated bytes; each cache-save row was the explicit
`workflow_dispatch-read-only` skip. All six cells, the sequential `rust-full`,
`rust-coverage`, and `rust-gate` jobs succeeded.

The action-level job total excludes runner setup; runner job wall time comes
from the GitHub job timestamps. The raw paired results are:

| Mode | Sample | Nextest | Policy | Report | End-to-end | Action job | Runner job |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| sequential | 1 | 64 s | 47 s | 19 s | 205 s | 244 s | 250 s |
| sequential | 2 | 54 s | 62 s | 19 s | 212 s | 248 s | 252 s |
| sequential | 3 | 64 s | 87 s | 25 s | 270 s | 309 s | 312 s |
| sequential | median | 64 s | 62 s | 19 s | 212 s | 248 s | 252 s |
| parallel | 1 | 75 s | 116 s | 24 s | 230 s | 280 s | 286 s |
| parallel | 2 | 84 s | 143 s | 28 s | 277 s | 325 s | 332 s |
| parallel | 3 | 72 s | 108 s | 20 s | 204 s | 248 s | 256 s |
| parallel | median | 75 s | 116 s | 24 s | 230 s | 280 s | 286 s |

Parallel missed the promotion condition: its median end-to-end duration was
18 seconds (8.5%) slower and its action-level job median was 32 seconds
(12.9%) slower. The three sequential runner jobs consumed 814 seconds in
total; the three parallel jobs consumed 874 seconds, 60 seconds (7.4%) more.
One parallel sample improved, but the other two regressed, so the sequential
producer remained the production shape after that benchmark. The same
dispatch's sequential `rust-full` took 325 seconds, `rust-coverage` took 3
seconds, and `rust-gate` took 3 seconds. The workflow elapsed 365 seconds.
Those manual-run values were context rather than a new production median; that
decision made no Rust-gate or workflow-path change.

Coverage and provenance remained equivalent to the control: every LCOV report
had 158,933 found lines, 17,614 found functions, and 14,594 hit functions.
The line-hit values ranged from 140,222 to 140,227 across both modes, with all
reports at 88.227% to 88.230%, above the unchanged 88.000% threshold. Every
policy-timing artifact listed the same 53 rules, every plugin projection check
passed, and all six verified executable artifacts had source SHA
`386ad7dfadd4630a1d5ab1de9860dba261ec8b77`, Rust-input SHA
`18dd663036f9c1255cafb27d7c7f642996ab01659b429e520fae5c7bbbfbebd2`, and
binary SHA-256
`a70f1aec93089738f2ede40f4e8716fe7437c64b64ea86aa0a9c9d49a49a28f6`.

The repository-policy `lint all` execution path writes only its supplied
runner-temp timing file, so it has no concurrent target-tree mutation path.
The candidate requires `%p` in `LLVM_PROFILE_FILE`, writes branch logs and
timing rows separately, and waits for both before report. A source-extracted
helper probe forced nextest to exit 23 while policy succeeded; it retained both
labeled phase records and returned failure, confirming that a branch failure
remains attributable and blocks the report.

#### Post-consolidation reevaluation

[Benchmark run 32678844308](https://github.com/character-ai/larch/actions/runs/32678844308)
reran the paired matrix at `11de90a0cbbcdcef8faf657742121ee3d11ba545`
after the Rust integration-test consolidation. All six cells were exact
coverage-target-cache hits with warm Cargo-input and coverage-tool caches. The
target restores reported 1,411,751,936 to 1,411,756,032 allocated bytes. Each
cell's generic cache-save row recorded `validation-read-only`; no validation
run published a cache. Every benchmark cell and the sequential `rust-full`,
`rust-coverage`, and `rust-gate` jobs succeeded.

| Mode | Sample | Nextest | Policy | Report | End-to-end | Action job | Runner job |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| sequential | 1 | 189 s | 121 s | 110 s | 624 s | 686 s | 690 s |
| sequential | 2 | 183 s | 130 s | 108 s | 620 s | 688 s | 695 s |
| sequential | 3 | 182 s | 129 s | 107 s | 620 s | 693 s | 699 s |
| sequential | median | 183 s | 129 s | 108 s | 620 s | 688 s | 695 s |
| parallel | 1 | 268 s | 301 s | 111 s | 620 s | 686 s | 691 s |
| parallel | 2 | 264 s | 311 s | 106 s | 624 s | 687 s | 694 s |
| parallel | 3 | 260 s | 301 s | 105 s | 609 s | 673 s | 679 s |
| parallel | median | 264 s | 301 s | 106 s | 620 s | 686 s | 691 s |

Parallel tied the 620-second median coverage-phase total. It reduced the
median action total by 2 seconds and runner wall time by 4 seconds, or 0.6%.
That is not the policy-duration reduction required for promotion. Contention
instead raised median nextest time by 81 seconds, or 44.3%, and policy time by
172 seconds, or 133.3%. The then-current protected `rust-full` producer
therefore remained sequential.

Every sample passed 5,479 tests and reported the same two skips. The parallel
samples classified two or three tests as slow, while the sequential samples
classified zero or one as slow; no sample failed. Every LCOV artifact found
322,054 lines and 34,090 functions. Line hits ranged from 283,690 to 283,705,
or 88.0877% to 88.0924%, and hit functions ranged from 28,244 to 28,248 across
both modes. The coverage shapes were equivalent, but exact line percentages
differed. Every policy artifact listed the same 57 rules, and every plugin
projection check and executable-artifact upload passed. The candidate missed
both the timing-gain and exact-coverage-identity conditions.

### Current-main-derived candidate evidence

[Benchmark run 31151194045](https://github.com/character-ai/larch/actions/runs/31151194045)
ran the exact coverage implementation at `9eba6b09204e76e9e2bf6f4cd30ee6b2a34891c2`,
rebased on `main` at dispatch
(`9fd7e6c794b488663cc061ebfbde9192960758b2`). All three opt-level-0 jobs
succeeded on `ubuntu-24.04` with the same warm Cargo-input and coverage-tool
cache class; every cache-save row was the explicit
`workflow_dispatch-read-only` skip. The raw end-to-end coverage-phase totals
and medians are:

| Test opt level | Nextest threads | Sample 1 | Sample 2 | Sample 3 | Median | Result |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| 0 | 4 | 481 s | 384 s | 505 s | 481 s | candidate |
| 0 | 6 | 478 s | 381 s | 501 s | 478 s | eligible |
| 0 | 8 | 479 s | 381 s | 502 s | 479 s | eligible |
| 0 | 10 | 480 s | 379 s | 497 s | 480 s | eligible |
| 0 | 12 | 475 s | 381 s | 498 s | 475 s | fastest eligible median |
| 0 | 14 | 479 s | 381 s | 503 s | 479 s | eligible |
| 0 | 16 | 481 s | 382 s | 505 s | 481 s | eligible |
| 1 | 4 | 590 s | 921 s | 866 s | 866 s | rejected: all reports failed the 88.000% line gate |

The third sample was collected for every candidate, including the candidates
whose first two values varied by more than 10%. The fastest eligible median is
475 s at 12 threads. Every eligible opt-level-0 thread count is within 1.3% of
it, so the lower-complexity tie-breaker selects 4 threads as the production
candidate (481 s median). Optimization level 1 is not eligible: its three
nextest runs completed, but each coverage report failed the unchanged baseline.

These are intentionally described as current-main-derived candidate samples,
not `main`-ref samples: GitHub dispatch executes the workflow from the pull
request ref before it can merge. Do not treat them as the umbrella issue's
final `main` evidence or declare a final winner from them. After merge, collect
three comparable successful `main`-ref samples of the configured profile before
making that final claim.

### Production main-run evidence

A final production claim needs three comparable warm full-path successful
`push` runs on `refs/heads/main` after the relevant repair. Record each run's
direct URL and the results for every `rust-full shard N` cell, `rust-full
policy`, `rust-full LCOV tool`, `rust-coverage`, `rust-gate`, and
the selected producer's bootstrap integration step.
For every sample, link each producer's coverage-timing TSV and LCOV artifact,
and the merged LCOV artifact. Record each job duration, cache hit
or miss, restored bytes and restore time, compile time, cache-save outcome and
time, and end-to-end time. Keep warm exact hits separate from cold or miss
samples. Report raw values and medians. A pull-request or manual run does not
substitute for a production push.

The raw end-to-end record, including its explicit separation of controlled
main-ref dispatches from production pushes, is in
[`ci-latency-evidence.md`](ci-latency-evidence.md).

### Post-policy nextest-tail candidate evidence

[Benchmark run 31219903417](https://github.com/character-ai/larch/actions/runs/31219903417)
ran the post-policy implementation at
`c53d70c035b19ca9da4016fe9d7b6c14ccbe0394` on `ubuntu-24.04`. Each
opt-level-0 path ran the complete repository-policy scan before its unchanged
88.000% report. All three opt-level-0 jobs passed every policy and report path.
The raw nextest phase timings were:

| Nextest threads | Sample 1 | Sample 2 | Sample 3 | Median | Result |
| ---: | ---: | ---: | ---: | ---: | --- |
| 4 | 65 s | 72 s | 72 s | 72 s | eligible |
| 6 | 56 s | 67 s | 67 s | 67 s | eligible |
| 8 | 56 s | 65 s | 65 s | 65 s | eligible |
| 10 | 54 s | 65 s | 63 s | 63 s | eligible |
| 12 | 51 s | 64 s | 63 s | 63 s | eligible |
| 14 | 51 s | 63 s | 63 s | 63 s | eligible |
| 16 | 50 s | 63 s | 62 s | 62 s | fastest supported candidate |

The 16-thread median is the best result in the existing sweep. It is still two
seconds above the 60-second nextest acceptance target, so it is a production
candidate, not final `main` evidence. The configured 16-thread profile must
collect three comparable successful `main`-ref samples before that target can
be claimed.

An event-level profile from [run 31226975876](https://github.com/character-ai/larch/actions/runs/31226975876)
identified two obsolete live-repository command-registry tests in the nextest
tail: the CLI explicit-root test took 53.7 s and the command-registry report
test took 33.0 s. Their contracts are command routing and report rendering, so
they use isolated tracked fixtures. The policy coverage executable remains the
only full-repository policy execution and still runs `larch lint all` before
its coverage report.

The remaining parallelization work keeps each independent Git differential
family in its own test entrypoint without removing a success or failure case.
The clean-install matrix uses deterministic, isolated partitions that together
cover every route once.

Every coverage job publishes a compact `rust-coverage-timings-*` TSV artifact
and a GitHub step summary. The dedicated policy job and combined benchmark jobs
also publish a `rust-repository-policy-rule-timings-*` artifact. The coverage
TSV records cache restore, tool setup, profile cleanup, compilation, doctests,
tests, repository policy, report, plugin validation, each end-to-end total, and
cache-candidate staging. A role records explicit skip rows for phases owned by
another producer. The policy artifact has one deterministically ordered
`rule\tmilliseconds` table per policy path; it is written by the covered
`larch lint all` invocation before its report. Cache
candidate staging records an explicit validation-read-only skip outside an
eligible merge-group miss. The documented pre-consolidation run measured 3–8 s for cache
restore, 8–12 s for tool setup, 172 s for the former post-report repository
validation, and 0 s for the intentionally skipped cache publication. Those historical
timings are not comparable with the covered policy phase; use the policy
artifact for current per-rule evidence.

Cargo registry and Git inputs use a restore-only cache action in every Rust
lane. The validation workflow never saves a production cache. On an exact
primary-key miss, its successful `merge_group` lanes stage a bounded candidate
artifact; the separate trusted-main publisher can use it only after `main`
contains that exact merge-group source SHA. Pull requests and manual benchmark
dispatches therefore use the same restore cache class but cannot publish Cargo
inputs; none of the coverage lanes caches `target/` as a broad entry.

The coverage dependency cache is enabled with a reviewed 1,400,000,000-byte
dependency-only bound. Its versioned exact key includes the runner,
architecture, target triple, toolchain, manifests and lockfile, coverage-tool
version, selected compiler profile, feature mode, linker, Cargo configuration,
and schema. It has no coverage-target `restore-keys` fallback. A bound above
2 GiB needs explicit transfer-cost evidence in the activating pull request.

A pull request may restore only its exact default-branch cache but can never
save it. Production publication requires a successful, exact-source
merge-group validation, the resulting `main` SHA, a primary-key miss, a
completed candidate artifact upload, a passing size guard, and a verified
candidate manifest. Before staging, the workflow removes profile/report data
and workspace products from `target/llvm-cov-target`, then publishes its
directory inventory as a separate artifact. A cache hit never replaces the
coverage report, executable smoke test, repository policy, plugin validation,
or bootstrap integration.

The target-cache benchmark uses a separate
`coverage-target-deps-benchmark-*` key, never the production key. It runs only
from an explicitly selected `workflow_dispatch` on `refs/heads/main`; pull
requests and ordinary manual runs cannot restore or save it. Its first dispatch
uses a zero bound to publish the dependency-only inventory without saving. A
later dispatch must pass that measured byte bound, capped at 2 GiB, to seed the
benchmark cache. During that dispatch, the full shard and policy path stays
cache-off as the matched control and the benchmark lane is the warm candidate.
The benchmark key cannot activate or supply the production cache. Its timing
and inventory artifacts use the `-target-cache-benchmark` suffix, and its
verification executable uses a distinct artifact name, so they remain
distinguishable from the control artifacts while retaining upload cost.

This workflow does not garbage-collect GitHub Actions caches. Add that behavior
only after a repository cache inventory demonstrates quota pressure or useful
cache eviction; constrain any future deletion to this repository's versioned
Rust-cache prefixes, preserve current keys, run it only from a scheduled or
manual trusted event, and test its selection without a network mutation.

The optional `coverage_profile_runner` input permits the documented
`large_ubuntu_4cpu` availability trial without changing a required PR lane.
That runner remained unassigned for ten minutes in
[trial run 31143192232](https://github.com/character-ai/larch/actions/runs/31143192232),
so it was unavailable to this workflow and does not affect the comparison.

The lint lane may restore a manifest-keyed dependency cache under `target/debug`,
then removes workspace products with `cargo clean --workspace` before a
merge-group candidate can be staged. Pull requests do not publish that target
cache. The coverage execution lane caches only its pruned dependency-only
`target/llvm-cov-target` directory, never broad `target/`.

The current CI floor is 88.000% lines. It is a no-regression floor, not a
chosen repository target. Raise it when coverage improves. Lower it only with
a documented reason and issue. Coverage excludes
the shared test-support crate, `tests/` and `fixtures/` trees, and build scripts.
Keep that exclusion expression in the CI workflow so the coverage job stays
reproducible.

The Rust suite remains one workspace test lane while it is fast. When it needs
partitioning, shard by Cargo package rather than test-name or source-file
globs. Assign every package to exactly one test shard, keep `--all-features`
and `--locked` on each shard, and retain one unsharded workspace coverage run.
Use recorded CI duration to balance package groups. Keep `rust-gate` as the
stable aggregate check when the internal shard count changes.

`larch test-shard` owns deterministic LPT packing and the literal
single-physical-line `test-harnesses-N:` Makefile grammar. The harness
rebalancer reaches it through `scripts/larch.sh`; any future Rust CI partition
uses the same packer for Cargo package groups.
