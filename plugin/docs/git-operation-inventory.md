# Git operation inventory

This matrix records every production source that reads local Git state or can
execute installed Git. The `gix-read` owner means the typed `RepositoryRead`
port implemented only by `crates/larch-adapters/src/git/repository.rs`.
The `git-cli` owner means a closed method on `GitCli`; it never means arbitrary
Git arguments. `later-domain` rows identify residual non-Rust production
surfaces.

Issue #8629 reuses the closed `ExactDiffRequest` and `PushRequest` families for
the standalone leaf driver's fixed `--numstat -z -M50%` measurement and remote
branch cleanup. `GitRefspec::deletion` constructs only the validated
empty-source deletion form; these typed options admit no arbitrary arguments
and add no Git request family.

The rule `git-ownership` compares this block with live production Rust,
skill, agent, hook, script, Makefile, and workflow surfaces. It also rejects
direct installed-Git construction through aliases, qualified constructors, or
constant and variable executable values. It pins the adapter's public methods
to the closed typed request families and rejects generic argv forwarding.
Keep the block tab-separated and sort each row's operation names.

<!-- markdownlint-disable MD010 -->
<!-- git-ownership-matrix:start -->
```text
surface	owner	issue	operations
.claude/skills/release/SKILL.md	later-domain	#7674	add,checkout,commit,fetch,rev-parse
.claude/skills/release/references/first-projection-release-runbook.md	later-domain	#7674	cat-file,diff,fetch,log,ls-remote,push,rev-parse,show,tag
.pre-commit-config.yaml	later-domain	#7686	rev-parse
agents/_implementer-base.md	later-domain	#7678	commit
agents/claude-self-reviewer.md	later-domain	#7678	merge-base
skills/implement/prompts/codex-implementer.md	later-domain	#7681	commit
skills/implement/prompts/cursor-implementer.md	later-domain	#7681	commit
crates/larch-adapters/src/git/mod.rs	git-cli	#7671	closed-cli-owner
crates/larch-adapters/src/git/repository.rs	gix-read	#7671	concrete-gix-owner
crates/larch-cli/src/admission_commands.rs	git-cli	#7671	typed-cli,typed-read
crates/larch-cli/src/audit_umbrella_commands.rs	gix-read	#7671	typed-read
crates/larch-cli/src/audit_runs_commands.rs	gix-read	#7671	typed-read
crates/larch-cli/src/analyze_bugs_commands.rs	gix-read	#7671	typed-read
crates/larch-cli/src/analyze_bugs_sweep.rs	gix-read	#7671	typed-read
crates/larch-cli/src/architectural_assessment_commands.rs	gix-read	#7671	typed-read
crates/larch-cli/src/architectural_preparation_commands.rs	gix-read	#7671	typed-read
crates/larch-cli/src/checks_identity_commands.rs	gix-read	#7671	typed-read
crates/larch-cli/src/checks_lint_fix_commands.rs	git-cli	#7671	typed-cli,typed-read
crates/larch-cli/src/checks_run_relevant_commands.rs	gix-read	#7671	typed-read
crates/larch-cli/src/checks_rust_clippy_commands.rs	gix-read	#7671	typed-read
crates/larch-cli/src/ci_failure_commands.rs	git-cli	#7671	typed-cli
crates/larch-cli/src/ci_monitor_commands.rs	git-cli	#7671	typed-cli
crates/larch-cli/src/ci_selection.rs	gix-read	#7671	typed-read
crates/larch-cli/src/complete_umbrella_commands.rs	git-cli	#7671	typed-cli,typed-read
crates/larch-cli/src/complete_umbrella_ship_commands.rs	git-cli	#7671	typed-cli,typed-read
crates/larch-cli/src/design_log_publish_commands.rs	gix-read	#7671	typed-read
crates/larch-cli/src/design_publish_commands.rs	gix-read	#7671	typed-read
crates/larch-cli/src/design_pause_commands.rs	gix-read	#7671	typed-read
crates/larch-cli/src/design_step1_commands.rs	gix-read	#7671	typed-read
crates/larch-cli/src/diagram_commands.rs	gix-read	#7671	typed-read
crates/larch-cli/src/dirty_tree_commands.rs	gix-read	#7671	typed-read
crates/larch-cli/src/developer_tooling_commands.rs	gix-read	#7671	typed-read
crates/larch-cli/src/eval_commands.rs	gix-read	#7671	typed-read
crates/larch-cli/src/forked_repo_commands.rs	git-cli	#7671	typed-cli,typed-read
crates/larch-cli/src/git_command_runtime.rs	git-cli	#7671	typed-cli
crates/larch-cli/src/drafter_commands.rs	gix-read	#7671	typed-read
crates/larch-cli/src/git_commands.rs	git-cli	#7671	typed-cli,typed-read
crates/larch-cli/src/github_repository_resolution.rs	gix-read	#7671	typed-read
crates/larch-cli/src/hook_commands.rs	gix-read	#7671	typed-read
crates/larch-cli/src/implement_bootstrap_continuation.rs	git-cli	#7671	typed-cli,typed-read
crates/larch-cli/src/implement_commit_route_commands.rs	gix-read	#7671	typed-read
crates/larch-cli/src/implement_dispatch_commands.rs	gix-read	#7671	typed-read
crates/larch-cli/src/implement_finalize_commands.rs	git-cli	#7671	typed-cli,typed-read
crates/larch-cli/src/implement_preflight_commands.rs	gix-read	#7671	typed-read
crates/larch-cli/src/implement_review_commands.rs	gix-read	#7671	typed-read
crates/larch-cli/src/implement_scope_disposition_commands.rs	gix-read	#7671	typed-read
crates/larch-cli/src/implement_scope_disposition_commands_impl.rs	gix-read	#7671	typed-read
crates/larch-cli/src/implement_step2_commands.rs	gix-read	#7671	typed-read
crates/larch-cli/src/implement_step2_commands_impl.rs	git-cli	#7671	typed-cli,typed-read
crates/larch-cli/src/implement_step2_post_commands.rs	gix-read	#7671	typed-read
crates/larch-cli/src/launcher_support.rs	gix-read	#7671	typed-read
crates/larch-cli/src/learn_from_bugs_commands.rs	gix-read	#7671	typed-read
crates/larch-cli/src/migration_audit_commands.rs	git-cli	#7671	typed-cli,typed-read
crates/larch-cli/src/migration_governance_commands.rs	git-cli	#7671	typed-cli,typed-read
crates/larch-cli/src/main.rs	git-cli	#7671	typed-cli,typed-read
crates/larch-cli/src/pr_commands.rs	gix-read	#7671	typed-read
crates/larch-cli/src/push_network.rs	git-cli	#7671	typed-cli,typed-read
crates/larch-cli/src/push_rebase.rs	git-cli	#7671	typed-cli,typed-read
crates/larch-cli/src/release_common.rs	git-cli	#7671	typed-cli,typed-read
crates/larch-cli/src/release_assets.rs	gix-read	#7671	typed-read
crates/larch-cli/src/release_plugin_runtime.rs	gix-read	#7671	typed-read
crates/larch-cli/src/release_prepare.rs	gix-read	#7671	typed-read
crates/larch-cli/src/release_publish.rs	gix-read	#7671	typed-read
crates/larch-cli/src/release_stage.rs	git-cli	#7671	typed-cli,typed-read
crates/larch-cli/src/rebalance_tests_workflow.rs	git-cli	#7671	typed-cli,typed-read
crates/larch-cli/src/redact_commands.rs	git-cli	#7671	typed-cli
crates/larch-cli/src/rejected_analysis_commands.rs	gix-read	#7671	typed-read
crates/larch-cli/src/rendering_commands.rs	gix-read	#7671	typed-read
crates/larch-cli/src/repo_size_commands.rs	gix-read	#7671	typed-read
crates/larch-cli/src/research_commands.rs	gix-read	#7671	typed-read
crates/larch-cli/src/review_commands.rs	gix-read	#7671	typed-read
crates/larch-cli/src/review_and_fix_commands.rs	git-cli	#7671	typed-cli,typed-read
crates/larch-cli/src/run_log_commands.rs	gix-read	#7671	typed-read
crates/larch-cli/src/session_closeout_commands.rs	git-cli	#7671	typed-cli,typed-read
crates/larch-cli/src/session_setup_commands.rs	gix-read	#7671	typed-read
crates/larch-cli/src/ship_pr_commands.rs	gix-read	#7671	typed-read
crates/larch-cli/src/merge_commands.rs	gix-read	#7671	typed-read
crates/larch-cli/src/ship_pre_driver_commands.rs	gix-read	#7671	typed-read
crates/larch-cli/src/stall_recovery_commands.rs	gix-read	#7671	typed-read
crates/larch-cli/src/stall_recovery_reporting.rs	gix-read	#7671	typed-read
crates/larch-cli/src/token_commands.rs	gix-read	#7671	typed-read
crates/larch-cli/src/token_measurement_commands.rs	gix-read	#7671	typed-read
crates/larch-cli/src/triage_commands.rs	git-cli	#7671	typed-cli,typed-read
crates/larch-lint/src/repository.rs	bootstrap	#7736	repository-discovery,tracked-paths
crates/larch-cli/src/agent_commands.rs	git-cli	#7671	typed-cli,typed-read
crates/larch-cli/src/oos_commands.rs	gix-read	#7671	typed-read
scripts/check-stale-plugin.sh	later-domain	#7674	rev-parse
skills/implement/references/checks-repair-loop.md	later-domain	#7681	rev-parse
skills/implement/references/codex-manifest-schema.md	later-domain	#7681	commit
skills/implement/references/step2-dispatch.md	later-domain	#7681	commit
skills/implement/scripts/oos-disposition-gate.md	later-domain	#7681	merge-base
```
<!-- git-ownership-matrix:end -->
<!-- markdownlint-enable MD010 -->

Tests and repository-only bootstrap code are not production exceptions.
`#[cfg(test)]` fixture setup and `larch-test-support` may execute Git as an
independent oracle. The lint bootstrap row above is confined to repository
discovery and tracked-path enumeration because `larch-lint` cannot depend on
product crates while it validates them. Production suppression cannot widen
either exception. The rule also re-runs the command registry's syntax-aware
Python retirement proof for every #7675 command and rejects the retired
`push rebase` state-machine symbols.
