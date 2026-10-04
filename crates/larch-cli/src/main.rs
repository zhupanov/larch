use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    ffi::{OsStr, OsString},
    fmt::Write as _,
    fs,
    io::ErrorKind,
    io::Write as _,
    path::{Path, PathBuf},
    process::ExitCode,
    thread,
    time::Duration,
};

use clap::{Args, CommandFactory, FromArgMatches, Parser, Subcommand};
use larch_cli::object_store_commands::{self, GcsArguments};
use larch_core::{ChangeKind, RepositoryStatus, StatusOptions, private_atomic_write};

use crate::argparse_compat::python_io_error;

mod admission_commands;
mod agent_commands;
mod agent_review;
mod analysis_state;
mod analyze_bugs_commands;
mod analyze_bugs_sweep;
mod analyze_issues_commands;
mod architectural_assessment_commands;
mod architectural_preparation_commands;
mod argparse_compat;
mod audit_runs_commands;
mod audit_umbrella_commands;
mod bgjob_adapt;
mod bgjob_commands;
mod blocker_commands;
pub(crate) mod bootstrap_commands;
mod bootstrap_support;
mod calibration_commands;
mod checks_identity_commands;
mod checks_lint_fix_commands;
mod checks_run_relevant_commands;
mod checks_rust_clippy_commands;
mod child_process;
mod ci_failure_commands;
mod ci_launcher_commands;
mod ci_monitor_commands;
mod ci_policy_candidate_commands;
mod ci_selection;
mod ci_timing;
mod clarify_commands;
mod clarify_orchestrator;
pub(crate) mod claude_commands;
mod cleanup_commands;
mod collector_commands;
mod combine_issues_commands;
mod complete_umbrella_commands;
mod complete_umbrella_ship_commands;
mod debate_commands;
mod debate_publication_commands;
mod decompose_commands;
mod deps_audit_commands;
mod design_commands;
mod design_dialectic_commands;
mod design_finalize_commands;
mod design_gate_summary_commands;
mod design_log_publish_commands;
mod design_oos_commands;
mod design_pause_commands;
mod design_publish_commands;
mod design_settle_commands;
mod design_step0_commands;
mod design_step1_commands;
mod design_step2b_commands;
mod design_step3_commands;
mod design_terminal_commands;
mod developer_tooling_commands;
mod diagram_commands;
mod difficulty_calibration_commands;
mod difficulty_commands;
mod dirty_tree_commands;
mod drafter_commands;
mod execution_issue_commands;
mod external_agent;
mod external_defaults_commands;
pub(crate) mod final_report_commands;
mod fluff_analysis_commands;
mod forked_repo_commands;
mod git_command_runtime;
mod git_commands;
mod github_repository_resolution;
mod github_service;
mod gitleaks;
mod hook_commands;
mod html;
pub(crate) mod implement_bootstrap_continuation;
mod implement_child_seam;
mod implement_closeout_commands;
mod implement_commands;
mod implement_commit_route_commands;
mod implement_dispatch_commands;
mod implement_finalize_commands;
mod implement_launcher_commands;
mod implement_leg_commands;
mod implement_preflight_commands;
mod implement_review_commands;
mod implement_scope_disposition_commands;
mod implement_ship_commands;
mod implement_step2_commands;
mod implement_step2_post_commands;
mod implement_terminal_commands;
mod issue_batch_create_commands;
mod issue_commands;
mod issue_create_commands;
mod issue_dependency_commands;
mod issue_input_commands;
mod issue_mutation_support;
mod issue_wire_commands;
mod kill_background;
mod launcher_support;
mod learn_from_bugs_commands;
mod merge_commands;
mod migration_audit_commands;
mod migration_governance_commands;
mod net_commands;
mod oos_commands;
mod oos_file_commands;
mod plan_prompt_commands;
mod plan_quality_commands;
mod plan_quality_revise_commands;
mod plan_review_commands;
mod plan_review_mav_commands;
mod plan_review_step3_review;
mod pr_commands;
mod progress_commands;
mod push_network;
mod push_rebase;
mod rebalance_tests;
mod rebalance_tests_workflow;
mod redact_commands;
mod rejected_analysis_commands;
mod release_assets;
mod release_common;
mod release_plugin_runtime;
mod release_prepare;
mod release_publish;
mod release_stage;
mod release_version;
mod rendering_commands;
mod repo_size_commands;
mod run_lifecycle_commands;
mod run_log_cleanup_commands;
mod run_log_commands;
mod run_log_entry_commands;
pub(crate) mod run_log_migration_commands;
mod run_log_publication_commands;
mod runtime_entrypoint;
mod scout_commands;
mod session_artifact_support;
mod validate_merged_commands;
#[rustfmt::skip]
mod run_log_flush_commands;
mod eval_commands;
mod ledger_append;
mod report_tokens_commands;
mod research_commands;
mod review_and_fix_commands;
mod review_commands;
mod review_compose_commands;
mod review_core_commands;
mod review_dispatch_panel;
mod review_findings_commands;
mod review_loop_identity_commands;
mod review_tally_commands;
mod session_closeout_commands;
mod session_env_commands;
mod session_gate_commands;
mod session_lifecycle_commands;
mod session_setup_commands;
mod ship_commands;
mod ship_pr_commands;
mod ship_pre_driver_commands;
mod ship_recovery_commands;
mod slack_commands;
mod slot_binding;
mod stall_recovery_commands;
mod stall_recovery_file_report;
mod stall_recovery_reporting;
mod state_commands;
mod status_commands;
mod test_shards;
mod timing_commands;
mod token_commands;
mod token_measurement_commands;
mod tracking_issue_commands;
mod triage_commands;
mod umbrella_commands;
mod voter_calibration_commands;
mod voter_dispatch_commands;
mod voting_commands;
mod waterfall_commands;

use agent_commands::AgentCommand;
use audit_umbrella_commands::AuditUmbrellaCommand;
use ci_selection::CiCommand;
use ci_timing::CiTimingCommand;
use complete_umbrella_commands::CompleteUmbrellaCommand;
use developer_tooling_commands::{AliasCommand, ResidualBashCommand, VerifyCommand};
use external_defaults_commands::ExternalDefaultsCommand;
use git_commands::GitCommand;
use net_commands::NetCommand;
use plan_review_commands::PlanReviewCommand;
use rebalance_tests::RebalanceTestsCommand;
use repo_size_commands::RepoCommand;
use review_and_fix_commands::ReviewAndFixCommand;
use review_commands::ReviewCommand;
use slack_commands::SlackCommand;
use test_shards::TestShardCommand;

#[derive(Parser)]
#[command(
    name = "larch",
    about = "Larch workflow automation",
    arg_required_else_help = true,
    subcommand_required = true
)]
struct Cli {
    #[command(subcommand)]
    domain: Domain,
}

#[derive(Subcommand)]
enum Domain {
    /// `/implement` entry admission, preflight, and fork bootstrap.
    #[command(subcommand)]
    Admission(AdmissionCommand),
    /// Vendor-agent launch and diagnostic commands.
    #[command(subcommand)]
    Agent(AgentCommand),
    /// Review pipeline commands.
    #[command(subcommand)]
    Review(ReviewCommand),
    /// Repair code-review findings.
    #[command(subcommand, name = "review-and-fix")]
    ReviewAndFix(ReviewAndFixCommand),
    /// Plan-review workflow commands.
    #[command(subcommand, name = "plan-review")]
    PlanReview(PlanReviewCommand),
    /// Alias generation and target-resolution helpers.
    #[command(subcommand)]
    Alias(AliasCommand),
    /// Issue blocker discovery.
    #[command(subcommand)]
    Blocker(BlockerCommand),
    /// Native issue blocked-by dependency mutations.
    #[command(subcommand, name = "block-issue")]
    BlockIssue(BlockIssueCommand),
    /// Recorded voter-calibration fixture replay commands.
    #[command(subcommand, name = "calibration-replay")]
    CalibrationReplay(CalibrationReplayCommand),
    /// Internal bootstrap commands used before installation completes.
    #[command(subcommand, hide = true)]
    Bootstrap(BootstrapCommand),
    /// The `/design` clarification round-trip: state, comments, and labels.
    #[command(subcommand)]
    Clarify(ClarifyCommand),
    /// Remove stale larch session directories and pointers.
    #[command(subcommand)]
    Cleanup(CleanupCommand),
    /// Durable background-job compatibility commands.
    #[command(subcommand)]
    Bgjob(BgjobCommand),
    /// Collect GitHub Actions timing inputs and resolve trusted source runs.
    #[command(subcommand)]
    CiTiming(CiTimingCommand),
    /// `/implement` checks-loop attribution reads.
    #[command(subcommand)]
    Checks(ChecksCommand),
    /// Fail-closed Rust CI selection, history helpers, and main-cache candidates.
    #[command(subcommand)]
    Ci(CiCommand),
    /// Step 8 architectural assessment materialize/submit/report helpers.
    #[command(subcommand, name = "architectural-assessment")]
    ArchitecturalAssessment(ArchitecturalAssessmentCommand),
    /// Prepare repository architectural guidelines for design and implementation gates.
    #[command(subcommand, name = "architectural-guidelines")]
    ArchitecturalGuidelines(ArchitecturalPreparationCommand),
    /// Prepare repository architectural invariants for design and implementation gates.
    #[command(subcommand, name = "architectural-invariants")]
    ArchitecturalInvariants(ArchitecturalPreparationCommand),
    /// Serially complete and audit every direct leaf of one umbrella issue.
    #[command(subcommand)]
    CompleteUmbrella(CompleteUmbrellaCommand),
    /// Audit one managed umbrella and reconcile its exhaustive corrective batch.
    #[command(subcommand, name = "audit-umbrella")]
    AuditUmbrella(AuditUmbrellaCommand),
    /// Combine related issues while preserving their dependency graph.
    #[command(subcommand, name = "combine-issues")]
    CombineIssues(CombineIssuesCommand),
    /// Durable two-round debate protocol commands.
    #[command(subcommand)]
    Debate(DebateCommand),
    /// Working-tree checkpoint and scope compatibility commands.
    #[command(subcommand)]
    DirtyTree(DirtyTreeCommand),
    /// The `/deps` open-issue dependency audit: reads, plan, and one apply.
    #[command(subcommand)]
    Deps(DepsCommand),
    /// `/design` Split-path decomposition and panel commands.
    #[command(subcommand)]
    Decompose(DecomposeCommand),
    /// `/design` Step 0 argv parsing, routing, and run-params initialization.
    #[command(subcommand)]
    Design(DesignCommand),
    /// Difficulty rating, record, panel, and label commands.
    #[command(subcommand)]
    Difficulty(DifficultyCommand),
    /// Retrospective predicted-versus-realized difficulty analysis.
    #[command(subcommand, name = "difficulty-calibration")]
    DifficultyCalibration(DifficultyCalibrationCommand),
    /// The `/implement` execution-issue ledger lifecycle.
    #[command(subcommand, name = "execution-issues")]
    ExecutionIssues(ExecutionIssuesCommand),
    /// External tool default readers.
    #[command(subcommand, name = "external-defaults")]
    ExternalDefaults(ExternalDefaultsCommand),
    /// Non-production commands that exercise dispatcher wiring.
    #[command(subcommand)]
    Example(ExampleCommand),
    /// The `/fluff-analysis` review-fluff report.
    #[command(subcommand, name = "fluff-analysis")]
    FluffAnalysis(FluffAnalysisCommand),
    /// Configure an upstream/fork open-source checkout.
    #[command(subcommand, name = "forked-repo")]
    ForkedRepo(ForkedRepoCommand),
    /// Local Git repository commands.
    #[command(subcommand)]
    Git(GitSubcommand),
    /// Pull-request merge and merge-queue commands.
    #[command(subcommand)]
    Merge(MergeCommand),
    /// Advisory Claude Code hook commands.
    #[command(subcommand)]
    Hook(HookCommand),
    /// `/implement` bootstrap, preflight, scout, recovery, and step checks.
    #[command(subcommand)]
    Implement(ImplementCommand),
    /// Post-ship rebase, push, local cleanup, and terminal teardown.
    #[command(subcommand, name = "implement-finalize")]
    ImplementFinalize(ImplementFinalizeCommand),
    /// Ship pre-driver routing, state, result, and rebase-repair commands.
    #[command(subcommand)]
    Ship(ShipCommand),
    /// GitHub issue reads and issue-body wire helpers.
    #[command(subcommand)]
    Issue(IssueCommand),
    /// The `larch:plan` issue-body block carrying the `/design` handoff.
    #[command(subcommand, name = "plan-block")]
    PlanBlock(PlanBlockCommand),
    /// Refresh the durable plan-receipt identity.
    #[command(subcommand, name = "plan-receipt")]
    PlanReceipt(PlanReceiptCommand),
    /// Inspect the installed larch and external tool health.
    Status(StatusArguments),
    /// One named `larch:<marker>` issue-body block.
    #[command(subcommand, name = "named-block")]
    NamedBlock(NamedBlockCommand),
    /// Implementation-plan readers.
    #[command(subcommand)]
    Plan(PlanCommand),
    /// Pull-request summary composition.
    #[command(subcommand)]
    Pr(PrCommand),
    /// Dynamic reviewer archetype scouting and manifest filtering.
    #[command(subcommand)]
    Scout(ScoutCommand),
    /// The tracking issue's lifecycle: reads, comments, titles, and summaries.
    #[command(subcommand, name = "tracking-issue")]
    TrackingIssue(TrackingIssueCommand),
    /// Implementation tracking-comment composition and publication.
    #[command(subcommand)]
    Tracking(TrackingCommand),
    /// Pre-`/design` issue verification: evidence, probes, and the one write.
    #[command(subcommand)]
    Triage(TriageCommand),
    /// Durable `/umbrella` preparation, record state, and completion proof.
    #[command(subcommand)]
    Umbrella(UmbrellaCommand),
    /// Envelopes that mark fetched text as data, never instructions.
    #[command(subcommand)]
    Untrusted(UntrustedCommand),
    /// Exact `KEY=value` stream readers.
    #[command(subcommand)]
    Kv(KvCommand),
    /// Repository policy lint commands.
    Lint(larch_lint::LintArguments),
    /// Plugin metadata commands.
    #[command(subcommand)]
    Plugin(PluginCommand),
    /// Clone-scoped progress breadcrumbs and the larch statusline.
    #[command(subcommand)]
    Progress(ProgressCommand),
    /// Regenerate or verify committed developer artifacts.
    #[command(disable_help_flag = true)]
    Generate(RawCompatibilityArguments),
    /// Generic ASCII Gantt rendering.
    #[command(subcommand)]
    Gantt(GanttCommand),
    /// Generate the committed-diff Mermaid code-flow diagram.
    #[command(subcommand)]
    Diagram(DiagramCommand),
    /// Rust-owned runtime prompt and view renderers.
    #[command(subcommand)]
    Render(RenderCommand),
    /// Validate and relay the `/design` plan-review scope anchor.
    #[command(subcommand, name = "scope-anchor")]
    ScopeAnchor(ScopeAnchorCommand),
    /// Mermaid safety checks.
    #[command(subcommand)]
    Mermaid(MermaidCommand),
    /// Shared issue-comment diagram publication.
    #[command(subcommand)]
    Diagrams(DiagramsCommand),
    /// Issue-backlog report rendering.
    #[command(subcommand, name = "analyze-issues")]
    AnalyzeIssues(AnalyzeIssuesCommand),
    /// Run-log audit preflight, titles, advisory, mapping, scanning, and closure.
    #[command(subcommand, name = "audit-runs")]
    AuditRuns(AuditRunsCommand),
    /// Bounded filed-bug evidence and verification commands.
    #[command(subcommand, name = "analyze-bugs")]
    AnalyzeBugs(AnalyzeBugsCommand),
    /// Recover verified rejected code-review findings from run logs.
    #[command(subcommand, name = "rejected-analysis")]
    RejectedAnalysis(RejectedAnalysisCommand),
    /// Prepare compact bug-learning evidence and maintain its durable marker.
    #[command(subcommand, name = "learn-from-bugs")]
    LearnFromBugs(LearnFromBugsCommand),
    /// Inspect recent first-parent merges for possible unfiled bugs.
    #[command(subcommand, name = "validate-merged")]
    ValidateMerged(ValidateMergedCommand),
    /// Narrow provider transports used by Rust-owned run-log workflows.
    #[command(subcommand)]
    ObjectStore(ObjectStoreCommand),
    /// Fixed-endpoint connectivity helpers.
    #[command(subcommand)]
    Net(NetCommand),
    /// Composition, capping, ordering, and disposition of a run's OOS batch.
    #[command(subcommand)]
    Oos(OosCommand),
    /// Release-maintenance commands.
    #[command(subcommand)]
    Release(ReleaseCommand),
    /// Repository-scoped developer commands.
    #[command(subcommand)]
    Repo(RepoCommand),
    /// List retained Bash paths from the residual manifest.
    #[command(subcommand, name = "residual-bash")]
    ResidualBash(ResidualBashCommand),
    /// Terminal `/implement` final-report composition and publication.
    #[command(subcommand, name = "final-report")]
    FinalReport(FinalReportCommand),
    /// Token-cost analysis over the synchronized run-log corpus.
    #[command(subcommand, name = "report-tokens")]
    ReportTokens(ReportTokensCommand),
    /// The `/research` preparation commands: banner, planner, findings, citations.
    #[command(subcommand)]
    Research(ResearchCommand),
    /// The `/research` evaluation commands: output validation and the eval harness.
    #[command(subcommand)]
    Eval(EvalCommand),
    /// CI test-rebalance planning, verification, and checked orchestration.
    #[command(subcommand, name = "rebalance-tests")]
    RebalanceTests(RebalanceTestsCommand),
    /// Secret, path, log, and submodule-finding redaction commands.
    #[command(subcommand)]
    Redact(RedactCommand),
    /// Session state compatibility commands.
    #[command(subcommand)]
    Session(SessionCommand),
    /// Slack announcement helpers.
    #[command(subcommand)]
    Slack(SlackCommand),
    /// Stall-recovery state and validation commands.
    #[command(name = "stall-recovery", disable_help_flag = true)]
    StallRecovery(RawCompatibilityArguments),
    /// Pack and rewrite deterministic test-shard assignments.
    #[command(subcommand)]
    TestShard(TestShardCommand),
    /// Small side-effect verification helpers.
    #[command(subcommand)]
    Verify(VerifyCommand),
    /// Timing-ledger marks, records, dumps, and reports.
    #[command(subcommand)]
    Timing(TimingCommand),
    /// Token-ledger marks, vendor rows, dumps, and lane telemetry.
    #[command(subcommand, name = "token")]
    Token(TokenCommand),
    /// GitHub workflow helper commands.
    #[command(subcommand)]
    Gh(GhCommand),
    /// Push commands with typed Git network operations.
    #[command(subcommand)]
    Push(PushSubcommand),
    /// Committed run-log identity and layout helpers.
    #[command(subcommand, name = "run-log")]
    RunLog(RunLogCommand),
    /// Vote parsing, tally state, panel rendering, and parse-rate checks.
    #[command(subcommand)]
    Voting(VotingCommand),
    /// Voter calibration snapshot and analyzer commands.
    #[command(subcommand, name = "voter-calibration")]
    VoterCalibration(VoterCalibrationCommand),
    /// Upgrade the installed larch plugin and executable.
    #[command(subcommand)]
    UpgradeLarch(UpgradeLarchCommand),
}

#[derive(Subcommand)]
enum CalibrationReplayCommand {
    #[command(name = "rebuild-ballot", disable_help_flag = true)]
    RebuildBallot(RawCompatibilityArguments),
    #[command(name = "run-replay", disable_help_flag = true)]
    RunReplay(RawCompatibilityArguments),
    #[command(name = "validate-manifest", disable_help_flag = true)]
    ValidateManifest(RawCompatibilityArguments),
}

#[derive(Subcommand)]
enum VoterCalibrationCommand {
    #[command(disable_help_flag = true)]
    Snapshot(RawCompatibilityArguments),
    /// Analyze voter agreement and severity calibration over one corpus.
    #[command(disable_help_flag = true)]
    Analyze(RawCompatibilityArguments),
}

macro_rules! voting_commands {
    ($(($variant:ident, $name:literal, $handler:ident)),+ $(,)?) => {
        #[derive(Subcommand)]
        enum VotingCommand {
            $(#[command(name = $name, disable_help_flag = true)]
            $variant(RawCompatibilityArguments),)+
        }
        impl VotingCommand {
            fn run(self) -> ExitCode {
                let arguments = std::env::args_os().skip(3).collect::<Vec<_>>();
                match self {
                    $(Self::$variant(_) => voting_commands::$handler(&arguments),)+
                }
            }
        }
    };
}
#[rustfmt::skip]
voting_commands!(
    (AcceptFinding, "accept-finding", accept_finding_command),
    (BallotParse, "ballot-parse", ballot_parse),
    (ClassifyResult, "classify-result", classify_result),
    (CodeReviewClassificationHeader, "code-review-classification-header", code_review_classification_header_command),
    (ComposeTallyRecord, "compose-tally-record", compose_tally_record),
    (DegradedWarning, "degraded-warning", degraded_warning),
    (EffectiveJudges, "effective-judges", effective_judges),
    (FalsePositiveMatch, "false-positive-match", false_positive_match_command),
    (FileLineRegex, "file-line-regex", file_line_regex_command),
    (FindingsClassificationHeader, "findings-classification-header", findings_classification_header),
    (IsSecurityBlock, "is-security-block", is_security_block),
    (PanelTier, "panel-tier", panel_tier_command),
    (ParseJudgeVote, "parse-judge-vote", parse_judge_vote),
    (ParseRateCheck, "parse-rate-check", parse_rate_check),
    (ParseRateDiagMatches, "parse-rate-diag-matches", parse_rate_diag_matches),
    (ParseRateRetry, "parse-rate-retry", parse_rate_retry),
    (ReviewerForBlock, "reviewer-for-block", reviewer_for_block),
    (Scoreboard, "scoreboard", scoreboard),
    (SplitBallot, "split-ballot", split_ballot),
    (TallyVote, "tally-vote", tally_vote),
    (VoterStatusBlock, "voter-status-block", voter_status_block),
    (VoteForId, "vote-for-id", vote_for_id),
    (WriteTally, "write-tally", write_tally),
);

#[derive(Subcommand)]
enum RunLogCommand {
    /// Package one completed run-log staging tree as a deterministic archive.
    #[command(name = "archive", disable_help_flag = true)]
    Archive(RawCompatibilityArguments),
    /// Refresh recoverable artifacts after one implementation checkpoint.
    #[command(name = "checkpoint", disable_help_flag = true)]
    Checkpoint(RawCompatibilityArguments),
    /// Render and stage one filtered session transcript.
    #[command(name = "capture-transcript", disable_help_flag = true)]
    CaptureTranscript(RawCompatibilityArguments),
    /// Synthesize a v2 run manifest for one skill and run id.
    #[command(name = "init", disable_help_flag = true)]
    Init(RawCompatibilityArguments),
    /// Replace one batch artifact from a redacted, validated payload.
    #[command(name = "write", disable_help_flag = true)]
    Write(RawCompatibilityArguments),
    /// Publish one review round's included artifacts.
    #[command(name = "write-round", disable_help_flag = true)]
    WriteRound(RawCompatibilityArguments),
    /// Append one record to an append-mode batch artifact.
    #[command(name = "append", disable_help_flag = true)]
    Append(RawCompatibilityArguments),
    /// Append one execution-issue entry under a category heading.
    #[command(name = "append-entry", disable_help_flag = true)]
    AppendEntry(RawCompatibilityArguments),
    /// Append one formatted tool-failure entry with captured diagnostics.
    #[command(name = "append-failure", disable_help_flag = true)]
    AppendFailure(RawCompatibilityArguments),
    /// Report whether a known batch artifact exists.
    #[command(name = "exists", disable_help_flag = true)]
    Exists(RawCompatibilityArguments),
    /// Verify a published run directory against the required-files manifest.
    #[command(name = "verify-completeness", disable_help_flag = true)]
    VerifyCompleteness(RawCompatibilityArguments),
    /// Update one versioned run-log manifest with durable atomic publication.
    #[command(name = "manifest", disable_help_flag = true)]
    Manifest(RawCompatibilityArguments),
    /// Verify and atomically materialize one archived run-log tree.
    #[command(name = "materialize", disable_help_flag = true)]
    Materialize(RawCompatibilityArguments),
    /// Plan, apply, and independently verify the one-time run-log layout migration.
    #[command(name = "migrate-layout", disable_help_flag = true)]
    MigrateLayout(RawCompatibilityArguments),
    /// Clean redundant artifacts from completed historical implement run logs.
    #[command(name = "cleanup-implement-logs", disable_help_flag = true)]
    CleanupImplementLogs(RawCompatibilityArguments),
    /// Publish one immutable completed run archive and verified local cache.
    #[command(name = "publish", disable_help_flag = true)]
    Publish(RawCompatibilityArguments),
    /// Publish the session's redacted quiet logs as a run's breadcrumbs.
    #[command(name = "publish-breadcrumbs", disable_help_flag = true)]
    PublishBreadcrumbs(RawCompatibilityArguments),
    /// Synchronize the immutable remote run-log corpus into the local cache.
    #[command(name = "sync", disable_help_flag = true)]
    Sync(RawCompatibilityArguments),
    /// Correct historical Cursor cost lines in committed run summaries.
    #[command(name = "retro-fix-cursor", disable_help_flag = true)]
    RetroFixCursor(RawCompatibilityArguments),
    /// Render one raw Claude Code session JSONL as the committed chat view.
    #[command(name = "render-session-transcript", disable_help_flag = true)]
    RenderSessionTranscript(RawCompatibilityArguments),
    /// Rewrite historical session transcripts to the v3 redaction policy.
    #[command(name = "retro-v3-sweep", disable_help_flag = true)]
    RetroV3Sweep(RawCompatibilityArguments),
    /// Prepare the complete mutable snapshot immediately before publication.
    #[command(name = "prepare-terminal-snapshot", disable_help_flag = true)]
    PrepareTerminalSnapshot(RawCompatibilityArguments),
    /// Refresh the mutable implement run-log staging tree.
    #[command(name = "refresh", disable_help_flag = true)]
    Refresh(RawCompatibilityArguments),
    /// Terminalize a run as operator-cancelled.
    #[command(name = "lifecycle-cancel")]
    LifecycleCancel(run_lifecycle_commands::LifecycleTerminalArguments),
    /// Terminalize a run after a non-error early return.
    #[command(name = "lifecycle-early-return")]
    LifecycleEarlyReturn(run_lifecycle_commands::LifecycleTerminalArguments),
    /// Terminalize a failed run.
    #[command(name = "lifecycle-failure")]
    LifecycleFailure(run_lifecycle_commands::LifecycleTerminalArguments),
    /// Terminalize a successful run.
    #[command(name = "lifecycle-finalize")]
    LifecycleFinalize(run_lifecycle_commands::LifecycleTerminalArguments),
    /// Admit and persist one shared lifecycle run.
    #[command(name = "lifecycle-start")]
    LifecycleStart(run_lifecycle_commands::LifecycleStartArguments),
    /// Resolve storage configuration and run provider prefix preflight.
    #[command(name = "storage-preflight", disable_help_flag = true)]
    StoragePreflight(RawCompatibilityArguments),
    /// Emit `VALID=true|false` for a run-log path slug.
    #[command(name = "validate-run-id", disable_help_flag = true)]
    ValidateRunId(RawCompatibilityArguments),
}

#[derive(Subcommand)]
enum BgjobCommand {
    /// Start or reattach to a durable background job.
    #[command(disable_help_flag = true)]
    Adapt(RawCompatibilityArguments),
    /// Launch a detached background job for one workflow step.
    #[command(disable_help_flag = true)]
    Start(RawCompatibilityArguments),
    /// Wait one bounded chunk for a background job to finish.
    #[command(disable_help_flag = true)]
    Wait(RawCompatibilityArguments),
    /// Print one row per durable background-job registry entry.
    #[command(disable_help_flag = true)]
    Status(RawCompatibilityArguments),
    /// Remove finished, unreadable, and expired registry entries.
    #[command(disable_help_flag = true)]
    Reap(RawCompatibilityArguments),
    /// Write a confined child-to-daemon merge-result envelope.
    #[command(name = "write-merge-result-env", disable_help_flag = true)]
    WriteMergeResultEnv(RawCompatibilityArguments),
}

#[derive(Subcommand)]
enum HookCommand {
    /// Warn after repeated identical Read calls without ever blocking the hook.
    #[command(name = "anti-read-poll", disable_help_flag = true)]
    AntiReadPoll(RawCompatibilityArguments),
    /// Append one opt-in Edit/Write audit record without blocking tool use.
    #[command(name = "audit-edit-write", disable_help_flag = true)]
    AuditEditWrite(RawCompatibilityArguments),
    /// Deny edits inside checked-out submodules of the current superproject.
    #[command(name = "block-submodule-edit", disable_help_flag = true)]
    BlockSubmoduleEdit(RawCompatibilityArguments),
    /// Launch the age-based `SessionStart` cleanup without blocking startup.
    #[command(name = "cleanup-sessionstart", disable_help_flag = true)]
    CleanupSessionstart(RawCompatibilityArguments),
    /// Confine active read-only skill writes to larch scratch roots.
    #[command(name = "deny-edit-write", disable_help_flag = true)]
    DenyEditWrite(RawCompatibilityArguments),
    /// Deny background Bash launches while this clone owns a bgjob row.
    #[command(name = "deny-run-in-background", disable_help_flag = true)]
    DenyRunInBackground(RawCompatibilityArguments),
    /// Emit non-blocking `SessionStart` environment and workflow advisories.
    #[command(name = "sessionstart-health", disable_help_flag = true)]
    SessionstartHealth(RawCompatibilityArguments),
    /// Reset and install the clone-local progress statusline at `SessionStart`.
    #[command(name = "sessionstart-statusline", disable_help_flag = true)]
    SessionstartStatusline(RawCompatibilityArguments),
    /// Block Stop at a pending post-review implementation boundary.
    #[command(name = "stop-fail-close", disable_help_flag = true)]
    StopFailClose(RawCompatibilityArguments),
}

#[derive(Subcommand)]
enum FinalReportCommand {
    /// Compose and publish the terminal report for one run.
    #[command(disable_help_flag = true)]
    Write(RawCompatibilityArguments),
    /// Refresh the terminal report during Step 18b.
    #[command(disable_help_flag = true)]
    Step18b(RawCompatibilityArguments),
}

#[derive(Subcommand)]
enum ReportTokensCommand {
    /// Price the synchronized run-log corpus and render the token report.
    #[command(disable_help_flag = true)]
    Analyze(RawCompatibilityArguments),
}

#[derive(Subcommand)]
enum ResearchCommand {
    /// Emit the reduced-lane-diversity banner for a lane-status file.
    #[command(name = "banner", disable_help_flag = true)]
    Banner(RawCompatibilityArguments),
    /// Sanitize raw candidate questions into the research planner output.
    #[command(name = "run-planner", disable_help_flag = true)]
    RunPlanner(RawCompatibilityArguments),
    /// Render the findings issue-batch payload from a research report.
    #[command(name = "render-findings-batch", disable_help_flag = true)]
    RenderFindingsBatch(RawCompatibilityArguments),
    /// Validate URL, DOI, and `file:line` citations into the sidecar.
    #[command(name = "validate-citations", disable_help_flag = true)]
    ValidateCitations(RawCompatibilityArguments),
}

impl ResearchCommand {
    fn run(self) -> ExitCode {
        let arguments = std::env::args_os().skip(3).collect::<Vec<_>>();
        match self {
            Self::Banner(_) => research_commands::banner(&arguments),
            Self::RunPlanner(_) => research_commands::run_planner(&arguments),
            Self::RenderFindingsBatch(_) => research_commands::render_findings_batch(&arguments),
            Self::ValidateCitations(_) => research_commands::validate_citations_command(&arguments),
        }
    }
}

#[derive(Subcommand)]
enum EvalCommand {
    /// Validate research or reviewer output against the frozen contracts.
    #[command(name = "validate-research-output", disable_help_flag = true)]
    ValidateResearchOutput(RawCompatibilityArguments),
    /// Run the live `/research` evaluation harness.
    #[command(name = "research", disable_help_flag = true)]
    Research(RawCompatibilityArguments),
}

impl EvalCommand {
    fn run(self) -> ExitCode {
        let arguments = std::env::args_os().skip(3).collect::<Vec<_>>();
        match self {
            Self::ValidateResearchOutput(_) => {
                eval_commands::validate_research_output_command(&arguments)
            }
            Self::Research(_) => eval_commands::eval_research_command(&arguments),
        }
    }
}

#[derive(Subcommand)]
enum TimingCommand {
    /// Record one step mark in the resolved timing ledger.
    #[command(disable_help_flag = true)]
    Mark(RawCompatibilityArguments),
    /// Record one vendor task in the resolved timing ledger.
    #[command(name = "record-vendor-task", disable_help_flag = true)]
    RecordVendorTask(RawCompatibilityArguments),
    /// Record one review round in the resolved timing ledger.
    #[command(name = "record-round", disable_help_flag = true)]
    RecordRound(RawCompatibilityArguments),
    /// Print the resolved ledger path and its raw rows.
    #[command(disable_help_flag = true)]
    Dump(RawCompatibilityArguments),
    /// Render the timing report for the resolved ledger.
    #[command(disable_help_flag = true)]
    Report(RawCompatibilityArguments),
    /// Run one command and publish its wall-clock duration.
    #[command(name = "harness-mark", disable_help_flag = true)]
    HarnessMark(RawCompatibilityArguments),
    /// Mark one `/implement` step in the token and timing ledgers.
    #[command(name = "telemetry-mark", disable_help_flag = true)]
    TelemetryMark(RawCompatibilityArguments),
    /// Print the canonical `--timing-task-kind` allow-list.
    #[command(name = "task-kinds", disable_help_flag = true)]
    TaskKinds(RawCompatibilityArguments),
}

#[derive(Subcommand)]
enum TokenCommand {
    /// Record one step mark in the resolved token ledger.
    #[command(disable_help_flag = true)]
    Mark(RawCompatibilityArguments),
    /// Check current vendor-token usage against a positive cap.
    #[command(name = "check-budget", disable_help_flag = true)]
    CheckBudget(RawCompatibilityArguments),
    /// Compute PR line totals split between code and committed run logs.
    #[command(name = "compute-pr-line-counts", disable_help_flag = true)]
    ComputePrLineCounts(RawCompatibilityArguments),
    /// Compatibility alias for `compute-pr-line-counts`.
    #[command(name = "compute-pr-lines", disable_help_flag = true)]
    ComputePrLines(RawCompatibilityArguments),
    /// Resolve and print the active Claude transcript source.
    #[command(name = "claude-source", disable_help_flag = true)]
    ClaudeSource(RawCompatibilityArguments),
    /// Record one vendor usage row in the resolved token ledger.
    #[command(name = "record-vendor", disable_help_flag = true)]
    RecordVendor(RawCompatibilityArguments),
    /// Append one active-ledger vendor row from a KEY=value sidecar.
    #[command(name = "record-vendor-sidecar", disable_help_flag = true)]
    RecordVendorSidecar(RawCompatibilityArguments),
    /// Append one staging NDJSON row from a KEY=value sidecar.
    #[command(name = "append-record", disable_help_flag = true)]
    AppendRecord(RawCompatibilityArguments),
    /// Print the resolved ledger path and its raw rows.
    #[command(disable_help_flag = true)]
    Dump(RawCompatibilityArguments),
    /// Render a token report from the resolved ledger and transcript.
    #[command(disable_help_flag = true)]
    Report(RawCompatibilityArguments),
    /// Price token buckets and emit the machine-readable cost block.
    #[command(disable_help_flag = true)]
    Cost(RawCompatibilityArguments),
    /// Price token buckets and emit the one-line cost summary.
    #[command(name = "render-cost-line", disable_help_flag = true)]
    RenderCostLine(RawCompatibilityArguments),
    /// Write one research/validation lane token sidecar.
    #[command(name = "lane-write", disable_help_flag = true)]
    LaneWrite(RawCompatibilityArguments),
    /// Render the research lane token-spend summary.
    #[command(name = "lane-report", disable_help_flag = true)]
    LaneReport(RawCompatibilityArguments),
    /// Measure tracked Markdown prompt cost.
    #[command(name = "measure-md-cost", disable_help_flag = true)]
    MeasureMdCost(RawCompatibilityArguments),
    /// Measure Claude cache creation efficiency.
    #[command(name = "measure-cache-efficiency", disable_help_flag = true)]
    MeasureCacheEfficiency(RawCompatibilityArguments),
    /// Measure checks-digest savings.
    #[command(name = "measure-checks-digest-savings", disable_help_flag = true)]
    MeasureChecksDigestSavings(RawCompatibilityArguments),
    /// Measure repeated prompt shingles.
    #[command(name = "measure-ngram-duplication", disable_help_flag = true)]
    MeasureNgramDuplication(RawCompatibilityArguments),
    /// Measure panel prompt cost.
    #[command(name = "measure-panel-cost", disable_help_flag = true)]
    MeasurePanelCost(RawCompatibilityArguments),
    /// Measure realized skill and reference cost.
    #[command(name = "measure-realized-cost", disable_help_flag = true)]
    MeasureRealizedCost(RawCompatibilityArguments),
    /// Measure reference-read frequency.
    #[command(name = "measure-references-heatmap", disable_help_flag = true)]
    MeasureReferencesHeatmap(RawCompatibilityArguments),
}

#[derive(Subcommand)]
enum GanttCommand {
    /// Render a rows TSV as a plain ASCII Gantt chart.
    #[command(name = "render", disable_help_flag = true)]
    Render(RawCompatibilityArguments),
}

#[derive(Subcommand)]
enum DiagramCommand {
    /// Generate the committed-diff Mermaid code-flow diagram.
    #[command(name = "code-flow", disable_help_flag = true)]
    CodeFlow(RawCompatibilityArguments),
}

#[derive(Subcommand)]
enum RenderCommand {
    /// Render one filtered view of a run's review-findings JSONL.
    #[command(name = "findings-view", disable_help_flag = true)]
    FindingsView(RawCompatibilityArguments),
    /// Render the /research per-lane attribution headers.
    #[command(name = "lane-status", disable_help_flag = true)]
    LaneStatus(RawCompatibilityArguments),
    /// Render the /research validation reviewer prompt.
    #[command(disable_help_flag = true)]
    Reviewer(RawCompatibilityArguments),
    /// Render one code-review specialist prompt.
    #[command(disable_help_flag = true)]
    Specialist(RawCompatibilityArguments),
    /// Render one panel-voter prompt.
    #[command(disable_help_flag = true)]
    Voter(RawCompatibilityArguments),
    /// Render the plan-review scope anchor as untrusted evidence.
    #[command(name = "scope-anchor", disable_help_flag = true)]
    ScopeAnchor(RawCompatibilityArguments),
    /// Render one design plan-review prompt.
    #[command(name = "plan-review", disable_help_flag = true)]
    PlanReview(RawCompatibilityArguments),
    /// Render the terminal `/implement` or `/design` run-summary block.
    #[command(name = "run-summary", disable_help_flag = true)]
    RunSummary(RawCompatibilityArguments),
}

#[derive(Subcommand)]
enum ScopeAnchorCommand {
    /// Select the first valid design-owned handoff candidate.
    #[command(name = "design-handoff", disable_help_flag = true)]
    DesignHandoff(RawCompatibilityArguments),
    /// Report whether the current review result may relay an anchor.
    #[command(name = "relay-allowed", disable_help_flag = true)]
    RelayAllowed(RawCompatibilityArguments),
    /// Select the valid re-tally handoff candidate.
    #[command(name = "retally-handoff", disable_help_flag = true)]
    RetallyHandoff(RawCompatibilityArguments),
    /// Validate one scope-anchor path for its consumer mode.
    #[command(disable_help_flag = true)]
    Validate(RawCompatibilityArguments),
}

#[derive(Subcommand)]
enum MermaidCommand {
    /// Validate Mermaid content without rendering it.
    #[command(disable_help_flag = true)]
    Sanitize(RawCompatibilityArguments),
}

#[derive(Subcommand)]
enum DiagramsCommand {
    /// Preserve or replace sections in the marker-owned diagrams comment.
    #[command(disable_help_flag = true)]
    Upsert(RawCompatibilityArguments),
}

#[derive(Subcommand)]
enum AnalyzeIssuesCommand {
    /// Fetch a bounded issue-backlog snapshot as private JSON.
    #[command(disable_help_flag = true)]
    Fetch(RawCompatibilityArguments),
    /// Analyze a recorded issue-backlog snapshot.
    #[command(disable_help_flag = true)]
    Analyze(RawCompatibilityArguments),
    /// Fetch the backlog and analyze the synchronized run-log corpus.
    #[command(disable_help_flag = true)]
    Run(RawCompatibilityArguments),
    /// Render the cumulative-growth chart from a bucketed TSV.
    #[command(name = "render-chart", disable_help_flag = true)]
    RenderChart(RawCompatibilityArguments),
}

#[derive(Subcommand)]
enum AuditRunsCommand {
    /// Refuse an unsafe audit checkout or unsynchronized run-log corpus.
    #[command(disable_help_flag = true)]
    Preflight(RawCompatibilityArguments),
    /// Resolve a bounded natural-language PR selection.
    #[command(name = "resolve-prs", disable_help_flag = true)]
    ResolvePrs(RawCompatibilityArguments),
    /// Map selected pull requests to run-log directories.
    #[command(name = "map-runs", disable_help_flag = true)]
    MapRuns(RawCompatibilityArguments),
    /// Scan one mapped run for audit evidence.
    #[command(name = "scan-run", disable_help_flag = true)]
    ScanRun(RawCompatibilityArguments),
    /// Aggregate scan NDJSON into report counters.
    #[command(name = "compute-counters", disable_help_flag = true)]
    ComputeCounters(RawCompatibilityArguments),
    /// Print the current `America/Los_Angeles` audit timestamp.
    #[command(name = "pacific-timestamp", disable_help_flag = true)]
    PacificTimestamp(RawCompatibilityArguments),
    /// Render one audit-report title from a PR list.
    #[command(disable_help_flag = true)]
    Title(RawCompatibilityArguments),
    /// Test whether a title belongs to the selected audit-report family.
    #[command(name = "title-match", disable_help_flag = true)]
    TitleMatch(RawCompatibilityArguments),
    /// Print a non-mutating /learn-from-bugs backlog advisory.
    #[command(name = "bugs-backlog-nudge", disable_help_flag = true)]
    BugsBacklogNudge(RawCompatibilityArguments),
    /// Close one unambiguous prior audit report with verified read-back.
    #[command(name = "close-priors", disable_help_flag = true)]
    ClosePriors(RawCompatibilityArguments),
    /// Search issues for proposal classification with local audit-title exclusion.
    #[command(name = "issue-search", disable_help_flag = true)]
    IssueSearch(RawCompatibilityArguments),
    /// Resolve the merged PR that fixed one issue, plus the issue timing.
    #[command(name = "fix-merge", disable_help_flag = true)]
    FixMerge(RawCompatibilityArguments),
    /// Resolve the first plugin-version bump after an instant from local history.
    #[command(name = "version-window", disable_help_flag = true)]
    VersionWindow(RawCompatibilityArguments),
    /// Check one repository label with an exact local match.
    #[command(name = "label-check", disable_help_flag = true)]
    LabelCheck(RawCompatibilityArguments),
    /// Post one supplementary comment through the shared issue-mutation owner.
    #[command(disable_help_flag = true)]
    Comment(RawCompatibilityArguments),
}

#[derive(Subcommand)]
enum AnalyzeBugsCommand {
    /// Fetch bounded, synced-main bug-fix evidence bundles.
    #[command(disable_help_flag = true)]
    Prefetch(RawCompatibilityArguments),
    /// Reconcile or ingest the append-only bug-verification ledger.
    #[command(disable_help_flag = true)]
    Ledger(RawCompatibilityArguments),
    /// Run bounded local verification for selected bug-fix evidence bundles.
    #[command(disable_help_flag = true)]
    Runtime(RawCompatibilityArguments),
    /// Render the report-only bug-verification result and optional follow-up body.
    #[command(disable_help_flag = true)]
    Report(RawCompatibilityArguments),
}

#[derive(Subcommand)]
enum RejectedAnalysisCommand {
    /// Select rejected findings and write read-only verifier prompts.
    #[command(disable_help_flag = true)]
    Prepare(RawCompatibilityArguments),
    /// Validate one launcher artifact and append its durable verdict status.
    #[command(name = "ingest-verdict", disable_help_flag = true)]
    IngestVerdict(RawCompatibilityArguments),
    /// Render bounded issue batches and pending analyzer-state rows.
    #[command(disable_help_flag = true)]
    Finalize(RawCompatibilityArguments),
    /// Commit verified issue dispositions into durable analyzer state.
    #[command(disable_help_flag = true)]
    Record(RawCompatibilityArguments),
}

#[derive(Subcommand)]
enum LearnFromBugsCommand {
    /// Refresh durable proposal status against repository and GitHub evidence.
    #[command(name = "check-proposals", disable_help_flag = true)]
    CheckProposals(RawCompatibilityArguments),
    /// Fetch and compact closed bug reports into bounded artifacts.
    #[command(disable_help_flag = true)]
    Prepare(RawCompatibilityArguments),
    /// Index the checkout's existing enforcement surface.
    #[command(name = "coverage-index", disable_help_flag = true)]
    CoverageIndex(RawCompatibilityArguments),
    /// Read the durable learn-from-bugs marker.
    #[command(name = "read-state", disable_help_flag = true)]
    ReadState(RawCompatibilityArguments),
    /// Atomically update the durable learn-from-bugs marker.
    #[command(name = "write-state", disable_help_flag = true)]
    WriteState(RawCompatibilityArguments),
    /// Render one zone-list GitHub search expression.
    #[command(name = "resolve-zones", disable_help_flag = true)]
    ResolveZones(RawCompatibilityArguments),
    /// Confirm that the analysis checkout's origin matches the publication repository.
    #[command(name = "verify-origin", disable_help_flag = true)]
    VerifyOrigin(RawCompatibilityArguments),
    /// Validate the generated learn-from-bugs report before publication or filing.
    #[command(name = "validate-report", disable_help_flag = true)]
    ValidateReport(RawCompatibilityArguments),
    /// Persist the checked proposal state without mutating a Git worktree.
    #[command(name = "state-publish", disable_help_flag = true)]
    StatePublish(RawCompatibilityArguments),
    /// Translate proposal dependencies into `/issue` batch dependency rows.
    #[command(name = "filing-deps", disable_help_flag = true)]
    FilingDeps(RawCompatibilityArguments),
}

#[derive(Subcommand)]
enum ValidateMergedCommand {
    /// Select recent first-parent merges and write finder/refuter capture paths.
    #[command(disable_help_flag = true)]
    Prepare(RawCompatibilityArguments),
    /// Validate finder JSONL and write the deterministic refuter queue.
    #[command(name = "ingest-finder", disable_help_flag = true)]
    IngestFinder(RawCompatibilityArguments),
    /// Validate refuter JSONL and write the validated merge artifact.
    #[command(name = "ingest-refuter", disable_help_flag = true)]
    IngestRefuter(RawCompatibilityArguments),
    /// Render the report-only merge-validation result and a publishable state file.
    #[command(disable_help_flag = true)]
    Report(RawCompatibilityArguments),
    /// Atomically publish the durable validate-merged marker.
    #[command(name = "write-state", disable_help_flag = true)]
    WriteState(RawCompatibilityArguments),
}

#[derive(Subcommand)]
enum ProgressCommand {
    /// Point the clone's active-run pointer at one run.
    #[command(disable_help_flag = true)]
    Activate(RawCompatibilityArguments),
    /// Remove stale clone-scoped progress runs and legacy flat logs.
    #[command(disable_help_flag = true)]
    Cleanup(RawCompatibilityArguments),
    /// Clear the active-run pointer when the named run still owns it.
    #[command(disable_help_flag = true)]
    Deactivate(RawCompatibilityArguments),
    /// Clear the active-run pointer regardless of its prior owner.
    #[command(disable_help_flag = true)]
    Clear(RawCompatibilityArguments),
    /// Append one breadcrumb to the active run or a named run.
    #[command(disable_help_flag = true)]
    Note(RawCompatibilityArguments),
    /// Render the larch statusline for the payload on stdin.
    #[command(disable_help_flag = true)]
    Statusline(RawCompatibilityArguments),
    /// Clear a stale active-run pointer when a fresh session starts.
    #[command(disable_help_flag = true)]
    SessionReset(RawCompatibilityArguments),
    /// Install the larch statusline into clone-local Claude settings.
    #[command(disable_help_flag = true)]
    InstallStatusline(RawCompatibilityArguments),
    /// Render the review-phase detail section for a completed workflow.
    #[command(name = "render-phase-detail", disable_help_flag = true)]
    RenderPhaseDetail(RawCompatibilityArguments),
    /// Write one `/design` review round's metadata artifact.
    #[command(name = "write-design-round-meta", disable_help_flag = true)]
    WriteDesignRoundMeta(RawCompatibilityArguments),
    /// Write one `/implement` review round's metadata artifact.
    #[command(name = "write-implement-round-meta", disable_help_flag = true)]
    WriteImplementRoundMeta(RawCompatibilityArguments),
}

#[derive(Subcommand)]
enum AdmissionCommand {
    /// Publish the fork metadata a `--forked` run consumes before Step 0.
    #[command(disable_help_flag = true)]
    ForkEnv(RawCompatibilityArguments),
    /// Decide whether one issue may enter a `/implement` run.
    #[command(disable_help_flag = true)]
    Gate(RawCompatibilityArguments),
    /// Enforce the clean-main entry contract and sync with `origin/main`.
    #[command(disable_help_flag = true)]
    Preflight(RawCompatibilityArguments),
}

#[derive(Subcommand)]
enum ImplementCommand {
    /// Remove one validated implementation session directory.
    #[command(disable_help_flag = true)]
    Cleanup(RawCompatibilityArguments),
    /// Generate the committed-diff Mermaid code-flow diagram.
    #[command(name = "code-flow-diagram", disable_help_flag = true)]
    CodeFlowDiagram(RawCompatibilityArguments),
    /// Compute, classify, or validate the checks bgjob input identity.
    #[command(name = "checks-result-identity", disable_help_flag = true)]
    ChecksResultIdentity(RawCompatibilityArguments),
    /// Run the per-site relevant checks, commit, and rebase-checkpoint composite.
    #[command(name = "checks-commit-route", disable_help_flag = true)]
    ChecksCommitRoute(RawCompatibilityArguments),
    /// Create the Step 4 implementation commit and emit its result envelope.
    #[command(name = "commit", disable_help_flag = true)]
    Commit(RawCompatibilityArguments),
    /// Route a review-fix commit at Steps 5/7, seeding a stall on failure.
    #[command(name = "commit-route", disable_help_flag = true)]
    CommitRoute(RawCompatibilityArguments),
    /// Run the Step 5 checks front-half then forward the resume leg.
    #[command(name = "checks-step5-resume", disable_help_flag = true)]
    ChecksStep5Resume(RawCompatibilityArguments),
    /// Emit this clone's tag and its expected implement tmpdir prefix.
    #[command(name = "clone-tag", disable_help_flag = true)]
    CloneTag(RawCompatibilityArguments),
    /// Terminate a stranded active-leg process group and clear its record.
    #[command(name = "kill-active-leg", disable_help_flag = true)]
    KillActiveLeg(RawCompatibilityArguments),
    /// Normalize a coder-produced dynamic-archetype manifest for Step 5.
    #[command(name = "normalize-coder-scout", disable_help_flag = true)]
    NormalizeCoderScout(RawCompatibilityArguments),
    /// Serialize and launch one Step 2 dispatch, then publish its envelope.
    #[command(name = "run-dispatch", disable_help_flag = true)]
    RunDispatch(RawCompatibilityArguments),
    /// Verify admission, extract the plan, and probe main's CI health.
    #[command(disable_help_flag = true)]
    Preflight(RawCompatibilityArguments),
    /// Compute NUL-delimited recovery paths against the prelaunch baseline.
    #[command(name = "recovery-paths", disable_help_flag = true)]
    RecoveryPaths(RawCompatibilityArguments),
    /// Launch or rejoin the per-site checks bgjob composite.
    #[command(name = "run-step-checks", disable_help_flag = true)]
    RunStepChecks(RawCompatibilityArguments),
    /// Compute, record, or validate the plan-coverage scope disposition.
    #[command(name = "scope-disposition", disable_help_flag = true)]
    ScopeDisposition(RawCompatibilityArguments),
    /// Probe, report, and route the Step 2 post-dispatch working-tree state.
    #[command(name = "step-2-post-dispatch", disable_help_flag = true)]
    Step2PostDispatch(RawCompatibilityArguments),
    /// Run the Step 2 external-implementer dispatch and emit its contract.
    #[command(name = "step2-dispatch", disable_help_flag = true)]
    Step2Dispatch(RawCompatibilityArguments),
    /// Validate Step 0 flags, rehydrate a resume, and adopt the run lifecycle.
    #[command(name = "step-0-bootstrap", disable_help_flag = true)]
    Step0Bootstrap(RawCompatibilityArguments),
    /// Probe the vendor reviewers and forward the degraded-tools gate.
    #[command(name = "step-0-degraded-gate", disable_help_flag = true)]
    Step0DegradedGate(RawCompatibilityArguments),
    /// Launch or rejoin the Step 5 resume bgjob leg.
    #[command(name = "step-5-resume", disable_help_flag = true)]
    Step5Resume(RawCompatibilityArguments),
    /// Launch or rejoin the Step 5 review bgjob leg.
    #[command(name = "step-5-review", disable_help_flag = true)]
    Step5Review(RawCompatibilityArguments),
    /// Launch or rejoin the Step 6 review-change-detection bgjob leg.
    #[command(name = "step-6-entry", disable_help_flag = true)]
    Step6Entry(RawCompatibilityArguments),
    /// Launch or run the Step 7a pre-ship checkpoint and code-flow diagram.
    #[command(name = "step-7a", disable_help_flag = true)]
    Step7a(RawCompatibilityArguments),
    /// Reconstruct and create the initial durable ship state.
    #[command(name = "step-8-seed-initial", disable_help_flag = true)]
    Step8SeedInitial(RawCompatibilityArguments),
    /// Launch or run the Step 8 ship bgjob.
    #[command(name = "step-8-ship", disable_help_flag = true)]
    Step8Ship(RawCompatibilityArguments),
    /// Route the OOS checkpoint result and persist successful bookkeeping.
    #[command(name = "step-8-oos-checkpoint", disable_help_flag = true)]
    Step8OosCheckpoint(RawCompatibilityArguments),
    /// Replay rejected findings for the Step 16 closeout checkpoint.
    #[command(name = "step-16", disable_help_flag = true)]
    Step16(RawCompatibilityArguments),
    /// Run Step 16 and its best-effort Step 16a Slack notification.
    #[command(name = "step-16-16a", disable_help_flag = true)]
    Step16_16a(RawCompatibilityArguments),
    /// Run the Step 16 through Step 17 closeout composite.
    #[command(name = "step-16-17", disable_help_flag = true)]
    Step16_17(RawCompatibilityArguments),
    /// Render the Step 17 final report.
    #[command(name = "step-17", disable_help_flag = true)]
    Step17(RawCompatibilityArguments),
    /// Run one Step 18 phase: the stall gate or the terminal logs flush.
    #[command(name = "step-18", disable_help_flag = true)]
    Step18(RawCompatibilityArguments),
    /// Run the Step 18 stall gate and terminal logs flush as one composite.
    #[command(name = "step-18-gate-logs-flush", disable_help_flag = true)]
    Step18GateLogsFlush(RawCompatibilityArguments),
    /// Tear the session down after Step 18 recorded run-log terminalization.
    #[command(name = "step-19", disable_help_flag = true)]
    Step19(RawCompatibilityArguments),
}

#[derive(Subcommand)]
enum ImplementFinalizeCommand {
    /// Rebase and lease-force-push the feature branch before PR creation.
    #[command(disable_help_flag = true)]
    Postbump(RawCompatibilityArguments),
    /// Synchronize main and delete the merged local feature branch.
    #[command(disable_help_flag = true)]
    Postmerge(RawCompatibilityArguments),
    /// Rename terminal issue state and clean or preserve session artifacts.
    #[command(disable_help_flag = true)]
    Teardown(RawCompatibilityArguments),
}

#[derive(Subcommand)]
enum ShipCommand {
    /// Run the Rust ship PR lifecycle owner.
    #[command(name = "pr", disable_help_flag = true)]
    Pr(RawCompatibilityArguments),
    /// Reconcile a manually merged PR into all durable ship layers.
    #[command(name = "reconcile-manual-merge", disable_help_flag = true)]
    ReconcileManualMerge(RawCompatibilityArguments),
    /// Normalize one architectural-assessment request handoff.
    #[command(name = "normalize-assessment-handoff", disable_help_flag = true)]
    NormalizeAssessmentHandoff(RawCompatibilityArguments),
    /// Validate scope and seed state before launching the ship driver.
    #[command(name = "pre-driver", disable_help_flag = true)]
    PreDriver(RawCompatibilityArguments),
    /// Repair the feature branch before a resumed ship attempt.
    #[command(name = "pre-fix-rebase", disable_help_flag = true)]
    PreFixRebase(RawCompatibilityArguments),
    /// Route one completed ship-driver result to the next action.
    #[command(name = "route-exit", disable_help_flag = true)]
    RouteExit(RawCompatibilityArguments),
    /// Refresh the plan receipt after a ship-gate stale-plan-base-scope.
    #[command(name = "governance-refresh", disable_help_flag = true)]
    GovernanceRefresh(RawCompatibilityArguments),
    /// Create the canonical first durable ship state.
    #[command(name = "seed-initial-state", disable_help_flag = true)]
    SeedInitialState(RawCompatibilityArguments),
    /// Validate or publish the ship result env from JSON on stdin.
    #[command(name = "write-result-env", disable_help_flag = true)]
    WriteResultEnv(RawCompatibilityArguments),
}

#[derive(Subcommand)]
enum MergeCommand {
    /// Classify and submit one pull-request merge.
    #[command(disable_help_flag = true)]
    Pr(RawCompatibilityArguments),
    /// Wait for one accepted merge-queue entry to merge.
    #[command(disable_help_flag = true)]
    Wait(RawCompatibilityArguments),
}

#[derive(Subcommand)]
enum ChecksCommand {
    /// Run the bounded changed-path Cargo Clippy selection.
    #[command(name = "rust-clippy", disable_help_flag = true)]
    RustClippy(RawCompatibilityArguments),
    /// Report the self-edit attribution records for one implement session.
    #[command(name = "self-edit-log", disable_help_flag = true)]
    SelfEditLog(RawCompatibilityArguments),
    /// Run the relevant checks for one changed-file selection and capture a log.
    #[command(name = "run-relevant", disable_help_flag = true)]
    RunRelevant(RawCompatibilityArguments),
    /// Assert that pinned `contains` literals are present in their targets.
    #[command(name = "contains-pins", disable_help_flag = true)]
    ContainsPins(RawCompatibilityArguments),
    /// Materialize a bounded, redacted checks-failure digest for the ci-fixer.
    #[command(name = "fixer-evidence", disable_help_flag = true)]
    FixerEvidence(RawCompatibilityArguments),
    /// Dispatch the delegated coder waterfall to fix relevant-checks failures.
    #[command(name = "lint-fix", disable_help_flag = true)]
    LintFix(RawCompatibilityArguments),
    /// Run the bounded lint-fix and relevant-checks repair orchestrator.
    #[command(name = "repair-loop", disable_help_flag = true)]
    RepairLoop(RawCompatibilityArguments),
}

#[derive(Subcommand)]
enum DecomposeCommand {
    /// Build the hash-compatible prepared-partition artifacts.
    #[command(disable_help_flag = true)]
    Prepare(RawCompatibilityArguments),
    /// Record the filed-issue mapping from an `/issue` batch capture.
    #[command(disable_help_flag = true)]
    Annotate(RawCompatibilityArguments),
    /// Migrate the original issue's dependency graph onto its pieces.
    #[command(name = "migrate-deps", disable_help_flag = true)]
    MigrateDeps(RawCompatibilityArguments),
    /// Comment on and close the partitioned original issue.
    #[command(name = "close-original", disable_help_flag = true)]
    CloseOriginal(RawCompatibilityArguments),
    /// Dispatch the four-archetype decomposition proposal panel.
    #[command(name = "panel-dispatch", disable_help_flag = true)]
    PanelDispatch(RawCompatibilityArguments),
    /// Merge the panel proposals into one canonical partition.
    #[command(disable_help_flag = true)]
    Aggregate(RawCompatibilityArguments),
}

#[derive(Subcommand)]
enum BlockerCommand {
    /// Emit the space-joined open blockers for one issue.
    #[command(disable_help_flag = true)]
    AllOpen(RawCompatibilityArguments),
}

#[derive(Subcommand)]
enum BlockIssueCommand {
    /// Record `ISSUE_A` as blocked by `ISSUE_B` and verify the relation.
    #[command(name = "add-blocked-by", disable_help_flag = true)]
    AddBlockedBy(RawCompatibilityArguments),
    /// Drop the `ISSUE_A` blocked-by `ISSUE_B` relation and verify its absence.
    #[command(name = "remove-blocked-by", disable_help_flag = true)]
    RemoveBlockedBy(RawCompatibilityArguments),
}

#[derive(Subcommand)]
enum IssueCommand {
    /// Record one issue as blocked by another and prove the edge by read-back.
    #[command(name = "add-blocked-by", disable_help_flag = true)]
    AddBlockedBy(RawCompatibilityArguments),
    /// Attach one direct native sub-issue and prove it by read-back.
    #[command(name = "add-sub-issue", disable_help_flag = true)]
    AddSubIssue(RawCompatibilityArguments),
    /// Allocate the bounded Phase 2 dedup candidate set from stdin rows.
    #[command(name = "allocate-candidates", disable_help_flag = true)]
    AllocateCandidates(RawCompatibilityArguments),
    /// Close one orphaned issue left by a partially created batch.
    #[command(name = "cleanup-failed", disable_help_flag = true)]
    CleanupFailed(RawCompatibilityArguments),
    /// File and wire one validated issue batch through a single Rust owner.
    #[command(name = "create-batch", disable_help_flag = true)]
    CreateBatch(RawCompatibilityArguments),
    /// Materialize one issue's title and body into a caller-named directory.
    #[command(disable_help_flag = true)]
    Context(RawCompatibilityArguments),
    /// File one GitHub issue and publish its number, URL, and node id.
    #[command(name = "create-one", disable_help_flag = true)]
    CreateOne(RawCompatibilityArguments),
    /// Write the untrusted candidate corpus Phase 2 reasons over.
    #[command(name = "fetch-issue-details", disable_help_flag = true)]
    FetchIssueDetails(RawCompatibilityArguments),
    /// Emit one issue field as the single `VALUE` row.
    #[command(disable_help_flag = true)]
    Info(RawCompatibilityArguments),
    /// Read one immutable migration-governance audit snapshot.
    #[command(name = "migration-audit", disable_help_flag = true)]
    MigrationAudit(RawCompatibilityArguments),
    /// Evaluate blocker, receipt, and owner admission policy.
    #[command(name = "governance-gate", disable_help_flag = true)]
    GovernanceGate(RawCompatibilityArguments),
    /// Insert one bracketed signal marker into an issue title.
    #[command(name = "insert-signal-marker", disable_help_flag = true)]
    InsertSignalMarker(RawCompatibilityArguments),
    /// Publish the open and recently closed issue snapshot as a TSV.
    #[command(name = "list-issues", disable_help_flag = true)]
    ListIssues(RawCompatibilityArguments),
    /// Parse one batch-input file into per-item rows and body files.
    #[command(name = "parse-input", disable_help_flag = true)]
    ParseInput(RawCompatibilityArguments),
    /// Find one open design or implementation issue that names a path.
    #[command(name = "search-implementing", disable_help_flag = true)]
    SearchImplementing(RawCompatibilityArguments),
    /// Emit one issue's state, URL, and pull-request discrimination.
    #[command(disable_help_flag = true)]
    State(RawCompatibilityArguments),
    /// Print the `jq` archival-eligibility filter.
    #[command(name = "title-archival-jq", disable_help_flag = true)]
    TitleArchivalJq(RawCompatibilityArguments),
    /// Report the archival-eligibility predicates for one issue title.
    #[command(name = "title-eligibility", disable_help_flag = true)]
    TitleEligibility(RawCompatibilityArguments),
    /// Record that one `/issue` run reached its end.
    #[command(name = "write-sentinel", disable_help_flag = true)]
    WriteSentinel(RawCompatibilityArguments),
}

#[derive(Subcommand)]
enum PlanBlockCommand {
    /// Materialize one issue's `larch:plan` inner text into a file.
    #[command(disable_help_flag = true)]
    Read(RawCompatibilityArguments),
    /// Remove the `larch:plan` block from a body file or stdin.
    #[command(name = "strip-body", disable_help_flag = true)]
    StripBody(RawCompatibilityArguments),
    /// Write, replace, or delete one issue's `larch:plan` block.
    #[command(disable_help_flag = true)]
    Write(RawCompatibilityArguments),
}

#[derive(Subcommand)]
enum PlanReceiptCommand {
    /// Refresh one preflight-bound receipt after semantic scope review.
    #[command(disable_help_flag = true)]
    Refresh(RawCompatibilityArguments),
}

#[derive(Subcommand)]
enum NamedBlockCommand {
    /// Write, replace, or delete one named issue-body block.
    #[command(disable_help_flag = true)]
    Write(RawCompatibilityArguments),
}

#[derive(Subcommand)]
enum DebateCommand {
    /// Initialize a debate: seat the panel and persist the first state.
    #[command(disable_help_flag = true)]
    Init(RawCompatibilityArguments),
    /// Prepare one negotiation round: seating, mailboxes, and turn prompts.
    #[command(name = "round-prep", disable_help_flag = true)]
    RoundPrep(RawCompatibilityArguments),
    /// Record one live slot's turn: run the vendor and bind the ledger.
    #[command(name = "record-turn", disable_help_flag = true)]
    RecordTurn(RawCompatibilityArguments),
    /// Prepare and record every live external slot's turn in one composite verb.
    #[command(name = "round-external", disable_help_flag = true)]
    RoundExternal(RawCompatibilityArguments),
    /// Record the claude turn, then compose, upsert, and verify the round digest.
    #[command(name = "round-ingest", disable_help_flag = true)]
    RoundIngest(RawCompatibilityArguments),
    /// Abort a debate and write the tracking-issue restore handoff.
    #[command(disable_help_flag = true)]
    Abort(RawCompatibilityArguments),
    /// Write the redacted operator adjudication preview.
    #[command(name = "adjudication-preview", disable_help_flag = true)]
    AdjudicationPreview(RawCompatibilityArguments),
    /// Adjudicate unresolved points via operator decisions or voter tally.
    #[command(name = "adjudicate", disable_help_flag = true)]
    Adjudicate(RawCompatibilityArguments),
    /// Synthesize a converged debate into one redacted proposal.
    #[command(disable_help_flag = true)]
    Synthesize(RawCompatibilityArguments),
    /// Write the idempotent local proposal-publication handoff.
    #[command(name = "publish-prepare", disable_help_flag = true)]
    PublishPrepare(RawCompatibilityArguments),
    /// Snapshot one open source issue and write the debate subject and metadata.
    #[command(name = "issue-prepare", disable_help_flag = true)]
    IssuePrepare(RawCompatibilityArguments),
    /// Apply a freshness-checked lifecycle title compare-and-swap.
    #[command(name = "title-transition", disable_help_flag = true)]
    TitleTransition(RawCompatibilityArguments),
    /// Append the canonical source backlink to a synthesized proposal body.
    #[command(name = "proposal-link", disable_help_flag = true)]
    ProposalLink(RawCompatibilityArguments),
    /// Verify one source comment's exact redacted postcondition by read-back.
    #[command(name = "comment-verify", disable_help_flag = true)]
    CommentVerify(RawCompatibilityArguments),
    /// Synthesize, prepare, and link the proposal publication in one composite verb.
    #[command(name = "publish-run", disable_help_flag = true)]
    PublishRun(RawCompatibilityArguments),
    /// Upsert and verify the forward-link comment, then finish the source title.
    #[command(name = "publish-finish", disable_help_flag = true)]
    PublishFinish(RawCompatibilityArguments),
    /// Abort, restore the owned title, and upsert the fixed aborted comment.
    #[command(name = "abort-run", disable_help_flag = true)]
    AbortRun(RawCompatibilityArguments),
    /// Initialize the debate, then adopt the run-owned source title in one composite verb.
    #[command(name = "init-run", disable_help_flag = true)]
    InitRun(RawCompatibilityArguments),
}

#[derive(Subcommand)]
enum ClarifyCommand {
    /// Evaluate the clarify thread state for one issue.
    #[command(disable_help_flag = true)]
    State(RawCompatibilityArguments),
    /// Fetch one clarify request body to a file.
    #[command(name = "comment-fetch", disable_help_flag = true)]
    CommentFetch(RawCompatibilityArguments),
    /// Post one clarify request or response comment.
    #[command(name = "comment-post", disable_help_flag = true)]
    CommentPost(RawCompatibilityArguments),
    /// Add or remove the clarification label on one issue.
    #[command(disable_help_flag = true)]
    Label(RawCompatibilityArguments),
}

impl ClarifyCommand {
    fn run(self) -> ExitCode {
        // Raw argv: the verbs own their `--flag value` parsing and error text.
        let arguments = std::env::args_os().skip(3).collect::<Vec<_>>();
        match self {
            Self::State(_) => clarify_commands::clarify_state_main(&arguments),
            Self::CommentFetch(_) => clarify_commands::clarify_comment_fetch_main(&arguments),
            Self::CommentPost(_) => clarify_commands::clarify_comment_post_main(&arguments),
            Self::Label(_) => clarify_commands::clarify_label_main(&arguments),
        }
    }
}

#[derive(Subcommand)]
enum DesignCommand {
    /// Run the pause-only prelude for generated design fences (#8593).
    #[command(disable_help_flag = true)]
    Prelude(RawCompatibilityArguments),
    /// Drive the Step 0b clarification fetch or publish phase.
    #[command(disable_help_flag = true)]
    Clarify(RawCompatibilityArguments),
    /// Validate the public `/design` argv and bind its flags.
    #[command(name = "parse-flags", disable_help_flag = true)]
    ParseFlags(RawCompatibilityArguments),
    /// Decide the Step 0b route for one issue.
    #[command(disable_help_flag = true)]
    Route(RawCompatibilityArguments),
    /// Refresh the session env, apply the `[DESIGNING]` rename, and write run-params.
    #[command(name = "init-runparams", disable_help_flag = true)]
    InitRunparams(RawCompatibilityArguments),
    /// Parse the public `/design` Step 0 argv and cache the parsed env (#8578).
    #[command(name = "step0-parse", disable_help_flag = true)]
    Step0Parse(RawCompatibilityArguments),
    /// Set up the `/design` Step 0 session (#8578).
    #[command(name = "step0-session", disable_help_flag = true)]
    Step0Session(RawCompatibilityArguments),
    /// Decide and drive the Step 0b route for one issue (#8578).
    #[command(name = "step0-route", disable_help_flag = true)]
    Step0Route(RawCompatibilityArguments),
    /// Emit the clarify hard-halt terminal state (#8578).
    #[command(name = "step0-clarify-hard-halt", disable_help_flag = true)]
    Step0ClarifyHardHalt(RawCompatibilityArguments),
    /// Fold the init-runparams driver into Step 0 (#8578).
    #[command(name = "step0-init", disable_help_flag = true)]
    Step0Init(RawCompatibilityArguments),
    /// Clean up after a `/design` Step 0 abort (#8578).
    #[command(name = "step0-abort-cleanup", disable_help_flag = true)]
    Step0AbortCleanup(RawCompatibilityArguments),
    /// Continue the auto-proceed Step 0 path (#8578).
    #[command(name = "step0-ap-continue", disable_help_flag = true)]
    Step0ApContinue(RawCompatibilityArguments),
    /// Write the Step 0c folded sentinel (#8578).
    #[command(name = "step0c", disable_help_flag = true)]
    Step0c(RawCompatibilityArguments),
    /// Decide the settle next action from site and postplan rc (#8578).
    #[command(name = "settle-next-action", disable_help_flag = true)]
    SettleNextAction(RawCompatibilityArguments),
    /// Drive the Step 1 plan-review/plan action loop (#8579).
    #[command(disable_help_flag = true)]
    Driver(RawCompatibilityArguments),
    /// Decide the Step 1d.5 brainstorm entry/collect/complete modes (#8579).
    #[command(name = "step1d5", disable_help_flag = true)]
    Step1d5(RawCompatibilityArguments),
    /// Write the Step 1d.7 drafter-prerequisite sentinels (#8579).
    #[command(name = "step1d7", disable_help_flag = true)]
    Step1d7(RawCompatibilityArguments),
    /// Unlink the Step 1e..4b reentry sentinels (#8579).
    #[command(name = "step1e-reentry", disable_help_flag = true)]
    Step1eReentry(RawCompatibilityArguments),
    /// Read an allowlisted phase-driver result env into a quoted output (#8580).
    #[command(name = "read-result-env", disable_help_flag = true)]
    ReadResultEnv(RawCompatibilityArguments),
    /// Stage the design terminal-failure state env (#8580).
    #[command(name = "stage-terminal-state", disable_help_flag = true)]
    StageTerminalState(RawCompatibilityArguments),
    /// Compose and file (or fall back on) a design failure report (#8580).
    #[command(name = "failure-report", disable_help_flag = true)]
    FailureReport(RawCompatibilityArguments),
    /// Emit, publish, or render the design run's final summary (#8580).
    #[command(name = "step-final-summary", disable_help_flag = true)]
    StepFinalSummary(RawCompatibilityArguments),
    /// Publish sanitized design run logs through the shared lifecycle (#8592).
    #[command(name = "log-publish", disable_help_flag = true)]
    LogPublish(RawCompatibilityArguments),
    /// Drive the Step 5c publish phase machine (#8591).
    #[command(disable_help_flag = true)]
    Publish(RawCompatibilityArguments),
    /// Recompose the Step 5c issue-body plan from canonical artifacts (#8586).
    #[command(name = "compose-plan-md", disable_help_flag = true)]
    ComposePlanMd(RawCompatibilityArguments),
    /// Run the Step 2b.5 plan-size routing check (#8586).
    #[command(name = "step2b5", disable_help_flag = true)]
    Step2b5(RawCompatibilityArguments),
    /// Publish the final design and emit its Step 5c result envelope (#8586).
    #[command(name = "step5c", disable_help_flag = true)]
    Step5c(RawCompatibilityArguments),
    /// Run the complete Step 6 prelude and cleanup sequence (#8586).
    #[command(name = "step6", disable_help_flag = true)]
    Step6(RawCompatibilityArguments),
    /// Mark Step 5d when the Step 5c result permits cleanup (#8586).
    #[command(name = "step6-prelude", disable_help_flag = true)]
    Step6Prelude(RawCompatibilityArguments),
    /// Preserve or clean the completed design session (#8586).
    #[command(name = "step6-cleanup", disable_help_flag = true)]
    Step6Cleanup(RawCompatibilityArguments),
    /// Compose the Step 2b plan drafter and delegate postplan (#8583).
    #[command(name = "step2b-drafter", disable_help_flag = true)]
    Step2bDrafter(RawCompatibilityArguments),
    /// Run the Step 2b postplan decision and completion contract (#8583).
    #[command(name = "step2b-postplan", disable_help_flag = true)]
    Step2bPostplan(RawCompatibilityArguments),
    /// Emit the post-plan validation and plan-size result env (#8583).
    #[command(name = "postplan-emit", disable_help_flag = true)]
    PostplanEmit(RawCompatibilityArguments),
    /// Run the Step 3b finalize or Step 5b.5 diagram entry (#8583).
    #[command(name = "step3b-entry", disable_help_flag = true)]
    Step3bEntry(RawCompatibilityArguments),
    /// Run the Step 4 rejected-findings and Gate C preview tail (#8931).
    #[command(name = "step4-tail", disable_help_flag = true)]
    Step4Tail(RawCompatibilityArguments),
    /// Write drafter-declared dialectic candidates (#8584).
    #[command(name = "dialectic-write-candidates", disable_help_flag = true)]
    DialecticWriteCandidates(RawCompatibilityArguments),
    /// Promote the pending dialectic sidecar against the final plan (#8584).
    #[command(name = "dialectic-promote-candidates", disable_help_flag = true)]
    DialecticPromoteCandidates(RawCompatibilityArguments),
    /// Validate and normalize dialectic candidate JSON (#8584).
    #[command(name = "dialectic-validate-candidates", disable_help_flag = true)]
    DialecticValidateCandidates(RawCompatibilityArguments),
    /// Clear stale dialectic candidate artifacts (#8584).
    #[command(name = "dialectic-clear-stale", disable_help_flag = true)]
    DialecticClearStale(RawCompatibilityArguments),
    /// Run the bounded Gate C dialectic clarifier (#8593).
    #[command(name = "dialectic-gatec", disable_help_flag = true)]
    DialecticGatec(RawCompatibilityArguments),
    /// Run one operator-requested Gate C dialectic debate (#8593).
    #[command(name = "dialectic-manual", disable_help_flag = true)]
    DialecticManual(RawCompatibilityArguments),
    /// Settle the Gate A/B/C or discussion-round2 post-plan state (#8585).
    #[command(name = "step35-settle", disable_help_flag = true)]
    Step35Settle(RawCompatibilityArguments),
    /// Prepare Step 5b out-of-scope filing (#8585).
    #[command(name = "step5b-prepare", disable_help_flag = true)]
    Step5bPrepare(RawCompatibilityArguments),
    /// Annotate Step 5b out-of-scope filing results (#8585).
    #[command(name = "step5b-annotate", disable_help_flag = true)]
    Step5bAnnotate(RawCompatibilityArguments),
    /// Prepare accepted design OOS items for `/issue` filing (#8590).
    #[command(name = "file-oos-prepare", disable_help_flag = true)]
    FileOosPrepare(RawCompatibilityArguments),
    /// Run Gate B completion and emit `APPROVE_REQUESTED=` (#8931).
    #[command(name = "gate-b", disable_help_flag = true)]
    GateB(RawCompatibilityArguments),
    /// Annotate accepted design OOS items after `/issue` filing (#8590).
    #[command(name = "file-oos-annotate", disable_help_flag = true)]
    FileOosAnnotate(RawCompatibilityArguments),
    /// Restore a verified cross-session `/design` pause snapshot (#8589).
    #[command(name = "pause-load", disable_help_flag = true)]
    PauseLoad(RawCompatibilityArguments),
    /// Publish one cross-session `/design` pause snapshot (#8589).
    #[command(name = "pause-save", disable_help_flag = true)]
    PauseSave(RawCompatibilityArguments),
    /// Render the /design Gate A/B/C prompt copy as KEY=value rows (#8581).
    #[command(name = "render-gate", disable_help_flag = true)]
    RenderGate(RawCompatibilityArguments),
    /// Render the /design run's enriched final summary (#8581).
    #[command(name = "render-final-summary", disable_help_flag = true)]
    RenderFinalSummary(RawCompatibilityArguments),
    /// Enter an automatic Step 3 continuation round (#8593).
    #[command(name = "step3-continuation-entry", disable_help_flag = true)]
    Step3ContinuationEntry(RawCompatibilityArguments),
    /// Combined `/design` Step 3 entry: state, scope-anchor, preview (#8931).
    #[command(name = "step3-entry", disable_help_flag = true)]
    Step3Entry(RawCompatibilityArguments),
}

impl DesignCommand {
    fn run(self) -> ExitCode {
        // Raw argv, not the clap-parsed compatibility vector: the Python
        // grammar treats `--` as a meaningful token, and clap would eat it.
        let arguments = std::env::args_os().skip(3).collect::<Vec<_>>();
        match self {
            Self::Prelude(_) => design_step0_commands::prelude(&arguments),
            Self::Clarify(_) => clarify_orchestrator::design_clarify_main(&arguments),
            Self::ParseFlags(_) => design_commands::parse_flags(&arguments),
            Self::Route(_) => design_commands::route(&arguments),
            Self::InitRunparams(_) => design_commands::init_runparams(&arguments),
            Self::Step0Parse(_) => design_step0_commands::step0_parse(&arguments),
            Self::Step0Session(_) => design_step0_commands::step0_session(&arguments),
            Self::Step0Route(_) => design_step0_commands::step0_route(&arguments),
            Self::Step0ClarifyHardHalt(_) => {
                design_step0_commands::step0_clarify_hard_halt(&arguments)
            }
            Self::Step0Init(_) => design_step0_commands::step0_init(&arguments),
            Self::Step0AbortCleanup(_) => design_step0_commands::step0_abort_cleanup(&arguments),
            Self::Step0ApContinue(_) => design_step0_commands::step0_ap_continue(&arguments),
            Self::Step0c(_) => design_step0_commands::step0c(&arguments),
            Self::SettleNextAction(_) => design_step0_commands::settle_next_action(&arguments),
            Self::Driver(_) => design_step1_commands::driver(&arguments),
            Self::Step1d5(_) => design_step1_commands::step1d5(&arguments),
            Self::Step1d7(_) => design_step1_commands::step1d7(&arguments),
            Self::Step1eReentry(_) => design_step1_commands::step1e_reentry(&arguments),
            Self::ReadResultEnv(_) => design_terminal_commands::read_result_env(&arguments),
            Self::StageTerminalState(_) => {
                design_terminal_commands::stage_terminal_state(&arguments)
            }
            Self::FailureReport(_) => design_terminal_commands::failure_report(&arguments),
            Self::StepFinalSummary(_) => design_terminal_commands::step_final_summary(&arguments),
            Self::LogPublish(_) => design_log_publish_commands::log_publish_main(&arguments),
            Self::Publish(_) => design_publish_commands::design_publish_main(&arguments),
            Self::ComposePlanMd(_) => design_finalize_commands::compose_plan_md(&arguments),
            Self::Step2b5(_) => design_step2b_commands::step2b5(&arguments),
            Self::Step5c(_) => design_finalize_commands::step5c(&arguments),
            Self::Step6(_) => design_finalize_commands::step6(&arguments),
            Self::Step6Prelude(_) => design_finalize_commands::step6_prelude(&arguments),
            Self::Step6Cleanup(_) => design_finalize_commands::step6_cleanup(&arguments),
            Self::Step2bDrafter(_) => design_step2b_commands::step2b_drafter(&arguments),
            Self::Step2bPostplan(_) => design_step2b_commands::step2b_postplan(&arguments),
            Self::PostplanEmit(_) => design_step2b_commands::postplan_emit(&arguments),
            Self::Step3bEntry(_) => design_step2b_commands::step3b_entry(&arguments),
            Self::Step4Tail(_) => design_step3_commands::step4_tail(&arguments),
            Self::DialecticWriteCandidates(_) => {
                design_dialectic_commands::write_candidates(&arguments)
            }
            Self::DialecticPromoteCandidates(_) => {
                design_dialectic_commands::promote_candidates(&arguments)
            }
            Self::DialecticValidateCandidates(_) => {
                design_dialectic_commands::validate_candidates(&arguments)
            }
            Self::DialecticClearStale(_) => {
                design_dialectic_commands::clear_stale_candidates(&arguments)
            }
            Self::DialecticGatec(_) => design_dialectic_commands::gatec(&arguments),
            Self::DialecticManual(_) => design_dialectic_commands::manual(&arguments),
            Self::Step35Settle(_) => design_settle_commands::step35_settle(&arguments),
            Self::Step5bPrepare(_) => design_settle_commands::step5b_prepare(&arguments),
            Self::Step5bAnnotate(_) => design_settle_commands::step5b_annotate(&arguments),
            Self::FileOosPrepare(_) => design_oos_commands::file_oos_prepare_main(&arguments),
            Self::GateB(_) => design_step3_commands::gate_b(&arguments),
            Self::FileOosAnnotate(_) => design_oos_commands::file_oos_annotate_main(&arguments),
            Self::PauseLoad(_) => design_pause_commands::pause_load_main(&arguments),
            Self::PauseSave(_) => design_pause_commands::pause_save_main(&arguments),
            Self::RenderGate(_) => design_gate_summary_commands::render_gate(&arguments),
            Self::RenderFinalSummary(_) => {
                design_gate_summary_commands::render_final_summary(&arguments)
            }
            Self::Step3ContinuationEntry(_) => {
                design_step0_commands::step3_continuation_entry(&arguments)
            }
            Self::Step3Entry(_) => design_step3_commands::step3_entry(&arguments),
        }
    }
}

#[derive(Subcommand)]
enum PlanCommand {
    /// Publish the scope paths one implementation plan declares.
    #[command(name = "scope-paths", disable_help_flag = true)]
    ScopePaths(RawCompatibilityArguments),
    /// Parse fenced plan commands into a TSV table.
    #[command(name = "parse-commands", disable_help_flag = true)]
    ParseCommands(RawCompatibilityArguments),
    /// Validate a plan-command TSV table.
    #[command(name = "validate-commands", disable_help_flag = true)]
    ValidateCommands(RawCompatibilityArguments),
    /// Validate one plan document end-to-end.
    #[command(disable_help_flag = true)]
    Validate(RawCompatibilityArguments),
    /// Measure plan size and drift against the design baseline.
    #[command(name = "check-size", disable_help_flag = true)]
    CheckSize(RawCompatibilityArguments),
    /// Write or remove the trusted oversize override trailer.
    #[command(name = "set-oversize-override", disable_help_flag = true)]
    SetOversizeOverride(RawCompatibilityArguments),
    /// Revise plan.txt through the vendor waterfall.
    #[command(name = "revise-waterfall", disable_help_flag = true)]
    ReviseWaterfall(RawCompatibilityArguments),
    /// Auto-fix plan-command validation defects.
    #[command(name = "auto-fix-commands", disable_help_flag = true)]
    AutoFixCommands(RawCompatibilityArguments),
    /// Coordinate one validator autofix cycle.
    #[command(name = "validator-autofix", disable_help_flag = true)]
    ValidatorAutofix(RawCompatibilityArguments),
    /// Read or snapshot optional plan size trailers.
    #[command(name = "optional-trailers", disable_help_flag = true)]
    OptionalTrailers(RawCompatibilityArguments),
    /// Compose the plan-goals-test markdown document.
    #[command(name = "compose-goals-test", disable_help_flag = true)]
    ComposeGoalsTest(RawCompatibilityArguments),
    /// Compose and log the `/implement` Step 1 plan-goals-test records (#8579).
    #[command(name = "step1-log", disable_help_flag = true)]
    Step1Log(RawCompatibilityArguments),
}

#[derive(Subcommand)]
enum TrackingIssueCommand {
    /// Render one issue and its human comments into a task file.
    #[command(disable_help_flag = true)]
    Read(RawCompatibilityArguments),
    /// File one tracking issue from a drafted title and body file.
    #[command(name = "create-issue", disable_help_flag = true)]
    CreateIssue(RawCompatibilityArguments),
    /// Append one comment, optionally tagged with a lifecycle marker.
    #[command(name = "append-comment", disable_help_flag = true)]
    AppendComment(RawCompatibilityArguments),
    /// Move one tracking title to the prefix a lifecycle state names.
    #[command(disable_help_flag = true)]
    Rename(RawCompatibilityArguments),
    /// Tag one title as a disproved finding.
    #[command(name = "mark-false-positive", disable_help_flag = true)]
    MarkFalsePositive(RawCompatibilityArguments),
    /// Keep exactly one marker-keyed summary comment on the issue.
    #[command(name = "upsert-summary", disable_help_flag = true)]
    UpsertSummary(RawCompatibilityArguments),
}

#[derive(Subcommand)]
enum TrackingCommand {
    /// Compose and publish one implementation metadata comment.
    #[command(name = "post-issue", disable_help_flag = true)]
    PostIssue(RawCompatibilityArguments),
}

#[derive(Subcommand)]
enum PrCommand {
    /// Create a prefixed branch or report current branch facts.
    #[command(name = "create-branch", disable_help_flag = true)]
    CreateBranch(RawCompatibilityArguments),
    /// Create or adopt a pull request for the current branch.
    #[command(disable_help_flag = true)]
    Create(RawCompatibilityArguments),
    /// Replace one pull request body.
    #[command(name = "body-update", disable_help_flag = true)]
    BodyUpdate(RawCompatibilityArguments),
    /// Render pull request check state.
    #[command(disable_help_flag = true)]
    Checks(RawCompatibilityArguments),
    /// Extract the first closing issue footer.
    #[command(name = "closes-issue", disable_help_flag = true)]
    ClosesIssue(RawCompatibilityArguments),
    /// Compose the implementation goal and changed-scope PR bullets.
    #[command(name = "compose-summary", disable_help_flag = true)]
    ComposeSummary(RawCompatibilityArguments),
}

#[derive(Subcommand)]
enum TriageCommand {
    /// Read evidence only through an immutable fixed-origin object.
    #[command(disable_help_flag = true)]
    Inspect(RawCompatibilityArguments),
    /// Run one fixed, bounded, no-shell reproduction probe.
    #[command(disable_help_flag = true)]
    Probe(RawCompatibilityArguments),
    /// Apply one verified verdict with compare-and-swap checks.
    #[command(disable_help_flag = true)]
    Apply(RawCompatibilityArguments),
}

#[derive(Subcommand)]
enum UmbrellaCommand {
    /// Validate one source issue and publish its bounded snapshot.
    #[command(disable_help_flag = true)]
    Prepare(RawCompatibilityArguments),
    /// Publish the durable proposal record before any leaf is filed.
    #[command(name = "persist-proposal", disable_help_flag = true)]
    PersistProposal(RawCompatibilityArguments),
    /// Record that one named leaf was handed to `/issue`.
    #[command(name = "mark-in-flight", disable_help_flag = true)]
    MarkInFlight(RawCompatibilityArguments),
    /// Bind one named leaf to the issue `/issue` created for it.
    #[command(name = "record-resolved", disable_help_flag = true)]
    RecordResolved(RawCompatibilityArguments),
    /// Bind one in-flight leaf to the single remote issue carrying it.
    #[command(name = "reconcile-in-flight", disable_help_flag = true)]
    ReconcileInFlight(RawCompatibilityArguments),
    /// Convert the source issue into its final `[UMBRELLA]` title and body.
    #[command(disable_help_flag = true)]
    Mutate(RawCompatibilityArguments),
    /// Prove the recorded graph landed, then publish the completion sentinel.
    #[command(disable_help_flag = true)]
    Verify(RawCompatibilityArguments),
    /// Prove one child's completion sentinel against the approved partition.
    #[command(name = "verify-completion", disable_help_flag = true)]
    VerifyCompletion(RawCompatibilityArguments),
}

#[derive(Subcommand)]
enum UntrustedCommand {
    /// Wrap `--text` or stdin in a labelled, redacted content block.
    #[command(name = "content-block", disable_help_flag = true)]
    ContentBlock(RawCompatibilityArguments),
    /// Wrap one file's contents in a labelled, redacted content block.
    #[command(name = "file-block", disable_help_flag = true)]
    FileBlock(RawCompatibilityArguments),
    /// Redact stdin and escape its markup delimiters.
    #[command(name = "redact-stream", disable_help_flag = true)]
    RedactStream(RawCompatibilityArguments),
    /// Escape stdin for an XML attribute.
    #[command(name = "xml-escape-attr", disable_help_flag = true)]
    XmlEscapeAttr(RawCompatibilityArguments),
}

#[derive(Subcommand)]
enum KvCommand {
    /// Extract one value from `KEY=value` input.
    #[command(disable_help_flag = true)]
    Get(RawCompatibilityArguments),
}

#[derive(Subcommand)]
enum SessionCommand {
    /// Set up a session temporary directory and emit its compatibility envelope.
    #[command(disable_help_flag = true)]
    Setup(RawCompatibilityArguments),
    /// Remove a session temporary directory confined to the session roots.
    #[command(disable_help_flag = true)]
    CleanupTmpdir(RawCompatibilityArguments),
    /// Switch to main, fast-forward it, and remove a completed feature branch.
    #[command(disable_help_flag = true)]
    LocalCleanup(RawCompatibilityArguments),
    /// Authorize one live GitHub issue mutation for a session-backed caller.
    #[command(disable_help_flag = true)]
    CheckLiveMutationAuth(RawCompatibilityArguments),
    /// Resolve the `/implement` or `/design` entry gate from branch facts.
    #[command(disable_help_flag = true)]
    EntryGate(RawCompatibilityArguments),
    /// Terminate background processes scoped to a session tmpdir.
    #[command(disable_help_flag = true)]
    KillBackgroundProcesses(RawCompatibilityArguments),
    /// Read one value from a session environment file.
    #[command(disable_help_flag = true)]
    ReadKey(RawCompatibilityArguments),
    /// Read several values from one session environment file.
    #[command(disable_help_flag = true)]
    ReadKeys(RawCompatibilityArguments),
    /// Fail closed when `CLAUDE_PLUGIN_ROOT` is unset or unexpanded.
    #[command(disable_help_flag = true)]
    RequirePluginRoot(RawCompatibilityArguments),
    /// Print the live implement temporary directory for one clone.
    #[command(disable_help_flag = true)]
    ResolveImplementTmpdir(RawCompatibilityArguments),
    /// Validate a design temporary directory against the session allowlist.
    #[command(disable_help_flag = true)]
    ValidateDesignTmpdir(RawCompatibilityArguments),
    /// Idempotently publish a session identity file.
    #[command(disable_help_flag = true)]
    WriteId(RawCompatibilityArguments),
    /// Write the implement session environment file.
    #[command(disable_help_flag = true)]
    WriteEnv(RawCompatibilityArguments),
    /// Write the design session environment file and its PID-keyed pointer.
    #[command(disable_help_flag = true)]
    WriteDesignEnv(RawCompatibilityArguments),
    /// Write the implement current-env pointer and stable launcher.
    #[command(disable_help_flag = true)]
    WriteImplementEnv(RawCompatibilityArguments),
    /// Remove the implement current-env pointer.
    #[command(disable_help_flag = true)]
    ClearImplementPointer(RawCompatibilityArguments),
    /// Persist validated implement run flags.
    #[command(disable_help_flag = true)]
    PersistRunFlags(RawCompatibilityArguments),
    /// Write the design run-params document.
    #[command(disable_help_flag = true)]
    WriteRunParams(RawCompatibilityArguments),
    /// Rebuild finalize state from the durable ship-pr state file.
    #[command(disable_help_flag = true)]
    RestoreFinalizeState(RawCompatibilityArguments),
    /// Resolve a design session-env pointer to its trusted target.
    #[command(disable_help_flag = true)]
    ResolveTrustedDesignEnv(RawCompatibilityArguments),
}

#[derive(Subcommand)]
enum DepsCommand {
    /// Resolve the audited repository and report whether `origin` matches it.
    #[command(name = "resolve-repo", disable_help_flag = true)]
    ResolveRepo(RawCompatibilityArguments),
    /// Read every open issue, its comments, and its dependency edges.
    #[command(disable_help_flag = true)]
    Fetch(RawCompatibilityArguments),
    /// Scan fetched issue prose for explicit dependency declarations.
    #[command(name = "explicit-refs", disable_help_flag = true)]
    ExplicitRefs(RawCompatibilityArguments),
    /// Validate and record the operator's proposal document.
    #[command(name = "write-proposals", disable_help_flag = true)]
    WriteProposals(RawCompatibilityArguments),
    /// Compose the one plan the operator approves.
    #[command(disable_help_flag = true)]
    Plan(RawCompatibilityArguments),
    /// Apply exactly the mutations the approved plan carries.
    #[command(disable_help_flag = true)]
    Apply(RawCompatibilityArguments),
}

#[derive(Subcommand)]
enum ScoutCommand {
    /// Scout dynamic reviewer archetypes through the Cursor then Claude tiers.
    #[command(name = "dynamic-archetypes", disable_help_flag = true)]
    DynamicArchetypes(RawCompatibilityArguments),
    /// Scout the one plan-review archetype a design plan justifies.
    #[command(name = "plan-archetypes", disable_help_flag = true)]
    PlanArchetypes(RawCompatibilityArguments),
    /// Filter a scouted manifest to the caller's cap and panel mode.
    #[command(name = "filter-manifest", disable_help_flag = true)]
    FilterManifest(RawCompatibilityArguments),
}

#[derive(Subcommand)]
enum CombineIssuesCommand {
    /// Fetch combinable open issues.
    #[command(disable_help_flag = true)]
    Fetch(RawCompatibilityArguments),
    /// Fetch native dependency edges for source issues.
    #[command(name = "fetch-deps", disable_help_flag = true)]
    FetchDeps(RawCompatibilityArguments),
    /// List normalized open issue rows.
    #[command(name = "list-open", disable_help_flag = true)]
    ListOpen(RawCompatibilityArguments),
    /// Decide which source issues may safely close.
    #[command(name = "close-eligible", disable_help_flag = true)]
    CloseEligible(RawCompatibilityArguments),
    /// Compose remapped inherited dependency edges.
    #[command(name = "plan-inherited", disable_help_flag = true)]
    PlanInherited(RawCompatibilityArguments),
    /// Audit explicit dependency declarations in issue prose.
    #[command(name = "prose-audit", disable_help_flag = true)]
    ProseAudit(RawCompatibilityArguments),
    /// Combine tier-one and tier-two dependency candidates.
    #[command(name = "plan-audit", disable_help_flag = true)]
    PlanAudit(RawCompatibilityArguments),
    /// Create a combined issue and optionally close its sources.
    #[command(disable_help_flag = true)]
    Apply(RawCompatibilityArguments),
    /// Close sources after the dependency plan permits it.
    #[command(name = "close-sources", disable_help_flag = true)]
    CloseSources(RawCompatibilityArguments),
    /// Close stale or rejected sources with an optional comment.
    #[command(name = "close-stale", disable_help_flag = true)]
    CloseStale(RawCompatibilityArguments),
}

#[derive(Subcommand)]
enum ExecutionIssuesCommand {
    /// Add one entry under its category heading, exactly once.
    #[command(disable_help_flag = true)]
    Append(RawCompatibilityArguments),
    /// Publish the pending ledger tail and clear the ledger.
    #[command(disable_help_flag = true)]
    Flush(RawCompatibilityArguments),
    /// Publish the pending ledger tail without clearing the ledger.
    #[command(name = "flush-safety-net", disable_help_flag = true)]
    FlushSafetyNet(RawCompatibilityArguments),
    /// Project the pending count onto the tracking issue's metadata comment.
    #[command(disable_help_flag = true)]
    Refresh(RawCompatibilityArguments),
}

#[derive(Subcommand)]
enum ArchitecturalAssessmentCommand {
    /// Materialize assessment evidence paths for the arch-assessor subagent.
    #[command(disable_help_flag = true)]
    Materialize(RawCompatibilityArguments),
    /// Persist one authored assessment note fail-closed.
    #[command(disable_help_flag = true)]
    Submit(RawCompatibilityArguments),
    /// Print architectural sections for the terminal final report.
    #[command(name = "final-report-sections", disable_help_flag = true)]
    FinalReportSections(RawCompatibilityArguments),
    /// Sanitize one diagnostic line for an assessor handoff.
    #[command(name = "sanitize-detail", disable_help_flag = true)]
    SanitizeDetail(RawCompatibilityArguments),
}

#[derive(Subcommand)]
enum ArchitecturalPreparationCommand {
    /// Read and emit parsed architectural knowledge.
    #[command(disable_help_flag = true)]
    Read(RawCompatibilityArguments),
    /// Present architectural knowledge or its clean acknowledgment at Gate C.
    #[command(name = "present-note", disable_help_flag = true)]
    PresentNote(RawCompatibilityArguments),
    /// Persist the Gate C design assessment artifact.
    #[command(name = "persist-design-assessment", disable_help_flag = true)]
    PersistDesignAssessment(RawCompatibilityArguments),
    /// Materialize the implementation diff used by architectural assessment.
    #[command(name = "materialize-diff", disable_help_flag = true)]
    MaterializeDiff(RawCompatibilityArguments),
    /// Read architectural knowledge and materialize its implementation diff.
    #[command(disable_help_flag = true)]
    Prepare(RawCompatibilityArguments),
    /// Prepare frozen compose-time assessment evidence.
    #[command(name = "prepare-compose", disable_help_flag = true)]
    PrepareCompose(RawCompatibilityArguments),
    /// Persist one compose-time assessment against frozen evidence.
    #[command(name = "write-compose-assessment", disable_help_flag = true)]
    WriteComposeAssessment(RawCompatibilityArguments),
    /// Persist one staged assessment and its diff identity.
    #[command(name = "write-staged-assessment", disable_help_flag = true)]
    WriteStagedAssessment(RawCompatibilityArguments),
    /// Append one deviation to the execution warnings ledger.
    #[command(name = "append-deviation-note", disable_help_flag = true)]
    AppendDeviationNote(RawCompatibilityArguments),
    /// Promote a still-current staged assessment to its durable note.
    #[command(name = "pin-note-from-staged", disable_help_flag = true)]
    PinNoteFromStaged(RawCompatibilityArguments),
    /// Remove stale assessment-note artifacts.
    #[command(disable_help_flag = true)]
    Invalidate(RawCompatibilityArguments),
}

#[derive(Subcommand)]
enum OosCommand {
    /// Materialize external implementer observations into accepted-OOS blocks.
    #[command(name = "materialize-manifest", disable_help_flag = true)]
    MaterializeManifest(RawCompatibilityArguments),
    /// Bound how many OOS issues one run may file.
    #[command(name = "issue-cap", disable_help_flag = true)]
    IssueCap(RawCompatibilityArguments),
    /// Emit the intra-batch dependency rows two conflicting items require.
    #[command(name = "file-conflict-deps", disable_help_flag = true)]
    FileConflictDeps(RawCompatibilityArguments),
    /// Refuse a run that silently dropped an accepted OOS record.
    #[command(name = "disposition-gate", disable_help_flag = true)]
    DispositionGate(RawCompatibilityArguments),
    /// Resolve the gate's inputs from one session directory and record them.
    #[command(name = "disposition-checkpoint", disable_help_flag = true)]
    DispositionCheckpoint(RawCompatibilityArguments),
    /// Renumber a findings file's accepted non-security OOS records into a batch.
    #[command(name = "serialize", disable_help_flag = true)]
    Serialize(RawCompatibilityArguments),
    /// Rewrite one block's first line to the canonical `### OOS_<seq>:` id.
    #[command(name = "normalize-header", disable_help_flag = true)]
    NormalizeHeader(RawCompatibilityArguments),
    /// File the run's accepted out-of-scope observations as public issues.
    #[command(disable_help_flag = true)]
    File(RawCompatibilityArguments),
}

#[derive(Subcommand)]
enum DifficultyCommand {
    /// Validate a raw model rating object.
    #[command(name = "validate-rating", disable_help_flag = true)]
    ValidateRating(RawCompatibilityArguments),
    /// Read plan-trailer difficulty metadata.
    #[command(name = "extract-plan-metadata", disable_help_flag = true)]
    ExtractPlanMetadata(RawCompatibilityArguments),
    /// Write or refresh a persisted difficulty record.
    #[command(name = "write-record", disable_help_flag = true)]
    WriteRecord(RawCompatibilityArguments),
    /// Print the shared rating rubric.
    #[command(name = "render-rubric", disable_help_flag = true)]
    RenderRubric(RawCompatibilityArguments),
    /// Render one human-readable record summary line.
    #[command(name = "render-line", disable_help_flag = true)]
    RenderLine(RawCompatibilityArguments),
    /// Resolve and persist panel fields for a record.
    #[command(name = "resolve-panel", disable_help_flag = true)]
    ResolvePanel(RawCompatibilityArguments),
    /// Replace difficulty labels on a GitHub issue.
    #[command(name = "sync-labels", disable_help_flag = true)]
    SyncLabels(RawCompatibilityArguments),
}

#[derive(Subcommand)]
enum DifficultyCalibrationCommand {
    /// Analyze one synchronized or explicitly supplied run-log corpus.
    #[command(disable_help_flag = true)]
    Analyze(RawCompatibilityArguments),
}

#[derive(Subcommand)]
enum FluffAnalysisCommand {
    /// Analyze review fluff over one synchronized or explicit run-log corpus.
    #[command(disable_help_flag = true)]
    Analyze(RawCompatibilityArguments),
}

impl DifficultyCommand {
    fn run(self) -> ExitCode {
        match self {
            Self::ValidateRating(arguments) => {
                difficulty_commands::validate_rating(&arguments.arguments)
            }
            Self::ExtractPlanMetadata(arguments) => {
                difficulty_commands::extract_plan_metadata(&arguments.arguments)
            }
            Self::WriteRecord(arguments) => difficulty_commands::write_record(&arguments.arguments),
            Self::RenderRubric(arguments) => {
                difficulty_commands::render_rubric(&arguments.arguments)
            }
            Self::RenderLine(arguments) => difficulty_commands::render_line(&arguments.arguments),
            Self::ResolvePanel(arguments) => {
                difficulty_commands::resolve_panel(&arguments.arguments)
            }
            Self::SyncLabels(arguments) => difficulty_commands::sync_labels(&arguments.arguments),
        }
    }
}

#[derive(Subcommand)]
enum DirtyTreeCommand {
    /// Classify tracked and new untracked paths against a baseline.
    #[command(disable_help_flag = true)]
    Baseline(RawCompatibilityArguments),
    /// Report whether a consumer worktree is clean.
    #[command(disable_help_flag = true)]
    Checkpoint(RawCompatibilityArguments),
    /// Reject recovered paths that are outside the plan scope.
    #[command(disable_help_flag = true)]
    ScopeCheck(RawCompatibilityArguments),
    /// Detect a scope-reduction review marker.
    #[command(disable_help_flag = true)]
    ScopeMarker(RawCompatibilityArguments),
}

#[derive(Subcommand)]
enum ForkedRepoCommand {
    /// Verify, mirror, and configure the current checkout.
    #[command(disable_help_flag = true)]
    Setup(RawCompatibilityArguments),
}

#[derive(Args)]
#[command(trailing_var_arg = true)]
struct RawCompatibilityArguments {
    /// Raw arguments parsed by the legacy-compatible command boundary.
    #[arg(allow_hyphen_values = true)]
    arguments: Vec<OsString>,
}

#[derive(Subcommand)]
enum RedactCommand {
    /// Scrub known secret families from stdin.
    #[command(disable_help_flag = true)]
    Secrets(RawCompatibilityArguments),
    /// Scrub session and operator paths from stdin.
    #[command(name = "tmpdir-paths", disable_help_flag = true)]
    TmpdirPaths(RawCompatibilityArguments),
    /// Scrub known secret families from every UTF-8 file below a directory.
    #[command(name = "scrub-log-secrets", disable_help_flag = true)]
    ScrubLogSecrets(RawCompatibilityArguments),
    /// Remove findings that belong to Git submodules.
    #[command(name = "scrub-submodule-paths", disable_help_flag = true)]
    ScrubSubmodulePaths(RawCompatibilityArguments),
}

#[derive(Subcommand)]
enum GitSubcommand {
    /// Stage paths and amend them into the current commit.
    AmendAdd(MutationPathsArguments),
    /// Emit `HEAD_SHA` and `CURRENT_BRANCH` for the cwd repository.
    BranchInfo(TrailingArguments),
    /// Classify repository changes against an untracked-path baseline.
    CheckPhantomDirty(CheckPhantomDirtyArguments),
    /// Probe whether a remote branch exists via typed ls-remote.
    CheckRemoteBranch(TrailingArguments),
    /// Classify local main synchronization against origin/main.
    CheckMainSync(TrailingArguments),
    /// Check out the current side of conflicted paths.
    CheckoutOurs(CheckoutOursArguments),
    /// Report whether the worktree is clean using machine-readable key/value rows.
    CleanTree(CleanTreeArguments),
    /// Stage optional paths and create a commit.
    Commit(CommitArguments),
    /// Print the files and index stages that are currently conflicted.
    ConflictFiles,
    /// Count commits on `HEAD` since `origin/main` or `main`.
    CountCommits(TrailingArguments),
    /// Emit `BRANCH` for the current symbolic `HEAD`.
    CurrentBranch(TrailingArguments),
    /// Classify phantom paths and append advisory warnings to the run ledger.
    PhantomProbe(PhantomProbeArguments),
    /// Abort an in-progress rebase, succeeding when no rebase is active.
    RebaseAbort(RebaseControlArguments),
    /// Skip the current commit in an in-progress rebase.
    RebaseSkip(RebaseControlArguments),
    /// Print the blob at an index conflict stage.
    ShowStage(TrailingArguments),
    /// Update a non-checked-out local main branch from its remote-tracking ref.
    SyncLocalMain(TrailingArguments),
    /// Atomically write the sorted untracked-path baseline to an output file.
    SnapshotUntracked(SnapshotUntrackedArguments),
    /// Stage one or more paths.
    Stage(MutationPathsArguments),
}

#[derive(Args)]
struct MutationPathsArguments {
    #[arg(allow_hyphen_values = true)]
    paths: Vec<PathBuf>,
}

#[derive(Args)]
struct CommitArguments {
    #[arg(short = 'm', default_value = "")]
    message: String,
    #[arg(long)]
    no_trailer: bool,
    #[arg(long)]
    only: bool,
    #[arg(long)]
    pathspec_from_file: Option<PathBuf>,
    #[arg(long)]
    pathspec_file_nul: bool,
    #[arg(allow_hyphen_values = true)]
    files: Vec<PathBuf>,
}

#[derive(Args)]
#[command(trailing_var_arg = true, disable_help_flag = true)]
struct CheckPhantomDirtyArguments {
    /// Raw compatibility arguments; parse errors are advisory command results.
    #[arg(allow_hyphen_values = true)]
    arguments: Vec<OsString>,
}

#[derive(Args)]
struct CheckoutOursArguments {
    /// Conflicted paths to replace with the current side.
    #[arg(allow_hyphen_values = true)]
    paths: Vec<PathBuf>,
}

#[derive(Args)]
struct RebaseControlArguments {
    #[arg(allow_hyphen_values = true)]
    extra: Vec<OsString>,
}

#[derive(Args, Clone, Copy)]
struct CleanTreeArguments {
    /// Treat a repository probe failure as an error instead of a clean tree.
    #[arg(long)]
    fail_closed: bool,
}

#[derive(Args)]
struct SnapshotUntrackedArguments {
    /// File that receives the sorted untracked-path baseline.
    #[arg(long)]
    output: Option<std::path::PathBuf>,
    /// Separate output paths with NUL bytes rather than line feeds.
    #[arg(long)]
    nul: bool,
}

#[derive(Args)]
struct PhantomProbeArguments {
    /// Stable token identifying the checkpoint that invoked the probe.
    #[arg(long)]
    step: String,
    /// Override the session's untracked-path baseline.
    #[arg(long)]
    baseline_file: Option<PathBuf>,
}

#[derive(Args)]
struct TrailingArguments {
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    args: Vec<String>,
}

#[derive(Subcommand)]
enum BootstrapCommand {
    /// Run the session bootstrap routing state machine.
    #[command(disable_help_flag = true)]
    Invoke(RawCompatibilityArguments),
    /// Parse a bootstrap stdout envelope into shell assignments.
    #[command(name = "parse-routing", disable_help_flag = true)]
    ParseRouting(RawCompatibilityArguments),
    /// Resolve the non-interactive execution predicate.
    #[command(name = "resolve-non-interactive", disable_help_flag = true)]
    ResolveNonInteractive(RawCompatibilityArguments),
    /// Print the compiled version and target as machine-readable JSON.
    SelfCheck,
}

#[derive(Subcommand)]
enum CleanupCommand {
    /// Remove stale session directories, temporary files, and dangling pointers.
    #[command(disable_help_flag = true)]
    Run(RawCompatibilityArguments),
}

#[derive(Subcommand)]
enum StatusCommand {
    /// Print plugin and external-reviewer health.
    #[command(disable_help_flag = true)]
    Check(RawCompatibilityArguments),
}

#[derive(Args)]
struct StatusArguments {
    #[command(subcommand)]
    command: Option<StatusCommand>,
}

#[derive(Subcommand)]
enum ExampleCommand {
    /// Print a message through the core library.
    Echo(EchoArguments),
}

#[derive(Subcommand)]
enum ObjectStoreCommand {
    /// Use Google Cloud Storage through validated Application Default Credentials.
    Gcs(GcsArguments),
}

#[derive(Subcommand)]
enum ReleaseCommand {
    /// Validate the tagged release identity against plugin and Cargo versions.
    AssetCandidate(AssetCandidateArguments),
    /// Resolve the exact tag-triggered release asset workflow run.
    AssetRun(AssetRunArguments),
    /// Classify the semantic version bump for the public plugin surface.
    ClassifyBump(ClassifyBumpArguments),
    /// Collect matrix archives into the final release asset set.
    CollectAssets(CollectAssetsArguments),
    /// Enable and verify release repository policy.
    EnsurePolicy(EnsurePolicyArguments),
    /// Publish, attest, and promote the merged release candidate.
    Finish(FinishReleaseArguments),
    /// Package one target archive and metadata fragment.
    PackageAsset(PackageAssetArguments),
    /// Prepare the release window, PR list, and aggregate bump.
    Prepare(PrepareReleaseArguments),
    /// Recompute release notes PRs against the merged source commit.
    ReconcileNotes(ReconcileNotesArguments),
    /// Promote one immutable release to Latest.
    Promote(PromoteReleaseArguments),
    /// Promote the newest immutable non-draft release to Latest.
    PromoteLatest(PromoteLatestArguments),
    /// Generate the runtime-only plugin projection.
    PluginRuntime(PluginRuntimeArguments),
    /// Update every synchronized release version surface.
    SetVersion(SetVersionArguments),
    /// Tag the candidate and create or resume its mutable draft.
    Stage(StageReleaseArguments),
    /// Validate the final release asset allowlist.
    ValidateAssets(ValidateAssetsArguments),
    /// Validate the candidate-bound draft and complete asset set.
    ValidateDraft(ValidateDraftArguments),
}

#[derive(Subcommand)]
enum PluginCommand {
    /// Print the active plugin version as a machine-readable row.
    ReadVersion(TrailingArguments),
    /// Print the canonical upstream repository from plugin metadata.
    ResolveRepository(TrailingArguments),
}

#[derive(Args)]
struct AssetCandidateArguments {
    #[arg(long)]
    repo_root: PathBuf,
    #[arg(long)]
    tag: String,
    #[arg(long)]
    source_commit: String,
    /// Require the typed repository reader to prove the checkout identity.
    #[arg(long)]
    verify_checkout: bool,
}

#[derive(Args)]
struct AssetRunArguments {
    #[arg(long = "repo")]
    repository: String,
    #[arg(long)]
    tag: String,
    #[arg(long)]
    source_commit: String,
}

#[derive(Args)]
struct EnsurePolicyArguments {
    #[arg(long = "repo")]
    repository: String,
}

#[derive(Args)]
struct FinishReleaseArguments {
    #[arg(long)]
    version: String,
    #[arg(long = "repo")]
    repository: String,
    #[arg(long)]
    pr: String,
    #[arg(long)]
    source_commit: String,
}

#[derive(Args)]
struct PromoteReleaseArguments {
    version: String,
    #[arg(long = "repo")]
    repository: Option<String>,
}

#[derive(Args)]
struct PromoteLatestArguments {
    #[arg(long = "repo", default_value = "zhupanov/larch")]
    repository: String,
    #[arg(long)]
    dry_run: bool,
}

#[derive(Args)]
struct StageReleaseArguments {
    #[arg(long)]
    version: String,
    /// Draft body; required unless --dry-run.
    #[arg(long, required_unless_present = "dry_run")]
    notes_file: Option<PathBuf>,
    #[arg(long = "repo")]
    repository: String,
    #[arg(long)]
    pr: String,
    /// Build and prove the projection commit without tagging, pushing, or
    /// creating a draft.
    #[arg(long)]
    dry_run: bool,
}

#[derive(Args)]
struct ValidateDraftArguments {
    #[arg(long)]
    version: String,
    #[arg(long = "repo")]
    repository: String,
    #[arg(long)]
    pr: String,
    #[arg(long)]
    source_commit: String,
}

#[derive(Args)]
struct PackageAssetArguments {
    #[arg(long)]
    version: String,
    #[arg(long)]
    tag: String,
    #[arg(long)]
    source_commit: String,
    #[arg(long)]
    target: String,
    #[arg(long)]
    binary: PathBuf,
    #[arg(long = "license")]
    license: PathBuf,
    #[arg(long)]
    output_dir: PathBuf,
}

#[derive(Args)]
struct CollectAssetsArguments {
    #[arg(long)]
    version: String,
    #[arg(long)]
    tag: String,
    #[arg(long)]
    source_commit: String,
    #[arg(long)]
    input_dir: PathBuf,
    #[arg(long)]
    output_dir: PathBuf,
    #[arg(long = "license")]
    license: PathBuf,
}

#[derive(Args)]
struct ValidateAssetsArguments {
    #[arg(long)]
    version: String,
    #[arg(long)]
    tag: String,
    #[arg(long)]
    source_commit: String,
    #[arg(long)]
    asset_dir: PathBuf,
    #[arg(long = "license")]
    license: PathBuf,
    #[arg(long)]
    verify_attestations: bool,
}

#[derive(Args)]
struct ClassifyBumpArguments {
    #[arg(long)]
    base: Option<String>,
    #[arg(long)]
    head: Option<String>,
}

#[derive(Args)]
struct PrepareReleaseArguments {
    #[arg(long = "repo", default_value = "zhupanov/larch", value_parser = parse_repository)]
    repository: larch_core::GitHubRepositoryRef,
    #[arg(long, value_parser = ["major", "minor", "patch"])]
    bump: Option<String>,
    #[arg(long, required = true)]
    out_dir: PathBuf,
    /// Plan from the existing origin/main ref without fetching Git objects or refs.
    #[arg(long)]
    no_fetch: bool,
}

#[derive(Args)]
struct ReconcileNotesArguments {
    #[arg(long = "repo", default_value = "zhupanov/larch", value_parser = parse_repository)]
    repository: larch_core::GitHubRepositoryRef,
    #[arg(long)]
    baseline_tag: String,
    #[arg(long)]
    source_commit: String,
    #[arg(long)]
    pr_list: PathBuf,
    #[arg(long)]
    exclude_pr: u64,
    #[arg(long, required = true)]
    out_dir: PathBuf,
}

#[derive(Args)]
struct SetVersionArguments {
    version: String,
}

#[derive(Subcommand)]
enum UpgradeLarchCommand {
    /// Resolve the cache root used by release Step 7.
    ReleaseStep7Root(ReleaseStep7Arguments),
    /// Upgrade to the latest verified stable release.
    Run(UpgradeLarchRunArguments),
    /// Print the legacy sparse-checkout allowlist.
    SparseDirs,
}

#[derive(Args)]
struct UpgradeLarchRunArguments {
    /// Use one validated installed root for release Step 7.
    #[arg(long, hide = true)]
    plugin_root: Option<PathBuf>,
}

#[derive(Args)]
struct ReleaseStep7Arguments {
    /// Current version used only to disambiguate one cache directory.
    #[arg(long, conflicts_with = "positional_current_version")]
    current_version: Option<String>,
    /// Backward-compatible positional spelling of the current version.
    #[arg(conflicts_with = "current_version")]
    positional_current_version: Option<String>,
}

#[derive(Args)]
struct PluginRuntimeArguments {
    /// Generate the projection into DIR.
    #[arg(long)]
    output: PathBuf,
}

#[derive(Subcommand)]
enum GhCommand {
    /// Read one agnix issue through the typed GitHub service.
    AgnixIssue(AgnixIssueArguments),
    /// Ensure the agnix fork's skip-changelog label exists through the typed service.
    AgnixEnsureLabel(AgnixEnsureLabelArguments),
    /// Parse a remote name or URL into OWNER/REPO.
    RemoteRepo(TrailingArguments),
    /// Resolve the ambient GitHub repository slug for the cwd.
    ResolveRepo(TrailingArguments),
    /// Print the complete log archive for a workflow run.
    RunLogs(RunLogsArguments),
    /// Print the retained workflow-path placeholder.
    WorkflowPath,
}

#[derive(Args)]
struct AgnixIssueArguments {
    /// Positive upstream issue number.
    #[arg(long)]
    issue: u64,
    /// GitHub repository in OWNER/REPO form.
    #[arg(long = "repo", value_parser = parse_repository)]
    repository: larch_core::GitHubRepositoryRef,
}

#[derive(Args)]
struct AgnixEnsureLabelArguments {
    /// GitHub repository in OWNER/REPO form.
    #[arg(long = "repo", value_parser = parse_repository)]
    repository: larch_core::GitHubRepositoryRef,
}

#[derive(Subcommand)]
enum PushSubcommand {
    /// Push the current branch to its explicit origin branch ref.
    Branch(TrailingArguments),
    /// Force-push the current branch with a lease.
    Force(PushForceArguments),
    /// Rebase the current branch onto its base, then optionally force-push.
    Rebase(TrailingArguments),
    /// Rebase checkpoint probe with trivial-conflict pre-pass and phantom tail.
    CheckpointProbe(TrailingArguments),
}

#[derive(Args)]
struct PushForceArguments {
    #[arg(long)]
    expected_remote_oid: Option<String>,
}

#[derive(Args)]
struct RunLogsArguments {
    /// Numeric GitHub Actions workflow run identifier.
    #[arg(long)]
    run_id: u64,
    /// GitHub repository in OWNER/REPO form.
    #[arg(long = "repo", value_parser = parse_repository)]
    repository: larch_core::GitHubRepositoryRef,
}

#[derive(Args)]
struct EchoArguments {
    /// Message to print.
    message: String,
}

/// Dispatch one `session` verb to its command module.
fn run_session(command: SessionCommand) -> ExitCode {
    match command {
        SessionCommand::CheckLiveMutationAuth(arguments) => {
            session_gate_commands::check_live_mutation_auth_command(&arguments.arguments)
        }
        SessionCommand::EntryGate(arguments) => {
            session_gate_commands::entry_gate(&arguments.arguments)
        }
        SessionCommand::KillBackgroundProcesses(arguments) => {
            kill_background::kill_background_processes(&arguments.arguments)
        }
        SessionCommand::Setup(arguments) => session_setup_commands::setup(&arguments.arguments),
        SessionCommand::ReadKey(arguments) => state_commands::read_key(&arguments.arguments),
        SessionCommand::ReadKeys(arguments) => state_commands::read_keys(&arguments.arguments),
        SessionCommand::CleanupTmpdir(arguments) => {
            session_lifecycle_commands::cleanup_tmpdir(&arguments.arguments)
        }
        SessionCommand::LocalCleanup(arguments) => {
            session_closeout_commands::local_cleanup(&arguments.arguments)
        }
        SessionCommand::RequirePluginRoot(arguments) => {
            session_lifecycle_commands::require_plugin_root(&arguments.arguments)
        }
        SessionCommand::ResolveImplementTmpdir(arguments) => {
            session_lifecycle_commands::resolve_implement_tmpdir_command(&arguments.arguments)
        }
        SessionCommand::ValidateDesignTmpdir(arguments) => {
            session_lifecycle_commands::validate_design_tmpdir_command(&arguments.arguments)
        }
        SessionCommand::WriteId(arguments) => {
            session_lifecycle_commands::write_id(&arguments.arguments)
        }
        SessionCommand::WriteEnv(arguments) => {
            session_env_commands::write_env(&arguments.arguments)
        }
        SessionCommand::WriteDesignEnv(arguments) => {
            session_env_commands::write_design_env(&arguments.arguments)
        }
        SessionCommand::WriteImplementEnv(arguments) => {
            session_env_commands::write_implement_env(&arguments.arguments)
        }
        SessionCommand::ClearImplementPointer(arguments) => {
            session_env_commands::clear_implement_pointer(&arguments.arguments)
        }
        SessionCommand::PersistRunFlags(arguments) => {
            session_env_commands::persist_run_flags(&arguments.arguments)
        }
        SessionCommand::WriteRunParams(arguments) => {
            session_env_commands::write_run_params(&arguments.arguments)
        }
        SessionCommand::RestoreFinalizeState(arguments) => {
            session_env_commands::restore_finalize_state(&arguments.arguments)
        }
        SessionCommand::ResolveTrustedDesignEnv(arguments) => {
            session_env_commands::resolve_trusted_design_env(&arguments.arguments)
        }
    }
}

#[allow(clippy::too_many_lines, clippy::cognitive_complexity)] // Domain dispatch enumerates every Rust-owned command pair.
fn run(
    cli: Cli,
    metadata: larch_core::BuildMetadata,
) -> Result<ExitCode, larch_adapters::upgrade_larch::Failure> {
    match cli.domain {
        Domain::Agent(command) => Ok(agent_commands::run(command)),
        Domain::Review(command) => Ok(review_commands::run(command)),
        Domain::ReviewAndFix(command) => Ok(review_and_fix_commands::run(command)),
        Domain::CalibrationReplay(CalibrationReplayCommand::RebuildBallot(arguments)) => Ok(
            calibration_commands::rebuild_ballot_command(&arguments.arguments),
        ),
        Domain::CalibrationReplay(CalibrationReplayCommand::RunReplay(arguments)) => Ok(
            calibration_commands::run_replay_command(&arguments.arguments),
        ),
        Domain::CalibrationReplay(CalibrationReplayCommand::ValidateManifest(arguments)) => Ok(
            calibration_commands::validate_manifest_command(&arguments.arguments),
        ),
        Domain::VoterCalibration(VoterCalibrationCommand::Snapshot(arguments)) => Ok(
            calibration_commands::voter_calibration_snapshot(&arguments.arguments),
        ),
        Domain::VoterCalibration(VoterCalibrationCommand::Analyze(arguments)) => {
            Ok(voter_calibration_commands::analyze(&arguments.arguments))
        }
        Domain::PlanReview(command) => Ok(plan_review_commands::run(command)),
        Domain::Redact(command) => Ok(match command {
            RedactCommand::Secrets(arguments) => redact_commands::secrets(&arguments.arguments),
            RedactCommand::TmpdirPaths(arguments) => {
                redact_commands::tmpdir_paths(&arguments.arguments)
            }
            RedactCommand::ScrubLogSecrets(arguments) => {
                redact_commands::scrub_log_secrets(&arguments.arguments)
            }
            RedactCommand::ScrubSubmodulePaths(arguments) => {
                redact_commands::scrub_submodule_paths(&arguments.arguments)
            }
        }),
        Domain::Alias(command) => Ok(developer_tooling_commands::run_alias(command)),
        Domain::Bootstrap(BootstrapCommand::Invoke(arguments)) => {
            Ok(bootstrap_commands::invoke(&arguments.arguments))
        }
        Domain::Bootstrap(BootstrapCommand::ParseRouting(arguments)) => {
            Ok(bootstrap_commands::parse_routing(&arguments.arguments))
        }
        Domain::Bootstrap(BootstrapCommand::ResolveNonInteractive(arguments)) => Ok(
            bootstrap_commands::resolve_non_interactive(&arguments.arguments),
        ),
        Domain::Bootstrap(BootstrapCommand::SelfCheck) => {
            println!("{}", larch_core::bootstrap_self_check(metadata));
            Ok(ExitCode::SUCCESS)
        }
        Domain::Bgjob(BgjobCommand::Adapt(arguments)) => {
            Ok(bgjob_adapt::adapt(&arguments.arguments))
        }
        Domain::Bgjob(BgjobCommand::Start(arguments)) => {
            Ok(bgjob_commands::start(&arguments.arguments))
        }
        Domain::Bgjob(BgjobCommand::Wait(arguments)) => {
            Ok(bgjob_commands::wait(&arguments.arguments))
        }
        Domain::Bgjob(BgjobCommand::Status(arguments)) => {
            Ok(bgjob_commands::status(&arguments.arguments))
        }
        Domain::Bgjob(BgjobCommand::Reap(arguments)) => {
            Ok(bgjob_commands::reap(&arguments.arguments))
        }
        Domain::Bgjob(BgjobCommand::WriteMergeResultEnv(arguments)) => {
            Ok(bgjob_commands::write_merge_result_env(&arguments.arguments))
        }
        Domain::Clarify(command) => Ok(command.run()),
        Domain::Cleanup(CleanupCommand::Run(arguments)) => {
            Ok(cleanup_commands::run(&arguments.arguments))
        }
        Domain::CiTiming(command) => Ok(ci_timing::run(command)),
        Domain::Ci(command) => Ok(ci_selection::run(command)),
        Domain::ArchitecturalAssessment(command) => Ok(match command {
            ArchitecturalAssessmentCommand::Materialize(arguments) => {
                architectural_assessment_commands::materialize_command(&arguments.arguments)
            }
            ArchitecturalAssessmentCommand::Submit(arguments) => {
                architectural_assessment_commands::submit_command(&arguments.arguments)
            }
            ArchitecturalAssessmentCommand::FinalReportSections(arguments) => {
                architectural_assessment_commands::final_report_sections_command(
                    &arguments.arguments,
                )
            }
            ArchitecturalAssessmentCommand::SanitizeDetail(arguments) => {
                architectural_assessment_commands::sanitize_detail_command(&arguments.arguments)
            }
        }),
        Domain::ArchitecturalGuidelines(command) => Ok(match command {
            ArchitecturalPreparationCommand::Read(arguments) => {
                architectural_preparation_commands::read_command(
                    larch_core::AssessmentKind::Guidelines,
                    &arguments.arguments,
                )
            }
            ArchitecturalPreparationCommand::PresentNote(arguments) => {
                architectural_preparation_commands::present_note_command(
                    larch_core::AssessmentKind::Guidelines,
                    &arguments.arguments,
                )
            }
            ArchitecturalPreparationCommand::PersistDesignAssessment(arguments) => {
                architectural_preparation_commands::persist_design_assessment_command(
                    larch_core::AssessmentKind::Guidelines,
                    &arguments.arguments,
                )
            }
            ArchitecturalPreparationCommand::MaterializeDiff(arguments) => {
                architectural_preparation_commands::materialize_diff_command(
                    larch_core::AssessmentKind::Guidelines,
                    &arguments.arguments,
                )
            }
            ArchitecturalPreparationCommand::Prepare(arguments) => {
                architectural_preparation_commands::prepare_command(
                    larch_core::AssessmentKind::Guidelines,
                    &arguments.arguments,
                )
            }
            ArchitecturalPreparationCommand::PrepareCompose(arguments) => {
                architectural_preparation_commands::prepare_compose_command(
                    larch_core::AssessmentKind::Guidelines,
                    &arguments.arguments,
                )
            }
            ArchitecturalPreparationCommand::WriteComposeAssessment(arguments) => {
                architectural_preparation_commands::write_compose_assessment_command(
                    larch_core::AssessmentKind::Guidelines,
                    &arguments.arguments,
                )
            }
            ArchitecturalPreparationCommand::WriteStagedAssessment(arguments) => {
                architectural_preparation_commands::write_staged_assessment_command(
                    larch_core::AssessmentKind::Guidelines,
                    &arguments.arguments,
                )
            }
            ArchitecturalPreparationCommand::AppendDeviationNote(arguments) => {
                architectural_preparation_commands::append_deviation_note_command(
                    larch_core::AssessmentKind::Guidelines,
                    &arguments.arguments,
                )
            }
            ArchitecturalPreparationCommand::PinNoteFromStaged(arguments) => {
                architectural_preparation_commands::pin_note_from_staged_command(
                    larch_core::AssessmentKind::Guidelines,
                    &arguments.arguments,
                )
            }
            ArchitecturalPreparationCommand::Invalidate(arguments) => {
                architectural_preparation_commands::invalidate_command(
                    larch_core::AssessmentKind::Guidelines,
                    &arguments.arguments,
                )
            }
        }),
        Domain::ArchitecturalInvariants(command) => Ok(match command {
            ArchitecturalPreparationCommand::Read(arguments) => {
                architectural_preparation_commands::read_command(
                    larch_core::AssessmentKind::Invariants,
                    &arguments.arguments,
                )
            }
            ArchitecturalPreparationCommand::PresentNote(arguments) => {
                architectural_preparation_commands::present_note_command(
                    larch_core::AssessmentKind::Invariants,
                    &arguments.arguments,
                )
            }
            ArchitecturalPreparationCommand::PersistDesignAssessment(arguments) => {
                architectural_preparation_commands::persist_design_assessment_command(
                    larch_core::AssessmentKind::Invariants,
                    &arguments.arguments,
                )
            }
            ArchitecturalPreparationCommand::MaterializeDiff(arguments) => {
                architectural_preparation_commands::materialize_diff_command(
                    larch_core::AssessmentKind::Invariants,
                    &arguments.arguments,
                )
            }
            ArchitecturalPreparationCommand::Prepare(arguments) => {
                architectural_preparation_commands::prepare_command(
                    larch_core::AssessmentKind::Invariants,
                    &arguments.arguments,
                )
            }
            ArchitecturalPreparationCommand::PrepareCompose(arguments) => {
                architectural_preparation_commands::prepare_compose_command(
                    larch_core::AssessmentKind::Invariants,
                    &arguments.arguments,
                )
            }
            ArchitecturalPreparationCommand::WriteComposeAssessment(arguments) => {
                architectural_preparation_commands::write_compose_assessment_command(
                    larch_core::AssessmentKind::Invariants,
                    &arguments.arguments,
                )
            }
            ArchitecturalPreparationCommand::WriteStagedAssessment(arguments) => {
                architectural_preparation_commands::write_staged_assessment_command(
                    larch_core::AssessmentKind::Invariants,
                    &arguments.arguments,
                )
            }
            ArchitecturalPreparationCommand::AppendDeviationNote(arguments) => {
                architectural_preparation_commands::append_deviation_note_command(
                    larch_core::AssessmentKind::Invariants,
                    &arguments.arguments,
                )
            }
            ArchitecturalPreparationCommand::PinNoteFromStaged(arguments) => {
                architectural_preparation_commands::pin_note_from_staged_command(
                    larch_core::AssessmentKind::Invariants,
                    &arguments.arguments,
                )
            }
            ArchitecturalPreparationCommand::Invalidate(arguments) => {
                architectural_preparation_commands::invalidate_command(
                    larch_core::AssessmentKind::Invariants,
                    &arguments.arguments,
                )
            }
        }),
        Domain::AuditUmbrella(command) => Ok(audit_umbrella_commands::run(command)),
        Domain::CompleteUmbrella(command) => Ok(complete_umbrella_commands::run(command)),
        Domain::Net(command) => Ok(net_commands::run(command)),
        Domain::DirtyTree(command) => Ok(match command {
            DirtyTreeCommand::Baseline(arguments) => {
                let raw = dirty_tree_raw_arguments("baseline");
                dirty_tree_commands::baseline(raw.as_deref().unwrap_or(&arguments.arguments))
            }
            DirtyTreeCommand::Checkpoint(arguments) => {
                let raw = dirty_tree_raw_arguments("checkpoint");
                dirty_tree_commands::checkpoint(raw.as_deref().unwrap_or(&arguments.arguments))
            }
            DirtyTreeCommand::ScopeCheck(arguments) => {
                let raw = dirty_tree_raw_arguments("scope-check");
                dirty_tree_commands::scope_check(raw.as_deref().unwrap_or(&arguments.arguments))
            }
            DirtyTreeCommand::ScopeMarker(arguments) => {
                let raw = dirty_tree_raw_arguments("scope-marker");
                dirty_tree_commands::scope_marker(raw.as_deref().unwrap_or(&arguments.arguments))
            }
        }),
        Domain::Oos(command) => Ok(match command {
            OosCommand::MaterializeManifest(arguments) => {
                oos_commands::materialize_manifest(&arguments.arguments)
            }
            OosCommand::IssueCap(arguments) => oos_commands::issue_cap(&arguments.arguments),
            OosCommand::FileConflictDeps(arguments) => {
                oos_commands::file_conflict_deps(&arguments.arguments)
            }
            OosCommand::DispositionGate(arguments) => {
                oos_commands::disposition_gate(&arguments.arguments)
            }
            OosCommand::File(arguments) => oos_file_commands::file(&arguments.arguments),
            OosCommand::DispositionCheckpoint(arguments) => {
                oos_commands::disposition_checkpoint(&arguments.arguments)
            }
            OosCommand::Serialize(arguments) => oos_commands::serialize(&arguments.arguments),
            OosCommand::NormalizeHeader(arguments) => {
                oos_commands::normalize_header(&arguments.arguments)
            }
        }),
        Domain::Difficulty(command) => Ok(command.run()),
        Domain::DifficultyCalibration(DifficultyCalibrationCommand::Analyze(arguments)) => Ok(
            difficulty_calibration_commands::analyze(&arguments.arguments),
        ),
        Domain::FluffAnalysis(FluffAnalysisCommand::Analyze(arguments)) => {
            Ok(fluff_analysis_commands::analyze(&arguments.arguments))
        }
        Domain::ForkedRepo(ForkedRepoCommand::Setup(arguments)) => {
            Ok(forked_repo_commands::setup(&arguments.arguments))
        }
        Domain::Debate(command) => Ok(match command {
            DebateCommand::Init(arguments) => debate_commands::init(&arguments.arguments),
            DebateCommand::RoundPrep(arguments) => {
                debate_commands::round_prep(&arguments.arguments)
            }
            DebateCommand::RecordTurn(arguments) => {
                debate_commands::record_turn(&arguments.arguments)
            }
            DebateCommand::RoundExternal(arguments) => {
                debate_commands::round_external(&arguments.arguments)
            }
            DebateCommand::RoundIngest(arguments) => {
                debate_commands::round_ingest(&arguments.arguments)
            }
            DebateCommand::Abort(arguments) => debate_commands::abort(&arguments.arguments),
            DebateCommand::AdjudicationPreview(arguments) => {
                debate_commands::adjudication_preview(&arguments.arguments)
            }
            DebateCommand::Adjudicate(arguments) => {
                debate_commands::adjudicate(&arguments.arguments)
            }
            DebateCommand::Synthesize(arguments) => {
                debate_commands::synthesize(&arguments.arguments)
            }
            DebateCommand::PublishPrepare(arguments) => {
                debate_commands::publish_prepare(&arguments.arguments)
            }
            DebateCommand::IssuePrepare(arguments) => {
                debate_publication_commands::issue_prepare(&arguments.arguments)
            }
            DebateCommand::TitleTransition(arguments) => {
                debate_publication_commands::title_transition(&arguments.arguments)
            }
            DebateCommand::ProposalLink(arguments) => {
                debate_publication_commands::proposal_link(&arguments.arguments)
            }
            DebateCommand::CommentVerify(arguments) => {
                debate_publication_commands::comment_verify(&arguments.arguments)
            }
            DebateCommand::PublishRun(arguments) => {
                debate_commands::publish_run(&arguments.arguments)
            }
            DebateCommand::PublishFinish(arguments) => {
                debate_commands::publish_finish(&arguments.arguments)
            }
            DebateCommand::AbortRun(arguments) => debate_commands::abort_run(&arguments.arguments),
            DebateCommand::InitRun(arguments) => debate_commands::init_run(&arguments.arguments),
        }),
        Domain::Deps(command) => Ok(match command {
            DepsCommand::ResolveRepo(arguments) => {
                deps_audit_commands::resolve_repo(&arguments.arguments)
            }
            DepsCommand::Fetch(arguments) => deps_audit_commands::fetch(&arguments.arguments),
            DepsCommand::ExplicitRefs(arguments) => {
                deps_audit_commands::explicit_refs(&arguments.arguments)
            }
            DepsCommand::WriteProposals(arguments) => {
                deps_audit_commands::write_proposals(&arguments.arguments)
            }
            DepsCommand::Plan(arguments) => deps_audit_commands::plan(&arguments.arguments),
            DepsCommand::Apply(arguments) => deps_audit_commands::apply(&arguments.arguments),
        }),
        Domain::CombineIssues(command) => Ok(match command {
            CombineIssuesCommand::Fetch(arguments) => {
                combine_issues_commands::dispatch("fetch", &arguments.arguments)
            }
            CombineIssuesCommand::FetchDeps(arguments) => {
                combine_issues_commands::dispatch("fetch-deps", &arguments.arguments)
            }
            CombineIssuesCommand::ListOpen(arguments) => {
                combine_issues_commands::dispatch("list-open", &arguments.arguments)
            }
            CombineIssuesCommand::CloseEligible(arguments) => {
                combine_issues_commands::dispatch("close-eligible", &arguments.arguments)
            }
            CombineIssuesCommand::PlanInherited(arguments) => {
                combine_issues_commands::dispatch("plan-inherited", &arguments.arguments)
            }
            CombineIssuesCommand::ProseAudit(arguments) => {
                combine_issues_commands::dispatch("prose-audit", &arguments.arguments)
            }
            CombineIssuesCommand::PlanAudit(arguments) => {
                combine_issues_commands::dispatch("plan-audit", &arguments.arguments)
            }
            CombineIssuesCommand::Apply(arguments) => {
                combine_issues_commands::dispatch("apply", &arguments.arguments)
            }
            CombineIssuesCommand::CloseSources(arguments) => {
                combine_issues_commands::dispatch("close-sources", &arguments.arguments)
            }
            CombineIssuesCommand::CloseStale(arguments) => {
                combine_issues_commands::dispatch("close-stale", &arguments.arguments)
            }
        }),
        Domain::ExecutionIssues(command) => Ok(match command {
            ExecutionIssuesCommand::Append(arguments) => {
                execution_issue_commands::append(&arguments.arguments)
            }
            ExecutionIssuesCommand::Flush(arguments) => {
                execution_issue_commands::flush(&arguments.arguments)
            }
            ExecutionIssuesCommand::FlushSafetyNet(arguments) => {
                execution_issue_commands::flush_safety_net(&arguments.arguments)
            }
            ExecutionIssuesCommand::Refresh(arguments) => {
                execution_issue_commands::refresh(&arguments.arguments)
            }
        }),
        Domain::ExternalDefaults(command) => Ok(external_defaults_commands::run(command)),
        Domain::Example(ExampleCommand::Echo(arguments)) => {
            println!("{}", larch_core::example::echo(&arguments.message));
            Ok(ExitCode::SUCCESS)
        }
        Domain::Admission(AdmissionCommand::ForkEnv(arguments)) => {
            Ok(admission_commands::fork_env(&arguments.arguments))
        }
        Domain::Admission(AdmissionCommand::Gate(arguments)) => {
            Ok(admission_commands::gate(&arguments.arguments))
        }
        Domain::Admission(AdmissionCommand::Preflight(arguments)) => {
            Ok(admission_commands::preflight(&arguments.arguments))
        }
        Domain::Checks(ChecksCommand::RustClippy(arguments)) => Ok(
            checks_rust_clippy_commands::rust_clippy(&arguments.arguments),
        ),
        Domain::Checks(ChecksCommand::SelfEditLog(arguments)) => Ok(
            checks_identity_commands::self_edit_log(&arguments.arguments),
        ),
        Domain::Checks(ChecksCommand::RunRelevant(arguments)) => Ok(
            checks_run_relevant_commands::checks_run_relevant(&arguments.arguments),
        ),
        Domain::Checks(ChecksCommand::FixerEvidence(arguments)) => Ok(
            checks_lint_fix_commands::checks_fixer_evidence(&arguments.arguments),
        ),
        Domain::Checks(ChecksCommand::LintFix(arguments)) => Ok(
            checks_lint_fix_commands::checks_lint_fix(&arguments.arguments),
        ),
        Domain::Checks(ChecksCommand::RepairLoop(arguments)) => Ok(
            checks_lint_fix_commands::checks_repair_loop(&arguments.arguments),
        ),
        Domain::Checks(ChecksCommand::ContainsPins(arguments)) => Ok(
            checks_run_relevant_commands::check_contains_pins(&arguments.arguments),
        ),
        Domain::Implement(command) => Ok(match command {
            ImplementCommand::Cleanup(arguments) => {
                implement_finalize_commands::cleanup(&arguments.arguments)
            }
            ImplementCommand::CodeFlowDiagram(arguments) => {
                diagram_commands::implement_code_flow_diagram(&arguments.arguments)
            }
            ImplementCommand::ChecksResultIdentity(arguments) => {
                checks_identity_commands::checks_result_identity(&arguments.arguments)
            }
            ImplementCommand::ChecksCommitRoute(arguments) => {
                implement_commit_route_commands::checks_commit_route(&arguments.arguments)
            }
            ImplementCommand::Commit(arguments) => {
                implement_commit_route_commands::commit(&arguments.arguments)
            }
            ImplementCommand::CommitRoute(arguments) => {
                implement_commit_route_commands::commit_route(&arguments.arguments)
            }
            ImplementCommand::ChecksStep5Resume(arguments) => {
                implement_review_commands::checks_step5_resume(&arguments.arguments)
            }
            ImplementCommand::CloneTag(arguments) => {
                implement_commands::clone_tag(&arguments.arguments)
            }
            ImplementCommand::KillActiveLeg(arguments) => {
                implement_leg_commands::kill_active_leg(&arguments.arguments)
            }
            ImplementCommand::NormalizeCoderScout(arguments) => {
                implement_commands::normalize_coder_scout(&arguments.arguments)
            }
            ImplementCommand::Preflight(arguments) => {
                implement_preflight_commands::preflight(&arguments.arguments)
            }
            ImplementCommand::RecoveryPaths(arguments) => {
                implement_dispatch_commands::recovery_paths(&arguments.arguments)
            }
            ImplementCommand::RunDispatch(arguments) => {
                implement_step2_commands::run_dispatch(&arguments.arguments)
            }
            ImplementCommand::RunStepChecks(arguments) => {
                implement_dispatch_commands::run_step_checks(&arguments.arguments)
            }
            ImplementCommand::ScopeDisposition(arguments) => {
                implement_scope_disposition_commands::scope_disposition(&arguments.arguments)
            }
            ImplementCommand::Step2Dispatch(arguments) => {
                implement_step2_commands::step2_dispatch(&arguments.arguments)
            }
            ImplementCommand::Step2PostDispatch(arguments) => {
                implement_step2_post_commands::step_2_post_dispatch(&arguments.arguments)
            }
            ImplementCommand::Step0Bootstrap(arguments) => {
                implement_commands::step0_bootstrap(&arguments.arguments)
            }
            ImplementCommand::Step0DegradedGate(arguments) => {
                implement_commands::step0_degraded_gate(&arguments.arguments)
            }
            ImplementCommand::Step5Resume(arguments) => {
                implement_review_commands::step5_resume(&arguments.arguments)
            }
            ImplementCommand::Step5Review(arguments) => {
                implement_review_commands::step5_review(&arguments.arguments)
            }
            ImplementCommand::Step6Entry(arguments) => {
                implement_review_commands::step6_entry(&arguments.arguments)
            }
            ImplementCommand::Step7a(arguments) => {
                implement_review_commands::step7a(&arguments.arguments)
            }
            ImplementCommand::Step8SeedInitial(arguments) => {
                implement_ship_commands::step8_seed_initial(&arguments.arguments)
            }
            ImplementCommand::Step8Ship(arguments) => {
                implement_ship_commands::step8_ship(&arguments.arguments)
            }
            ImplementCommand::Step8OosCheckpoint(arguments) => {
                implement_ship_commands::step8_oos_checkpoint(&arguments.arguments)
            }
            ImplementCommand::Step16(arguments) => {
                implement_closeout_commands::step_16(&arguments.arguments)
            }
            ImplementCommand::Step16_16a(arguments) => {
                implement_closeout_commands::step_16_16a(&arguments.arguments)
            }
            ImplementCommand::Step16_17(arguments) => {
                implement_closeout_commands::step_16_17(&arguments.arguments)
            }
            ImplementCommand::Step17(arguments) => {
                implement_closeout_commands::step_17(&arguments.arguments)
            }
            ImplementCommand::Step18(arguments) => {
                implement_terminal_commands::step_18(&arguments.arguments)
            }
            ImplementCommand::Step18GateLogsFlush(arguments) => {
                implement_terminal_commands::step_18_gate_logs_flush(&arguments.arguments)
            }
            ImplementCommand::Step19(arguments) => {
                implement_terminal_commands::step_19(&arguments.arguments)
            }
        }),
        Domain::ImplementFinalize(command) => Ok(match command {
            ImplementFinalizeCommand::Postbump(arguments) => implement_finalize_commands::command(
                implement_finalize_commands::FinalizePhase::Postbump,
                &arguments.arguments,
            ),
            ImplementFinalizeCommand::Postmerge(arguments) => implement_finalize_commands::command(
                implement_finalize_commands::FinalizePhase::Postmerge,
                &arguments.arguments,
            ),
            ImplementFinalizeCommand::Teardown(arguments) => implement_finalize_commands::command(
                implement_finalize_commands::FinalizePhase::Teardown,
                &arguments.arguments,
            ),
        }),
        Domain::Ship(command) => Ok(match command {
            ShipCommand::Pr(arguments) => ship_pr_commands::pr(&arguments.arguments),
            ShipCommand::ReconcileManualMerge(arguments) => {
                ship_recovery_commands::reconcile_manual_merge(&arguments.arguments)
            }
            ShipCommand::NormalizeAssessmentHandoff(arguments) => {
                ship_pre_driver_commands::normalize_assessment_handoff(&arguments.arguments)
            }
            ShipCommand::PreDriver(arguments) => {
                ship_pre_driver_commands::pre_driver(&arguments.arguments)
            }
            ShipCommand::PreFixRebase(arguments) => {
                ship_pre_driver_commands::pre_fix_rebase(&arguments.arguments)
            }
            ShipCommand::RouteExit(arguments) => {
                ship_pre_driver_commands::route_exit(&arguments.arguments)
            }
            ShipCommand::GovernanceRefresh(arguments) => {
                ship_pre_driver_commands::governance_refresh(&arguments.arguments)
            }
            ShipCommand::SeedInitialState(arguments) => {
                ship_commands::seed_initial_state(&arguments.arguments)
            }
            ShipCommand::WriteResultEnv(arguments) => {
                ship_commands::write_result_env(&arguments.arguments)
            }
        }),
        Domain::Blocker(BlockerCommand::AllOpen(arguments)) => {
            Ok(blocker_commands::all_open(&arguments.arguments))
        }
        Domain::BlockIssue(BlockIssueCommand::AddBlockedBy(arguments)) => Ok(
            issue_dependency_commands::block_issue_add(&arguments.arguments),
        ),
        Domain::BlockIssue(BlockIssueCommand::RemoveBlockedBy(arguments)) => Ok(
            issue_dependency_commands::block_issue_remove(&arguments.arguments),
        ),
        Domain::Git(command) => run_git(command).map_err(command_failure),
        Domain::Merge(MergeCommand::Pr(arguments)) => Ok(merge_commands::pr(&arguments.arguments)),
        Domain::Merge(MergeCommand::Wait(arguments)) => {
            Ok(merge_commands::wait(&arguments.arguments))
        }
        Domain::Hook(HookCommand::AntiReadPoll(arguments)) => {
            Ok(hook_commands::anti_read_poll(&arguments.arguments))
        }
        Domain::Hook(HookCommand::AuditEditWrite(arguments)) => {
            Ok(hook_commands::audit_edit_write(&arguments.arguments))
        }
        Domain::Hook(HookCommand::BlockSubmoduleEdit(arguments)) => {
            Ok(hook_commands::block_submodule_edit(&arguments.arguments))
        }
        Domain::Hook(HookCommand::CleanupSessionstart(arguments)) => {
            Ok(hook_commands::cleanup_sessionstart(&arguments.arguments))
        }
        Domain::Hook(HookCommand::DenyEditWrite(arguments)) => {
            Ok(hook_commands::deny_edit_write(&arguments.arguments))
        }
        Domain::Hook(HookCommand::DenyRunInBackground(arguments)) => {
            Ok(hook_commands::deny_run_in_background(&arguments.arguments))
        }
        Domain::Hook(HookCommand::SessionstartHealth(arguments)) => {
            Ok(hook_commands::sessionstart_health(&arguments.arguments))
        }
        Domain::Hook(HookCommand::SessionstartStatusline(arguments)) => {
            Ok(hook_commands::sessionstart_statusline(&arguments.arguments))
        }
        Domain::Hook(HookCommand::StopFailClose(arguments)) => {
            Ok(hook_commands::stop_fail_close(&arguments.arguments))
        }
        Domain::Issue(command) => Ok(match command {
            IssueCommand::AddBlockedBy(arguments) => {
                issue_dependency_commands::add_blocked_by(&arguments.arguments)
            }
            IssueCommand::AddSubIssue(arguments) => {
                issue_dependency_commands::add_sub_issue(&arguments.arguments)
            }
            IssueCommand::AllocateCandidates(arguments) => {
                issue_input_commands::allocate_candidates(&arguments.arguments)
            }
            IssueCommand::CleanupFailed(arguments) => {
                issue_create_commands::cleanup_failed(&arguments.arguments)
            }
            IssueCommand::CreateBatch(arguments) => {
                issue_batch_create_commands::create_batch(&arguments.arguments)
            }
            IssueCommand::Context(arguments) => issue_commands::context(&arguments.arguments),
            IssueCommand::CreateOne(arguments) => {
                issue_create_commands::create_one(&arguments.arguments)
            }
            IssueCommand::FetchIssueDetails(arguments) => {
                issue_input_commands::fetch_issue_details(&arguments.arguments)
            }
            IssueCommand::Info(arguments) => issue_commands::info(&arguments.arguments),
            IssueCommand::MigrationAudit(arguments) => {
                migration_audit_commands::run(&arguments.arguments)
            }
            IssueCommand::GovernanceGate(arguments) => {
                migration_governance_commands::governance_gate(&arguments.arguments)
            }
            IssueCommand::InsertSignalMarker(arguments) => {
                issue_wire_commands::insert_signal_marker_command(&arguments.arguments)
            }
            IssueCommand::TitleArchivalJq(arguments) => {
                issue_wire_commands::title_archival_jq(&arguments.arguments)
            }
            IssueCommand::TitleEligibility(arguments) => {
                issue_wire_commands::title_eligibility(&arguments.arguments)
            }
            IssueCommand::ListIssues(arguments) => {
                issue_input_commands::list_issues(&arguments.arguments)
            }
            IssueCommand::ParseInput(arguments) => {
                issue_input_commands::parse_input(&arguments.arguments)
            }
            IssueCommand::SearchImplementing(arguments) => {
                issue_commands::search_implementing(&arguments.arguments)
            }
            IssueCommand::State(arguments) => issue_commands::state(&arguments.arguments),
            IssueCommand::WriteSentinel(arguments) => {
                issue_create_commands::write_sentinel(&arguments.arguments)
            }
        }),
        Domain::PlanBlock(command) => Ok(match command {
            PlanBlockCommand::Read(arguments) => {
                issue_wire_commands::plan_block_read(&arguments.arguments)
            }
            PlanBlockCommand::StripBody(arguments) => {
                issue_wire_commands::plan_block_strip_body(&arguments.arguments)
            }
            PlanBlockCommand::Write(arguments) => {
                issue_wire_commands::plan_block_write(&arguments.arguments)
            }
        }),
        Domain::PlanReceipt(PlanReceiptCommand::Refresh(arguments)) => Ok(
            migration_governance_commands::plan_receipt_refresh(&arguments.arguments),
        ),
        Domain::NamedBlock(NamedBlockCommand::Write(arguments)) => {
            Ok(issue_wire_commands::named_block_write(&arguments.arguments))
        }
        Domain::Decompose(command) => Ok(match command {
            DecomposeCommand::Prepare(arguments) => {
                decompose_commands::prepare_main(&arguments.arguments)
            }
            DecomposeCommand::Annotate(arguments) => {
                decompose_commands::annotate_main(&arguments.arguments)
            }
            DecomposeCommand::MigrateDeps(arguments) => {
                decompose_commands::migrate_deps_main(&arguments.arguments)
            }
            DecomposeCommand::CloseOriginal(arguments) => {
                decompose_commands::close_original_main(&arguments.arguments)
            }
            DecomposeCommand::PanelDispatch(arguments) => {
                decompose_commands::panel_dispatch_main(&arguments.arguments)
            }
            DecomposeCommand::Aggregate(arguments) => {
                decompose_commands::aggregate_main(&arguments.arguments)
            }
        }),
        Domain::Design(command) => Ok(command.run()),
        Domain::Scout(command) => Ok(match command {
            ScoutCommand::DynamicArchetypes(arguments) => {
                scout_commands::dynamic_archetypes(&arguments.arguments)
            }
            ScoutCommand::PlanArchetypes(arguments) => {
                scout_commands::plan_archetypes(&arguments.arguments)
            }
            ScoutCommand::FilterManifest(arguments) => {
                scout_commands::filter_manifest(&arguments.arguments)
            }
        }),
        Domain::Plan(command) => Ok(match command {
            PlanCommand::ScopePaths(arguments) => {
                issue_wire_commands::scope_paths(&arguments.arguments)
            }
            PlanCommand::ParseCommands(arguments) => {
                plan_quality_commands::parse_commands(&arguments.arguments)
            }
            PlanCommand::ValidateCommands(arguments) => {
                plan_quality_commands::validate_commands(&arguments.arguments)
            }
            PlanCommand::Validate(arguments) => {
                plan_quality_commands::validate(&arguments.arguments)
            }
            PlanCommand::CheckSize(arguments) => {
                plan_quality_commands::check_size(&arguments.arguments)
            }
            PlanCommand::SetOversizeOverride(arguments) => {
                plan_quality_commands::set_oversize_override(&arguments.arguments)
            }
            PlanCommand::ReviseWaterfall(arguments) => {
                plan_quality_revise_commands::revise_waterfall(&arguments.arguments)
            }
            PlanCommand::AutoFixCommands(arguments) => {
                plan_quality_revise_commands::auto_fix_commands(&arguments.arguments)
            }
            PlanCommand::ValidatorAutofix(arguments) => {
                plan_quality_revise_commands::validator_autofix(&arguments.arguments)
            }
            PlanCommand::OptionalTrailers(arguments) => {
                plan_quality_commands::optional_trailers(&arguments.arguments)
            }
            PlanCommand::ComposeGoalsTest(arguments) => {
                plan_quality_commands::compose_goals_test(&arguments.arguments)
            }
            PlanCommand::Step1Log(arguments) => {
                design_step1_commands::step1_log(&arguments.arguments)
            }
        }),
        Domain::Pr(command) => Ok(match command {
            PrCommand::CreateBranch(arguments) => pr_commands::create_branch(&arguments.arguments),
            PrCommand::Create(arguments) => pr_commands::create(&arguments.arguments),
            PrCommand::BodyUpdate(arguments) => pr_commands::body_update(&arguments.arguments),
            PrCommand::Checks(arguments) => pr_commands::checks(&arguments.arguments),
            PrCommand::ClosesIssue(arguments) => pr_commands::closes_issue(&arguments.arguments),
            PrCommand::ComposeSummary(arguments) => {
                pr_commands::compose_summary(&arguments.arguments)
            }
        }),
        Domain::TrackingIssue(command) => Ok(match command {
            TrackingIssueCommand::Read(arguments) => {
                tracking_issue_commands::read(&arguments.arguments)
            }
            TrackingIssueCommand::CreateIssue(arguments) => {
                tracking_issue_commands::create_issue(&arguments.arguments)
            }
            TrackingIssueCommand::AppendComment(arguments) => {
                tracking_issue_commands::append_comment(&arguments.arguments)
            }
            TrackingIssueCommand::Rename(arguments) => {
                tracking_issue_commands::rename(&arguments.arguments)
            }
            TrackingIssueCommand::MarkFalsePositive(arguments) => {
                tracking_issue_commands::mark_false_positive(&arguments.arguments)
            }
            TrackingIssueCommand::UpsertSummary(arguments) => {
                tracking_issue_commands::upsert_summary(&arguments.arguments)
            }
        }),
        Domain::Tracking(TrackingCommand::PostIssue(arguments)) => {
            Ok(tracking_issue_commands::post_issue(&arguments.arguments))
        }
        Domain::Triage(command) => Ok(match command {
            TriageCommand::Inspect(arguments) => triage_commands::inspect(&arguments.arguments),
            TriageCommand::Probe(arguments) => triage_commands::probe(&arguments.arguments),
            TriageCommand::Apply(arguments) => triage_commands::apply(&arguments.arguments),
        }),
        Domain::Umbrella(command) => Ok(match command {
            UmbrellaCommand::Prepare(arguments) => umbrella_commands::prepare(&arguments.arguments),
            UmbrellaCommand::PersistProposal(arguments) => {
                umbrella_commands::persist_proposal(&arguments.arguments)
            }
            UmbrellaCommand::MarkInFlight(arguments) => {
                umbrella_commands::mark_in_flight(&arguments.arguments)
            }
            UmbrellaCommand::RecordResolved(arguments) => {
                umbrella_commands::record_resolved(&arguments.arguments)
            }
            UmbrellaCommand::ReconcileInFlight(arguments) => {
                umbrella_commands::reconcile_in_flight_command(&arguments.arguments)
            }
            UmbrellaCommand::Mutate(arguments) => umbrella_commands::mutate(&arguments.arguments),
            UmbrellaCommand::Verify(arguments) => umbrella_commands::verify(&arguments.arguments),
            UmbrellaCommand::VerifyCompletion(arguments) => {
                umbrella_commands::verify_completion(&arguments.arguments)
            }
        }),
        Domain::Untrusted(command) => Ok(match command {
            UntrustedCommand::ContentBlock(arguments) => {
                issue_wire_commands::untrusted_content_block(&arguments.arguments)
            }
            UntrustedCommand::FileBlock(arguments) => {
                issue_wire_commands::untrusted_file_block(&arguments.arguments)
            }
            UntrustedCommand::RedactStream(arguments) => {
                issue_wire_commands::untrusted_redact_stream(&arguments.arguments)
            }
            UntrustedCommand::XmlEscapeAttr(arguments) => {
                issue_wire_commands::untrusted_xml_escape_attr(&arguments.arguments)
            }
        }),
        Domain::Kv(KvCommand::Get(arguments)) => Ok(state_commands::kv_get(&arguments.arguments)),
        Domain::Lint(arguments) => match arguments.into_dispatch() {
            larch_lint::LintDispatch::Gitleaks(arguments) => Ok(gitleaks::run(&arguments)),
            larch_lint::LintDispatch::Native(arguments) => {
                Ok(ExitCode::from(larch_lint::run_cli(arguments).as_u8()))
            }
        },
        Domain::Plugin(PluginCommand::ReadVersion(arguments)) => {
            Ok(release_prepare::read_plugin_version(&arguments.args))
        }
        Domain::Plugin(PluginCommand::ResolveRepository(arguments)) => {
            Ok(release_prepare::resolve_plugin_repository(&arguments.args))
        }
        Domain::ObjectStore(ObjectStoreCommand::Gcs(arguments)) => {
            Ok(object_store_commands::run(&arguments))
        }
        Domain::Generate(arguments) => Ok(rendering_commands::generate(&arguments.arguments)),
        Domain::Gantt(GanttCommand::Render(arguments)) => {
            Ok(rendering_commands::gantt_render(&arguments.arguments))
        }
        Domain::Diagram(DiagramCommand::CodeFlow(arguments)) => {
            Ok(diagram_commands::code_flow(&arguments.arguments))
        }
        Domain::Render(command) => Ok(dispatch_render(command)),
        Domain::ScopeAnchor(command) => Ok(dispatch_scope_anchor(command)),
        Domain::Mermaid(MermaidCommand::Sanitize(arguments)) => {
            Ok(diagram_commands::mermaid_sanitize(&arguments.arguments))
        }
        Domain::Diagrams(DiagramsCommand::Upsert(arguments)) => {
            Ok(diagram_commands::diagrams_upsert(&arguments.arguments))
        }
        Domain::AnalyzeIssues(command) => Ok(match command {
            AnalyzeIssuesCommand::Fetch(arguments) => {
                analyze_issues_commands::fetch(&arguments.arguments)
            }
            AnalyzeIssuesCommand::Analyze(arguments) => {
                analyze_issues_commands::analyze(&arguments.arguments)
            }
            AnalyzeIssuesCommand::Run(arguments) => {
                analyze_issues_commands::run(&arguments.arguments)
            }
            AnalyzeIssuesCommand::RenderChart(arguments) => {
                rendering_commands::render_chart(&arguments.arguments)
            }
        }),
        Domain::AuditRuns(command) => Ok(match command {
            AuditRunsCommand::Preflight(arguments) => {
                audit_runs_commands::preflight(&arguments.arguments)
            }
            AuditRunsCommand::ResolvePrs(arguments) => {
                audit_runs_commands::resolve_prs(&arguments.arguments)
            }
            AuditRunsCommand::MapRuns(arguments) => {
                audit_runs_commands::map_runs(&arguments.arguments)
            }
            AuditRunsCommand::ScanRun(arguments) => {
                audit_runs_commands::scan_run(&arguments.arguments)
            }
            AuditRunsCommand::ComputeCounters(arguments) => {
                audit_runs_commands::compute_counters(&arguments.arguments)
            }
            AuditRunsCommand::PacificTimestamp(arguments) => {
                audit_runs_commands::pacific_timestamp(&arguments.arguments)
            }
            AuditRunsCommand::Title(arguments) => audit_runs_commands::title(&arguments.arguments),
            AuditRunsCommand::TitleMatch(arguments) => {
                audit_runs_commands::title_match(&arguments.arguments)
            }
            AuditRunsCommand::BugsBacklogNudge(arguments) => {
                audit_runs_commands::bugs_backlog_nudge(&arguments.arguments)
            }
            AuditRunsCommand::ClosePriors(arguments) => {
                audit_runs_commands::close_priors(&arguments.arguments)
            }
            AuditRunsCommand::IssueSearch(arguments) => {
                audit_runs_commands::issue_search(&arguments.arguments)
            }
            AuditRunsCommand::FixMerge(arguments) => {
                audit_runs_commands::fix_merge(&arguments.arguments)
            }
            AuditRunsCommand::VersionWindow(arguments) => {
                audit_runs_commands::version_window(&arguments.arguments)
            }
            AuditRunsCommand::LabelCheck(arguments) => {
                audit_runs_commands::label_check(&arguments.arguments)
            }
            AuditRunsCommand::Comment(arguments) => {
                audit_runs_commands::comment(&arguments.arguments)
            }
        }),
        Domain::AnalyzeBugs(command) => Ok(match command {
            AnalyzeBugsCommand::Prefetch(arguments) => {
                analyze_bugs_commands::prefetch(&arguments.arguments)
            }
            AnalyzeBugsCommand::Ledger(arguments) => {
                analyze_bugs_commands::ledger(&arguments.arguments)
            }
            AnalyzeBugsCommand::Runtime(arguments) => {
                analyze_bugs_commands::runtime(&arguments.arguments)
            }
            AnalyzeBugsCommand::Report(arguments) => {
                analyze_bugs_commands::report(&arguments.arguments)
            }
        }),
        Domain::RejectedAnalysis(command) => Ok(match command {
            RejectedAnalysisCommand::Prepare(arguments) => {
                rejected_analysis_commands::prepare(&arguments.arguments)
            }
            RejectedAnalysisCommand::IngestVerdict(arguments) => {
                rejected_analysis_commands::ingest_verdict(&arguments.arguments)
            }
            RejectedAnalysisCommand::Finalize(arguments) => {
                rejected_analysis_commands::finalize(&arguments.arguments)
            }
            RejectedAnalysisCommand::Record(arguments) => {
                rejected_analysis_commands::record(&arguments.arguments)
            }
        }),
        Domain::LearnFromBugs(command) => Ok(match command {
            LearnFromBugsCommand::CheckProposals(arguments) => {
                learn_from_bugs_commands::check_proposals(&arguments.arguments)
            }
            LearnFromBugsCommand::Prepare(arguments) => {
                learn_from_bugs_commands::prepare(&arguments.arguments)
            }
            LearnFromBugsCommand::CoverageIndex(arguments) => {
                learn_from_bugs_commands::coverage_index_command(&arguments.arguments)
            }
            LearnFromBugsCommand::ReadState(arguments) => {
                learn_from_bugs_commands::read_state(&arguments.arguments)
            }
            LearnFromBugsCommand::WriteState(arguments) => {
                learn_from_bugs_commands::write_state(&arguments.arguments)
            }
            LearnFromBugsCommand::ResolveZones(arguments) => {
                learn_from_bugs_commands::resolve_zones(&arguments.arguments)
            }
            LearnFromBugsCommand::VerifyOrigin(arguments) => {
                learn_from_bugs_commands::verify_origin(&arguments.arguments)
            }
            LearnFromBugsCommand::ValidateReport(arguments) => {
                learn_from_bugs_commands::validate_report(&arguments.arguments)
            }
            LearnFromBugsCommand::StatePublish(arguments) => {
                learn_from_bugs_commands::state_publish(&arguments.arguments)
            }
            LearnFromBugsCommand::FilingDeps(arguments) => {
                learn_from_bugs_commands::filing_deps(&arguments.arguments)
            }
        }),
        Domain::ValidateMerged(command) => Ok(match command {
            ValidateMergedCommand::Prepare(arguments) => {
                validate_merged_commands::prepare(&arguments.arguments)
            }
            ValidateMergedCommand::IngestFinder(arguments) => {
                validate_merged_commands::ingest_finder(&arguments.arguments)
            }
            ValidateMergedCommand::IngestRefuter(arguments) => {
                validate_merged_commands::ingest_refuter(&arguments.arguments)
            }
            ValidateMergedCommand::Report(arguments) => {
                validate_merged_commands::report(&arguments.arguments)
            }
            ValidateMergedCommand::WriteState(arguments) => {
                validate_merged_commands::write_state(&arguments.arguments)
            }
        }),
        Domain::Progress(command) => Ok(match command {
            ProgressCommand::Activate(arguments) => {
                progress_commands::activate(&arguments.arguments)
            }
            ProgressCommand::Cleanup(arguments) => progress_commands::cleanup(&arguments.arguments),
            ProgressCommand::Deactivate(arguments) => {
                progress_commands::deactivate(&arguments.arguments)
            }
            ProgressCommand::Clear(arguments) => progress_commands::clear(&arguments.arguments),
            ProgressCommand::Note(arguments) => progress_commands::note(&arguments.arguments),
            ProgressCommand::Statusline(arguments) => {
                progress_commands::render_statusline(&arguments.arguments)
            }
            ProgressCommand::SessionReset(arguments) => {
                progress_commands::session_reset(&arguments.arguments)
            }
            ProgressCommand::InstallStatusline(arguments) => {
                progress_commands::install_statusline(&arguments.arguments)
            }
            ProgressCommand::RenderPhaseDetail(arguments) => {
                progress_commands::render_phase_detail(&arguments.arguments)
            }
            ProgressCommand::WriteDesignRoundMeta(arguments) => {
                progress_commands::write_design_round_meta(&arguments.arguments)
            }
            ProgressCommand::WriteImplementRoundMeta(arguments) => {
                progress_commands::write_implement_round_meta(&arguments.arguments)
            }
        }),
        Domain::Release(command) => run_release(command),
        Domain::Repo(command) => Ok(repo_size_commands::run(command)),
        Domain::ResidualBash(command) => Ok(developer_tooling_commands::run_residual_bash(command)),
        Domain::FinalReport(command) => Ok(match command {
            FinalReportCommand::Write(arguments) => {
                final_report_commands::write(&arguments.arguments)
            }
            FinalReportCommand::Step18b(arguments) => {
                final_report_commands::step18b(&arguments.arguments)
            }
        }),
        Domain::Research(command) => Ok(command.run()),
        Domain::Eval(command) => Ok(command.run()),
        Domain::ReportTokens(command) => Ok(match command {
            ReportTokensCommand::Analyze(arguments) => {
                report_tokens_commands::analyze(&arguments.arguments)
            }
        }),
        Domain::RebalanceTests(command) => Ok(rebalance_tests::run(&command)),
        Domain::Session(command) => Ok(run_session(command)),
        Domain::Slack(command) => Ok(slack_commands::run(command)),
        Domain::StallRecovery(arguments) => Ok(stall_recovery_commands::run(&arguments.arguments)),
        Domain::TestShard(command) => Ok(test_shards::run(command)),
        Domain::Verify(command) => Ok(developer_tooling_commands::run_verify(command)),
        Domain::Timing(command) => Ok(match command {
            TimingCommand::Mark(arguments) => timing_commands::mark(&arguments.arguments),
            TimingCommand::RecordVendorTask(arguments) => {
                timing_commands::record_vendor_task(&arguments.arguments)
            }
            TimingCommand::RecordRound(arguments) => {
                timing_commands::record_round(&arguments.arguments)
            }
            TimingCommand::Dump(arguments) => timing_commands::dump(&arguments.arguments),
            TimingCommand::Report(arguments) => timing_commands::report(&arguments.arguments),
            TimingCommand::HarnessMark(arguments) => {
                timing_commands::harness_mark(&arguments.arguments)
            }
            TimingCommand::TelemetryMark(arguments) => {
                timing_commands::telemetry_mark(&arguments.arguments)
            }
            TimingCommand::TaskKinds(arguments) => {
                timing_commands::task_kinds(&arguments.arguments)
            }
        }),
        Domain::Token(command) => Ok(match command {
            TokenCommand::Mark(arguments) => token_commands::mark(&arguments.arguments),
            TokenCommand::CheckBudget(arguments) => {
                token_commands::check_budget(&arguments.arguments)
            }
            TokenCommand::ComputePrLineCounts(arguments) => {
                token_commands::compute_pr_line_counts(&arguments.arguments)
            }
            TokenCommand::ComputePrLines(arguments) => {
                token_commands::compute_pr_lines(&arguments.arguments)
            }
            TokenCommand::ClaudeSource(arguments) => {
                token_commands::claude_source(&arguments.arguments)
            }
            TokenCommand::RecordVendor(arguments) => {
                token_commands::record_vendor(&arguments.arguments)
            }
            TokenCommand::RecordVendorSidecar(arguments) => {
                token_commands::record_vendor_sidecar(&arguments.arguments)
            }
            TokenCommand::AppendRecord(arguments) => {
                token_commands::append_record(&arguments.arguments)
            }
            TokenCommand::Dump(arguments) => token_commands::dump(&arguments.arguments),
            TokenCommand::Report(arguments) => token_commands::report(&arguments.arguments),
            TokenCommand::Cost(arguments) => token_commands::cost(&arguments.arguments),
            TokenCommand::RenderCostLine(arguments) => {
                token_commands::render_cost_line_command(&arguments.arguments)
            }
            TokenCommand::LaneWrite(arguments) => token_commands::lane_write(&arguments.arguments),
            TokenCommand::LaneReport(arguments) => {
                token_commands::lane_report(&arguments.arguments)
            }
            TokenCommand::MeasureMdCost(arguments) => {
                token_measurement_commands::measure_md_cost(&arguments.arguments)
            }
            TokenCommand::MeasureCacheEfficiency(arguments) => {
                token_measurement_commands::measure_cache_efficiency(&arguments.arguments)
            }
            TokenCommand::MeasureChecksDigestSavings(arguments) => {
                token_measurement_commands::measure_checks_digest_savings(&arguments.arguments)
            }
            TokenCommand::MeasureNgramDuplication(arguments) => {
                token_measurement_commands::measure_ngram_duplication(&arguments.arguments)
            }
            TokenCommand::MeasurePanelCost(arguments) => {
                token_measurement_commands::measure_panel_cost(&arguments.arguments)
            }
            TokenCommand::MeasureRealizedCost(arguments) => {
                token_measurement_commands::measure_realized_cost(&arguments.arguments)
            }
            TokenCommand::MeasureReferencesHeatmap(arguments) => {
                token_measurement_commands::measure_references_heatmap(&arguments.arguments)
            }
        }),
        Domain::Gh(GhCommand::WorkflowPath) => {
            print!("{}", larch_core::workflow_path());
            Ok(ExitCode::SUCCESS)
        }
        Domain::Gh(GhCommand::RemoteRepo(arguments)) => Ok(run_remote_repo(&arguments)),
        Domain::Gh(GhCommand::ResolveRepo(arguments)) => Ok(run_resolve_repo(&arguments)),
        Domain::Gh(GhCommand::RunLogs(arguments)) => Ok(run_logs(&arguments)),
        Domain::Gh(GhCommand::AgnixIssue(arguments)) => Ok(
            github_repository_resolution::agnix_issue(&arguments.repository, arguments.issue),
        ),
        Domain::Gh(GhCommand::AgnixEnsureLabel(arguments)) => Ok(
            github_repository_resolution::agnix_ensure_label(&arguments.repository),
        ),
        Domain::Push(PushSubcommand::Branch(arguments)) => {
            Ok(push_network::branch(&arguments.args))
        }
        Domain::Push(PushSubcommand::Force(arguments)) => Ok(push_network::force(
            arguments.expected_remote_oid.as_deref(),
        )),
        Domain::Push(PushSubcommand::Rebase(arguments)) => Ok(push_rebase::rebase(&arguments.args)),
        Domain::Push(PushSubcommand::CheckpointProbe(arguments)) => {
            Ok(push_rebase::checkpoint_probe(&arguments.args))
        }
        Domain::RunLog(RunLogCommand::StoragePreflight(arguments)) => {
            Ok(run_log_commands::storage_preflight(&arguments.arguments))
        }
        Domain::RunLog(RunLogCommand::Archive(arguments)) => {
            Ok(run_log_commands::archive(&arguments.arguments))
        }
        Domain::RunLog(RunLogCommand::Manifest(arguments)) => {
            Ok(run_log_commands::manifest(&arguments.arguments))
        }
        Domain::RunLog(RunLogCommand::Materialize(arguments)) => {
            Ok(run_log_commands::materialize(&arguments.arguments))
        }
        Domain::RunLog(RunLogCommand::MigrateLayout(arguments)) => Ok(
            run_log_migration_commands::migrate_layout(&arguments.arguments),
        ),
        Domain::RunLog(RunLogCommand::CleanupImplementLogs(arguments)) => Ok(
            run_log_cleanup_commands::cleanup_implement_logs(&arguments.arguments),
        ),
        Domain::Status(StatusArguments {
            command: Some(StatusCommand::Check(arguments)),
        }) => Ok(status_commands::check(&arguments.arguments)),
        Domain::Status(StatusArguments { command: None }) => Ok(status_commands::check(&[])),
        Domain::RunLog(RunLogCommand::Publish(arguments)) => {
            Ok(run_log_publication_commands::publish(&arguments.arguments))
        }
        Domain::RunLog(RunLogCommand::LifecycleStart(arguments)) => {
            Ok(run_lifecycle_commands::start(&arguments))
        }
        Domain::RunLog(RunLogCommand::LifecycleFinalize(arguments)) => {
            Ok(run_lifecycle_commands::terminal(
                &arguments,
                "finalize",
                larch_core::LifecycleOutcome::Success,
            ))
        }
        Domain::RunLog(RunLogCommand::LifecycleFailure(arguments)) => {
            Ok(run_lifecycle_commands::terminal(
                &arguments,
                "failure",
                larch_core::LifecycleOutcome::Failure,
            ))
        }
        Domain::RunLog(RunLogCommand::LifecycleCancel(arguments)) => {
            Ok(run_lifecycle_commands::terminal(
                &arguments,
                "cancel",
                larch_core::LifecycleOutcome::Cancelled,
            ))
        }
        Domain::RunLog(RunLogCommand::LifecycleEarlyReturn(arguments)) => {
            Ok(run_lifecycle_commands::terminal(
                &arguments,
                "early-return",
                larch_core::LifecycleOutcome::EarlyReturn,
            ))
        }
        Domain::RunLog(RunLogCommand::Init(arguments)) => {
            Ok(run_log_entry_commands::init(&arguments.arguments))
        }
        Domain::RunLog(RunLogCommand::Write(arguments)) => {
            Ok(run_log_entry_commands::write(&arguments.arguments))
        }
        Domain::RunLog(RunLogCommand::WriteRound(arguments)) => {
            Ok(run_log_entry_commands::write_round(&arguments.arguments))
        }
        Domain::RunLog(RunLogCommand::Append(arguments)) => {
            Ok(run_log_entry_commands::append(&arguments.arguments))
        }
        Domain::RunLog(RunLogCommand::AppendEntry(arguments)) => {
            Ok(run_log_entry_commands::append_entry(&arguments.arguments))
        }
        Domain::RunLog(RunLogCommand::AppendFailure(arguments)) => {
            Ok(run_log_entry_commands::append_failure(&arguments.arguments))
        }
        Domain::RunLog(RunLogCommand::Exists(arguments)) => {
            Ok(run_log_entry_commands::exists(&arguments.arguments))
        }
        Domain::RunLog(RunLogCommand::VerifyCompleteness(arguments)) => Ok(
            run_log_entry_commands::verify_completeness(&arguments.arguments),
        ),
        Domain::RunLog(RunLogCommand::ValidateRunId(arguments)) => {
            Ok(run_log_commands::validate_run_id(&arguments.arguments))
        }
        Domain::RunLog(RunLogCommand::PublishBreadcrumbs(arguments)) => {
            Ok(run_log_commands::publish_breadcrumbs(&arguments.arguments))
        }
        Domain::RunLog(RunLogCommand::Sync(arguments)) => {
            Ok(run_log_publication_commands::sync(&arguments.arguments))
        }
        Domain::RunLog(RunLogCommand::RenderSessionTranscript(arguments)) => Ok(
            run_log_commands::render_session_transcript(&arguments.arguments),
        ),
        Domain::RunLog(RunLogCommand::RetroFixCursor(arguments)) => Ok(
            run_log_migration_commands::retro_fix_cursor(&arguments.arguments),
        ),
        Domain::RunLog(RunLogCommand::RetroV3Sweep(arguments)) => Ok(
            run_log_migration_commands::retro_v3_sweep(&arguments.arguments),
        ),
        Domain::RunLog(RunLogCommand::Checkpoint(arguments)) => {
            Ok(run_log_flush_commands::checkpoint(&arguments.arguments))
        }
        Domain::RunLog(RunLogCommand::CaptureTranscript(arguments)) => Ok(
            run_log_flush_commands::capture_transcript(&arguments.arguments),
        ),
        Domain::RunLog(RunLogCommand::PrepareTerminalSnapshot(arguments)) => Ok(
            run_log_flush_commands::prepare_terminal_snapshot(&arguments.arguments),
        ),
        Domain::RunLog(RunLogCommand::Refresh(arguments)) => {
            Ok(run_log_flush_commands::refresh(&arguments.arguments))
        }
        Domain::Voting(command) => Ok(command.run()),
        Domain::UpgradeLarch(command) => match command {
            UpgradeLarchCommand::ReleaseStep7Root(arguments) => {
                let version = arguments
                    .current_version
                    .as_deref()
                    .or(arguments.positional_current_version.as_deref());
                larch_adapters::upgrade_larch::release_step7_root(version)
                    .map(|()| ExitCode::SUCCESS)
            }
            UpgradeLarchCommand::Run(arguments) => {
                larch_adapters::upgrade_larch::run(arguments.plugin_root.as_deref())
                    .map(|()| ExitCode::SUCCESS)
            }
            UpgradeLarchCommand::SparseDirs => {
                larch_adapters::upgrade_larch::sparse_dirs();
                Ok(ExitCode::SUCCESS)
            }
        },
    }
}

fn dirty_tree_raw_arguments(command: &str) -> Option<Vec<OsString>> {
    let mut values = env::args_os().skip(1);
    (values.next()?.as_os_str() == OsStr::new("dirty-tree")).then_some(())?;
    (values.next()?.as_os_str() == OsStr::new(command)).then_some(())?;
    Some(values.collect())
}

fn run_release(
    command: ReleaseCommand,
) -> Result<ExitCode, larch_adapters::upgrade_larch::Failure> {
    match command {
        ReleaseCommand::AssetCandidate(arguments) => Ok(release_assets::asset_candidate(
            &release_assets::CandidateArguments {
                repo_root: arguments.repo_root,
                tag: arguments.tag,
                source_commit: arguments.source_commit,
                verify_checkout: arguments.verify_checkout,
            },
        )),
        ReleaseCommand::AssetRun(arguments) => Ok(release_stage::asset_run(
            &arguments.repository,
            &arguments.tag,
            &arguments.source_commit,
        )),
        ReleaseCommand::ClassifyBump(arguments) => Ok(release_prepare::classify_bump(
            &release_prepare::ClassifyArguments {
                base: arguments.base,
                head: arguments.head,
            },
        )),
        ReleaseCommand::CollectAssets(arguments) => Ok(release_assets::collect_assets(
            &release_assets::CollectArguments {
                version: arguments.version,
                tag: arguments.tag,
                source_commit: arguments.source_commit,
                input_dir: arguments.input_dir,
                output_dir: arguments.output_dir,
                license: arguments.license,
            },
        )),
        ReleaseCommand::EnsurePolicy(arguments) => {
            Ok(release_stage::ensure_policy(&arguments.repository))
        }
        ReleaseCommand::Finish(arguments) => Ok(release_publish::finish(
            &arguments.version,
            &arguments.repository,
            &arguments.pr,
            &arguments.source_commit,
        )),
        ReleaseCommand::PackageAsset(arguments) => Ok(release_assets::package_asset(
            &release_assets::PackageArguments {
                version: arguments.version,
                tag: arguments.tag,
                source_commit: arguments.source_commit,
                target: arguments.target,
                binary: arguments.binary,
                license: arguments.license,
                output_dir: arguments.output_dir,
            },
        )),
        ReleaseCommand::Prepare(arguments) => Ok(run_release_prepare(arguments)),
        ReleaseCommand::ReconcileNotes(arguments) => Ok(run_release_reconcile_notes(arguments)),
        ReleaseCommand::Promote(arguments) => Ok(release_publish::promote(
            &arguments.version,
            arguments.repository.as_deref(),
        )),
        ReleaseCommand::PromoteLatest(arguments) => Ok(release_publish::promote_latest(
            &arguments.repository,
            arguments.dry_run,
        )),
        ReleaseCommand::PluginRuntime(arguments) => run_release_plugin_runtime(&arguments),
        ReleaseCommand::SetVersion(arguments) => Ok(release_version::run(&arguments.version)),
        ReleaseCommand::Stage(arguments) => Ok(release_stage::stage(
            &arguments.version,
            arguments.notes_file.as_deref(),
            &arguments.repository,
            &arguments.pr,
            arguments.dry_run,
        )),
        ReleaseCommand::ValidateAssets(arguments) => Ok(release_assets::validate_assets(
            &release_assets::ValidateArguments {
                version: arguments.version,
                tag: arguments.tag,
                source_commit: arguments.source_commit,
                asset_dir: arguments.asset_dir,
                license: arguments.license,
                verify_attestations: arguments.verify_attestations,
            },
        )),
        ReleaseCommand::ValidateDraft(arguments) => Ok(release_stage::validate_draft(
            &arguments.version,
            &arguments.repository,
            &arguments.pr,
            &arguments.source_commit,
        )),
    }
}

fn run_release_prepare(arguments: PrepareReleaseArguments) -> ExitCode {
    let bump = arguments.bump.as_deref().map(|value| match value {
        "major" => release_prepare::BumpType::Major,
        "minor" => release_prepare::BumpType::Minor,
        _ => release_prepare::BumpType::Patch,
    });
    release_prepare::prepare(&release_prepare::PrepareArguments {
        repository: arguments.repository,
        bump,
        out_dir: arguments.out_dir,
        no_fetch: arguments.no_fetch,
    })
}

fn run_release_reconcile_notes(arguments: ReconcileNotesArguments) -> ExitCode {
    release_prepare::reconcile_notes(&release_prepare::ReconcileNotesArguments {
        repository: arguments.repository,
        baseline_tag: arguments.baseline_tag,
        source_commit: arguments.source_commit,
        pr_list: arguments.pr_list,
        exclude_pr: arguments.exclude_pr,
        out_dir: arguments.out_dir,
    })
}

fn run_release_plugin_runtime(
    arguments: &PluginRuntimeArguments,
) -> Result<ExitCode, larch_adapters::upgrade_larch::Failure> {
    release_plugin_runtime::run(&arguments.output)
        .map(|()| ExitCode::SUCCESS)
        .map_err(command_failure)
}

const fn command_failure(message: String) -> larch_adapters::upgrade_larch::Failure {
    larch_adapters::upgrade_larch::Failure { code: 1, message }
}

fn run_remote_repo(arguments: &TrailingArguments) -> ExitCode {
    github_repository_resolution::run_remote_repo(&arguments.args)
}

fn run_resolve_repo(arguments: &TrailingArguments) -> ExitCode {
    github_repository_resolution::run_resolve_repo(&arguments.args)
}

fn run_logs(arguments: &RunLogsArguments) -> ExitCode {
    let output = match larch_adapters::runtime::LarchRuntime::new() {
        Ok(runtime) => runtime.block_on(async {
            let cancellation = larch_adapters::runtime::Cancellation::new();
            let runner = larch_adapters::TokioProcessRunner::default();
            let working_directory = match std::env::current_dir() {
                Ok(path) => path,
                Err(error) => {
                    return larch_core::run_logs_setup_failure(
                        &arguments.repository,
                        arguments.run_id,
                        format!("cannot resolve current directory: {error}"),
                    );
                }
            };
            let service = match larch_adapters::github::OctocrabGitHubService::from_gh(
                &runner,
                &working_directory,
                &cancellation,
            )
            .await
            {
                Ok(service) => service,
                Err(error) => {
                    return larch_core::run_logs_setup_failure(
                        &arguments.repository,
                        arguments.run_id,
                        &error,
                    );
                }
            };
            larch_core::run_logs(
                &service,
                &arguments.repository,
                arguments.run_id,
                &cancellation,
            )
            .await
        }),
        Err(error) => larch_core::run_logs_setup_failure(
            &arguments.repository,
            arguments.run_id,
            format!("cannot initialize larch runtime: {error}"),
        ),
    };
    std::io::stdout()
        .write_all(output.stdout())
        .expect("write command output");
    ExitCode::from(output.exit_code())
}

pub(crate) fn parse_repository(value: &str) -> Result<larch_core::GitHubRepositoryRef, String> {
    let Some((owner, name)) = value.split_once('/') else {
        return Err(String::from("repository must use OWNER/REPO form"));
    };
    if name.contains('/') {
        return Err(String::from("repository must use OWNER/REPO form"));
    }
    larch_core::GitHubRepositoryRef::new(owner, name).map_err(|error| error.to_string())
}

fn run_git(command: GitSubcommand) -> Result<ExitCode, String> {
    match command {
        GitSubcommand::AmendAdd(arguments) => Ok(git_commands::run(GitCommand::AmendAdd {
            paths: arguments.paths,
        })),
        GitSubcommand::CheckPhantomDirty(arguments) => {
            check_phantom_dirty_command(&arguments);
            Ok(ExitCode::SUCCESS)
        }
        GitSubcommand::CheckoutOurs(arguments) => checkout_ours(arguments),
        GitSubcommand::ConflictFiles => {
            conflict_files()?;
            Ok(ExitCode::SUCCESS)
        }
        GitSubcommand::CleanTree(arguments) => clean_tree(arguments),
        GitSubcommand::Commit(arguments) => Ok(git_commands::run(GitCommand::Commit {
            message: arguments.message,
            no_trailer: arguments.no_trailer,
            only: arguments.only,
            pathspec_from_file: arguments.pathspec_from_file,
            pathspec_file_nul: arguments.pathspec_file_nul,
            paths: arguments.files,
        })),
        GitSubcommand::SnapshotUntracked(arguments) => {
            snapshot_untracked(arguments);
            Ok(ExitCode::SUCCESS)
        }
        GitSubcommand::PhantomProbe(arguments) => {
            phantom_probe(&arguments);
            Ok(ExitCode::SUCCESS)
        }
        GitSubcommand::RebaseAbort(arguments) => Ok(rebase_abort(&arguments)),
        GitSubcommand::RebaseSkip(arguments) => rebase_skip(&arguments),
        GitSubcommand::BranchInfo(arguments) => Ok(git_commands::run(GitCommand::BranchInfo {
            args: arguments.args,
        })),
        GitSubcommand::CheckMainSync(arguments) => {
            Ok(git_commands::run(GitCommand::CheckMainSync {
                args: arguments.args,
            }))
        }
        GitSubcommand::CheckRemoteBranch(arguments) => {
            Ok(git_commands::run(GitCommand::CheckRemoteBranch {
                args: arguments.args,
            }))
        }
        GitSubcommand::CountCommits(arguments) => Ok(git_commands::run(GitCommand::CountCommits {
            args: arguments.args,
        })),
        GitSubcommand::CurrentBranch(arguments) => {
            Ok(git_commands::run(GitCommand::CurrentBranch {
                args: arguments.args,
            }))
        }
        GitSubcommand::ShowStage(arguments) => Ok(git_commands::run(GitCommand::ShowStage {
            args: arguments.args,
        })),
        GitSubcommand::SyncLocalMain(arguments) => {
            Ok(git_commands::run(GitCommand::SyncLocalMain {
                args: arguments.args,
            }))
        }
        GitSubcommand::Stage(arguments) => Ok(git_commands::run(GitCommand::Stage {
            paths: arguments.paths,
        })),
    }
}

#[derive(Debug, Eq, PartialEq)]
struct PhantomDirtyResult {
    status: &'static str,
    reason: Option<&'static str>,
    count: usize,
    paths_file: Option<PathBuf>,
}

impl PhantomDirtyResult {
    const fn status(status: &'static str) -> Self {
        Self {
            status,
            reason: None,
            count: 0,
            paths_file: None,
        }
    }

    const fn unknown(reason: &'static str) -> Self {
        Self {
            status: "unknown",
            reason: Some(reason),
            count: 0,
            paths_file: None,
        }
    }
}

fn check_phantom_dirty_command(arguments: &CheckPhantomDirtyArguments) {
    let parsed = parse_check_phantom_arguments(&arguments.arguments);
    let result = match parsed {
        Ok((baseline, step, paths_dir)) => check_phantom_dirty(&baseline, &step, &paths_dir),
        Err(reason) => PhantomDirtyResult::unknown(reason),
    };
    emit_phantom_dirty(&result, "");
}

fn parse_check_phantom_arguments(
    arguments: &[OsString],
) -> Result<(PathBuf, String, PathBuf), &'static str> {
    let mut baseline = None;
    let mut step = None;
    let mut paths_dir = None;
    let mut index = 0;
    while index < arguments.len() {
        let argument = arguments[index].as_os_str();
        let (target, missing_reason) = if argument == "--baseline" {
            (&mut baseline, "baseline-missing-value")
        } else if argument == "--step" {
            if index + 1 >= arguments.len() {
                return Err("step-missing-value");
            }
            step = arguments[index + 1].to_str().map(str::to_owned);
            if step.is_none() {
                return Err("bad-step");
            }
            index += 2;
            continue;
        } else if argument == "--phantom-paths-dir" {
            (&mut paths_dir, "phantom-paths-dir-missing-value")
        } else {
            return Err("unknown-flag");
        };
        if index + 1 >= arguments.len() {
            return Err(missing_reason);
        }
        *target = Some(PathBuf::from(&arguments[index + 1]));
        index += 2;
    }
    let baseline = baseline
        .filter(|value| !value.as_os_str().is_empty())
        .ok_or("baseline-required")?;
    let step = step
        .filter(|value| !value.is_empty())
        .ok_or("step-required")?;
    let paths_dir = paths_dir
        .filter(|value| !value.as_os_str().is_empty())
        .ok_or("phantom-paths-dir-required")?;
    Ok((baseline, step, paths_dir))
}

fn check_phantom_dirty(baseline: &Path, step: &str, paths_dir: &Path) -> PhantomDirtyResult {
    if !valid_step(step) {
        return PhantomDirtyResult::unknown("bad-step");
    }
    if !valid_meta_path(baseline.as_os_str()) {
        return PhantomDirtyResult::unknown("bad-baseline-path");
    }
    let Ok(status) = repository_status() else {
        return PhantomDirtyResult::unknown("git-status-failed");
    };
    let current_untracked = untracked_paths(&status);
    let baseline_paths = if baseline.is_file() {
        match fs::read(baseline) {
            Ok(data) => split_nul(&data),
            Err(_) => return PhantomDirtyResult::unknown("baseline-sort-failed"),
        }
    } else if current_untracked.is_empty() {
        BTreeSet::new()
    } else {
        return PhantomDirtyResult::unknown("baseline-missing-untracked-ambiguous");
    };
    let new_untracked = current_untracked
        .difference(&baseline_paths)
        .cloned()
        .collect::<Vec<_>>();
    if new_untracked.is_empty() {
        return if status.tree_to_index.entries().is_empty()
            && status.index_to_worktree.entries().is_empty()
            && status.unmerged.is_empty()
        {
            PhantomDirtyResult::status("clean")
        } else {
            PhantomDirtyResult::status("tracked-only")
        };
    }
    if fs::create_dir_all(paths_dir).is_err() {
        return PhantomDirtyResult::unknown("phantom-paths-dir-create-failed");
    }
    let paths_file = paths_dir.join(format!("phantom-paths-{step}.z"));
    let mut data = Vec::new();
    for path in &new_untracked {
        data.extend(path);
        data.push(0);
    }
    if fs::write(&paths_file, data).is_err() {
        return PhantomDirtyResult::unknown("phantom-paths-write-failed");
    }
    let count = match fs::read(&paths_file) {
        Ok(data) => data
            .iter()
            .fold(0, |count, byte| count + usize::from(*byte == 0)),
        Err(_) => return PhantomDirtyResult::unknown("phantom-count-failed"),
    };
    PhantomDirtyResult {
        status: "phantom",
        reason: None,
        count,
        paths_file: Some(paths_file),
    }
}

fn phantom_probe(arguments: &PhantomProbeArguments) {
    for line in phantom_probe_lines(&arguments.step, arguments.baseline_file.as_deref(), true) {
        println!("{line}");
    }
}

/// Produce the `PHANTOM_*` advisory rows for a checkpoint step. Shared by the
/// `git phantom-probe` command and the `push checkpoint-probe` success tail so
/// both compose the #7757 phantom inspection through one owner. `announce`
/// mirrors the command's stderr banner; the checkpoint tail suppresses it
/// because Python swallowed the probe subprocess's stderr.
pub(crate) fn phantom_probe_lines(
    step: &str,
    baseline_override: Option<&Path>,
    announce: bool,
) -> Vec<String> {
    if announce {
        eprintln!("→ phantom-probe: {step}");
    }
    let Some(implement_tmpdir) = env::var_os("IMPLEMENT_TMPDIR").filter(|value| !value.is_empty())
    else {
        return phantom_dirty_lines(
            &PhantomDirtyResult::unknown("IMPLEMENT_TMPDIR-unset"),
            "PHANTOM_",
        );
    };
    let implement_tmpdir = PathBuf::from(implement_tmpdir);
    let baseline = baseline_override.map_or_else(
        || implement_tmpdir.join("untracked-baseline.z"),
        Path::to_path_buf,
    );
    let result = check_phantom_dirty(&baseline, step, &implement_tmpdir);
    let append_error = append_phantom_warning(&implement_tmpdir, step, &result);
    let mut lines = phantom_dirty_lines(&result, "PHANTOM_");
    if let Some(error) = append_error {
        lines.push(format!(
            "PHANTOM_APPEND_WARN_ERROR={}",
            fold_whitespace(&error)
        ));
    }
    lines
}

fn append_phantom_warning(
    implement_tmpdir: &Path,
    step: &str,
    result: &PhantomDirtyResult,
) -> Option<String> {
    let entry = match result.status {
        "phantom" => format!(
            "- **Step {step} — phantom untracked files:** {} file(s) appeared since session baseline (inspect {}/phantom-paths-{step}.z locally)",
            result.count,
            implement_tmpdir.display()
        ),
        "unknown" => format!(
            "- **Step {step} — phantom detection inconclusive:** STATUS=unknown REASON={}",
            result.reason.unwrap_or("unknown")
        ),
        _ => return None,
    };
    let log = implement_tmpdir.join("execution-issues.md");
    match write_execution_warning(&log, &entry) {
        Ok(()) => None,
        Err(error) => {
            let folded = fold_whitespace(&error);
            let fallback = format!("- **Step {step} — phantom warning append failed: {folded}**");
            let _ = write_execution_warning(&log, &fallback);
            Some(folded)
        }
    }
}

fn write_execution_warning(log: &Path, entry: &str) -> Result<(), String> {
    reject_symlink_path_or_ancestors(log)?;
    let parent = log
        .parent()
        .ok_or_else(|| String::from("log path has no parent"))?;
    fs::create_dir_all(parent).map_err(|error| python_io_error(&error, parent))?;
    reject_symlink_path_or_ancestors(log)?;
    let lock = log.with_file_name(format!(
        "{}.lock.d",
        log.file_name().unwrap_or_default().to_string_lossy()
    ));
    let mut acquired = false;
    for attempt in 0..100 {
        match fs::create_dir(&lock) {
            Ok(()) => {
                acquired = true;
                break;
            }
            Err(error) if error.kind() == ErrorKind::AlreadyExists && attempt < 99 => {
                thread::sleep(Duration::from_millis(50));
            }
            Err(error) if error.kind() == ErrorKind::AlreadyExists => {
                return Err(format!("could not acquire lock: {}", lock.display()));
            }
            Err(error) => return Err(python_io_error(&error, &lock)),
        }
    }
    if !acquired {
        return Err(format!("could not acquire lock: {}", lock.display()));
    }
    let result = (|| {
        let bytes = match fs::read(log) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == ErrorKind::NotFound => Vec::new(),
            Err(error) => return Err(python_io_error(&error, log)),
        };
        let text = String::from_utf8_lossy(&bytes).into_owned();
        let new_text = insert_warning_entry(&text, entry);
        reject_symlink_path_or_ancestors(log)?;
        private_atomic_write(log, &new_text, parent).map_err(|error| error.to_string())
    })();
    let _ = fs::remove_dir(&lock);
    result
}

fn insert_warning_entry(text: &str, entry: &str) -> String {
    const HEADER: &str = "### Warnings";
    if !text.lines().any(|line| line == HEADER) {
        let prefix = if text.is_empty() { "" } else { "\n" };
        return format!(
            "{}{prefix}{HEADER}\n\n{}\n",
            text.trim_end_matches('\n'),
            entry.trim_end_matches('\n')
        );
    }
    let lines = text.lines().collect::<Vec<_>>();
    let mut output = Vec::new();
    let mut inserted = false;
    let mut in_target = false;
    for line in lines {
        if line == HEADER {
            in_target = true;
            output.push(line);
            continue;
        }
        if in_target && line.starts_with("### ") {
            if !inserted {
                output.extend(["", entry.trim_end_matches('\n')]);
                inserted = true;
            }
            in_target = false;
        }
        output.push(line);
    }
    if in_target && !inserted {
        output.extend(["", entry.trim_end_matches('\n')]);
    }
    output.join("\n") + "\n"
}

fn phantom_dirty_lines(result: &PhantomDirtyResult, prefix: &str) -> Vec<String> {
    let mut lines = vec![format!("{prefix}STATUS={}", result.status)];
    if let Some(reason) = result.reason {
        lines.push(format!("{prefix}REASON={reason}"));
    }
    if result.status == "phantom" {
        lines.push(format!("PHANTOM_COUNT={}", result.count));
        if let Some(paths_file) = &result.paths_file {
            lines.push(format!("PHANTOM_PATHS_FILE={}", paths_file.display()));
        }
    }
    lines
}

fn emit_phantom_dirty(result: &PhantomDirtyResult, prefix: &str) {
    for line in phantom_dirty_lines(result, prefix) {
        println!("{line}");
    }
}

fn split_nul(data: &[u8]) -> BTreeSet<Vec<u8>> {
    data.split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
        .map(<[u8]>::to_vec)
        .collect()
}

fn untracked_paths(status: &RepositoryStatus) -> BTreeSet<Vec<u8>> {
    status
        .untracked
        .iter()
        .map(|path| path.as_bytes().to_vec())
        .collect()
}

fn fold_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Refuse a phantom-warning log path whose leaf or any ancestor is a symlink.
///
/// The log lives under `$IMPLEMENT_TMPDIR`, which the `/tmp` session fallback can
/// place beneath a root-owned platform alias, so the same exemption as
/// [`larch_adapters::assert_no_symlink_path_or_ancestors`] applies.
fn reject_symlink_path_or_ancestors(path: &Path) -> Result<(), String> {
    use larch_adapters::refuses_symlink;
    use std::os::unix::fs::MetadataExt as _;

    let mut current = Some(path);
    while let Some(candidate) = current {
        match fs::symlink_metadata(candidate) {
            Ok(metadata) if refuses_symlink(metadata.file_type().is_symlink(), metadata.uid()) => {
                return Err(format!(
                    "refusing symlinked path or ancestor: {}",
                    candidate.display()
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => return Err(python_io_error(&error, candidate)),
        }
        current = candidate.parent();
    }
    Ok(())
}

fn valid_step(step: &str) -> bool {
    !step.is_empty()
        && step
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'-'))
}

pub(crate) fn valid_meta_path(path: &OsStr) -> bool {
    let bytes = path.as_encoded_bytes();
    !bytes.is_empty()
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'/' | b'_' | b'-'))
}

enum GitControl {
    CheckoutOurs(Vec<larch_adapters::git::GitPath>),
    RebaseAbort,
    RebaseSkip,
}

fn checkout_ours(arguments: CheckoutOursArguments) -> Result<ExitCode, String> {
    if arguments.paths.is_empty() {
        eprintln!("git-checkout-ours.sh: at least one file argument is required");
        eprintln!("usage: git-checkout-ours.sh <file> [<file> ...]");
        return Ok(ExitCode::from(1));
    }
    let paths = arguments
        .paths
        .into_iter()
        .map(|path| larch_adapters::git::GitPath::new(path.into_os_string()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    run_git_control(GitControl::CheckoutOurs(paths))
}

fn rebase_abort(arguments: &RebaseControlArguments) -> ExitCode {
    if let Some(argument) = arguments.extra.first() {
        eprintln!(
            "git-rebase-abort.sh: unknown argument: {}",
            argument.to_string_lossy()
        );
        return ExitCode::SUCCESS;
    }
    let _ = run_git_control(GitControl::RebaseAbort);
    ExitCode::SUCCESS
}

fn rebase_skip(arguments: &RebaseControlArguments) -> Result<ExitCode, String> {
    if let Some(argument) = arguments.extra.first() {
        eprintln!(
            "git-rebase-skip.sh: unknown argument: {}",
            argument.to_string_lossy()
        );
        return Ok(ExitCode::from(1));
    }
    run_git_control(GitControl::RebaseSkip)
}

fn run_git_control(control: GitControl) -> Result<ExitCode, String> {
    use larch_adapters::git::{CheckoutRequest, GitCli, GitCliError, GitCliPolicy, RebaseRequest};

    let idempotent_abort = matches!(control, GitControl::RebaseAbort);
    let working_directory = std::env::current_dir()
        .map_err(|error| format!("cannot resolve Git working directory: {error}"))?;
    let policy = GitCliPolicy::new(working_directory).map_err(|error| error.to_string())?;
    let runner = larch_adapters::TokioProcessRunner::default();
    let runtime = larch_adapters::runtime::LarchRuntime::new()
        .map_err(|error| format!("cannot initialize larch runtime: {error}"))?;
    let cancellation = larch_adapters::runtime::Cancellation::new();
    let git = GitCli::new(&runner, policy);
    let result = runtime.block_on(async {
        match control {
            GitControl::CheckoutOurs(paths) => {
                git.checkout(
                    CheckoutRequest::Paths {
                        ours: true,
                        theirs: false,
                        paths,
                    },
                    &cancellation,
                )
                .await
            }
            GitControl::RebaseAbort => git.rebase(RebaseRequest::Abort, &cancellation).await,
            GitControl::RebaseSkip => git.rebase(RebaseRequest::Skip, &cancellation).await,
        }
    });
    if idempotent_abort {
        return Ok(ExitCode::SUCCESS);
    }
    match result {
        Ok(result) | Err(GitCliError::Failed(result)) => emit_git_result(result.output()),
        Err(GitCliError::Process(error)) => {
            if let Some(output) = error.output() {
                let _ = emit_git_result(output)?;
            }
            Err(error.to_string())
        }
        Err(error) => Err(error.to_string()),
    }
}

fn emit_git_result(output: &larch_core::ProcessOutput) -> Result<ExitCode, String> {
    std::io::stdout()
        .write_all(output.stdout())
        .map_err(|error| format!("cannot write Git stdout: {error}"))?;
    std::io::stderr()
        .write_all(output.stderr())
        .map_err(|error| format!("cannot write Git stderr: {error}"))?;
    let code = output
        .status()
        .code()
        .and_then(|code| u8::try_from(code).ok())
        .unwrap_or(1);
    Ok(ExitCode::from(code))
}

fn repository_status() -> Result<RepositoryStatus, larch_core::RepositoryError> {
    larch_adapters::git::GixRepository::discover(".")?.local_status(&StatusOptions::default())
}

fn conflict_files() -> Result<(), String> {
    let status = repository_status().map_err(|error| error.to_string())?;
    for entry in status.unmerged {
        println!("FILE={}", display_path(entry.path.as_bytes()));
        for stage in 1..=3 {
            println!(
                "STAGE_{stage}={}",
                entry.stages.iter().any(|item| item.stage == stage)
            );
        }
        println!();
    }
    Ok(())
}

fn clean_tree(arguments: CleanTreeArguments) -> Result<ExitCode, String> {
    match repository_status() {
        Ok(status) => {
            if status.is_dirty() {
                println!("CLEAN=false");
                println!("DIRTY_OUT={}", one_line(&porcelain(&status)));
            } else {
                println!("CLEAN=true");
            }
            Ok(ExitCode::SUCCESS)
        }
        Err(_error) if !arguments.fail_closed => {
            println!("CLEAN=true");
            Ok(ExitCode::SUCCESS)
        }
        Err(error) => {
            println!("CLEAN=unknown");
            println!(
                "PROBE_ERROR=git exited 1 ({})",
                one_line(&error.to_string())
            );
            Err(String::new())
        }
    }
}

fn snapshot_untracked(arguments: SnapshotUntrackedArguments) {
    let Some(output) = arguments.output else {
        eprintln!("snapshot-untracked.sh: --output is required");
        return;
    };
    let mut temporary_name = output
        .file_name()
        .map_or_else(OsString::new, OsString::from);
    temporary_name.push(".tmp");
    let temporary = output.with_file_name(temporary_name);
    let result = repository_status();
    let cleanup = || {
        remove_if_present(&output);
        remove_if_present(&temporary);
    };
    let Ok(status) = result else {
        cleanup();
        return;
    };
    let paths = untracked_paths(&status);
    let separator = if arguments.nul { 0 } else { b'\n' };
    let mut data = Vec::new();
    for path in paths {
        data.extend(path);
        data.push(separator);
    }
    if fs::write(&temporary, data).is_err() || fs::rename(&temporary, &output).is_err() {
        cleanup();
    }
}

fn remove_if_present(path: &Path) {
    let _ = fs::remove_file(path).or_else(|error| {
        if error.kind() == ErrorKind::NotFound {
            Ok(())
        } else {
            Err(error)
        }
    });
}

fn display_path(path: &[u8]) -> String {
    String::from_utf8_lossy(path).into_owned()
}

fn one_line(value: &str) -> String {
    value
        .replace(['\n', '\r', '\t'], " ")
        .chars()
        .take(256)
        .collect()
}

/// Render `git status --porcelain` text for one repository work tree.
pub(crate) fn repository_porcelain(repo: &Path) -> Option<String> {
    let status = larch_adapters::git::GixRepository::discover(repo)
        .ok()?
        .local_status(&StatusOptions::default())
        .ok()?;
    Some(porcelain(&status))
}

fn porcelain(status: &RepositoryStatus) -> String {
    let mut rows = BTreeMap::<Vec<u8>, [char; 2]>::new();
    for change in status.tree_to_index.entries() {
        rows.entry(change.path.as_bytes().to_vec())
            .or_insert([' ', ' '])[0] = status_code(change.kind);
    }
    for change in status.index_to_worktree.entries() {
        rows.entry(change.path.as_bytes().to_vec())
            .or_insert([' ', ' '])[1] = status_code(change.kind);
    }
    for entry in &status.unmerged {
        rows.insert(
            entry.path.as_bytes().to_vec(),
            conflict_code(entry.kind)
                .chars()
                .collect::<Vec<_>>()
                .try_into()
                .expect("two-byte conflict code"),
        );
    }
    for path in &status.untracked {
        rows.insert(path.as_bytes().to_vec(), ['?', '?']);
    }
    let mut output = String::new();
    for (path, code) in rows {
        let _ = writeln!(output, "{}{} {}", code[0], code[1], display_path(&path));
    }
    output
}

const fn status_code(kind: ChangeKind) -> char {
    match kind {
        ChangeKind::Added => 'A',
        ChangeKind::Deleted => 'D',
        ChangeKind::Modified | ChangeKind::SubmoduleModified => 'M',
        ChangeKind::TypeChanged => 'T',
        ChangeKind::Renamed => 'R',
        ChangeKind::Copied => 'C',
    }
}

const fn conflict_code(kind: larch_core::ConflictKind) -> &'static str {
    match kind {
        larch_core::ConflictKind::BothDeleted => "DD",
        larch_core::ConflictKind::AddedByUs => "AU",
        larch_core::ConflictKind::DeletedByThem => "UD",
        larch_core::ConflictKind::AddedByThem => "UA",
        larch_core::ConflictKind::DeletedByUs => "DU",
        larch_core::ConflictKind::BothAdded => "AA",
        larch_core::ConflictKind::BothModified => "UU",
    }
}

fn dispatch_render(command: RenderCommand) -> ExitCode {
    match command {
        RenderCommand::FindingsView(arguments) => {
            rendering_commands::render_findings_view(&arguments.arguments)
        }
        RenderCommand::LaneStatus(arguments) => {
            rendering_commands::render_lane_status(&arguments.arguments)
        }
        RenderCommand::Reviewer(arguments) => {
            rendering_commands::render_reviewer(&arguments.arguments)
        }
        RenderCommand::Specialist(arguments) => {
            rendering_commands::render_specialist(&arguments.arguments)
        }
        RenderCommand::Voter(arguments) => rendering_commands::render_voter(&arguments.arguments),
        RenderCommand::ScopeAnchor(arguments) => {
            rendering_commands::render_scope_anchor(&arguments.arguments)
        }
        RenderCommand::PlanReview(arguments) => {
            plan_prompt_commands::render_plan_review(&arguments.arguments)
        }
        RenderCommand::RunSummary(arguments) => ExitCode::from(
            u8::try_from(rendering_commands::run_summary(&arguments.arguments)).unwrap_or(2),
        ),
    }
}

fn dispatch_scope_anchor(command: ScopeAnchorCommand) -> ExitCode {
    match command {
        ScopeAnchorCommand::DesignHandoff(arguments) => {
            rendering_commands::scope_anchor_design_handoff(&arguments.arguments)
        }
        ScopeAnchorCommand::RelayAllowed(arguments) => {
            rendering_commands::scope_anchor_relay_allowed(&arguments.arguments)
        }
        ScopeAnchorCommand::RetallyHandoff(arguments) => {
            rendering_commands::scope_anchor_retally_handoff(&arguments.arguments)
        }
        ScopeAnchorCommand::Validate(arguments) => {
            rendering_commands::scope_anchor_validate(&arguments.arguments)
        }
    }
}

fn main() -> ExitCode {
    let metadata = larch_adapters::build_metadata();
    let matches = Cli::command().version(metadata.version()).get_matches();
    let cli = Cli::from_arg_matches(&matches)
        .expect("arguments already validated by the generated Clap command");
    match run(cli, metadata) {
        Ok(exit_code) => exit_code,
        Err(error) => {
            if !error.message.is_empty() {
                eprintln!("{}", error.message);
            }
            ExitCode::from(error.code)
        }
    }
}
