# Linting

Larch uses [pre-commit](https://pre-commit.com/) as the source of truth for linter configuration. Linter definitions, versions, and file filters live in `.pre-commit-config.yaml`. CI adds dedicated per-tool jobs on top of pre-commit for the most safety-relevant checks (secret scanning, agent-config linting) so individual failures can be diagnosed and re-run independently.

## Linters

| Linter | File Types | Description |
|--------|-----------|-------------|
| [shellcheck](https://www.shellcheck.net/) | `.sh` | Shell script analysis, supplied by the pre-commit hook env through pinned `shellcheck-py==0.10.0.1` |
| [markdownlint](https://github.com/igorshubovych/markdownlint-cli) | `.md` | Markdown style enforcement (config: `.markdownlint.json`). `larch-logs/` is excluded via `.markdownlintignore` — those files are runtime artifact archives, not authoring-quality docs. |
| [jq](https://jqlang.github.io/jq/) | `.json` | JSON syntax validation |
| [actionlint](https://github.com/rhysd/actionlint) | `.yml`, `.yaml` | GitHub Actions workflow validation |
| [rustfmt](https://github.com/rust-lang/rustfmt) | `.rs` | Workspace formatting through `make rust-fmt`, using the toolchain pinned in `rust-toolchain.toml`. |
| [Clippy](https://github.com/rust-lang/rust-clippy) | Changed Rust packages and targets | The default pre-commit hook calls the Rust-owned `scripts/larch.sh checks rust-clippy` with the absolute repository root and changed paths. It uses locked Cargo metadata to select only the affected default production targets or integration-test, example, benchmark, and binary targets. A shared target-family module selects every target in that family. It runs one default-feature Clippy configuration with warnings denied, incremental compilation off, and dev/test debug information off. CI separately runs `make rust-clippy` with its exhaustive all-target/all-feature contract. |
| [cargo-deny](https://github.com/EmbarkStudios/cargo-deny) | `Cargo.lock`, Cargo manifests | CI checks advisories, allowed licenses, duplicate versions, wildcard requirements, and dependency sources through `deny.toml`. |
| [agnix](https://github.com/agent-sh/agnix) | `SKILL.md`, `CLAUDE.md`, agent configs | AI agent configuration linting (config: `.agnix.toml`). The full repository scan is manual locally; the dedicated CI job runs it in strict mode. |
| [lintlang](https://github.com/hermes-labs-ai/lintlang) | `agents/`, `.claude/agents/`, `skills/`, `.claude/skills/` | Static language checks for agent and skill prompts. CI runs the pinned release in the consolidated `agent-lint` job, reports HIGH and CRITICAL findings, and fails on either severity. |
| Agent tool contract | `agents/*.md`, `.claude/agents/*.md` | The pinned `agent-lint` rules A012 and A013 run two independent checks: (1) rejects agent frontmatter that explicitly restricts tools without `Read` while the prompt body instructs the agent to read files, bundles, paths, diffs, artifacts, markdown, or logs; (2) rejects prompts that pair read intent with a machine-parsed-only JSON or JSONL output mandate but carry no fail-closed instruction for unreadable evidence. |
| Tier-1 instruction import size | `AGENTS.md`, `KARPATHY_CLAUDE.md`, `BASH_AUTHORING.md` | Agent-lint D004 enforces the distinct path-specific caps in `agent-lint.toml` for the root imports loaded by `CLAUDE.md`. |
| Shell contract rules | runtime `.sh` and shell test harnesses | Rust `larch-lint` rules reject post-`larch_quiet_init` `echo`/`printf`/`cat >&2` and require harnesses to clear inherited session state or carry a reason-bearing suppression. They run through `make rust-lint` in CI and the explicit manual pre-commit stage. |
| Residual Bash shim boundary | Tracked production `.sh` files | `larch lint rule residual-bash-shim` requires each production shell script to be listed in `scripts/residual-bash-paths.txt` or be an at-most-25-line shim whose only operational command is an `exec` of `scripts/larch.sh`. Fixtures and `test-*.sh` harnesses are excluded. The rule has no baseline or suppression. Distributed rule registration includes it in `make rust-lint`, local `make lint`, and each CI `lint all` policy lane. Focused coverage lives in `crates/larch-lint/tests/residual_bash_shim.rs`. |
| Inline `gh --body` / `--notes` | `.sh`, `.py` | The pinned `agent-lint` rule G008 rejects inline `--body` / `--notes` argv in shell and Python argv-list forms; use `--body-file` / `--notes-file`. This is the backstop for GitHub CLI body-like payloads; see `BASH_AUTHORING.md` §4 for authoring guidance. |
| Dead path pointers in Tier-1 docs | `AGENTS.md`, `SECURITY.md` | The pinned `agent-lint` rule D005 rejects fence-outside inline-backtick tokens that start with an approved repo prefix (`skills/`, `scripts/`, `docs/`, `hooks/`, `agents/`, `.claude/`, `.claude-plugin/`, `.github/`), contain `/`, and contain none of the placeholder characters `< > * $ { } ?` or whitespace, when the stripped file path does not exist under the repository root. The approved prefix set is configured in `agent-lint.toml` under `inline-path-prefixes`. Fenced code is skipped. There is no baseline; existing violations must be fixed. Run explicitly with `make agent-lint` or the manual pre-commit stage; CI runs `agent-lint`. |
| Security reference packaging | Root `SECURITY.md`, `ARCHITECTURE.md`, `docs/security/*.md`, linked service inventories, shipped `skills/**/*.md`, and generated projection output | CI runs `scripts/larch.sh release plugin-runtime --output "$RUNNER_TEMP/plugin"` through the verified policy executable to generate and validate the complete runtime projection. Generation requires the root policy, security index, linked architecture and service-owner references, and every tracked focused security reference. It rejects a shipped skill reference to an absent `docs/security/*.md` target. Focused Rust unit coverage lives in `crates/larch-cli/src/release_plugin_runtime.rs`. |
| Rust policy rules | Rust workspace, Markdown, skills, catalog docs, `ARCHITECTURAL_GUIDELINES.md`, and run-log/topology paths | `cargo run --locked --package larch-cli -- lint all` runs the registered repository rules reported by `scripts/larch.sh lint rules`. `command-registry` validates the final Rust or retired owner, exact implementation leaf, machine-stdout contract, clean-install fixture, and production caller inventory for every command; its workflow is in [`docs/rust-command-registry.md`](rust-command-registry.md). `topology-rule-paths` validates the topology TSV grammar, containment, content, and tracked authorities. `developer-tooling-crate-process` rejects developer-tooling spawns of crate-owned CLIs outside reviewed exceptions, while the permanent `developer-tooling-rust-owned-python` rule rejects restoration of retired dispatcher calls. `readability-preamble` validates shared-style directives for shipped skills and reviewer agents. Layering uses Cargo metadata to enforce the product graph in [`ARCHITECTURE.md`](../ARCHITECTURE.md), `workspace-dependency-policy` rejects member-local dependency versions and features, and Rust tests must be crate-local `#[cfg(test)]` modules or integration tests. Focused rule coverage lives under `crates/larch-lint/tests/`. |
| Verified Rust runtime entrypoint | Production Rust, skills, agents, hooks, and residual scripts | `larch lint rule larch-runtime-entrypoint` rejects direct `bin/larch` callers and the retired Rust-to-`python/cli.py` bridge APIs and path construction. Production commands must enter through `${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh` so the matching executable is verified. Normal first use may install it; short-timeout deny, advisory, maintenance, audit, Stop-boundary, and anti-read-poll hooks select the verified no-install path. Only `scripts/larch.sh` may reference the installed binary directly. Generated `plugin/` copies and test-only scripts are excluded because canonical sources own enforcement. The rule has no baseline or suppression. It runs through `make rust-lint` and local `make lint`; coverage lives in `crates/larch-lint/tests/larch_runtime_entrypoint.rs`. |
| Release and upgrade retirement boundary | Release and upgrade registry rows, callers, implementations, and asset workflow | `larch lint rule release-python-free` pins the final #7674 command set and clean-install fixtures. It rejects restoration of retired runtime registrations, implementations, or selectors; direct-binary callers; and direct `gh` processes in release implementations. `scripts/larch.sh` and the asset workflow's newly built binary are the executable exceptions. The no-baseline rule runs through `make rust-lint`; tests: `crates/larch-lint/tests/release_python_free.rs`. |
| Session and background-job retirement boundary | Final #7677 registry rows and retired command surfaces | `larch lint rule state-python-free` requires all pinned #7677 rows to remain Rust-owned and assigned to exact implementation leaves. It rejects revived dispatch, entrypoints, tests, or modules from the retired runtime. Tests: `crates/larch-lint/tests/state_python_free.rs`. |
| Closed #7683 command and mutation boundary | Final reporting, run-log, progress, timing, and rendering owners | `larch lint rule reporting-python-free` pins the closed #7683 command set and exact implementation leaves. It rejects restoration of retired registrations, modules, mutation helpers, or callers. Production mutations must use the Rust `run-log manifest`, `progress`, and `timing` owners. The no-baseline rule runs through `make rust-lint`; tests: `crates/larch-lint/tests/reporting_python_free.rs`. |
| Closed #7682 issue-domain boundary | Final issue-domain registry rows and tracking and execution workflow callers | `larch lint rule issue-python-free` pins the issue-domain command set, exact implementation leaves, and receiving-umbrella handoffs. It rejects restored retired-runtime issue modules or command equivalents and a restored Bash refresh implementation. `larch-runtime-entrypoint` separately enforces `scripts/larch.sh`. The no-baseline rule runs through `make rust-lint`; tests: `crates/larch-lint/tests/issue_python_free.rs`. |
| Closed #7679 review boundary | Final review and calibration rows, receiving-umbrella handoffs, and production callers | `larch lint rule review-python-free` pins the review and calibration selectors, exact implementation leaves, Rust ownership, and receiving owners. It rejects restoration of the retired review package or live references to it. Historical documentation and test fixtures remain outside the live-caller check. The no-baseline rule runs through `make rust-lint`; tests: `crates/larch-lint/tests/review_python_free.rs`. |
| Closed #7678 vendor-agent boundary | Final vendor-agent rows and live runtime or tooling references | `larch lint rule agent-python-free` pins the vendor-agent, external-default, and Slack selectors to Rust ownership and exact implementation leaves. It rejects registry drift, restoration of the retired agent package or dispatch, and live runtime or tooling references. The no-baseline rule runs through `make rust-lint`; tests: `crates/larch-lint/tests/agent_python_free.rs`. |
| No tracked Python source | Every tracked path | `larch lint rule no-tracked-python-source` fails when any tracked path ends in `.py`. After the Rust migration the six `*-python-free` boundary rules and `python-boundary` remain domain tripwires; this rule is the single global backstop that replaced the per-rule `.py` scanners removed once `python/` was deleted and no Python source remained. The rule has no baseline or suppression. It runs through `make rust-lint`; coverage lives in `crates/larch-lint/src/rules/no_tracked_python_source.rs`. |
| Typed issue-mutation ownership | Skills, agents, hooks, residual scripts, and non-owner Rust crates | `larch lint rule issue-mutation-owner` rejects raw issue-edit helpers, direct `gh issue edit` argv, issue REST `PATCH` requests, GraphQL issue title/body/label mutations, and raw Rust service writes outside `larch_adapters::github::IssueMutationOwner`. Generated `plugin/` copies and bounded test or fixture roots are excluded because canonical sources own enforcement. The rule has no baseline or production suppression. Run `cargo test --locked --package larch-lint --test integration issue_mutation_owner::` for focused coverage. |
| Production Cargo and target execution | Skills, agents, hooks, and residual scripts | `larch lint rule production-cargo-run` rejects commands that execute `cargo run`, `cargo install`, or `target/{debug,release}/larch`. Production runtime must use `scripts/larch.sh`. The rule inspects shell commands, executable Markdown fences, agent command examples, and hook command fields. Development docs and make targets, CI workflows, release construction, and recognized test fixtures stay outside its production scope. Prose and comments do not count as execution. The rule has no baseline, suppression, or runtime exemption. It runs through `make rust-lint` and local `make lint`; coverage lives in `crates/larch-lint/tests/production_cargo_run.rs`. |
| Service adapter ownership | Production Rust, service inventories, skills, and residual scripts | `larch lint rule service-ownership` confines concrete GitHub and Google clients, service request hosts, and GraphQL documents to `crates/larch-adapters`; rejects duplicate clients, generic GitHub credential fallback, `gcloud`, and service-credential child environments; and validates the machine-readable GitHub operation matrix against the command registry. Matrix omissions, duplicate operation rows, #7687 placeholders, missing adapter paths, and false owner or cutover claims fail without a baseline. The clean-install `gh` bootstrap in `scripts/larch.sh` is a separate installer surface. Inline suppression uses `lint-service-ownership: ok <reason>`. Focused coverage lives in `crates/larch-lint/tests/service_ownership.rs`. |
| Git operation ownership | Production Rust, skills, agents, hooks, scripts, Makefile, workflows, and the command registry | `larch lint rule git-ownership` compares live Git surfaces with `docs/git-operation-inventory.md`, confines concrete `gix` use to `larch-adapters`, and rejects duplicate or arbitrary Git owners. Syntax-aware Rust checks cover adapter-local generic runners, constructor and process aliases, qualified constructors, constant or variable Git executables, generic argv forwarding, raw process requests, and suppression attempts. The rule pins the closed #7671 operation enum, typed request families, public `GitCli` methods, and final #7675 command rows. `#[cfg(test)]` and `larch-test-support` fixture oracles plus the lint discovery bootstrap are explicit bounded non-production exceptions. The rule has no baseline or production suppression. Focused coverage lives in `crates/larch-lint/tests/git_ownership.rs`. |
| Rust duplicate-code | Tracked production `*.rs` sources | `larch lint rule duplicate-code` (also under `make rust-lint` / `cargo run --locked --package larch-cli -- lint all`) reports exact normalized cross-module clone families of at least 50 tokens. Normalization drops comments and formatting via token lexing, skips imports, generated-file markers, test paths, `#[cfg(test)]` items, `#[test]` functions, and `impl Rule for …` boilerplate, and carries no baseline. Survey notes live in `docs/rust-dependency-survey.md`. Coverage lives in `crates/larch-lint/src/rules/duplicate_code.rs` and `crates/larch-lint/tests/duplicate_code.rs`. |
| Static refusal-token fan-out | Production Rust under `crates/*/src/` | `larch lint rule static-token-refusal-fanout` flags a function that returns one public `&'static str` refusal-newtype constant from at least three distinct sites. Supported sites are `Err`, `ok_or`, `ok_or_else`, and `map_err`. Use `lint-static-token-refusal-fanout: ok <reason>` on an intentional site. The reason-bearing migration ledger grandfathers only live `(path, function, constant)` rows and rejects stale rows. Focused coverage lives in `crates/larch-lint/tests/static_token_refusal_fanout.rs`. |
| Raw run-log cache walkers | Rust production sources | Rust `larch lint rule run-log-corpus-walkers` rejects raw synchronized-corpus walkers and copied classification triple-globs outside `crates/larch-core/src/report/run_log_corpus.rs`. Fixed per-run artifact reads and validated-run recursive inspection through the shared Rust helpers are allowed. It runs once through `make rust-lint`; focused coverage lives in `crates/larch-lint/tests/run_log_corpus_walkers.rs`. |
| Universal skill run lifecycle | `skills/*/SKILL.md`, `.claude/skills/*/SKILL.md`, `skills/shared/run-lifecycle-ownership.tsv` | Rust `larch lint rule skill-run-lifecycle` inventories every public and dev-only skill, requires one exact `shared-v1` declaration under its directory name, the matching generic or specialized mandatory instruction, and Bash access when `allowed-tools` constrains the skill. For externally owned lifecycle prompts, the required instruction forbids the shared generic commands and routes parent context only through Step 0 to the registered start owner. The rule rejects migration markers, validates the generic and specialized start and terminal owners in the registry, pins all terminal verbs, and rejects direct archive publisher calls outside the shared lifecycle boundary. It runs once through `make rust-lint`; focused coverage lives in `crates/larch-lint/tests/skill_run_lifecycle.rs`. |
| Live skill prompt structure | Paths declared in `crates/larch-lint/config/skill-structure-pins.jsonl` | Rust `larch lint rule skill-structure` evaluates the live contains, absence, count, order, line, proximity, and path contracts migrated from the retired test suite. It is separate from `skill-documentation`: that rule owns catalog identity, while this rule owns exact prompt structure and reports the manifest pin ID. Focused coverage lives in `crates/larch-lint/tests/skill_structure.rs`. |
| Bash 3.2 portability | paths in `scripts/agent-lint-script-inventory.txt` | Agent-lint G010 is promoted to an error and scans the explicit inventory on every invocation, including pre-commit runs, independent of global exclusions. It rejects Bash 4+ constructs such as associative arrays, namerefs, `mapfile`/`readarray`, case-conversion expansions (`${var^^}` / `${var^}` / `${var,,}` / `${var,}`), `&>>`, coprocs, unsafe empty-array expansions, and direct `command <grep-family>` probes in `if`/`elif` conditions. Run via `make agent-lint` and CI's dedicated `agent-lint` job. |
| Bash 3.2 non-ASCII expansion boundary | tracked `.sh` and `.inc.bash` files plus Bash, sh, and shell Markdown fences | Rust `larch lint rule bash32-nonascii-expansion` rejects a bare `$name` immediately followed by a non-ASCII character. Use `${name}` to make the expansion boundary explicit. Single-quoted literals, comments, braced expansions, and ASCII suffixes are accepted. Suppress a reviewed fixture on the same line with `lint-bash32-nonascii-expansion: ok <reason>`. Focused coverage lives in `crates/larch-lint/tests/bash32_nonascii_expansion.rs`. |
| Renderer substitution safety | paths in `scripts/agent-lint-script-inventory.txt` | Agent-lint G009 rejects unsafe `${var//pattern/$replacement}` substitutions, including reason-bearing waivers and heredoc exclusions. |
| SKILL.md flag signatures | `skills/**/SKILL.md` plus target `.sh` scripts | The pinned `agent-lint` rule S059 scans fenced shell invocations in public skill prompts, assembles multiline commands, and verifies each `--flag` has a matching case arm in the shipped target script. Run explicitly with `make agent-lint` or the manual pre-commit stage; CI runs `agent-lint`. |
| SKILL.md awk field references | `skills/*/SKILL.md`, `.claude/skills/*/SKILL.md` | The pinned `agent-lint` rule S060 scans fenced `bash`, `sh`, and `shell` prompt snippets and rejects bare awk `$<digit>` field references. Prompt rendering may strip `$1`/`$2`; move KV parsing behind a typed `scripts/larch.sh` command. Run explicitly with `make agent-lint` or the manual pre-commit stage; CI runs `agent-lint`. |
| Prompt-source closure budgets | `design`, `implement`, `review`, and `panel-tier` prompt sources | Agent-lint S062 enforces tracked root, transitive-closure, conditional, line, estimated-token, and content-token caps from `agent-lint.toml`. Report deterministic measurements with `agent-lint --closure-report .`. |
| Em dash output literals | `skills/**/*.md`, `agents/**/*.md`, and Rust source | Rust `larch lint rule em-dash-output` rejects U+2014 in user-facing Markdown templates and Rust output macros. Markdown quotes and fenced content remain excluded. Suppress rare intentional literals with `lint-em-dash-output: ok <reason>`. It runs once under `make rust-lint`; coverage lives in `crates/larch-lint/tests/em_dash_output.rs`. |
| Unwired `codex exec` call sites | Rust, `scripts/*.sh`, `skills/*/scripts/*.sh`, and Bash/SH/Shell Markdown fences | Rust `larch lint rule codex-exec-auth` rejects raw `codex exec` unless the line carries `# lint-codex-exec-auth: ok <reason>`. Production callers must use `scripts/larch.sh agent launch-codex-exec`. It covers shell, Markdown, and Rust command builders and runs once under `make rust-lint`; coverage lives in `crates/larch-lint/tests/codex_exec_auth.rs`. |
| Timing task-kind allow-list | Rust, skill Markdown, and skill shell scripts | Rust `larch lint rule timing-task-kind-allowlist` scans Rust arrays and builders, Clap defaults, Markdown, and shell literals. It validates static values against `crates/larch-core/src/report/timing.rs`'s `TIMING_TASK_KINDS_ALLOWED` and validates static fallbacks in supported environment lookup forms. Dynamic environment values retain the runtime timing-token grammar check. The rule runs under `make rust-lint` and is covered by `crates/larch-lint/tests/timing_task_kind_allowlist.rs`. |
| Consecutive Bash tool-call fences | `skills/*/SKILL.md`, `.claude/skills/*/SKILL.md`, `skills/*/references/*.md` | Agent-lint S021 rejects consecutive Bash fences while preserving reviewed reason-bearing suppressions and documented carve-outs. |
| Bare top-level grep in orchestrator markdown | `skills/**/*.md`, `.claude/skills/**/*.md` | The pinned `agent-lint` rule S061 scans fenced `bash`, `sh`, and `shell` blocks and rejects bare top-level `grep` wrapper-trap shapes, no-path grep-family probes, and parent-directory ascents in grep-family path operands. Run explicitly with `make agent-lint` or the manual pre-commit stage; CI runs `agent-lint`; see `BASH_AUTHORING.md` §1 for authoring guidance. |
| Non-ASCII bytes in dynamic awk regex | paths in `scripts/agent-lint-script-inventory.txt`, including standalone `.awk` helpers | Agent-lint G011 is promoted to an error and scans the explicit inventory on every invocation, including pre-commit runs, independent of global exclusions. It rejects non-ASCII `awk -v VAR=value` values and non-ASCII regex contexts in `match`, `gsub`, `sub`, `split`, `~`, and `!~`; display-only strings remain legal. Suppress fixtures only with a trailing `# lint-awk-multibyte-regex: ok <reason>` pragma. Run via `make agent-lint` and CI's dedicated `agent-lint` job. |
| [gitleaks](https://github.com/gitleaks/gitleaks) | all tracked files | Secret detection (pre-commit + dedicated CI job, full-history). Path allowlist in `.gitleaks.toml`. See the [canonical scanner model](security/artifacts-redaction-and-publication.md#secret-scanning-layers). |

## Migration Governance Aggregate

Run `"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" issue migration-audit --repo owner/name --chief 7687`
to compose the migration checks into one stable JSON report and count table. The
Rust aggregate runs the canonical lint owners in process for command-registry,
caller-surface, retired-runtime regression, clean-install, and production-runtime
checks. It exits `2` when required evidence is unavailable. Production callers
enter only through the verified bootstrap. See
[Migration Governance Audit](migration-governance.md) for the schema, channels,
exit codes, and read-only boundary.

Run the focused Rust command coverage with:

```bash
cargo test --locked --package larch-cli --test integration migration_audit_commands::
```

## Usage

### Rust checks

Install the pinned toolchain with [rustup](https://rustup.rs/). The
`rust-toolchain.toml` file installs the required `rustfmt` and Clippy
components. Run `make rust-check` after Rust changes. It discovers the changed
paths and invokes the same bounded targeted-Clippy driver as pre-commit. It does
not build every target, run tests, or create coverage artifacts. CI owns the
exhaustive lint, build, test, dependency-policy, and coverage lanes behind the
stable `rust-gate`. See
[`docs/rust-dependency-survey.md`](rust-dependency-survey.md) for crate choices
and [`docs/rust-testing.md`](rust-testing.md) for test boundaries.

### Developer-tooling guards

`larch lint` rules `developer-tooling-rust-owned-python`,
`developer-tooling-crate-process`, and `developer-tooling-7685-closure` watch
`Makefile`, `.pre-commit-config.yaml`,
`.github/workflows/*.{yml,yaml}`, composite `.github/actions/*/action.{yml,yaml}`,
non-test `scripts/*.sh`, non-test, non-fixture runtime scripts named by
`scripts/residual-bash-paths.txt`, and retained developer-only
`.claude/skills/**/*.{md,sh}` for ownership drift:

- `developer-tooling-rust-owned-python` fails when those surfaces still invoke
  the retired dispatcher for a selector the command registry marks Rust-owned,
  including prompt-authored commands.
- `developer-tooling-crate-process` uses the maintained shell, Markdown, and
  supplementary-script syntax readers to reject static `git`, `gh`, and
  `gcloud` child processes. Workflow and pre-commit command fields are parsed
  as shell commands. Approved external products and the rule's narrow release,
  bootstrap, credential, and residual-script exceptions remain allowed.
- `developer-tooling-7685-closure` independently checks every #7685
  command-registry row and exact leaf, rejects restoration of retired
  entrypoints or callers, validates #7685 GitHub-service ownership, and asks
  the `git-ownership` rule's canonical inventory parser for unresolved rows.

Focused coverage lives in `crates/larch-lint/tests/developer_tooling_guards.rs`.

For command-registry changes, run the focused integration suite and live-ledger
validation:

```bash
cargo test --locked --package larch-lint --test integration command_registry::
cargo nextest run --locked --package larch-cli --test integration -E 'test(/^parity::/)'
cargo run --quiet --locked --package larch-cli -- lint rule command-registry
```

The focused lint validates `clean_install_test` references against the shared
parity matrix. Use `larch lint command-registry audit --input INPUT.json` for
typed issue-to-registry parity evidence.

### Analytics migration closure

`analytics-7684-closure` independently proves that every #7684
command-registry row has its final owner and exact leaf, no retired-runtime
registration, entrypoint, or production caller is restored, and the canonical
GitHub-service and Git inventory parsers have no unresolved #7684 rows. It
also parses the shipped `/fluff-analysis` and `/voter-calibration` command
fences, requiring their `scripts/larch.sh` Rust owner and rejecting either
retired standalone analyzer. Missing or malformed ledger evidence is an
error. `release plugin-runtime --output "$PWD/target/plugin-runtime-check"`
generates and validates the plugin projection of those prompt sources.

For analytics #7684 closure changes, run:

```bash
cargo test --locked --package larch-lint --test integration analytics_7684_closure::
cargo run --quiet --locked --package larch-cli -- lint rule analytics-7684-closure
cargo run --quiet --locked --package larch-cli -- release plugin-runtime --output "$PWD/target/plugin-runtime-check"
```

### SKILL.md closure growth ratchet

Agent-lint's `--closure-report` emits the same named groups as stable JSON rows. Changes to a closure budget are explicit reviewed edits to `agent-lint.toml`; there is no generated baseline.

Scope is narrow by design:

- Count only `design`, `implement`, and `review` skill closures plus the fixed `panel-tier` source set.
- Ratchet eager closure for all four targets. Ratchet conditional closure for the declared `conditional-sources` of `design`, `implement`, and `review`.
- Count each skill `SKILL.md` plus direct always-loaded prompt-source references.
- Do not recurse from a referenced file into its references.
- Track conditional bullets, branch-only routing-table rows, other route-predicate contexts, and the `/implement` `Checks Failure Entry Macro` and `Durable Bail to Step 18 Macro` sections as conditional closure.
- Support non-markdown prompt sources only for session-start `step-name-registry.tsv` reads under `skills/*/scripts/`.
- Count only four narrow eager phrase patterns beyond mandatory/direct read clauses: `session-setup-output.md` setup use, `external-reviewers.md` procedure use, session-start `step-name-registry.tsv` reads, and `final-summary-emit.md` follow instructions.
- Harvest only the matched directive clause from matching lines. Later citations, harness docs, and unrelated non-markdown references on the same line do not enter the closure.

### Policy-rule details

Rust `larch lint rule kv-codec` rejects new production loops that split raw
`KEY=value` rows in Rust, shell `awk -F=` / `cut -d=` readers, and private
emitters. Use the shared Rust codec or `scripts/larch.sh kv get` in Bash;
option and tab parsing are outside this
narrow rule. It has no baseline and runs once through `make rust-lint`.

`larch lint rule prefix-case-variant` scans regular Markdown under `skills/`, `.claude/skills/`, and `agents/`, plus paths declared in `scripts/residual-bash-paths.txt`. It flags bracketed tokens whose case-insensitive form matches a canonical lifecycle or bug token but whose original bytes differ (for example `[Bug]` or `[bug]`). Exact-case tokens such as `[BUG]`, `[DONE]`, and `[STALLED]` remain allowed. Suppress one line with a trailing non-empty reason: Markdown `<!-- lint-prefix-case-variant: ok <reason> -->`, Bash `# lint-prefix-case-variant: ok <reason>`. Empty-reason pragmas are ineffective. There is no baseline or `--write` mode. Intentional residual-Bash fixture variants, including paired assertion messages that embed legacy wrong-case tokens, need line-local reason-bearing suppressions. The rule runs through `make rust-lint` and local `make lint`; coverage lives in `crates/larch-lint/tests/prefix_case_variant.rs`.

The agent tool-contract checks (rules A012 and A013) are owned by the pinned `agent-lint` release. They scan `agents/*.md` and `.claude/agents/*.md` non-recursively: A012 fails when an explicit empty or Read-less tool list is paired with read-file, open-file, or use-Read instructions in the body, and A013 fails when read intent coexists with a machine-parsed-only JSON or JSONL output mandate and the body has no fail-closed instruction for unreadable evidence. Granting `Read` disarms only A012. There is no baseline by policy.

`larch lint rule guideline-no-exception` scans `ARCHITECTURAL_GUIDELINES.md` for entries whose deviate bullet starts with `n/a` or `never`, while ignoring Markdown fences. The baseline file is `crates/larch-lint/config/guideline-no-exception-baseline.json`, with rows shaped exactly as `{guideline_id, reason}`. New live findings fail, baselined live findings warn, and stale rows fail so the baseline shrinks when an entry is promoted or gets a real deviate clause. There is no `--write` mode or regen target; edit the baseline by hand.

There are four pre-commit-driven paths:

- **CI**: The `lint` job runs `make lint-only` (repo-wide pre-commit over all files) with dedicated-job hooks skipped. `agent-lint`, `agnix`, and `lintlang` share the consolidated `agent-lint` job; `shellcheck` and `gitleaks` have dedicated jobs. CI owns every exhaustive Rust operation: `rust-lint` runs format and Clippy, `rust-deny` runs dependency policy in parallel, and the parallel coverage jobs own the full workspace tests, doctests, coverage, repository policy, plugin projection validation, Linux executable, and bootstrap integration checks. `rust-full-lcov-tool` prepares the pinned LCOV runtime while those producers run. The required `rust-coverage` job validates the selected execution topology and owns the full-mode LCOV merge and line gate. The required `rust-gate` job runs independently after `rust-lint`, `rust-deny`, and the raw execution producers, so it does not add another runner hop after `rust-coverage`. CI runs regression harnesses through the two-cell `test-harnesses` matrix (`make test-harnesses-1` and `make test-harnesses-2`) instead of one serial harness job. Non-secret-scan jobs use sparse checkouts. `gitleaks` and `trufflehog` keep full source history. Remote run archives pass the run-log scrubber before publication and are not fetched into CI. Local `make lint` runs regression harnesses, Rust policy rules, and pre-commit. CI also runs `contains-pins`.

  Pull requests, merge groups, and manual dispatches run this validation
  workflow with cache restores only. A normal `main` push runs the separate
  `Main cache publication` workflow: it populates exact cache misses but does
  not rerun lint, tests, coverage, or secret scans. Its cache inventory and
  candidate-verification boundary are documented in
  [Supply Chain, Credentials, and Services](security/supply-chain-credentials-and-services.md#ci-tool-bootstrap-and-caches).

  Pull requests run the `rust-selection` observation job before the Rust
  lanes. It runs the selector from the trusted pull-request base and publishes
  a redacted proposed/effective decision artifact. The recorded independent
  observation windows promote partial and skip enforcement to `true`.
  `partial` retains selected package tests, Clippy, doctests, candidate-built
  repository policy, plugin validation, and bootstrap integration. `skip` keeps
  non-Rust owners and uses a checksum-verified, input-keyed trusted-main policy
  executable. Missing trust evidence falls back to `full`. The stable
  `rust-coverage` status aggregates the one effective execution path, while
  the merge queue remains the full per-merge backstop.
  See [Rust testing](rust-testing.md) for ownership, cache identity, the
  `full-rust-ci` escape hatch, and the recorded observation window.
- **Relevant checks CLI (`scripts/larch.sh checks run-relevant`)**: The Rust dispatcher finds branch, staged, unstaged, and untracked changes; filters existing regular files for `pre-commit run --files`; and runs the contains-pin scanner. For Rust paths, the filename-aware hook selects and logs the exact Cargo packages and targets, then runs one bounded Clippy configuration. A deleted or otherwise non-regular Rust path skips that hook and uses the same bounded entry point once as a compatibility fallback. A missing proof marker fails closed. The CLI never follows pre-commit with `make rust-check`, `cargo check`, a full-repository `agent-lint`, tests, or coverage. A no-change run is a fast freshness check. `/implement` and `/review` use the CLI to capture verbose output under the session tmpdir and emit a one-line `RELEVANT_CHECKS_OK=true ...` green-path envelope when checks succeed. The default path fails closed on structural errors; `RELEVANT_CHECKS_SKIPPED=true` is reserved for explicit `--allow-skip` test paths. On failure, orchestrators read `DIGEST_FILE` first when the helper envelope includes it, then fall back to `REDACTED_LOG_FILE`; folded composite stdout may place those keys after leading file or git KVs, so consumers must scan the full composite stdout for both keys. `REDACTED_LOG_FILE` remains the full-log fallback and repair-loop input.
- **Local git hook** — Run `make setup` (or `pre-commit install`) to enable pre-commit hooks on every commit. Bypassable via `git commit --no-verify`; the CI jobs are the enforced backstop.
- **Manual pre-commit stage** — Run `pre-commit run --hook-stage manual --all-files` only when deliberately requesting repository-wide policy, agent/config, secret, or type scans. It is not part of relevant checks or the default git hook.

`.github/workflows/requirements-lint.txt` pins the pre-commit environment used
by CI lint jobs. That utility environment does not run or test larch runtime
code. `agent-sync` restores the exact
trusted Cargo-input and lint-dependency caches, then runs `make agent-sync`.

## Shellcheck Engine

The shellcheck pre-commit hook is local so it can run one file per shellcheck process through `scripts/pre-commit-shellcheck.sh` and `xargs -P`. The hook still uses the same pinned engine family as the old upstream hook: `shellcheck-py==0.10.0.1` in `.pre-commit-config.yaml` `additional_dependencies`.

A local `shellcheck` binary on PATH is no longer required for `make lint`, `make shellcheck`, or the relevant-checks script path above; pre-commit provides the binary inside the hook environment. Installing shellcheck directly remains useful only for ad-hoc debugging outside pre-commit, for example with `brew install shellcheck` on macOS or `apt-get install shellcheck` on Linux.

When adding a new pre-commit hook, decide explicitly whether `lint`, the dedicated `shellcheck` job, and `scripts/larch.sh checks run-relevant` should run or skip it. The dedicated `shellcheck` job should continue to run only the `shellcheck` hook.

## CI sharding of `test-harnesses`

`make test-harnesses` remains the local umbrella target and runs every regression harness wired into the `test-harnesses-N` shards plus the partition guard (`make test-harness-shards-coverage` reports the active inventory; the `Makefile` is the source of truth). CI fans the same inventory out across two parallel matrix cells named `test-harnesses (1)` and `test-harnesses (2)`, each invoking `make test-harnesses-N`.

The shard lists live directly in `Makefile`. Harness shards contain only **direct Bash leaves**: recipe-bearing `test-*` targets whose complete recipe is Bash-harness work, plus valid non-`test-*` `*-bash-harness` leaves such as `write-final-report-bash-harness`. Cargo-focused Make targets remain available for local debugging, while the `rust-full-shards` jobs own their required full-workspace nextest coverage; see [Bash-shard Cargo target ownership](rust-testing.md#bash-shard-cargo-target-ownership). Public aggregates and language-specific leaves stay outside the shards. The skill-structure aliases invoke the Rust `skill-structure` rule and likewise remain standalone. `make test-oos-disposition-gate` now runs only the thin-wrapper delegation smoke; OOS gate and checkpoint behavior is Rust-owned by `crates/larch-cli/src/oos_commands.rs` and `crates/larch-core/src/issue/oos_disposition.rs`, with full-workspace coverage in the `rust-full-shards` jobs. Rebalance shards by measured CI timing with `/rebalance-tests --kind harness` (see Refreshing shard balance below). New Bash leaves must be assigned to exactly one `test-harnesses-N:` prerequisite list; `make test-harness-shards-coverage` checks for missing, orphaned, non-leaf, duplicated, or non-standard entries. The matrix uses `fail-fast: false`, so both shards finish even after one fails. This spends more CI minutes but preserves complete diagnostics.

Local ordering changed: under `make test-harnesses` and therefore `make lint`, harnesses execute in shard order (`test-harnesses-1`, then `test-harnesses-2`), not in the old single prerequisite-list order. Direct `make test-X` invocations are unchanged. CI shards run on separate VMs; local `make -j20 test-harnesses` can run shard targets concurrently, so fixed `/tmp` paths in individual harnesses remain a local-parallelism limitation even though the CI split is isolated.

The workflow has exactly two non-empty cells. The Rust-hook leg stays isolated
for focused failure diagnosis, while the other leg owns every remaining direct
Bash leaf. Rebalance only when complete `larch ci-timing harness` evidence
shows material, non-local imbalance.

### Refreshing shard balance

Rebalance the harness shards by measured CI timing with the `/rebalance-tests`
dev skill (`.claude/skills/rebalance-tests/`):

```bash
"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" rebalance-tests run --kind harness
```

Harness mode builds a startup- and affinity-aware cost model before packing the
`test-harnesses-N` shard lists. It combines `LARCH_HARNESS_TIMING` work rows,
`LARCH_HARNESS_BOOTSTRAP` cold/warm measurements, and real jobs-API wall-clock
for one exact successful-run cohort. Fixed job startup, every target marker's
warm bootstrap, cold-minus-warm shared setup, and named compile-affinity setup
are charged before LPT packing. The current post-cleanup inventory has no
Cargo-backed targets. A future reviewed shared-compile exception must declare
`--compile-affinity TARGET=GROUP:SECONDS`; the packer keeps that group together
and charges `SECONDS` once (or zero to require co-location without adding a
second measured cost). Missing bootstrap evidence, skipped or incompatible
cohorts, unstable marker counts, and target inventory drift stop the rewrite.
The planner compares candidate active-runner counts; the checked-in workflow
must not retain an empty matrix cell.

Verification treats real CI job wall-clock as authoritative: the measured
slowest shard must not exceed `--max-shard-wall-clock` (default 300s) or the
input layout's approved threshold, and median summed harness-runner time may
not regress. The command verifies the exact dispatched workflow runs.
`--experimental-wall-clock-override NOTE` is limited to a
documented experiment with a predicted or measured regression; it never
bypasses incomplete evidence. To add a single new target, append it to any
shard, run `make test-harness-shards-coverage` to verify the partition, then
rebalance by timing.

For full Rust coverage shards, use the same dev skill with an explicit target
count:

```bash
"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" rebalance-tests run --kind rust --n-rust-shards 4
```

Rust mode reads complete jobs-API cohorts for the configured
`rust-full-shards` matrix. The legacy monolithic `rust-full` job is a valid
one-shard baseline. A resize updates the matrix, producer count, and
`rust-coverage` gate's test-report count in one atomic workflow write. The
dedicated policy report remains the one additional merge input. Verification
requires every expected shard in every dispatched run, then rejects a slowest
shard above the approved baseline cap or `--max-rust-shard-wall-clock`
(default 600s). Missing, skipped, duplicate, or incomplete job rows stop the
workflow.

See `.claude/skills/rebalance-tests/SKILL.md` for flags and full workflow
documentation. Rust fixture and wire-contract tests live in
`crates/larch-core/src/ci_timing.rs` and `crates/larch-core/src/test_shards.rs`.
The orchestration boundary is covered by
`crates/larch-cli/src/rebalance_tests_workflow.rs` and
`crates/larch-cli/tests/rebalance_tests_workflow.rs`. Run the targeted Cargo
commands before changing the rebalance contract.

`larch rebalance-tests run` owns the checked Git, GitHub Actions, pull request,
atomic-write, and repository-state workflow. `larch rebalance-tests plan` and
`larch rebalance-tests verify` remain its versioned pure JSON decision core.
They consume already-collected Rust CI-timing reports and reuse the Rust shard
packer. They do not inspect or rewrite `Makefile` or
`.github/workflows/ci.yaml`, make Git or GitHub calls, or dispatch CI. See
`.claude/skills/rebalance-tests/scripts/rebalance.md` for the exact contract.

**Harness timing formats.** The Makefile's `HARNESS_MARK` invokes the
dependency-free Rust `larch-harness-mark` binary in `target/harness-mark`. It
uses `rustc` directly for its standard-library-only sources, rather than
starting the released `larch-cli` package or Cargo's workspace machinery. That
separate target directory preserves the existing `target/debug/larch` probe
behavior.

Each wrapped command still emits the Rust-owned `LARCH_HARNESS_TIMING` row to
stdout:

```
LARCH_HARNESS_TIMING<TAB><test-name><TAB><N.NNs>
```

The trailing `s` suffix is stripped and the remainder parsed as decimal seconds.
Current output uses exactly two fractional digits (e.g. `0.34s`, `7.62s`);
older published logs may contain integer-only seconds — both forms are accepted.

Before its child begins, the wrapper also emits a separate bootstrap diagnostic:

```text
LARCH_HARNESS_BOOTSTRAP<TAB><test-name><TAB><cold|warm|unknown><TAB><N.NNs>
```

`cold` means the isolated timer binary was absent before that recipe invoked
`rustc`; `warm` means it already existed. The duration starts immediately
before the helper build starts and ends immediately before the child command
begins. It is not folded into the child row, so a future harness-packing report
can charge fixed startup separately from target work. A comparable fresh-runner
sample retains the first cold row, subsequent warm rows, the printed Make
recipe that names the child command, and the GitHub Actions job timestamps used
for total-job and summed-runner timing.

### CI and branch-safety ruleset

The active `CI and branch safety` ruleset protects `refs/heads/main`. It
requires only stable, unconditional aggregate or single-lane checks. Every
required context is source-bound to the GitHub Actions integration (`15368`):

- `lint`
- `lint-local`
- `shellcheck`
- `test-harnesses-gate`
- `agent-lint`
- `rust-coverage`
- `rust-gate`
- `contains-pins`
- `gitleaks`
- `agent-sync`
- `trufflehog`

Do not require a matrix leg or a conditional implementation detail. In
particular, `rust-selection`, `rust-lint`, `rust-deny`,
`rust-full-lcov-tool`, `rust-full-shards`, `rust-full-policy`, `rust-partial`, and
`rust-skip` are inputs to the stable Rust aggregates, not proof that every
required Rust obligation ran.

The ruleset requires a merge queue with `ALLGREEN`, a 60-minute check-response
timeout, `max_entries_to_build=1`, `max_entries_to_merge=1`,
`min_entries_to_merge=1`, `min_entries_to_merge_wait_minutes=0`, and squash
merges. The workflow receives `merge_group` `checks_requested` events and runs
the full, read-only validation lane before each merge. A normal push to `main`
starts only the trusted cache-publication workflow. It may publish an expensive
cache only from a successful `CI` merge-group source with the exact final main
SHA; merge-group validation itself cannot write a cache. The publisher's
lightweight cache jobs do not run validation checks. `strict_required_status_checks_policy`
remains false: the merge queue, rather than a stale branch-head check, validates
the integrated candidate against the current queued base.

If GitHub cannot provide a merge queue, enable
`strict_required_status_checks_policy` with this same source-bound context list
and record the reason in the tracking issue. Keep the `merge_group` trigger so
the workflow remains ready for a later queue activation.

The `shellcheck` job runs as a dedicated CI job in parallel with `lint`. It
compiles the dependency-free `larch-residual-bash-paths` reader directly with
`rustc`, so manifest validation stays shared with `larch residual-bash paths`
without building the released CLI. The `lint` job skips the shellcheck hook to
avoid paying the pre-commit environment-install cost twice.

### Changing the shard count (lockstep edit)

The shard count today is `2`, hard-coded in two places (the partition guard is shard-count-agnostic — it discovers `test-harnesses-N:` rules by parsing the Makefile):

1. `Makefile` — two `test-harnesses-N:` shard targets and the umbrella `test-harnesses:` aggregating them.
2. `.github/workflows/ci.yaml` — the matrix `shard: [1, 2]` strategy on the `test-harnesses` job.

`scripts/test-harness-shards-coverage.sh` does NOT need editing on a shard-count change: it discovers the active `test-harnesses-N:` rules from the Makefile (`extract_shard_prereqs` parses them) and validates that `test-harness-shards-coverage` is first in the shard that contains it. The umbrella-expected list is built from the same discovered set.

A partial edit that updates the Makefile but forgets the workflow YAML would silently drop a CI shard while local `make test-harness-shards-coverage` still passes. Any change to shard count must touch both locations in the same commit and update the required-check list above with the new stable gate identity.

## CI secret scanning

Two scanners run as dedicated CI jobs in `.github/workflows/ci.yaml`:

- **`gitleaks`**: The manual local pre-commit stage calls `scripts/larch.sh lint gitleaks`; the Rust command verifies the pinned `v8.18.4` release archive, extracted binary, and reported version. CI uses a separate checksum-pinned workflow installer, revalidates its cache entry before both scans, and does not build `larch-cli`. It scans the working tree and PR commit range; the manual local scan uses `--no-git` on the working tree.
- **`trufflehog`** — Runs `trufflesecurity/trufflehog` pinned to its commit SHA for `v3.82.13` (supply-chain: tags are mutable) with `version: 3.82.13` pinning the Docker image and `--only-verified`, so findings fire only for credentials that authenticate against a live provider API.

See the canonical
[secret-scanning layers](security/artifacts-redaction-and-publication.md#secret-scanning-layers)
for the three-layer model, allowlist limits, and content-classification warning.

## Manual Release Gates

Some regression tests intentionally short-circuit on the enforced CI runners and rely on a manual run before a release that touches the affected surface.

### macOS bash 3.2 regression coverage

The PR creation surface now lives in `scripts/larch.sh pr create`. Before cutting a release whose changes touch PR creation, run the focused Rust PR and black-box parity tests on a macOS workstation in addition to CI so file-backed body handling, `--repo` threading, and fork-mode behavior are verified on the release host.

## Makefile Targets

| Target | Description |
|--------|-------------|
| `make lint` | Run all linters repo-wide |
| `make rust-lint` | Run the Rust-owned repository policy rules, including the cross-language em dash, Codex execution, run-log walker, skill lifecycle, skill structure, and KV codec checks. |
| `make shellcheck` | Run shellcheck only |
| `make markdownlint` | Run markdownlint only |
| `make jsonlint` | Run JSON validation only |
| `make actionlint` | Run actionlint only |
| `make agnix` | Run agnix only |
| `make gitleaks` | Run Gitleaks only via pre-commit. The verified Rust command downloads the pinned release binary on first use, then scans the working tree with `--no-git`. |
| `make trufflehog` | Run trufflehog via Docker in `filesystem` mode over the working tree (same pinned image and `--only-verified` flag as the CI `trufflehog` job, but CI uses the action's default `git` mode over the PR range — local and CI are not byte-identical invocations). Requires Docker daemon running locally |
| `make setup` | Install pre-commit git hooks |
| `make test-check-mid-run-dirty-tree` | Run Rust CLI coverage for `scripts/larch.sh dirty-tree` baseline and checkpoint behavior. Exercises clean/dirty states, sidecar emission, missing-baseline ambiguity, scope validation, scope-marker parsing, and NUL-safe non-UTF-8 paths. |
| `make test-check-stale-plugin` | Run the offline stale-plugin detector harness. Exercises ahead/match/not-a-dev-clone/install-ahead cases plus skip handling for missing plugin manifests, missing version fields, unset `CLAUDE_PLUGIN_ROOT`, and invalid working-tree roots. A `make lint` prerequisite via `test-harnesses-2`. |
| `make test-check-phantom-dirty` | Run Rust CLI coverage for `bin/larch git check-phantom-dirty`. Exercises clean, tracked-only, phantom, mixed, missing and malformed baseline, repository-probe failure, NUL-safe non-UTF-8 paths, output failure, and `--step` validation. |
| `make test-phantom-probe-with-warn` | Run Rust CLI coverage for `bin/larch git phantom-probe`. Exercises phantom status output, execution-warning append behavior and failure classification, one breadcrumb, and bad `--step` handling. |
| `make test-wait-for-reviewers` | Run Rust CLI coverage in `crates/larch-cli/tests/agent_commands.rs`. Pins `scripts/larch.sh agent wait-reviewers`'s `--timeout` rejection contract (exit 1 + stderr `must be a positive integer` for `0` / `00` / `000` / non-numeric, `--timeout requires a value` for missing arg), `WAIT_FOR_REVIEWERS_POLL_INTERVAL=00` / `000` rejection, and the indexed `DONE <idx> <basename>` / `TIMEOUT <idx> <basename>` stdout grammar. |
| `make test-compose-collector-failure-log` | Run Rust CLI coverage in `crates/larch-cli/tests/agent_commands.rs`. Pins section placeholders, stderr-tail redaction, byte bounds, private output mode, and invalid-argument no-output behavior. |
| `make test-gather-branch-context` | Run Rust CLI coverage in `crates/larch-cli/tests/agent_commands.rs`. Pins diff, file-list, commit-log KVs, `origin/main` preference, and `larch-logs/**` exclusion for `scripts/larch.sh agent gather-branch-context`. |
| `make test-classify-diff-mode` | Run Rust CLI coverage in `crates/larch-cli/tests/agent_commands.rs` plus specialist-renderer coverage in `crates/larch-cli/tests/rendering_migrated_parity.rs`. Pins the `DIFF_MODE=` CLI response, fail-closed missing or malformed `scripts/generators.tsv` handling, and in-process docs-only prompt selection. |
| `make test-run-external-agent` | Run Rust CLI coverage for `scripts/larch.sh agent run-external-agent`. Exercises artifact sidecars, inner-sentinel replacement, deadline process-group cleanup, policy-rejection fast-fail, and unsafe output rejection before side effects. |
| `make test-token-ledger` | Run the `/implement` token ledger harness. Exercises JSONL mark/vendor records, session-id precedence, hashed safe filenames, `--ledger` containment, JSON-safe `raw=`, and mode `600`. |
| `make test-token-report` | Run Rust `token report` integration coverage. Exercises JSON, Markdown, summary, terse, and bucket output from fixture ledgers and transcripts, `--output`, idempotent `## Token Report` sentinel replacement, confined scraper sidecars, and fail-open unavailable reporting. |
| `make test-timing-ledger` | Run the timing ledger harness. Exercises fixed 13-column TSV rows, timing task-kind validation, basename-only output storage, `LARCH_TIMING_LEDGER` containment, chmod mode, negative-duration clamping, and parallel append integrity. |
| `make test-token-vendor-scrapers` | Run the external-vendor token scrape harness. Exercises Codex JSONL usage parsing, Cursor `.usage` totals, malformed JSON fallback, Cursor review JSON-sidecar `.result` extraction back to plain reviewer prose, and `scripts/larch.sh agent launch-cursor-implement` / `scripts/larch.sh agent launch-codex-implement` `record-vendor` smoke (raw=cursor_implement / raw=codex_implement attribution). Also covers per-bucket Codex `BUCKETS_codex` / no blended warning plus legacy aggregate-only `total` rendering for #1427. A `make lint` prerequisite via `test-harnesses-2`. |
| `make test-token-claude-source` | Run the Claude transcript resolver harness. Exercises `LARCH_CLAUDE_SOURCE_FILE` snapshot replay short-circuit, snapshot fall-through on stale / garbage / missing files, the live mtime resolver, `LARCH_CLAUDE_SESSION_ID` override, malformed session-id rejection, empty project dir failure, and concurrent-session pinning (snapshot wins over a newer transcript in the project dir). |
| `make test-external-tool-registry` | Run the offline regression harness for `scripts/external-tool-registry.sh`, the canonical external-tool and implementer-coder taxonomy library. Exercises registry order, predicates, brace-list formatters, double-source idempotency, strict-shell-option preservation, no source-time stdout/stderr, `agent check-reviewers` registry consistency, and nested-cwd `implement step2-dispatch --coder claude` path resolution. A `make lint` prerequisite via `test-harnesses-2`. |
| `make test-analyze` | Run Rust behavioral regression tests for `analyze-issues`. They exercise the recorded 10-issue fixture, category and waste-signature output, deterministic growth-series data, and the shared growth-chart renderer. Focused local target; required CI ownership is [rust-full-shards](rust-testing.md#bash-shard-cargo-target-ownership). |
| `make test-fluff-analysis` | Run Rust black-box contract coverage for `fluff-analysis analyze`. Exercises exact stdout, stderr, exits, and report files over synthetic implement and design corpora: baselines and OOS/rejected rows, TSV-primary false-negative joins with the `blocking` alias, multi-round and round-local JSONL, compact 18-column and named 21-column TSVs, the self-review tally fallback and its malformed-JSONL suppression, assessment and ship-outcome coverage, all supported filters and output modes, and the missing-`--log-root` exit code `2`. Focused local target; required CI ownership is [rust-full-shards](rust-testing.md#bash-shard-cargo-target-ownership). |
| `make test-rejected-analysis` | Run the offline structural harness for `/rejected-analysis`. It pins the interface, wire parsing, durable ingest state, launcher posture, Rust command ownership, and finding-hash prose. Rust unit and black-box coverage verify the preparation, verdict-ingestion, finalization, and recording contract. A `make lint` prerequisite via `test-harnesses-2`. |
| `make test-difficulty-calibration` | Run Rust black-box contract coverage for `difficulty-calibration analyze`. Exercises exact stdout, stderr, exits, report files, malformed inputs, empty corpora, and non-file read boundaries. |
| `make test-voter-calibration` | Run Rust black-box contract coverage for `voter-calibration analyze` (the `/voter-calibration` analyzer). Exercises exact stdout, stderr, exits, and report files over synthetic classification corpora, including both design schemas, code-review schemas, exclusions, outlier scoring, threshold flips, era segmentation, offline outcome details, degradation tokens, and the missing-log-root exit `2`. This Cargo-backed target is a standalone local alias. |
| `make test-audit-runs` | Run Rust audit scan, counter, timestamp, title, backlog-advisory, and prior-closure wire coverage. |
| `cargo test --locked --package larch-cli --test integration checks_lint_fix_commands::` | Run black-box Rust coverage for `checks fixer-evidence`, `checks lint-fix`, and `checks repair-loop`. Pins exact help, stdout, stderr, exits, check-fix wiring, structural routing, bgjob launch, redacted-log progression, merge-result-env capture, self-edit attribution, and tmpdir confinement. Pure state and routing coverage also lives in the `larch-core` `implement::checks_lint_fix` tests. |
| `bash scripts/test-rust-integration-consumer.sh` | Exercise a checksum-verified `LARCH_TEST_RUST_BINARY` through the shipped lifecycle bootstrap. Requires `LARCH_TEST_RUST_BINARY_SHA256` and `RUST_CI_MODE`; verifies disabled-storage start/finalize wires and confined LLVM profile output. CI also covers workflow topology, main-cache publication, typed GitHub authentication, and hermetic Gitleaks behavior through their Rust owners. |
| `scripts/larch.sh checks contains-pins` | Run the contains-pin scanner directly. Verifies `contains "$VAR" "literal"` assertions in shell harnesses still pin text that exists in the resolved target files. |
| `make test-classify-bump` | Run the Rust CLI coverage for release bump classification. Covers transparent bump-pipeline idempotency (`Bump version` / legacy `chore(larch-logs)` stacks still emit `BUMP_TYPE=NONE`) and fail-closed transparent-subject spoofing. This Cargo-backed target is a standalone local alias. |
| `make test-stall-recovery-report` | Run the Rust contract-lint test plus the focused Rust core, adapter, and `file-report` suites. Runtime classification, attempts, normalization, escalation, state, validation, reporting, corpus, exact-marker deduplication, typed GitHub create and comment operations, lookup fail-open behavior, dry-run, Tier B bounded-comment validation, test mutation denial, missing response URLs, and public-surface redaction coverage lives in Rust unit, integration, and parity tests. |
| `make test-resolve-upstream-larch-repo` | Run the offline delegation harness for `scripts/resolve-upstream-larch-repo.sh`, covering adjacent-root binding, exact Rust-verb delegation, failure propagation, and the thin wrapper. Rust integration coverage owns GitHub URL forms, OWNER/REPO normalization, malformed repository metadata, and newline-bearing metadata rejection. A `make lint` prerequisite via `test-harnesses-2`. |
| `crates/larch-cli/tests/implement_terminal_parity.rs` | Offline Rust parity coverage for the Step 18 stall/logs owner and Step 19 cleanup owner. Covers the pinned stall predicate, Step 17 flag forwarding, full-snapshot handoff, transcript recapture, publication and failure retention, terminalization fencing, marker caching, exact teardown argv, no log writes during cleanup, and teardown tail relay. Part of the Rust workspace suite. |
| `make test-step-18b-final-report` | Run the Rust `final_report_commands` suite for Step 18b and final-report wiring, snapshot-copy failure promotion, and failure-envelope behavior without network access. It is a standalone alias, not a `make lint` prerequisite; see `CARVE_OUTS` in `scripts/test-harness-shards-coverage.sh`. |
| `make test-launch-ci-fixers` | Run the Rust CI-fix launcher suite for `agent launch-codex-ci`, `agent launch-cursor-ci`, and `agent launch-claude-ci`: argument refusal order, the missing-binary result, prompt composition and redaction, and the Claude envelope branches. A standalone alias, not a `make lint` prerequisite. |
| `make test-check-reviewers` | Run Rust CLI smoke coverage for `scripts/larch.sh agent check-reviewers` / `degraded-tools-gate` / `resolve-model-pins` (`cargo test -p larch-cli --test integration reviewer_availability_commands::`). Standalone carve-out (see `CARVE_OUTS`); covered by the coverage execution lane / focused cargo, not a `test-harnesses-N` member. |
| `make test-run-negotiation-round` | Run Rust integration coverage for `scripts/larch.sh agent run-negotiation-round`. Covers usage and missing-prompt exits, Codex/Cursor success and failure paths, the `RESPONSE_FILE=` envelope, Darwin startup-lock handling, typed trust/auth argv, events and sidecar paths, and Codex temporary-home cleanup. |
| `cargo test --locked --package larch-cli --test integration forked_repo_parity::` | Run black-box parity coverage for the Rust-owned `/set-up-forked-open-source-repo` command. Pins help, argument refusals, exit status, stream routing, and repository preflight behavior. Pure remote classification and typed Git/GitHub adapter coverage live in the focused core and adapter suites. |
| `make test-design-driver` | Run the Rust design-driver recorded-contract suite. |
| `make test-invoke-plan-validator` | Run the Rust plan-quality contract harness, including validator dispatch, quoted plan paths, and the required `DESIGN_TMPDIR` guard. |
| `make test-parse-plan-commands` | Run the Rust plan-quality migrated-parity suite, including parser TSV output for fenced blocks, allow-list rows, `parse_note` paths, and arithmetic versus command substitution. |
| `make test-persist-retally-step3-env` | Run the Rust `/design` MainAgent re-tally persistence harness. Exercises Step 3 result-env refresh, scope-anchor filtering, tally-error cleanup, and count persistence. |
| `cargo test --locked --package larch-cli --test integration plan_review_mav_commands::` | Run the Rust Step 3 MainAgent vote and re-tally contract suite. Covers pause preemption, allowlisted result-env rehydration, trusted KV frames, scope evidence, successful and failed tally routing, warning and timing side effects, and distinct artifact/resume round precedence. |
| `make test-validate-plan-commands` | Run the Rust plan-quality migrated-parity suite for Tier 2 and Tier 3 validation contracts, composed-plan paths, registry dry runs, and unknown flags. |
| `make test-emit-plan` | Run the Rust `/design` plan emission harness for valid and invalid `diff_lines`, empty plans, and idempotent re-invocation. |
| `make test-emit-design-plan-preview` | Run the Rust Step 3 / Gate C plan-preview harness. It freezes headers, small and large plan rendering, threshold handling, empty-path warnings, and the Step 3 entry sentinel. |
| `make test-step3-review-cap` | Run the `/design` Step 3 review-round cap harness. It verifies first-entry round numbering, cap short-circuit behavior, post-launch round consumption, tally-error rollback, fixed-cap blocking, the Gate B bypass prose, and the disk-derived auto-continuation guard. This mixed Rust and Bash target is a standalone local alias. |
| `make test-run-step3-review` | Run the Rust `/design` Step 3 launcher harness for loop transcript, resume state, and result-envelope persistence. |
| `make test-review-design-step3-loop` | Run the Rust script-internal `/design` Step 3 loop harness for result envelopes and durable phase/resume markers. |
| `make test-tally-plan-review` | Run the Rust `/design` tally's recorded-round byte golden and degraded zero-voter/MainAgent contract cases. This Cargo-backed target is a standalone local alias. |
| `make test-findings-classification` | Run the `/design` forensic TSV harness for the 21-column plan-review schema. This build-backed Bash target is a standalone local alias. |
| `make test-review-findings-classification` | Run the Rust `review_tally_commands` integration suite for representative tally-code-votes fixtures, emit-tally, and log-phase coverage. This Cargo-backed target is a standalone local alias. |
| `make test-finalize-plan` | Run the Rust `/design` final artifact harness for creation, invalid-artifact refusal, and idempotence. |
| `make test-launch-codex-exec` | Run Rust integration coverage for `scripts/larch.sh agent launch-codex-exec`. Exercises argv validation, the `LAUNCHER_EXIT` stdout contract, prompt sidecar creation, and the typed process boundary. |
| `make test-brainstorm-prompts` | Run the offline harness for `/design` Step 1d.5 brainstorm prompt tokens. It pins the framing, scope, and pragmatic prompt markers across their canonical files. A `make lint` prerequisite via `test-harnesses-2`. |
| `make test-dispatch-plan-voters` | Run the Rust-owned `/design` Step 3 panel and voter dispatch contract. Exercises `scripts/test-plan-review-dispatch.sh` through the real waterfall owner with a confined launcher fixture, covering static and dynamic manifests, prompt payload sidecars, pruning, failure classification, voter row order, and the single-Claude degraded floor. |
| `make test-launch-claude-review` | Run the Rust launcher-contract harness for `scripts/larch.sh agent launch-claude-review`. Exercises the larch CLI with a PATH-stubbed `claude` binary, covering prompt-file and inline-prompt launches, result promotion, `.done` sentinels, and timeout validation. Focused local target; required CI ownership is [rust-full-shards](rust-testing.md#bash-shard-cargo-target-ownership). |
| `make test-dispatch-with-waterfall` | Run the three-phase waterfall dispatcher harness against a fixture plugin root with a stub child launcher and the real in-process collector. It covers success, each fallback, no-fallback drops, slot-row grammar, result gates, competition notices, straggler cutoffs, cost warnings, and orphan-free termination. Focused local target; required CI ownership is [rust-full-shards](rust-testing.md#bash-shard-cargo-target-ownership). |
| `make test-collect-agent-results` | Run the Rust coverage for `scripts/larch.sh agent collect-results` via `crates/larch-cli/tests/collector_commands.rs`. Covers arg validation, paths-file errors, every terminal status, retry success and metadata fail-closed handling, transient retry routing, outer-launcher retries into `agent launch-review` and `agent launch-codex-exec`, substantive and structured validation, summary-only output, stderr-tail resolution and cksum dedup, phase-output tail fallback, invalid-sentinel empty-output retry, registry-backed tool derivation, retired outer-launcher rejection, and `cap_hit` status. Focused local target; required CI ownership is [rust-full-shards](rust-testing.md#bash-shard-cargo-target-ownership). |
| `make test-blocker` | Run Rust coverage for the shared prose blocker parser used by `/combine-issues`. Native dependency reads, fail-open behavior, and `blocker all-open` output are covered by the `blocker-all-open-*` parity goldens and the `larch-core` admission tests. Focused local target; required CI ownership is [rust-full-shards](rust-testing.md#bash-shard-cargo-target-ownership). |
| `make test-implement-bootstrap` | Run Rust tracking, plan, and coder continuation coverage after Step 0 infrastructure completes, plus a clean-install case for native plan, coder, and tail routing. This Cargo-backed target is a standalone local alias. |
| `make test-implement-bootstrap-invoke` | Run Rust Step 0 bootstrap coverage for command arguments, routing-envelope parsing, handoff safety, and the fresh/resume stdout contract. This Cargo-backed target is a standalone local alias. |
| `make test-git-push` | Run focused Rust integration coverage for `git push`: named-branch guards, retry exit propagation, stderr handling, and final non-zero push status. This Cargo-backed target is a standalone local alias; `rust-full` owns the required CI coverage. |
| `make eval-research [ARGS="--id ..."]` | Run the **opt-in** `/research` evaluation harness (closes #419 under umbrella #413) through the Rust owner (#8500). Reads `skills/research/references/eval-set.md`, runs each entry through `/research` as a fresh `claude -p` subprocess, scores the output along deterministic + LLM-as-judge axes, and emits a markdown summary table (or a populated `eval-baseline.json`-shaped file with `--write-baseline`). **Not a `make lint` prerequisite** — runs ~20 questions × ~30-60s each, costs real tokens. Pass flags via `ARGS=`; the target dispatches to `scripts/larch.sh eval research`. Rust parity coverage lives in `crates/larch-cli/tests/parity.rs`. |
| `make test-implement-step2-routing` | Run the structural harness for Rust-owned implementer selection and Step 2 dispatch. It pins difficulty-keyed coder order, omitted-coder fallback, explicit-coder refusal, the manifest fallback flag, informational `diff_lines`, and review-health routing. A `make lint` prerequisite via `test-harnesses-2`. |
| `make test-implement-preflight` | Run Rust integration coverage for `scripts/larch.sh implement preflight` and sibling admission commands, including usage refusal, clone-tag derivation, coder-scout tmpdir validation, and Step 0 argument validation. This Cargo-backed target is a standalone local alias. |
| `make test-implement-fence-shape` | Run the structural harness for `/implement` SKILL.md Bash fence shape. It requires each Bash fence to be a plugin-root guard plus one repo script invocation and bans consecutive, telemetry-only, or inline state-reader fences. A `make lint` prerequisite via `test-harnesses-2`. |
| `make test-implement-timing-rehydration` | Run the structural harness for `/implement` timing-ledger isolation and plugin-root recovery. It pins timing and tmpdir rehydration and same-fence plugin-root guards. A `make lint` prerequisite via `test-harnesses-2`. |
| `make test-finalize-sanity-check` | Run focused Rust cleanup-target validation for prefix matches, session identity, missing identity, and unrelated-root refusal. This Cargo-backed target is a standalone local alias. |
| `make test-cache-root-validation` | Run the regression harness for cache-backed session root acceptance while preserving system temporary-root acceptance and unrelated-path rejection. A `make lint` prerequisite via `test-harnesses-2`. |
| `make test-flush-execution-issues` | Run the delegation smoke for `/implement` Step 7a's thin `flush-execution-issues.sh` wrapper. Behavioral coverage, including empty input, section-split NDJSON records, replay idempotency, and failure retention, lives in the Rust execution-issue unit and process-contract suites. |
| `make test-step-7a` | Run the thin `step-7a.sh` delegation smoke. Step 7a behavior is Rust-owned by `crates/larch-cli/src/implement_review_commands.rs` (orchestration, diagram outcomes, rebase exit propagation, the execution-issues checkpoint, terminal KVs, bgjob transport, and argument validation), with parity coverage in `crates/larch-cli/tests/implement_review_parity.rs`; the smoke checks plugin-root selection, exact `scripts/larch.sh` routing, argv forwarding, and stream/exit passthrough. |
| `make test-generate-code-flow-diagram` | Run Rust black-box coverage for `implement code-flow-diagram`. It pins argument validation, prompt and artifact paths, launcher failures and retries, Mermaid rejection, and the `STATUS` / `DIAGRAM_FILE` / `SKIP_REASON` stdout contract. |
| `make test-cache-key-discipline` | Run the structural guard for prompt cache-key discipline. It scans audited prompt-construction surfaces and requires legitimate per-session external-tool prompt paths to carry a nearby non-stable annotation. A `make lint` prerequisite via `test-harnesses-2`. |
| `cargo test --locked --package larch-adapters session_lifecycle::tests::` | Run Rust resolver regression coverage for `/implement` and `/design` temporary-directory binding, confinement, and symlink refusal. |
| `cargo test --locked --package larch-cli --test integration parity::cleanup_run_preserves_live_session_directory` | Run focused Rust regression coverage for `/cleanup` age-based session pruning and active-session preservation. Rust unit coverage also pins pattern matching and implement-pointer symlink refusal. |
| `make test-redact-tmpdir-paths` | Run the Rust `redact_parity` integration suite, covering tmpdir and operator-path redaction together with every `redact` command contract. This Cargo-backed target is a standalone local alias; the `rust-full-shards` jobs own the required CI coverage. |
| `make test-check-clean-tree` | Run Rust CLI coverage for `bin/larch git clean-tree`, the shared working-tree cleanliness predicate. Exercises clean, tracked-dirty, and untracked worktrees with the `--fail-closed` form. Focused local target; required CI ownership is [rust-full-shards](rust-testing.md#bash-shard-cargo-target-ownership). |
| `make test-check-main-sync` | Run focused Rust integration coverage for `git check-main-sync`. Exercises in-sync, not-on-main, blocked ahead commits, missing `origin/main`, bad args, and CLI stdout contracts. This Cargo-backed target is a standalone local alias; the `rust-full-shards` jobs own the required CI coverage. |
| `cargo test --locked --package larch-cli --bin larch hook_commands::tests::` | Run focused Rust coverage for the advisory hook owners: exact health and Stop envelopes, dirty trees, managed stashes, interrupted Git state, unmerged branches, stalled-run sentinels, sparse-cone drift, active-run boundaries, audit confinement, and detached cleanup ordering. The full Rust suite owns required CI coverage. |
| `crates/larch-cli/tests/review_and_fix_commands.rs` | Focused Rust coverage for the migrated `review-and-fix` repair surface: no-findings output, untracked-baseline change detection, Step 5 preflight envelope persistence, captured-status normalization, and self-review delta-only commits. |
| `make test-implement-launchers` | Run the Rust implement and Claude fix launcher suite for `agent launch-codex-implement`, `agent launch-cursor-implement`, `agent launch-claude-lint-fix`, and `agent launch-claude-review-fix`: argument refusal order, session-directory confinement, prompt composition with the architectural knowledge block, the token-budget cap artifacts, and the Claude envelope branches. A standalone alias, not a `make lint` prerequisite. |
| `make test-run-step2-dispatch` | Run the focused Rust unit tests for `scripts/larch.sh implement run-dispatch`, including required arguments, durable parent/child adapter routing, `--answers` completed-result replacement, telemetry, and result-env publication. A standalone alias; the `rust-full-shards` jobs own the complete Rust suite in CI. |
| `cargo test --locked --package larch-cli --test integration architectural_assessment_write_parity::` | Run Rust parity coverage for architectural assessment writes. The staged-guideline cases include positional assessment paths, materialize-env validation, diff handoff, and relative and absolute path resolution. |
| `cargo test --locked --package larch-cli --test integration implement_finalize_recorded::` | Run recorded Rust coverage for `implement-finalize`, including byte-stable `CLEANED=true` success and `CLEANED=false` rejection envelopes. |
| `make test-step2-dispatch` | Run the Rust black-box parity suite for `scripts/larch.sh implement step2-dispatch`, including argument validation, Claude fallback, post-dispatch routing, and active-leg termination. A standalone alias; the `rust-full-shards` jobs own the complete Rust suite in CI. |
| `make test-commit-implementation` | Run the Rust black-box contract harness for `implement commit`: usage and hint refusals, help, and result envelopes. This Cargo-backed target is a standalone local alias. |
| `make test-git-commit-only` | Run Rust CLI coverage for `scripts/larch.sh git commit --only --pathspec-from-file`, proving NUL-delimited recovery pathspec commits include paths with spaces while leaving unrelated pre-staged content staged and uncommitted. Focused local target; required CI ownership is [rust-full-shards](rust-testing.md#bash-shard-cargo-target-ownership). |
| `make test-run-external-agent-args` | Run Rust CLI argument-validation coverage for `scripts/larch.sh agent run-external-agent`. Pins that unsafe output paths create no sidecars and `--timeout 0` exits 1 with `ERROR: --timeout must be a positive integer, got '0'`. A standalone Rust integration-test alias; the Rust CI suite covers it. |
| `make test-reviewer-prune` | Run Rust integration coverage for `scripts/larch.sh review reviewer-prune`, including ledger recording, attribution, replacement, filtering, the off switch, and all-pruned markers. This Cargo-backed target is a standalone local alias. |
| `make test-token-cost` | Run focused Rust `token cost` CLI coverage: per-bucket parsing, machine-readable totals, blended-warning suppression, and malformed-flag exit behavior. Exercises `crates/larch-cli/tests/token_commands.rs`. |
| `make test-render-cost-line` | Run focused Rust `token render-cost-line` CLI coverage: terminal-line grammar and `--quiet-on-empty`. Exercises `crates/larch-cli/tests/token_commands.rs`. |
| `make test-token-report-dedup` | Run the Rust transcript scanner contract for request/message identity and usage-fingerprint deduplication, including cache-bucket preservation. This Cargo-backed target is a standalone local alias; `rust-full` owns the required CI coverage. |
| `make test-token-cost-per-bucket` | Run the Rust token-cost suite, including per-bucket arithmetic, environment overrides, blended fallback, malformed inputs, and rendering. This Cargo-backed target is a standalone local alias; `rust-full` owns the required CI coverage. |
| `make test-render-cost-line-realism` | Compatibility alias for the same Rust token-cost suite as `make test-token-cost-per-bucket`. This Cargo-backed target is a standalone local alias; `rust-full` owns the required CI coverage. |
| `make test-render-cost-line-callsites` | Structure check that final-summary callsites use full-body top-chat emission and reject retired standalone cost-line helpers. A `make lint` prerequisite via `test-harnesses-2`. |
| `make test-token-report-summary-format` | Pins Rust `token report --summary` non-dollar `Tokens:` + per-vendor line (no `💰 Cost:`). Exercises `crates/larch-cli/tests/token_commands.rs`. |
| `make test-fetch-combinable-issues-filter` | Rust regression coverage for the `/combine-issues` title-prefix filter and planner helpers. Covers managed prefixes, legacy `[PLANNED]` / `[IN PROGRESS]` busy-title exclusion, `[LOCKED]` exclusion, `[DESIGNED]` retention, source mapping, and the OOS inherited-edge boundary. Exercises `crates/larch-cli/src/combine_issues_commands.rs`. Focused local target; required CI ownership is [rust-full-shards](rust-testing.md#bash-shard-cargo-target-ownership). |
| `make test-legacy-title-prefix-literals-scope` | **Normative scope** for legacy `[IN PROGRESS]` / `[PLANNED]` bracket literals outside `larch-logs/`: the `ALLOW=(…)` list inside `scripts/test-legacy-title-prefix-literals-scope.sh` is the source of truth (not a naive repo-wide “zero `git grep` hits” rule). The harness runs allow-listed `git grep` over hits from the same pattern. It runs through the CI harness lane and can be selected explicitly through the manual pre-commit stage. Exercises `scripts/test-legacy-title-prefix-literals-scope.sh`. A `make lint` prerequisite via the `test-harnesses-2` shard partition. |
| `make test-quick-mode-docs-sync` | Run the regression and self-test harnesses for `/implement` Step 5 public-doc sync and required cross-references. A `make lint` prerequisite via `test-harnesses-2`. |
| `make test-harness-shards-coverage` | Run the structural drift detector for the `test-harnesses-N` shard lists. It checks assignment, uniqueness, naming, `.PHONY` membership, aggregate membership, self-reference, and rejection of non-Bash recipes. A `make lint` prerequisite via `test-harnesses-2`. |
| `cargo test --locked --package larch-cli --test integration developer_tooling_commands::` | Run focused Rust regression coverage for `alias generate`, `alias resolve-target`, `residual-bash paths`, and `verify skill-called`. Covers alias YAML bytes, plugin/consumer target resolution, private routing, invalid names, path spaces, manifest validation and typed Git intersection, sentinels, regex verification, and commit deltas. |
| `make test-alias-structure` | Run the shared Rust `skill-structure` rule over the declarative live prompt contracts. The file-bug, design, implement, learn-from-bugs, research, and review structure aliases invoke the same rule for compatibility. These Cargo-backed aliases stay outside the Bash shard lists. |
| `make test-prompt-template-invariants` | Run the cross-cutting prompt-template structural invariants harness across review dispatch, plan voters, review-and-fix, renderers, scout prompts, and vendor review. It prevents refactors from silently removing structured-output and anti-narrative requirements. A `make lint` prerequisite via `test-harnesses-2`. |
| `make test-decompose-file-issues` | Run Rust design-decomposition integration coverage for partition filing and original-issue close helpers. |
| `make test-decompose-panel-dispatch` | Run Rust design-decomposition integration coverage for panel dispatch, partial output binding, and degraded-panel signaling. |
| `make test-decompose-aggregator` | Run Rust design-decomposition integration coverage for aggregation and waterfall failure handling. |
| `make test-scout-plan-archetypes-wrapper` | Run Cargo coverage for `/design` plan-review scout wrapper filtering, role-id forwarding, and retry behavior. This Cargo-backed target is a standalone local alias. |
| `make test-scout-dynamic-archetypes` | Run Cargo coverage for `/review` dynamic archetype scouting, including both context modes, staged-context warnings, and launcher tiers. This Cargo-backed target is a standalone local alias. |
| `make test-lib-scope-anchor-handoff` | Run Rust contract coverage for scope-anchor validate, render, relay, and handoff commands. This Cargo-backed target is a standalone local alias. |

### Claude drafter and voter harnesses

- `make test-launch-drafters` covers `/design` Step 2b drafter subprocess behavior through the Rust launcher authority: native Claude argv compatibility, JSON `.result` fail-closed handling, whole-line delimiter promotion, token/timing ledger rows, `.dirty-tree` sidecar scope, and baseline-delta cases.
- `make test-launch-claude-review` also pins the Rust launcher's `LARCH_VOTER_MODEL` role-based default for `launch-claude-review --role voter`: default `claude-sonnet-4-6`, environment override, and explicit `--model` precedence.
- `scripts/larch.sh plan-review preview` preserves small-plan and threshold-normalization behavior; `crates/larch-cli/tests/plan_review_loop_commands.rs` freezes its visible bytes.
- Step 2b drafter postplan fallback is once-only via `$DESIGN_TMPDIR/.step2b-postplan-inline-retry-done`; repeated postplan failures route to the existing validator-failure / abort branches rather than looping back into inline drafting again.

`render voter` and scope-anchor renderer coverage lives in the focused `larch-cli`
tests and frozen parity cases (`render_voter_parity.rs`, `scope_anchor_migrated_parity.rs`).
Rust rendering, Mermaid, and diagram publication coverage lives in the same suite.

## /design auto-reporting harnesses

Focused harnesses cover the /design auto-reporting port:

- `test-design-stage-terminal-state` runs the Rust terminal-command migrated-parity suite, including terminal-state staging and generic-token validation.
- `test-design-failure-report` runs the Rust terminal-command migrated-parity suite for teardown gates, operator-action skips, fallback chat output, and successful escalation reports.
- `test-design-step3-review` validates Step 3 resume-state, bgjob adapter, and merge-envelope ownership in the Rust `plan_review_step3_review` suite.
- `test-design-step5c` and `test-design-clarify` run the Rust owner suites directly. Their retired Bash harnesses are not members of a `test-harnesses-N` shard.
- `test-stall-recovery-report` includes design-prefix Tier B corpus coverage, generic-profile `/design` validation, prefixed artifacts, and skill-aware dedup signatures.

`scripts/larch.sh checks run-relevant --site <site> --tmpdir "$IMPLEMENT_TMPDIR"` maps the Rust `file-report` verb, its docs, render-final-summary, publish, Split-path prose, Step 3 review, cross-repo filing, and stall-report changes to these focused targets.

## Residual Bash lint scope

Bash-targeting linters use the residual shell manifest instead of broad repo
discovery. Runtime and local callers use
`"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" residual-bash paths [--root PATH]`;
fixture harnesses pass `--root` so they read the fixture manifest. CI
shellcheck compiles the dependency-free `larch-residual-bash-paths` binary from
the same canonical Rust module and enables existence checks unconditionally.

The residual manifest covers kept hooks, non-shim wrappers, the approved
`scripts/larch.sh` clean-install bootstrap, `scripts/sleep-seconds.sh`,
residual harnesses, and any standalone source `.awk` helper. Pure production
shims may stay outside the manifest only when `residual-bash-shim` verifies
their line bound and sole `scripts/larch.sh` exec. Terminal shared libraries,
retired non-thin helpers, and verified-zero-consumer includes are absent.

Agent-lint G010/G011 treat `scripts/agent-lint-script-inventory.txt` as their authoritative explicit scope, even if another rule excludes one of its paths. Add standalone awk helpers to that inventory in the same change. `crates/larch-lint/tests/agent_lint_script_inventory.rs` parses both manifests with the residual reader's fail-closed semantics and requires every residual Bash path to appear in the agent-lint inventory. CI shellcheck continues to read `scripts/residual-bash-paths.txt`. Test shard rebalance is deferred to `/rebalance-tests`.

## Bgjob background-launch lint

`larch lint rule bg-wait-coverage` rejects `run_in_background: true` prose anywhere under `skills/**`. The allowlist is `crates/larch-lint/config/bg-wait-allowlist.txt`; new rows must carry a reason and should be treated as temporary debt.

`make test-bgjob` runs `crates/larch-cli/tests/bgjob.rs`, a real-process harness for bgjob start, wait, cancelled wait, owner death, timeout, external daemon death, status, reap, confined merge-result publication, and slug rejection. The commands are Rust-owned, so the harness is a standalone `cargo test` alias carved out of the `test-harnesses` shards and covered in CI by the coverage execution lane.
