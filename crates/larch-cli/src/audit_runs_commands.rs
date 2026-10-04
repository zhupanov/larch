//! Rust owner for the run-audit scan, mapping, and counter commands.
//!
//! The audit reads hostile archived artifacts and GitHub text, so this module
//! keeps its filesystem traversal bounded and reaches GitHub only through the
//! hardened typed service.  Its line-oriented output deliberately retains the
//! former Python command wires consumed by the audit skill.
#![allow(
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::collapsible_if,
    clippy::filter_map_bool_then,
    clippy::float_cmp,
    clippy::items_after_statements,
    clippy::manual_let_else,
    clippy::needless_pass_by_value,
    clippy::option_if_let_else,
    clippy::semicolon_if_nothing_returned,
    clippy::similar_names,
    clippy::single_match_else,
    clippy::too_many_arguments,
    clippy::too_many_lines
)] // The compatibility scanner keeps ordered classifications and wire fields adjacent.

use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::ExitCode,
    sync::LazyLock,
};

use chrono::{
    DateTime, Datelike, Duration, NaiveDate, NaiveDateTime, TimeZone, Timelike, Utc, Weekday,
};
use larch_adapters::{
    GixRepository,
    github::{
        AuditPullRequest, AuditRunsService, IssueMutationOwner, LiveMutationRequest,
        OctocrabGitHubService,
    },
    runtime::Cancellation,
};
use larch_core::{
    AssessmentKind, BUG_PREFIX, CompletenessOutcome, GitHubCloseReason, GitHubIssue,
    GitHubIssueBodyMode, GitHubIssueList, GitHubIssueListMode, GitHubIssueSearch,
    GitHubIssueSearchSort, GitHubIssueState, GitHubOperationErrorKind, GitHubService, GitPath,
    Head, ISSUE_DEDUP_LIMIT, ReachabilityContext, RepositoryRead, Revision, RunLogCorpus,
    bug_title_match, glob_matches, scan_required_files, single_line, validate_ship_outcome_record,
};
use regex::Regex;
use serde_json::{Value, json};

use crate::{
    admission_commands::{fetch_origin_main, pull_origin_main},
    argparse_compat::{
        looks_like_option, missing, parse_with_flags, split_inline_option, usage_error,
    },
    claude_commands::parse_uint,
    github_repository_resolution::{RemoteRepoResult, repository_ref, resolve_remote_repo},
    github_service::with_github_service,
    issue_mutation_support::{
        EXIT_MUTATION_REFUSED, authorization_request, authorized, flat_error,
        format_mutation_refusal_reason,
    },
    learn_from_bugs_commands::audit_scan_boundary,
    run_log_publication_commands::synchronized_corpus_root,
};

const DESIGN_TITLE: &str = r"^chore\(larch-logs\): design run [0-9A-F]{8}-[0-9A-F]{4}-[0-9A-F]{4}-[0-9A-F]{4}-[0-9A-F]{12}(?: \(issue #[0-9]+\))?$";
const DESIGN_ID: &str = r"^chore\(larch-logs\): design run ([0-9A-F]{8}-[0-9A-F]{4}-[0-9A-F]{4}-[0-9A-F]{4}-[0-9A-F]{12})(?: \(issue #[0-9]+\))?$";
const FOCUS_AREAS: &[&str] = &[
    "code-quality",
    "risk-integration",
    "correctness",
    "architecture",
    "security",
];
const AUDIT_NUDGE_THRESHOLD: usize = 25;
const AUDIT_ERROR_CHARS: usize = 500;
const AUDIT_NEVER_RUN_ADVISORY: &str =
    "Advisory: /learn-from-bugs has never run for this repo; consider running /learn-from-bugs.";

static DESIGN_TITLE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(DESIGN_TITLE).expect("static design title regex"));
static DESIGN_ID_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(DESIGN_ID).expect("static design id regex"));
static AUDIT_IMPLEMENT_TITLE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\[(?:Run Logs Audit |Implement Run Logs Audit ).* Report\]")
        .expect("static implement audit title regex")
});
static AUDIT_DESIGN_TITLE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\[Design Run Logs Audit .* Report\]").expect("static design audit title regex")
});
static LEGACY_REPOSITORY_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$").expect("static legacy repository regex")
});
static EXON_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\| FINDING_.* \| 0 \| 0 \| [1-9][0-9]* \|.*\| rejected \|")
        .expect("static exon regex")
});
static FINDING_CATEGORY_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)###[ \t]+FINDING_[0-9A-Za-z_]+:[ \t]*(code-quality|risk-integration|correctness|architecture|security)(:|\n|$)")
        .expect("static finding category regex")
});
static CODER_TOOL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"CODER_TOOL=([^\s]+)").expect("static coder tool regex"));

fn wants_help(arguments: &[OsString]) -> bool {
    arguments
        .iter()
        .any(|argument| argument == "--help" || argument == "-h")
}

fn audit_help(verb: &str) -> ExitCode {
    let usage = match verb {
        "preflight" => {
            "usage: cli.py audit-runs preflight [-h] --skill SKILL [--repo REPO] [--allow-concurrent]"
        }
        "resolve-prs" => {
            "usage: cli.py audit-runs resolve-prs [-h] --skill SKILL [--repo REPO] [--verbal-description VERBAL_DESCRIPTION]"
        }
        "map-runs" => {
            "usage: cli.py audit-runs map-runs [-h] --skill SKILL --pr-list PR_LIST [--repo REPO] [--log-root LOG_ROOT]"
        }
        "scan-run" => {
            "usage: cli.py audit-runs scan-run [-h] --skill SKILL [--run-dir RUN_DIR] --pr PR --scans-tsv SCANS_TSV [--required-files-tsv REQUIRED_FILES_TSV] [--current-version CURRENT_VERSION]"
        }
        "compute-counters" => {
            "usage: cli.py audit-runs compute-counters [-h] --scan-results-dir SCAN_RESULTS_DIR [--prior-frontmatter PRIOR_FRONTMATTER]"
        }
        "pacific-timestamp" => "usage: cli.py audit-runs pacific-timestamp [-h]",
        "title" => {
            "usage: cli.py audit-runs title [-h] --skill SKILL --pr-list PR_LIST --timestamp TIMESTAMP"
        }
        "title-match" => "usage: cli.py audit-runs title-match [-h] --skill SKILL --title TITLE",
        "bugs-backlog-nudge" => {
            "usage: cli.py audit-runs bugs-backlog-nudge [-h] --repo REPO --root ROOT"
        }
        "close-priors" => {
            "usage: cli.py audit-runs close-priors [-h] --skill SKILL --new-issue-number NEW_ISSUE_NUMBER [--repo REPO] [--operator-invoked]"
        }
        "issue-search" => {
            "usage: cli.py audit-runs issue-search [-h] --repo REPO --keywords KEYWORDS"
        }
        "fix-merge" => "usage: cli.py audit-runs fix-merge [-h] --repo REPO --issue ISSUE",
        "version-window" => {
            "usage: cli.py audit-runs version-window [-h] --repo-root REPO_ROOT --after AFTER [--audited-versions AUDITED_VERSIONS]"
        }
        "label-check" => "usage: cli.py audit-runs label-check [-h] --repo REPO --label LABEL",
        "comment" => {
            "usage: cli.py audit-runs comment [-h] --repo REPO --issue ISSUE --body-file BODY_FILE [--operator-invoked]"
        }
        _ => "usage: cli.py audit-runs [options]",
    };
    println!("{usage}");
    ExitCode::SUCCESS
}

fn string_option(parsed: &crate::argparse_compat::ParsedCommandLine, name: &str) -> String {
    parsed
        .value(name)
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn parsed_or_usage(
    arguments: &[OsString],
    options: &[&'static str],
    flags: &[&'static str],
    usage: &str,
    program: &str,
) -> Result<crate::argparse_compat::ParsedCommandLine, ExitCode> {
    let parsed = parse_with_flags(arguments, options, flags, 0);
    if let Some(error) = parsed.value_error() {
        return Err(usage_error(usage, program, error, 2));
    }
    if let Some(error) = parsed.error() {
        return Err(usage_error(usage, program, &error, 2));
    }
    Ok(parsed)
}

fn parsed_or_usage_without_abbreviation(
    arguments: &[OsString],
    options: &[&'static str],
    usage: &str,
    program: &str,
) -> Result<crate::argparse_compat::ParsedCommandLine, ExitCode> {
    let parsed = parse_with_flags(arguments, options, &[], 0);
    if let Some(error) = parsed.value_error() {
        return Err(usage_error(usage, program, error, 2));
    }
    let abbreviated = arguments
        .iter()
        .filter_map(|argument| {
            let text = argument.to_string_lossy();
            let (name, _) = split_inline_option(&text);
            (looks_like_option(argument) && name.starts_with('-') && !options.contains(&name))
                .then(|| argument.clone())
        })
        .collect::<Vec<_>>();
    if !abbreviated.is_empty() {
        let error = crate::argparse_compat::join_arguments(&abbreviated);
        return Err(usage_error(
            usage,
            program,
            &format!("unrecognized arguments: {error}"),
            2,
        ));
    }
    if let Some(error) = parsed.error() {
        return Err(usage_error(usage, program, &error, 2));
    }
    Ok(parsed)
}

fn audit_title_skill(skill: &str, program: &str) -> bool {
    if matches!(skill, "design" | "implement") {
        return true;
    }
    if skill.is_empty() {
        eprintln!("{program}: --skill is required (allowed: design, implement)");
    } else {
        eprintln!("{program}: --skill must be design or implement (got: {skill})");
    }
    false
}

fn audit_pr_numbers(value: &str) -> Vec<String> {
    let mut numbers = value
        .split(',')
        .filter_map(|token| {
            let token = token.trim();
            (!token.is_empty() && token.bytes().all(|byte| byte.is_ascii_digit())).then(|| {
                let normalized = token.trim_start_matches('0');
                if normalized.is_empty() {
                    "0".to_owned()
                } else {
                    normalized.to_owned()
                }
            })
        })
        .collect::<Vec<_>>();
    numbers.sort_by(|left, right| left.len().cmp(&right.len()).then_with(|| left.cmp(right)));
    numbers.dedup();
    numbers
}

fn increment_decimal(value: &str) -> String {
    let mut bytes = value.as_bytes().to_vec();
    for index in (0..bytes.len()).rev() {
        if bytes[index] != b'9' {
            bytes[index] += 1;
            return String::from_utf8(bytes).expect("decimal bytes stay UTF-8");
        }
        bytes[index] = b'0';
    }
    let mut incremented = String::with_capacity(value.len() + 1);
    incremented.push('1');
    incremented.push_str(&String::from_utf8(bytes).expect("decimal bytes stay UTF-8"));
    incremented
}

fn render_audit_title(skill: &str, pr_list: &str, timestamp: &str) -> Option<String> {
    let numbers = audit_pr_numbers(pr_list);
    let first = numbers.first()?;
    let last = numbers.last()?;
    let prs = if numbers.len() == 1 {
        format!("#{first}")
    } else if numbers
        .windows(2)
        .all(|pair| increment_decimal(&pair[0]) == pair[1])
    {
        format!("#{first}-#{last}")
    } else {
        format!("#{first}-#{last} ({} total)", numbers.len())
    };
    let prefix = if skill == "implement" {
        "Implement Run Logs Audit"
    } else {
        "Design Run Logs Audit"
    };
    Some(format!("[{prefix} {timestamp} Report] PRs {prs}"))
}

/// Print the title wire for one new audit report.
#[must_use]
pub fn title(arguments: &[OsString]) -> ExitCode {
    if wants_help(arguments) {
        return audit_help("title");
    }
    const USAGE: &str =
        "usage: cli.py audit-runs title [-h] --skill SKILL --pr-list PR_LIST --timestamp TIMESTAMP";
    let parsed = match parsed_or_usage(
        arguments,
        &["--skill", "--pr-list", "--timestamp"],
        &[],
        USAGE,
        "cli.py audit-runs title",
    ) {
        Ok(parsed) => parsed,
        Err(code) => return code,
    };
    let required = [
        ("--skill", parsed.value("--skill").is_some()),
        ("--pr-list", parsed.value("--pr-list").is_some()),
        ("--timestamp", parsed.value("--timestamp").is_some()),
    ];
    if required.iter().any(|(_, present)| !present) {
        return usage_error(USAGE, "cli.py audit-runs title", &missing(&required), 2);
    }
    let skill = string_option(&parsed, "--skill");
    if !audit_title_skill(&skill, "audit-title.sh") {
        return ExitCode::FAILURE;
    }
    let pr_list = string_option(&parsed, "--pr-list");
    let timestamp = string_option(&parsed, "--timestamp");
    let Some(title) = render_audit_title(&skill, &pr_list, &timestamp) else {
        eprintln!("audit-title.sh: --pr-list contains no valid PR numbers");
        return ExitCode::FAILURE;
    };
    println!("TITLE={title}");
    ExitCode::SUCCESS
}

/// Exit successfully when a title is one of the audit-report title families.
#[must_use]
pub fn title_match(arguments: &[OsString]) -> ExitCode {
    if wants_help(arguments) {
        return audit_help("title-match");
    }
    const USAGE: &str = "usage: cli.py audit-runs title-match [-h] --skill SKILL --title TITLE";
    let parsed = match parsed_or_usage(
        arguments,
        &["--skill", "--title"],
        &[],
        USAGE,
        "cli.py audit-runs title-match",
    ) {
        Ok(parsed) => parsed,
        Err(code) => return code,
    };
    let required = [
        ("--skill", parsed.value("--skill").is_some()),
        ("--title", parsed.value("--title").is_some()),
    ];
    if required.iter().any(|(_, present)| !present) {
        return usage_error(
            USAGE,
            "cli.py audit-runs title-match",
            &missing(&required),
            2,
        );
    }
    let skill = string_option(&parsed, "--skill");
    if !audit_title_skill(&skill, "audit-title-matcher.sh") {
        return ExitCode::FAILURE;
    }
    if matches_audit_title(&skill, &string_option(&parsed, "--title")) {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn parse_utc_instant(raw: &str) -> Option<DateTime<Utc>> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    if let Ok(value) = DateTime::parse_from_rfc3339(raw) {
        return Some(value.with_timezone(&Utc));
    }
    for format in [
        "%Y-%m-%dT%H:%M:%S%.f",
        "%Y-%m-%d %H:%M:%S%.f",
        "%Y-%m-%dT%H:%M",
        "%Y-%m-%d %H:%M",
    ] {
        if let Ok(value) = NaiveDateTime::parse_from_str(raw, format) {
            return Some(Utc.from_utc_datetime(&value));
        }
    }
    NaiveDate::parse_from_str(raw, "%Y-%m-%d")
        .ok()
        .and_then(|value| value.and_hms_opt(0, 0, 0))
        .map(|value| Utc.from_utc_datetime(&value))
}

#[derive(Debug)]
enum NudgeReadFailure {
    Malformed(String),
    Service(String),
}

impl NudgeReadFailure {
    fn from_github(error: larch_core::GitHubOperationError) -> Self {
        let detail = error.to_string();
        if error.kind() == GitHubOperationErrorKind::MalformedResponse {
            Self::Malformed(detail)
        } else {
            Self::Service(detail)
        }
    }

    fn into_wire(self) -> String {
        match self {
            Self::Malformed(detail) => format!("malformed:{detail}"),
            Self::Service(detail) => format!("service:{detail}"),
        }
    }
}

async fn bugs_backlog_nudge_count_remote(
    service: &OctocrabGitHubService,
    cancellation: &Cancellation,
    repository: &larch_core::GitHubRepositoryRef,
    boundary: DateTime<Utc>,
) -> Result<usize, NudgeReadFailure> {
    let boundary_text = boundary.format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let request = GitHubIssueSearch {
        repo: repository.clone(),
        query: format!("{BUG_PREFIX} in:title closed:>{boundary_text}"),
        limit: service.transport_policy().limits().items(),
        sort: GitHubIssueSearchSort::BestMatch,
    };
    let result = service
        .search_issues(&request, cancellation)
        .await
        .map_err(NudgeReadFailure::from_github)?;
    if result.continuation_unread {
        return Err(NudgeReadFailure::Service(
            "bug backlog search stopped at its transport bound with matches unread".to_owned(),
        ));
    }
    Ok(result
        .issues
        .iter()
        .filter(|issue| {
            issue.state == GitHubIssueState::Closed
                && !issue.is_pull_request
                && bug_title_match(&issue.title)
                && parse_utc_instant(&issue.closed_at).is_some_and(|closed_at| closed_at > boundary)
        })
        .count())
}

fn print_bugs_backlog_nudge_failure(detail: String) {
    if let Some(detail) = detail.strip_prefix("malformed:") {
        eprintln!(
            "audit-runs bugs-backlog-nudge: gh issue list returned invalid JSON: {}",
            flat_error(detail, AUDIT_ERROR_CHARS)
        );
    } else {
        let detail = detail.strip_prefix("service:").unwrap_or(&detail);
        eprintln!(
            "audit-runs bugs-backlog-nudge: gh issue list failed: {}",
            flat_error(detail, AUDIT_ERROR_CHARS)
        );
    }
}

/// Read the bounded bug backlog and print its non-mutating advisory when needed.
#[must_use]
pub fn bugs_backlog_nudge(arguments: &[OsString]) -> ExitCode {
    if wants_help(arguments) {
        return audit_help("bugs-backlog-nudge");
    }
    const USAGE: &str = "usage: cli.py audit-runs bugs-backlog-nudge [-h] --repo REPO --root ROOT";
    let parsed = match parsed_or_usage_without_abbreviation(
        arguments,
        &["--repo", "--root"],
        USAGE,
        "cli.py audit-runs bugs-backlog-nudge",
    ) {
        Ok(parsed) => parsed,
        Err(code) => return code,
    };
    let required = [
        ("--repo", parsed.value("--repo").is_some()),
        ("--root", parsed.value("--root").is_some()),
    ];
    if required.iter().any(|(_, present)| !present) {
        return usage_error(
            USAGE,
            "cli.py audit-runs bugs-backlog-nudge",
            &missing(&required),
            2,
        );
    }
    let repo = string_option(&parsed, "--repo");
    if !LEGACY_REPOSITORY_RE.is_match(&repo) {
        eprintln!("audit-runs bugs-backlog-nudge: --repo must be OWNER/REPO");
        return ExitCode::from(2);
    }
    let root = PathBuf::from(string_option(&parsed, "--root"));
    let Some((state_repo, boundary)) = audit_scan_boundary(&root) else {
        println!("{AUDIT_NEVER_RUN_ADVISORY}");
        return ExitCode::SUCCESS;
    };
    if !state_repo.eq_ignore_ascii_case(&repo) {
        println!("{AUDIT_NEVER_RUN_ADVISORY}");
        return ExitCode::SUCCESS;
    }
    let Some(boundary) = parse_utc_instant(&boundary) else {
        println!("{AUDIT_NEVER_RUN_ADVISORY}");
        return ExitCode::SUCCESS;
    };
    let repository = match repository_ref(&repo) {
        Ok(repository) => repository,
        Err(()) => {
            print_bugs_backlog_nudge_failure("service:invalid repository".to_owned());
            return ExitCode::FAILURE;
        }
    };
    let count = with_github_service(async |service, cancellation| {
        bugs_backlog_nudge_count_remote(service, cancellation, &repository, boundary)
            .await
            .map_err(NudgeReadFailure::into_wire)
    });
    let count = match count {
        Ok(count) => count,
        Err(error) => {
            print_bugs_backlog_nudge_failure(error.into_detail());
            return ExitCode::FAILURE;
        }
    };
    if count > AUDIT_NUDGE_THRESHOLD {
        println!(
            "Advisory: {count} closed [BUG] issues accumulated since the last /learn-from-bugs scan; consider running /learn-from-bugs."
        );
    }
    ExitCode::SUCCESS
}

#[derive(Debug, Eq, PartialEq)]
enum ClosePriorsOutcome {
    NoPrior,
    Ambiguous,
    Closed(u64),
    Failed { number: u64, reason: String },
}

#[derive(Debug)]
enum ClosePriorsFailure {
    ListMalformed,
    ListFailed,
}

impl ClosePriorsFailure {
    fn into_wire(self) -> String {
        match self {
            Self::ListMalformed => "malformed-list".to_owned(),
            Self::ListFailed => "failed-list".to_owned(),
        }
    }
}

fn close_prior_failure_reason(reason: &str) -> String {
    match reason {
        "comment-write-failed" => "gh issue comment failed".to_owned(),
        "close-failed" => "gh issue close failed".to_owned(),
        "close-read-back-failed" => "close-postcondition-unverified".to_owned(),
        _ => reason.to_owned(),
    }
}

async fn close_priors_remote(
    service: &OctocrabGitHubService,
    cancellation: &Cancellation,
    repository: &larch_core::GitHubRepositoryRef,
    skill: &str,
    new_issue_number: &str,
    authorization: &LiveMutationRequest<'_>,
) -> Result<ClosePriorsOutcome, ClosePriorsFailure> {
    let rows = service
        .list_issues(
            &GitHubIssueList {
                repo: repository.clone(),
                state: GitHubIssueState::Open,
                labels: vec!["audit-report".to_owned()],
                limit: service.transport_policy().limits().items(),
                // Closing every prior report is fail-closed, so an over-bound
                // corpus refuses rather than leaving priors open.
                mode: GitHubIssueListMode::Exhaustive,
                body_mode: GitHubIssueBodyMode::Include,
            },
            cancellation,
        )
        .await
        .map_err(|error| {
            if error.kind() == GitHubOperationErrorKind::MalformedResponse {
                ClosePriorsFailure::ListMalformed
            } else {
                ClosePriorsFailure::ListFailed
            }
        })?;
    let candidates = rows
        .issues
        .iter()
        .filter(|issue| {
            issue.state == GitHubIssueState::Open
                && !issue.is_pull_request
                && issue.number.to_string() != new_issue_number
                && matches_audit_title(skill, &issue.title)
        })
        .collect::<Vec<_>>();
    if candidates.len() > 1 {
        return Ok(ClosePriorsOutcome::Ambiguous);
    }
    let Some(prior) = candidates.first() else {
        return Ok(ClosePriorsOutcome::NoPrior);
    };
    let comment = format!("Superseded by #{new_issue_number}");
    let owner = IssueMutationOwner::new(service);
    match owner
        .close_with_comment(
            cancellation,
            authorization,
            repository,
            prior.number,
            GitHubCloseReason::Completed,
            Some(&comment),
        )
        .await
    {
        Ok(_) => Ok(ClosePriorsOutcome::Closed(prior.number)),
        Err(error) => Ok(ClosePriorsOutcome::Failed {
            number: prior.number,
            reason: close_prior_failure_reason(error.reason()),
        }),
    }
}

/// Close the one unambiguous prior audit report after a new report is filed.
#[must_use]
pub fn close_priors(arguments: &[OsString]) -> ExitCode {
    if wants_help(arguments) {
        return audit_help("close-priors");
    }
    const USAGE: &str = "usage: cli.py audit-runs close-priors [-h] --skill SKILL --new-issue-number NEW_ISSUE_NUMBER [--repo REPO] [--operator-invoked]";
    let parsed = match parsed_or_usage(
        arguments,
        &["--skill", "--new-issue-number", "--repo"],
        &["--operator-invoked"],
        USAGE,
        "cli.py audit-runs close-priors",
    ) {
        Ok(parsed) => parsed,
        Err(code) => return code,
    };
    let required = [
        ("--skill", parsed.value("--skill").is_some()),
        (
            "--new-issue-number",
            parsed.value("--new-issue-number").is_some(),
        ),
    ];
    if required.iter().any(|(_, present)| !present) {
        return usage_error(
            USAGE,
            "cli.py audit-runs close-priors",
            &missing(&required),
            2,
        );
    }
    let skill = string_option(&parsed, "--skill");
    if !audit_title_skill(&skill, "audit-close-priors.sh") {
        return ExitCode::FAILURE;
    }
    let authorization = authorization_request("", "", "", parsed.flag("--operator-invoked"));
    if let Err(reason) = authorized(&authorization) {
        println!("CLOSE_PRIORS_REFUSED=true");
        println!("REASON={}", format_mutation_refusal_reason(reason));
        return ExitCode::from(EXIT_MUTATION_REFUSED);
    }
    let repo = parsed.value("--repo").map_or_else(
        || "zhupanov/larch".to_owned(),
        |value| value.to_string_lossy().into_owned(),
    );
    let repository = match repository_ref(&repo) {
        Ok(repository) => repository,
        Err(()) => {
            println!("ISSUE_LIST_FAILED=true");
            println!("REASON=gh issue list failed");
            return ExitCode::FAILURE;
        }
    };
    let new_issue_number = string_option(&parsed, "--new-issue-number");
    let result = with_github_service(async |service, cancellation| {
        close_priors_remote(
            service,
            cancellation,
            &repository,
            &skill,
            &new_issue_number,
            &authorization,
        )
        .await
        .map_err(ClosePriorsFailure::into_wire)
    });
    let outcome = match result {
        Ok(outcome) => outcome,
        Err(error) => {
            println!("ISSUE_LIST_FAILED=true");
            let reason = if error.into_detail() == "malformed-list" {
                "gh issue list returned invalid JSON"
            } else {
                "gh issue list failed"
            };
            println!("REASON={reason}");
            return ExitCode::FAILURE;
        }
    };
    match outcome {
        ClosePriorsOutcome::NoPrior => ExitCode::SUCCESS,
        ClosePriorsOutcome::Ambiguous => {
            println!("CLOSE_PRIORS_REFUSED=true");
            println!("REASON=ambiguous-match");
            ExitCode::from(EXIT_MUTATION_REFUSED)
        }
        ClosePriorsOutcome::Closed(number) => {
            println!("CLOSED_NUMBER={number}");
            ExitCode::SUCCESS
        }
        ClosePriorsOutcome::Failed { number, reason } => {
            println!("CLOSE_FAILED={number}\tREASON={reason}");
            ExitCode::SUCCESS
        }
    }
}

/// Bounded first-parent window the version-window walk inspects from `HEAD`.
const VERSION_WINDOW_COMMIT_LIMIT: usize = 4000;
const PLUGIN_MANIFEST_PATH: &str = ".claude-plugin/plugin.json";
const BUMP_SUBJECT_NEEDLE: &str = "Bump version";

const fn issue_state_wire(state: GitHubIssueState) -> &'static str {
    match state {
        GitHubIssueState::Open => "open",
        GitHubIssueState::Closed => "closed",
        GitHubIssueState::All => "unknown",
    }
}

fn audit_report_family_title(title: &str) -> bool {
    AUDIT_IMPLEMENT_TITLE_RE.is_match(title) || AUDIT_DESIGN_TITLE_RE.is_match(title)
}

async fn issue_search_remote<S: GitHubService + ?Sized>(
    service: &S,
    cancellation: &Cancellation,
    repository: &larch_core::GitHubRepositoryRef,
    keywords: &str,
) -> Result<Vec<GitHubIssue>, String> {
    let request = GitHubIssueSearch::for_dedup(
        repository.clone(),
        keywords.to_owned(),
        service.transport_policy(),
    );
    let result = service
        .search_issues(&request, cancellation)
        .await
        .map_err(|error| format!("gh issue list failed: {error}"))?;
    let rows = result.issues;
    if result.continuation_unread {
        eprintln!(
            "WARN: audit-runs proposal search stopped at its bound with matches unread; additional matches were omitted"
        );
    } else if rows.len() == request.limit {
        eprintln!(
            "WARN: audit-runs proposal search reached the {ISSUE_DEDUP_LIMIT}-issue dedup cap; additional matches, if any, were omitted"
        );
    }
    // GitHub search is only a recall filter: the exclusion predicate is
    // re-applied locally so ranking or index drift cannot flip a
    // classification. Only audit-report family titles and pull-request rows
    // are dropped; `[IMPLEMENTING]` titles never match either family regex
    // and are always retained for the augmentation route.
    Ok(rows
        .into_iter()
        .filter(|issue| !issue.is_pull_request && !audit_report_family_title(&issue.title))
        .collect())
}

/// Search open and closed issues for proposal classification, excluding only
/// audit-report noise locally.
#[must_use]
pub fn issue_search(arguments: &[OsString]) -> ExitCode {
    if wants_help(arguments) {
        return audit_help("issue-search");
    }
    const USAGE: &str =
        "usage: cli.py audit-runs issue-search [-h] --repo REPO --keywords KEYWORDS";
    const PROGRAM: &str = "cli.py audit-runs issue-search";
    let parsed = match parsed_or_usage_without_abbreviation(
        arguments,
        &["--repo", "--keywords"],
        USAGE,
        PROGRAM,
    ) {
        Ok(parsed) => parsed,
        Err(code) => return code,
    };
    let required = [
        ("--repo", parsed.value("--repo").is_some()),
        ("--keywords", parsed.value("--keywords").is_some()),
    ];
    if required.iter().any(|(_, present)| !present) {
        return usage_error(USAGE, PROGRAM, &missing(&required), 2);
    }
    let repo = string_option(&parsed, "--repo");
    if !LEGACY_REPOSITORY_RE.is_match(&repo) {
        eprintln!("{PROGRAM}: --repo must be OWNER/REPO");
        return ExitCode::from(2);
    }
    let keywords = string_option(&parsed, "--keywords");
    if keywords.trim().is_empty() {
        return usage_error(USAGE, PROGRAM, "--keywords must be non-empty", 2);
    }
    let repository = match repository_ref(&repo) {
        Ok(repository) => repository,
        Err(()) => {
            eprintln!("{PROGRAM}: --repo must be OWNER/REPO");
            return ExitCode::from(2);
        }
    };
    let result = with_github_service(async |service, cancellation| {
        issue_search_remote(service, cancellation, &repository, &keywords).await
    });
    let matches = match result {
        Ok(matches) => matches,
        Err(error) => {
            println!("ISSUE_SEARCH_FAILED=true");
            println!(
                "REASON={}",
                flat_error(&error.into_detail(), AUDIT_ERROR_CHARS)
            );
            return ExitCode::FAILURE;
        }
    };
    println!("MATCH_COUNT={}", matches.len());
    for issue in matches {
        println!(
            "MATCH={}\t{}\t{}\t{}",
            issue.number,
            issue_state_wire(issue.state),
            single_line(&issue.closed_at),
            single_line(&issue.title)
        );
    }
    ExitCode::SUCCESS
}

#[derive(Debug, Eq, PartialEq)]
struct FixMergeOutcome {
    matched: Option<(u64, String)>,
    ambiguous: bool,
    candidates: Vec<u64>,
}

struct FixMergeReport {
    created_at: String,
    closed_at: String,
    outcome: FixMergeOutcome,
}

fn issue_mention_regexes(issue: u64) -> (Regex, Regex) {
    let mention = Regex::new(&format!(r"#{issue}\b")).expect("issue mention regex");
    let closing = Regex::new(&format!(
        r"(?i)\b(?:close[sd]?|fix(?:es|ed)?|resolve[sd]?):?\s+#{issue}\b"
    ))
    .expect("closing reference regex");
    (mention, closing)
}

fn fix_merge_result(pull: &AuditPullRequest, candidates: Vec<u64>) -> FixMergeOutcome {
    FixMergeOutcome {
        matched: Some((pull.number, pull.merged_at.clone().unwrap_or_default())),
        ambiguous: false,
        candidates,
    }
}

/// Apply the skill's normative disambiguation to the referencing merged PRs.
///
/// Order: explicit closing reference, smallest positive `mergedAt - createdAt`
/// delta, then latest `mergedAt` when no candidate strictly follows creation.
/// An unresolvable tie is reported as ambiguous instead of being suppressed.
fn fix_merge_disposition(
    pulls: &[AuditPullRequest],
    issue: u64,
    created_at: &str,
) -> FixMergeOutcome {
    let (mention, closing) = issue_mention_regexes(issue);
    let referencing = pulls
        .iter()
        .filter(|pull| pull.merged_at.is_some())
        .filter(|pull| mention.is_match(&pull.title) || mention.is_match(&pull.body))
        .collect::<Vec<_>>();
    let closing_refs = referencing
        .iter()
        .copied()
        .filter(|pull| closing.is_match(&pull.title) || closing.is_match(&pull.body))
        .collect::<Vec<_>>();
    let pool = if closing_refs.is_empty() {
        referencing
    } else {
        closing_refs
    };
    let candidates = pool.iter().map(|pull| pull.number).collect::<Vec<_>>();
    if pool.is_empty() {
        return FixMergeOutcome {
            matched: None,
            ambiguous: false,
            candidates,
        };
    }
    if pool.len() == 1 {
        return fix_merge_result(pool[0], candidates);
    }
    let merged_instant =
        |pull: &AuditPullRequest| pull.merged_at.as_deref().and_then(parse_utc_instant);
    if let Some(created) = parse_utc_instant(created_at) {
        let positive = pool
            .iter()
            .copied()
            .filter(|pull| merged_instant(pull).is_some_and(|merged| merged > created))
            .collect::<Vec<_>>();
        if !positive.is_empty() {
            // The smallest positive delta is the earliest merge after creation.
            let best = positive
                .iter()
                .copied()
                .min_by_key(|pull| merged_instant(pull))
                .expect("non-empty positive candidate set");
            let tied = positive
                .iter()
                .filter(|pull| merged_instant(pull) == merged_instant(best))
                .count();
            if tied == 1 {
                return fix_merge_result(best, candidates);
            }
            return FixMergeOutcome {
                matched: None,
                ambiguous: true,
                candidates,
            };
        }
    }
    let best = pool
        .iter()
        .copied()
        .max_by(|left, right| left.merged_at.cmp(&right.merged_at))
        .expect("non-empty candidate pool");
    let tied = pool
        .iter()
        .filter(|pull| pull.merged_at == best.merged_at)
        .count();
    if tied == 1 {
        fix_merge_result(best, candidates)
    } else {
        FixMergeOutcome {
            matched: None,
            ambiguous: true,
            candidates,
        }
    }
}

async fn fix_merge_remote<S: AuditRunsService + GitHubService + ?Sized>(
    service: &S,
    cancellation: &Cancellation,
    repo: &larch_core::GitHubRepositoryRef,
    issue: u64,
) -> Result<FixMergeReport, String> {
    let details = service
        .issue(repo, issue, cancellation)
        .await
        .map_err(|error| format!("gh issue view failed: {error}"))?;
    let pulls = service
        .list_audit_merged_main_pull_requests(cancellation, repo.owner(), repo.name())
        .await
        .map_err(|error| format!("gh pr list failed: {error}"))?;
    let outcome = fix_merge_disposition(&pulls, issue, &details.created_at);
    Ok(FixMergeReport {
        created_at: details.created_at,
        closed_at: details.closed_at,
        outcome,
    })
}

fn print_fix_merge(report: &FixMergeReport) {
    println!("ISSUE_CREATED_AT={}", single_line(&report.created_at));
    println!("ISSUE_CLOSED_AT={}", single_line(&report.closed_at));
    match &report.outcome.matched {
        Some((number, merged_at)) => {
            println!("MATCHED_PR={number}");
            println!("MERGED_AT={}", single_line(merged_at));
        }
        None => {
            println!("MATCHED_PR=unknown");
            println!("MERGED_AT=unknown");
        }
    }
    println!("AMBIGUOUS={}", report.outcome.ambiguous);
    println!(
        "CANDIDATES={}",
        report
            .outcome
            .candidates
            .iter()
            .map(u64::to_string)
            .collect::<Vec<_>>()
            .join(",")
    );
}

/// Resolve which merged-to-main PR fixed one issue, plus the issue's timing.
///
/// The merged-PR history read is bounded, so a fix PR older than the window
/// degrades to the zero-candidate `MATCHED_PR=unknown` path — the same
/// timing-only fallback the skill takes when no PR references the issue.
#[must_use]
pub fn fix_merge(arguments: &[OsString]) -> ExitCode {
    if wants_help(arguments) {
        return audit_help("fix-merge");
    }
    const USAGE: &str = "usage: cli.py audit-runs fix-merge [-h] --repo REPO --issue ISSUE";
    const PROGRAM: &str = "cli.py audit-runs fix-merge";
    let parsed = match parsed_or_usage_without_abbreviation(
        arguments,
        &["--repo", "--issue"],
        USAGE,
        PROGRAM,
    ) {
        Ok(parsed) => parsed,
        Err(code) => return code,
    };
    let required = [
        ("--repo", parsed.value("--repo").is_some()),
        ("--issue", parsed.value("--issue").is_some()),
    ];
    if required.iter().any(|(_, present)| !present) {
        return usage_error(USAGE, PROGRAM, &missing(&required), 2);
    }
    let repo = string_option(&parsed, "--repo");
    let repository = match repository_ref(&repo) {
        Ok(repository) => repository,
        Err(()) => {
            eprintln!("{PROGRAM}: --repo must be OWNER/REPO");
            return ExitCode::from(2);
        }
    };
    let Some(issue) = parse_uint(&string_option(&parsed, "--issue")) else {
        return usage_error(USAGE, PROGRAM, "--issue must be a positive integer", 2);
    };
    let result = with_github_service(async |service, cancellation| {
        fix_merge_remote(service, cancellation, &repository, issue).await
    });
    match result {
        Ok(report) => {
            print_fix_merge(&report);
            ExitCode::SUCCESS
        }
        Err(error) => {
            println!("FIX_MERGE_FAILED=true");
            println!(
                "REASON={}",
                flat_error(&error.into_detail(), AUDIT_ERROR_CHARS)
            );
            ExitCode::FAILURE
        }
    }
}

/// Strictly normalize one dotted version for the version-window comparison.
///
/// Strips a single leading `v`, trims ASCII whitespace, and requires exactly
/// three integer components. This is deliberately distinct from the loose
/// [`version_numbers`] sibling below (G-Dup-1 deviation note): that helper
/// orders arbitrary strings for the cache-freshness lens, while this contract
/// is strict-or-`unknown` so an unparseable version can never suppress a
/// proposal.
fn strict_window_version(value: &str) -> Option<(u64, u64, u64)> {
    let trimmed = value.trim();
    let trimmed = trimmed.strip_prefix('v').unwrap_or(trimmed);
    strict_version_tuple(trimmed)
}

fn commit_touches_manifest(
    repository: &GixRepository,
    commit: &larch_core::Commit,
) -> Result<bool, String> {
    let Some(parent) = commit.parents.first() else {
        // A root commit "touches" the manifest exactly when it introduces it,
        // matching `git log -- <path>` semantics for the initial commit.
        return Ok(repository
            .blob_at_commit(&commit.id, &GitPath::new(PLUGIN_MANIFEST_PATH))
            .map_err(|_| "manifest blob read failed".to_owned())?
            .is_some());
    };
    let parent_commit = repository
        .walk_commits(parent, 1)
        .map_err(|_| "parent commit read failed".to_owned())?
        .into_iter()
        .next()
        .ok_or_else(|| "parent commit read failed".to_owned())?;
    let changes = repository
        .tree_changes(&parent_commit.tree, &commit.tree)
        .map_err(|_| "tree comparison failed".to_owned())?;
    Ok(changes
        .paths()
        .any(|path| path.as_bytes() == PLUGIN_MANIFEST_PATH.as_bytes()))
}

/// The earliest first-parent bump commit strictly after `after`, if any.
fn version_window_bump(
    repository: &GixRepository,
    after_epoch: i64,
) -> Result<Option<larch_core::Commit>, String> {
    let head = repository
        .resolve_revision(&Revision::new("HEAD"))
        .map_err(|_| "HEAD is not resolvable".to_owned())?;
    let mut current = head;
    let mut earliest: Option<larch_core::Commit> = None;
    for _ in 0..VERSION_WINDOW_COMMIT_LIMIT {
        let commit = repository
            .walk_commits(&current, 1)
            .map_err(|_| "commit walk failed".to_owned())?
            .into_iter()
            .next()
            .ok_or_else(|| "commit walk failed".to_owned())?;
        let next = commit.parents.first().cloned();
        let subject = String::from_utf8_lossy(&commit.subject).into_owned();
        if subject.contains(BUMP_SUBJECT_NEEDLE)
            && crate::analyze_bugs_sweep::committer_time(repository, &commit.id)
                .is_ok_and(|time| time > after_epoch)
            && commit_touches_manifest(repository, &commit)?
        {
            // The walk is newest-first, so the last hit is the earliest bump.
            earliest = Some(commit);
        }
        match next {
            Some(parent) => current = parent,
            None => break,
        }
    }
    Ok(earliest)
}

fn manifest_version_at(repository: &GixRepository, commit: &larch_core::Commit) -> Option<String> {
    let blob = repository
        .blob_at_commit(&commit.id, &GitPath::new(PLUGIN_MANIFEST_PATH))
        .ok()??;
    let manifest: Value = serde_json::from_slice(&blob).ok()?;
    manifest
        .get("version")
        .and_then(Value::as_str)
        .map(str::to_owned)
}

/// `skip` only when the shipped fix provably post-dates every audited run.
///
/// Any parse failure or unknown on either side proposes, mirroring the
/// skill's normative unknown-proposes rule.
fn version_window_decision(shipped: Option<(u64, u64, u64)>, audited: &str) -> bool {
    let Some(shipped) = shipped else {
        return false;
    };
    let versions = audited
        .split(',')
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .map(strict_window_version)
        .collect::<Option<Vec<_>>>();
    match versions {
        Some(versions) if !versions.is_empty() => versions.iter().all(|version| shipped > *version),
        _ => false,
    }
}

/// Resolve the first plugin-version bump after an instant from local history.
#[must_use]
pub fn version_window(arguments: &[OsString]) -> ExitCode {
    if wants_help(arguments) {
        return audit_help("version-window");
    }
    const USAGE: &str = "usage: cli.py audit-runs version-window [-h] --repo-root REPO_ROOT --after AFTER [--audited-versions AUDITED_VERSIONS]";
    const PROGRAM: &str = "cli.py audit-runs version-window";
    let parsed = match parsed_or_usage_without_abbreviation(
        arguments,
        &["--repo-root", "--after", "--audited-versions"],
        USAGE,
        PROGRAM,
    ) {
        Ok(parsed) => parsed,
        Err(code) => return code,
    };
    let required = [
        ("--repo-root", parsed.value("--repo-root").is_some()),
        ("--after", parsed.value("--after").is_some()),
    ];
    if required.iter().any(|(_, present)| !present) {
        return usage_error(USAGE, PROGRAM, &missing(&required), 2);
    }
    let Some(after) = parse_utc_instant(&string_option(&parsed, "--after")) else {
        return usage_error(USAGE, PROGRAM, "--after must be an ISO-8601 instant", 2);
    };
    let root = PathBuf::from(string_option(&parsed, "--repo-root"));
    let repository = match GixRepository::discover(&root) {
        Ok(repository) => repository,
        Err(_) => {
            println!("VERSION_WINDOW_FAILED=true");
            println!("REASON=--repo-root is not a Git repository");
            return ExitCode::FAILURE;
        }
    };
    let bump = match version_window_bump(&repository, after.timestamp()) {
        Ok(bump) => bump,
        Err(reason) => {
            println!("VERSION_WINDOW_FAILED=true");
            println!("REASON={}", flat_error(&reason, AUDIT_ERROR_CHARS));
            return ExitCode::FAILURE;
        }
    };
    let shipped = bump.as_ref().and_then(|commit| {
        manifest_version_at(&repository, commit)
            .as_deref()
            .and_then(strict_window_version)
    });
    match &bump {
        Some(commit) => println!("BUMP_SHA={}", commit.id.to_hex()),
        None => println!("BUMP_SHA=none"),
    }
    match shipped {
        Some((major, minor, patch)) => println!("FIX_SHIPPED_VERSION={major}.{minor}.{patch}"),
        None => println!("FIX_SHIPPED_VERSION=unknown"),
    }
    if let Some(audited) = parsed.value("--audited-versions") {
        let skip = version_window_decision(shipped, &audited.to_string_lossy());
        println!("IN_SCOPE={}", !skip);
        println!("DECISION={}", if skip { "skip" } else { "propose" });
    }
    ExitCode::SUCCESS
}

async fn label_present_remote<S: GitHubService + ?Sized>(
    service: &S,
    cancellation: &Cancellation,
    repository: &larch_core::GitHubRepositoryRef,
    label: &str,
) -> Result<bool, String> {
    let labels = service
        .list_labels(repository, cancellation)
        .await
        .map_err(|error| format!("gh label list failed: {error}"))?;
    // GitHub's label search is substring recall; the precondition is an
    // exact, case-sensitive local match (G-Ext-3).
    Ok(labels.iter().any(|candidate| candidate.name == label))
}

/// Check whether one label exists in the repository, matched exactly.
#[must_use]
pub fn label_check(arguments: &[OsString]) -> ExitCode {
    if wants_help(arguments) {
        return audit_help("label-check");
    }
    const USAGE: &str = "usage: cli.py audit-runs label-check [-h] --repo REPO --label LABEL";
    const PROGRAM: &str = "cli.py audit-runs label-check";
    let parsed = match parsed_or_usage_without_abbreviation(
        arguments,
        &["--repo", "--label"],
        USAGE,
        PROGRAM,
    ) {
        Ok(parsed) => parsed,
        Err(code) => return code,
    };
    let required = [
        ("--repo", parsed.value("--repo").is_some()),
        ("--label", parsed.value("--label").is_some()),
    ];
    if required.iter().any(|(_, present)| !present) {
        return usage_error(USAGE, PROGRAM, &missing(&required), 2);
    }
    let repo = string_option(&parsed, "--repo");
    let repository = match repository_ref(&repo) {
        Ok(repository) => repository,
        Err(()) => {
            eprintln!("{PROGRAM}: --repo must be OWNER/REPO");
            return ExitCode::from(2);
        }
    };
    let label = string_option(&parsed, "--label");
    if label.is_empty() {
        return usage_error(USAGE, PROGRAM, "--label must be non-empty", 2);
    }
    let result = with_github_service(async |service, cancellation| {
        label_present_remote(service, cancellation, &repository, &label).await
    });
    match result {
        Ok(present) => {
            println!("LABEL_PRESENT={present}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            println!("LABEL_CHECK_FAILED=true");
            println!(
                "REASON={}",
                flat_error(&error.into_detail(), AUDIT_ERROR_CHARS)
            );
            ExitCode::FAILURE
        }
    }
}

async fn comment_remote(
    service: &OctocrabGitHubService,
    cancellation: &Cancellation,
    authorization: &LiveMutationRequest<'_>,
    repository: &larch_core::GitHubRepositoryRef,
    issue: u64,
    body: &str,
) -> Result<String, String> {
    let owner = IssueMutationOwner::new(service);
    owner
        .create_comment(cancellation, authorization, repository, issue, body)
        .await
        .map(|comment| comment.url)
        .map_err(|error| error.reason().to_owned())
}

/// Post one supplementary comment through the shared issue-mutation owner.
///
/// Requires `--operator-invoked` and refuses before any network access
/// without it. The owner applies authorization, outbound redaction, and an
/// exact comment read-back.
#[must_use]
pub fn comment(arguments: &[OsString]) -> ExitCode {
    if wants_help(arguments) {
        return audit_help("comment");
    }
    const USAGE: &str = "usage: cli.py audit-runs comment [-h] --repo REPO --issue ISSUE --body-file BODY_FILE [--operator-invoked]";
    const PROGRAM: &str = "cli.py audit-runs comment";
    let parsed = match parsed_or_usage(
        arguments,
        &["--repo", "--issue", "--body-file"],
        &["--operator-invoked"],
        USAGE,
        PROGRAM,
    ) {
        Ok(parsed) => parsed,
        Err(code) => return code,
    };
    let required = [
        ("--repo", parsed.value("--repo").is_some()),
        ("--issue", parsed.value("--issue").is_some()),
        ("--body-file", parsed.value("--body-file").is_some()),
    ];
    if required.iter().any(|(_, present)| !present) {
        return usage_error(USAGE, PROGRAM, &missing(&required), 2);
    }
    let authorization = authorization_request("", "", "", parsed.flag("--operator-invoked"));
    if let Err(reason) = authorized(&authorization) {
        println!("COMMENT_REFUSED=true");
        println!("REASON={}", format_mutation_refusal_reason(reason));
        return ExitCode::from(EXIT_MUTATION_REFUSED);
    }
    let repo = string_option(&parsed, "--repo");
    let repository = match repository_ref(&repo) {
        Ok(repository) => repository,
        Err(()) => {
            eprintln!("{PROGRAM}: --repo must be OWNER/REPO");
            return ExitCode::from(2);
        }
    };
    let Some(issue) = parse_uint(&string_option(&parsed, "--issue")) else {
        return usage_error(USAGE, PROGRAM, "--issue must be a positive integer", 2);
    };
    let body_file = PathBuf::from(string_option(&parsed, "--body-file"));
    let body = match fs::read_to_string(&body_file) {
        Ok(body) => body,
        Err(error) => {
            println!("COMMENT_FAILED=true");
            println!(
                "REASON={}",
                flat_error(
                    &format!("cannot read --body-file: {error}"),
                    AUDIT_ERROR_CHARS
                )
            );
            return ExitCode::FAILURE;
        }
    };
    let result = with_github_service(async |service, cancellation| {
        comment_remote(
            service,
            cancellation,
            &authorization,
            &repository,
            issue,
            &body,
        )
        .await
    });
    match result {
        Ok(url) => {
            println!("COMMENT_POSTED=true");
            println!("COMMENT_URL={}", single_line(&url));
            ExitCode::SUCCESS
        }
        Err(error) => {
            println!("COMMENT_FAILED=true");
            println!(
                "REASON={}",
                flat_error(&error.into_detail(), AUDIT_ERROR_CHARS)
            );
            ExitCode::FAILURE
        }
    }
}

fn valid_skill(skill: &str, program: &str) -> bool {
    if matches!(skill, "design" | "implement") {
        true
    } else {
        eprintln!("{program}: --skill must be design or implement (got: {skill})");
        false
    }
}

fn print_preflight(ok: bool, reason: &str, corpus: Option<&Path>) -> ExitCode {
    println!("PREFLIGHT_OK={ok}");
    println!("REASON={reason}");
    if let Some(root) = corpus {
        println!("CORPUS_ROOT={}", root.display());
    }
    ExitCode::SUCCESS
}

/// Verify the checkout, remote identity, concurrency window, and synchronized corpus.
#[must_use]
pub fn preflight(arguments: &[OsString]) -> ExitCode {
    if wants_help(arguments) {
        return audit_help("preflight");
    }
    const USAGE: &str =
        "usage: cli.py audit-runs preflight [-h] --skill SKILL [--repo REPO] [--allow-concurrent]";
    let parsed = match parsed_or_usage(
        arguments,
        &["--skill", "--repo"],
        &["--allow-concurrent"],
        USAGE,
        "cli.py audit-runs preflight",
    ) {
        Ok(parsed) => parsed,
        Err(code) => return code,
    };
    let skill = string_option(&parsed, "--skill");
    if parsed.value("--skill").is_none() {
        return usage_error(
            USAGE,
            "cli.py audit-runs preflight",
            &missing(&[("--skill", false)]),
            2,
        );
    }
    if !matches!(skill.as_str(), "design" | "implement") {
        return print_preflight(
            false,
            &format!("--skill must be design or implement (got: {skill})"),
            None,
        );
    }
    let repo_slug = {
        let value = string_option(&parsed, "--repo");
        if value.is_empty() {
            "zhupanov/larch".to_owned()
        } else {
            value
        }
    };
    let Ok(repo_ref) = repository_ref(&repo_slug) else {
        return print_preflight(false, "--repo must be OWNER/REPO", None);
    };
    let Ok(cwd) = env::current_dir() else {
        return print_preflight(false, "could not determine current directory", None);
    };
    if !fetch_origin_main(&cwd) {
        return print_preflight(false, "git fetch origin main failed", None);
    }
    let mut repository = match GixRepository::discover(&cwd) {
        Ok(repository) => repository,
        Err(_) => {
            return print_preflight(false, "local main or origin/main is not resolvable", None);
        }
    };
    let checked_out_main = matches!(
        repository.head(),
        Ok(Head::Symbolic { name, .. }) if name.as_bytes() == b"refs/heads/main"
    );
    if checked_out_main && !pull_origin_main(&cwd) {
        return print_preflight(
            false,
            "git pull --ff-only origin main failed (working tree may be dirty or branch is not ff-only)",
            None,
        );
    }
    if checked_out_main {
        repository = match GixRepository::discover(&cwd) {
            Ok(repository) => repository,
            Err(_) => {
                return print_preflight(false, "local main or origin/main is not resolvable", None);
            }
        };
    }
    let main = repository.resolve_revision(&Revision::new("main"));
    let origin = repository.resolve_revision(&Revision::new("origin/main"));
    let (Ok(main), Ok(origin)) = (main, origin) else {
        return print_preflight(false, "local main or origin/main is not resolvable", None);
    };
    if main != origin {
        return print_preflight(
            false,
            "local main is stale or diverged from origin/main",
            None,
        );
    }
    if repository
        .local_status(&larch_core::StatusOptions::default())
        .map_or(true, |status| status.is_dirty())
    {
        return print_preflight(false, "working tree is dirty", None);
    }
    let remote = match resolve_remote_repo("origin", Some(&repository)) {
        RemoteRepoResult::Ok { repo } => repo,
        RemoteRepoResult::Usage | RemoteRepoResult::ParseFailure => {
            return print_preflight(
                false,
                "could not determine repo identity (remote=<empty> gh=<empty>)",
                None,
            );
        }
    };
    let allow_concurrent = parsed.flag("--allow-concurrent");
    let remote_check = with_github_service(async |service, cancellation| {
        let metadata = service
            .repository(&repo_ref, cancellation)
            .await
            .map_err(|error| error.to_string())?;
        if metadata.name_with_owner != remote {
            return Err(format!(
                "repo mismatch: normalized_remote_origin={remote} gh_repo_identity={} (expected clone to match gh repo view {repo_slug})",
                metadata.name_with_owner
            ));
        }
        if !allow_concurrent {
            // The legacy probe deliberately degrades a listing failure to an
            // empty recent-report set. Repo identity remains fail-closed above.
            let issues = service
                .list_issues(
                    &GitHubIssueList {
                        repo: repo_ref.clone(),
                        state: GitHubIssueState::All,
                        labels: vec!["audit-report".to_owned()],
                        limit: 50,
                        // The probe degrades to an empty set on failure, so a
                        // bounded partial snapshot is the intended contract.
                        mode: GitHubIssueListMode::BoundedPartial,
                        body_mode: GitHubIssueBodyMode::Include,
                    },
                    cancellation,
                )
                .await
                .map(|listed| listed.issues)
                .unwrap_or_default();
            let cutoff = (Utc::now() - Duration::minutes(5)).to_rfc3339();
            if issues
                .iter()
                .any(|issue| !issue.is_pull_request && issue.created_at > cutoff)
            {
                return Err("audit-report filed within the 5-minute concurrency window; use --allow-concurrent to override".to_owned());
            }
        }
        Ok(())
    });
    if let Err(error) = remote_check {
        return print_preflight(false, &single_line(&error.into_detail()), None);
    }
    let corpus = match synchronized_corpus_root(&cwd) {
        Ok(root) => root,
        Err(error) => {
            return print_preflight(
                false,
                &format!(
                    "run-log corpus synchronization failed: {}",
                    single_line(&error)
                ),
                None,
            );
        }
    };
    print_preflight(true, "", Some(&corpus))
}

/// Print the timestamp used in audit titles from a single UTC instant.
#[must_use]
pub fn pacific_timestamp(arguments: &[OsString]) -> ExitCode {
    if !arguments.is_empty() {
        eprintln!("audit-pacific-timestamp.sh: unexpected argument(s)");
        return ExitCode::FAILURE;
    }
    println!("PACIFIC_TIMESTAMP={}", render_pacific_timestamp(Utc::now()));
    println!("PACIFIC_TIMESTAMP_SOURCE=tz_america_los_angeles");
    ExitCode::SUCCESS
}

fn render_pacific_timestamp(now: DateTime<Utc>) -> String {
    let offset_hours = pacific_offset_hours(now);
    let local = now + Duration::hours(i64::from(offset_hours));
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}{:+03}:00",
        local.year(),
        local.month(),
        local.day(),
        local.hour(),
        local.minute(),
        offset_hours
    )
}

fn pacific_offset_hours(now: DateTime<Utc>) -> i32 {
    let year = now.year();
    let march = second_sunday(year, 3);
    let november = first_sunday(year, 11);
    let daylight_start = march.and_hms_opt(10, 0, 0).expect("valid DST start");
    let daylight_end = november.and_hms_opt(9, 0, 0).expect("valid DST end");
    if now.naive_utc() >= daylight_start && now.naive_utc() < daylight_end {
        -7
    } else {
        -8
    }
}

fn first_sunday(year: i32, month: u32) -> NaiveDate {
    let first = NaiveDate::from_ymd_opt(year, month, 1).expect("valid calendar month");
    let days = (7 + Weekday::Sun.num_days_from_sunday() as i64
        - first.weekday().num_days_from_sunday() as i64)
        % 7;
    first + Duration::days(days)
}

fn second_sunday(year: i32, month: u32) -> NaiveDate {
    first_sunday(year, month) + Duration::days(7)
}

#[derive(Debug)]
struct ResolvedPrs {
    implicit: bool,
    prior: String,
    numbers: Vec<u64>,
    echo: String,
}

fn print_resolve_error(message: &str) -> ExitCode {
    println!("IMPLICIT_SINCE_LAST_AUDIT=false");
    println!("PRIOR_REPORT_NUMBER=");
    println!("PR_LIST=");
    println!("PR_COUNT=0");
    println!("RESOLVED_ECHO=");
    println!("ERROR={}", clean_controls(message));
    ExitCode::SUCCESS
}

fn print_resolved(output: ResolvedPrs) -> ExitCode {
    println!("IMPLICIT_SINCE_LAST_AUDIT={}", output.implicit);
    println!("PRIOR_REPORT_NUMBER={}", output.prior);
    println!(
        "PR_LIST={}",
        output
            .numbers
            .iter()
            .map(u64::to_string)
            .collect::<Vec<_>>()
            .join(",")
    );
    println!("PR_COUNT={}", output.numbers.len());
    println!("RESOLVED_ECHO={}", clean_controls(&output.echo));
    println!("ERROR=");
    ExitCode::SUCCESS
}

/// Resolve audit PRs without allowing an untyped GitHub API escape hatch.
#[must_use]
pub fn resolve_prs(arguments: &[OsString]) -> ExitCode {
    if wants_help(arguments) {
        return audit_help("resolve-prs");
    }
    const USAGE: &str = "usage: cli.py audit-runs resolve-prs [-h] --skill SKILL [--repo REPO] [--verbal-description VERBAL_DESCRIPTION]";
    let parsed = match parsed_or_usage(
        arguments,
        &["--skill", "--repo", "--verbal-description"],
        &[],
        USAGE,
        "cli.py audit-runs resolve-prs",
    ) {
        Ok(parsed) => parsed,
        Err(code) => return code,
    };
    if parsed.value("--skill").is_none() {
        return usage_error(
            USAGE,
            "cli.py audit-runs resolve-prs",
            &missing(&[("--skill", false)]),
            2,
        );
    }
    let skill = string_option(&parsed, "--skill");
    if !valid_skill(&skill, "audit-resolve-prs.sh") {
        return ExitCode::FAILURE;
    }
    let repo_slug = {
        let value = string_option(&parsed, "--repo");
        if value.is_empty() {
            "zhupanov/larch".to_owned()
        } else {
            value
        }
    };
    let Ok(repo) = repository_ref(&repo_slug) else {
        return print_resolve_error("--repo must be OWNER/REPO");
    };
    let verbal = string_option(&parsed, "--verbal-description")
        .trim()
        .to_owned();
    let result = with_github_service(async |service, cancellation| {
        resolve_prs_remote(service, cancellation, &repo, &skill, &verbal).await
    });
    match result {
        Ok(output) => print_resolved(output),
        Err(error) => print_resolve_error(&error.into_detail()),
    }
}

async fn resolve_prs_remote<S: AuditRunsService + GitHubService + ?Sized>(
    service: &S,
    cancellation: &larch_adapters::runtime::Cancellation,
    repo: &larch_core::GitHubRepositoryRef,
    skill: &str,
    verbal: &str,
) -> Result<ResolvedPrs, String> {
    let list = |scope: String| async move {
        service
            .list_audit_merged_main_pull_requests(cancellation, repo.owner(), repo.name())
            .await
            .map_err(|_| {
                format!(
                    "merged PR listing/filter failed during {scope}: gh api failed or returned invalid merged PR data"
                )
            })
    };
    if verbal.is_empty() || verbal == "since last audit" {
        let implicit = verbal.is_empty();
        let issues = service
            .list_issues(
                &GitHubIssueList {
                    repo: repo.clone(),
                    state: GitHubIssueState::All,
                    labels: vec!["audit-report".to_owned()],
                    limit: service.transport_policy().limits().items(),
                    // Resolving the newest prior report must see every prior, so
                    // an over-bound corpus fails closed rather than mis-ordering.
                    mode: GitHubIssueListMode::Exhaustive,
                    body_mode: GitHubIssueBodyMode::Include,
                },
                cancellation,
            )
            .await
            .map_err(|_| format!("gh issue list failed while resolving prior audit-report issue for --skill={skill}"))?;
        let mut reports = issues
            .issues
            .into_iter()
            .filter(|issue| !issue.is_pull_request && matches_audit_title(skill, &issue.title))
            .collect::<Vec<_>>();
        reports.sort_by(|left, right| right.created_at.cmp(&left.created_at));
        let Some(prior) = reports.first() else {
            return Err(format!(
                "no prior audit-report issue found for --skill={skill}"
            ));
        };
        let body = service
            .issue(repo, prior.number, cancellation)
            .await
            .map_err(|_| {
                format!(
                    "gh issue view failed for prior audit-report #{}",
                    prior.number
                )
            })?
            .body;
        let Some(last_pr) = frontmatter_last_pr(&body) else {
            return Err(format!(
                "prior audit-report #{} has malformed or missing frontmatter (audited_pr_range.last)",
                prior.number
            ));
        };
        let last = service
            .audit_pull_request(cancellation, repo.owner(), repo.name(), last_pr)
            .await
            .map_err(|_| format!("could not get mergedAt for prior PR #{last_pr}"))?;
        let Some(merged_at) = last.merged_at else {
            return Err(format!("could not get mergedAt for prior PR #{last_pr}"));
        };
        let numbers = filtered_prs(
            skill,
            &list("since last audit".to_owned()).await?,
            Some(&merged_at),
        );
        if numbers.is_empty() {
            return Err(format!(
                "no new PRs merged after prior audit (last PR: #{last_pr}, skill={skill})"
            ));
        }
        let refs = numbers
            .iter()
            .map(|number| format!("#{number}"))
            .collect::<Vec<_>>()
            .join(", ");
        let extra = if implicit {
            ", implicit default: empty/omitted positional"
        } else {
            ""
        };
        return Ok(ResolvedPrs {
            implicit,
            prior: prior.number.to_string(),
            numbers,
            echo: format!(
                "Resolved since last audit (--skill={skill}{extra}) to: [{refs}]. Proceeding."
            ),
        });
    }
    if let Some(captures) = Regex::new(r"^last\s+([0-9]+)\s+PRs?$")
        .expect("static last regex")
        .captures(verbal)
    {
        let count_text = &captures[1];
        let count = count_text.parse::<usize>().unwrap_or(usize::MAX);
        let all = filtered_prs(skill, &list(format!("last {count_text} PRs")).await?, None);
        let numbers = all
            .into_iter()
            .rev()
            .take(count)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>();
        if numbers.is_empty() {
            return Err(format!(
                "empty PR list after merge-time sort (last {count_text} PRs, skill={skill})"
            ));
        }
        let refs = numbers
            .iter()
            .map(|number| format!("#{number}"))
            .collect::<Vec<_>>()
            .join(", ");
        return Ok(ResolvedPrs {
            implicit: false,
            prior: String::new(),
            numbers,
            echo: format!(
                "Resolved last {count_text} PRs (--skill={skill}) to: [{refs}]. Proceeding."
            ),
        });
    }
    if let Some(raw) = verbal.strip_prefix("since ") {
        let instant = raw.trim();
        if !valid_full_instant(instant) {
            return Err(format!(
                "since <ISO> must be a full instant (YYYY-MM-DDThh:mm[:ss][.frac][Z|±hh:mm]); got: {instant}"
            ));
        }
        let numbers = filtered_prs(
            skill,
            &list(format!("since {instant}")).await?,
            Some(instant),
        );
        if numbers.is_empty() {
            return Err(format!(
                "no PRs merged after {instant} (or empty gh result, skill={skill})"
            ));
        }
        let refs = numbers
            .iter()
            .map(|number| format!("#{number}"))
            .collect::<Vec<_>>()
            .join(", ");
        return Ok(ResolvedPrs {
            implicit: false,
            prior: String::new(),
            numbers,
            echo: format!("Resolved since {instant} (--skill={skill}) to: [{refs}]. Proceeding."),
        });
    }
    if let Some(number) = explicit_pr_number(verbal) {
        let pull = service
            .audit_pull_request(cancellation, repo.owner(), repo.name(), number)
            .await
            .map_err(|_| format!("could not resolve PR #{number} title for --skill={skill}"))?;
        if pull.title.is_empty() {
            return Err(format!(
                "could not resolve PR #{number} title for --skill={skill}"
            ));
        }
        if !matches_skill(skill, &pull.title) {
            return Err(format!("PR #{number} title does not match --skill={skill}"));
        }
        return Ok(ResolvedPrs {
            implicit: false,
            prior: String::new(),
            numbers: vec![number],
            echo: format!("Resolved {verbal} (--skill={skill}) to: [#{number}]. Proceeding."),
        });
    }
    Err(format!("unrecognized verbal description: {verbal}"))
}

fn filtered_prs(skill: &str, pulls: &[AuditPullRequest], after: Option<&str>) -> Vec<u64> {
    pulls
        .iter()
        .filter(|pull| matches_skill(skill, &pull.title))
        .filter(|pull| {
            after.is_none_or(|instant| {
                pull.merged_at
                    .as_deref()
                    .is_some_and(|merged| merged > instant)
            })
        })
        .map(|pull| pull.number)
        .collect()
}

fn matches_skill(skill: &str, title: &str) -> bool {
    if skill == "design" {
        DESIGN_TITLE_RE.is_match(title)
    } else {
        !DESIGN_TITLE_RE.is_match(title)
    }
}

fn matches_audit_title(skill: &str, title: &str) -> bool {
    (skill == "implement" && AUDIT_IMPLEMENT_TITLE_RE.is_match(title))
        || (skill == "design" && AUDIT_DESIGN_TITLE_RE.is_match(title))
}

fn frontmatter_last_pr(body: &str) -> Option<u64> {
    let frontmatter = top_frontmatter(body);
    Regex::new(r#"(?s)audited_pr_range:.*?\n\s*last:\s*['"]?([0-9]+)['"]?"#)
        .expect("static frontmatter regex")
        .captures(frontmatter)
        .and_then(|found| found[1].parse().ok())
}

fn valid_full_instant(value: &str) -> bool {
    Regex::new(r"^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}(:[0-9]{2})?(\.[0-9]+)?(Z|[+-][0-9]{2}:[0-9]{2})$")
        .expect("static instant regex").is_match(value)
}

fn explicit_pr_number(value: &str) -> Option<u64> {
    Regex::new(r"^(?:PR\s+)?#([0-9]+)$")
        .expect("static explicit PR regex")
        .captures(value)
        .and_then(|found| found[1].parse().ok())
}

/// Map each comma-separated PR number to a run-log row.
#[must_use]
pub fn map_runs(arguments: &[OsString]) -> ExitCode {
    if wants_help(arguments) {
        return audit_help("map-runs");
    }
    const USAGE: &str = "usage: cli.py audit-runs map-runs [-h] --skill SKILL --pr-list PR_LIST [--repo REPO] [--log-root LOG_ROOT]";
    let parsed = match parsed_or_usage(
        arguments,
        &["--skill", "--pr-list", "--repo", "--log-root"],
        &[],
        USAGE,
        "cli.py audit-runs map-runs",
    ) {
        Ok(parsed) => parsed,
        Err(code) => return code,
    };
    let skill_present = parsed.value("--skill").is_some();
    let prs_present = parsed.value("--pr-list").is_some();
    if !skill_present || !prs_present {
        return usage_error(
            USAGE,
            "cli.py audit-runs map-runs",
            &missing(&[("--skill", skill_present), ("--pr-list", prs_present)]),
            2,
        );
    }
    let skill = string_option(&parsed, "--skill");
    if !valid_skill(&skill, "audit-map-runs.sh") {
        return ExitCode::FAILURE;
    }
    let supplied_root = string_option(&parsed, "--log-root");
    let root_text = normalized_log_root(&supplied_root, &skill);
    let is_wrong_skill_root = Regex::new(r"(^|/)larch-logs/(design|implement)$")
        .expect("static log root regex")
        .is_match(&root_text)
        && !root_text.ends_with(&format!("/{skill}"))
        && root_text != format!("larch-logs/{skill}");
    if !supplied_root.is_empty() && is_wrong_skill_root {
        eprintln!(
            "audit-map-runs.sh: --log-root must be larch-logs/{skill} when --skill={skill} (got: {supplied_root})"
        );
        return ExitCode::FAILURE;
    }
    let root = PathBuf::from(&root_text);
    if !root.is_dir() {
        eprintln!("audit-map-runs.sh: log root not found: {root_text}");
        return ExitCode::FAILURE;
    }
    let repo_slug = {
        let value = string_option(&parsed, "--repo");
        if value.is_empty() {
            "zhupanov/larch".to_owned()
        } else {
            value
        }
    };
    let Ok(repo) = repository_ref(&repo_slug) else {
        eprintln!("audit-map-runs.sh: --repo must be OWNER/REPO");
        return ExitCode::FAILURE;
    };
    let tokens = string_option(&parsed, "--pr-list")
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    for token in &tokens {
        if parse_uint(token).is_none() {
            eprintln!(
                "audit-map-runs.sh: skipping invalid PR token in --pr-list (non-integer): {token}"
            );
        }
    }
    let numbers = tokens
        .iter()
        .filter_map(|token| parse_uint(token))
        .collect::<Vec<_>>();
    if numbers.is_empty() {
        return ExitCode::SUCCESS;
    }
    let queried = with_github_service(async |service, cancellation| {
        let mut rows = BTreeMap::new();
        for number in &numbers {
            rows.insert(
                *number,
                AuditRunsService::audit_pull_request(
                    service,
                    cancellation,
                    repo.owner(),
                    repo.name(),
                    *number,
                )
                .await
                .map_err(|error| error.to_string()),
            );
        }
        Ok(rows)
    });
    let pulls = match queried {
        Ok(rows) => rows,
        Err(error) => numbers
            .iter()
            .map(|number| (*number, Err(error.clone().into_detail())))
            .collect(),
    };
    let corpus = RunLogCorpus::new(&root);
    let runs = corpus.safe_child_run_directories();
    for number in numbers {
        let Some(result) = pulls.get(&number) else {
            continue;
        };
        match result {
            Err(error) => {
                eprintln!(
                    "audit-map-runs.sh: MAP_GH_PR_VIEW_FAILED=true PR={number} FIELD={} REASON={}",
                    if skill == "design" { "title" } else { "body" },
                    clean_reason(error)
                );
                println!("{number}\t\t\t\t");
            }
            Ok(pull) if skill == "design" => {
                let run_id = design_run_id(&pull.title).unwrap_or_default();
                let manifest =
                    (!run_id.is_empty()).then(|| root.join(&run_id).join("manifest.json"));
                let (started, version, _) =
                    manifest.as_deref().map(manifest_fields).unwrap_or_default();
                println!("{number}\t{run_id}\t{started}\t{version}\t");
            }
            Ok(pull) => {
                let closes = closing_issue(&pull.body);
                let mut candidate = closes
                    .as_deref()
                    .map_or_else(Vec::new, |issue| parent_issue_candidates(&runs, issue));
                candidate
                    .sort_by(|left, right| manifest_epoch(right).total_cmp(&manifest_epoch(left)));
                let mut row = (
                    String::new(),
                    String::new(),
                    String::new(),
                    closes.unwrap_or_default(),
                );
                if let Some(best) = candidate.first() {
                    let epoch = manifest_epoch(best);
                    let tied = candidate
                        .iter()
                        .filter(|path| manifest_epoch(path) == epoch)
                        .collect::<Vec<_>>();
                    if tied.len() > 1 {
                        let mut names = tied
                            .iter()
                            .filter_map(|path| path.file_name())
                            .map(|name| name.to_string_lossy().into_owned())
                            .collect::<Vec<_>>();
                        names.sort();
                        eprintln!(
                            "audit-map-runs.sh: MAP_PARENT_ISSUE_AMBIGUOUS=true ISSUE_NUMBER={} RUNS={}",
                            row.3,
                            names.join(",")
                        );
                    } else {
                        let (started, version, close_from_manifest) =
                            manifest_fields(&best.join("manifest.json"));
                        row = (
                            run_name(best),
                            started,
                            version,
                            if row.3.is_empty() {
                                close_from_manifest
                            } else {
                                row.3
                            },
                        );
                    }
                }
                if row.0.is_empty() {
                    let mut manifests = runs
                        .iter()
                        .filter(|run| {
                            let manifest = run.join("manifest.json");
                            manifest.is_file()
                                && !manifest.is_symlink()
                                && manifest_json(&manifest).is_some_and(|value| {
                                    value_string(&value, "pr_number") == number.to_string()
                                })
                        })
                        .collect::<Vec<_>>();
                    manifests.sort_by(|left, right| {
                        manifest_epoch(right).total_cmp(&manifest_epoch(left))
                    });
                    if let Some(best) = manifests.first() {
                        let (started, version, close_from_manifest) =
                            manifest_fields(&best.join("manifest.json"));
                        row = (
                            run_name(best),
                            started,
                            version,
                            if row.3.is_empty() {
                                close_from_manifest
                            } else {
                                row.3
                            },
                        );
                    }
                }
                println!("{number}\t{}\t{}\t{}\t{}", row.0, row.1, row.2, row.3);
            }
        }
    }
    ExitCode::SUCCESS
}

fn normalized_log_root(supplied: &str, skill: &str) -> String {
    if supplied.is_empty() {
        return format!("larch-logs/{skill}");
    }
    if supplied == "larch-logs" || supplied.ends_with("/larch-logs") {
        return format!("{supplied}/{skill}");
    }
    supplied.to_owned()
}

fn design_run_id(title: &str) -> Option<String> {
    DESIGN_ID_RE
        .captures(title)
        .map(|captures| captures[1].to_owned())
}

fn closing_issue(body: &str) -> Option<String> {
    for keyword in ["Closes", "Fixes", "Resolves"] {
        let pattern = Regex::new(&format!(r"(?i){}\s+#([0-9]+)", regex::escape(keyword)))
            .expect("keyword regex");
        let numbers = pattern
            .captures_iter(body)
            .map(|captures| captures[1].to_owned())
            .collect::<BTreeSet<_>>();
        if numbers.len() == 1 {
            return numbers.first().cloned();
        }
        if numbers.len() > 1 {
            eprintln!("audit-map-runs.sh: MAP_PR_BODY_CLOSING_AMBIGUOUS=true KEYWORD={keyword}");
            return None;
        }
    }
    None
}

fn parent_issue_candidates(runs: &[PathBuf], closes: &str) -> Vec<PathBuf> {
    runs.iter()
        .filter(|run| parent_issue_number(&run.join("parent-issue.md")).as_deref() == Some(closes))
        .cloned()
        .collect()
}

fn parent_issue_number(path: &Path) -> Option<String> {
    let text = read_text(path)?;
    Regex::new(r"(?m)^ISSUE_NUMBER=([0-9]+)\s*$")
        .expect("parent issue regex")
        .captures(&text)
        .map(|captures| captures[1].to_owned())
}

fn run_name(path: &Path) -> String {
    path.file_name()
        .map_or_else(String::new, |name| name.to_string_lossy().into_owned())
}

fn manifest_epoch(run: &Path) -> f64 {
    manifest_json(&run.join("manifest.json"))
        .and_then(|value| {
            value
                .get("started_at")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .and_then(|value| DateTime::parse_from_rfc3339(&value).ok())
        .map_or(-9e18, |value| value.timestamp() as f64)
}

fn manifest_fields(path: &Path) -> (String, String, String) {
    let Some(value) = manifest_json(path) else {
        return Default::default();
    };
    (
        value_string(&value, "started_at"),
        value_string(&value, "larch_version"),
        value_string(&value, "closes_issue"),
    )
}

/// Aggregate all direct `scan-results-*.ndjson` files into stable report keys.
#[must_use]
pub fn compute_counters(arguments: &[OsString]) -> ExitCode {
    if wants_help(arguments) {
        return audit_help("compute-counters");
    }
    const USAGE: &str = "usage: cli.py audit-runs compute-counters [-h] --scan-results-dir SCAN_RESULTS_DIR [--prior-frontmatter PRIOR_FRONTMATTER]";
    let parsed = match parsed_or_usage(
        arguments,
        &["--scan-results-dir", "--prior-frontmatter"],
        &[],
        USAGE,
        "cli.py audit-runs compute-counters",
    ) {
        Ok(parsed) => parsed,
        Err(code) => return code,
    };
    if parsed.value("--scan-results-dir").is_none() {
        return usage_error(
            USAGE,
            "cli.py audit-runs compute-counters",
            &missing(&[("--scan-results-dir", false)]),
            2,
        );
    }
    let directory = PathBuf::from(string_option(&parsed, "--scan-results-dir"));
    if !directory.is_dir() {
        eprintln!(
            "audit-compute-counters.sh: directory not found: {}",
            directory.display()
        );
        return ExitCode::FAILURE;
    }
    let prior_path = string_option(&parsed, "--prior-frontmatter");
    let frontmatter = if prior_path.is_empty() {
        String::new()
    } else {
        read_text(Path::new(&prior_path))
            .map_or_else(String::new, |text| top_frontmatter(&text).to_owned())
    };
    let prior = CounterValues {
        exon: prior_value(&frontmatter, "exon_misclassifications"),
        mangled: prior_value(&frontmatter, "oos_categories_mangled"),
        clean: prior_value(&frontmatter, "oos_categories_clean"),
        blank: prior_value(&frontmatter, "oos_categories_blank"),
        ns: prior_value(&frontmatter, "ns_retries_cursor_specialist").max(prior_value(
            &frontmatter,
            "ns_retries_cursor_specialist_launches",
        )),
        changelog: prior_value(&frontmatter, "changelog_rebase_conflicts"),
        ..CounterValues::default()
    };
    let mut totals = CounterValues::default();
    let mut files = 0_u64;
    let mut partial = false;
    let Ok(entries) = fs::read_dir(&directory) else {
        return ExitCode::FAILURE;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !name.starts_with("scan-results-") || !name.ends_with(".ndjson") || !path.is_file() {
            continue;
        }
        files += 1;
        for row in ndjson_rows(&path).0 {
            match value_string(&row, "scan").as_str() {
                "exon-misclassification" => totals.exon += number_value(row.get("count")),
                "oos-category-mangle" => totals.mangled += number_value(row.get("count")),
                "category-stats" => {
                    let incomplete = row.get("partial_data") == Some(&Value::Bool(true));
                    partial |= incomplete;
                    if !(incomplete
                        && value_string(&row, "detail")
                            .contains("review-findings-full.jsonl not found"))
                    {
                        totals.clean += number_value(row.get("canonical"));
                        totals.blank += number_value(row.get("oos_blank"));
                    }
                }
                "ns-retry-sidecars" if value_string(&row, "result") == "fail" => {
                    totals.ns += number_value(row.get("count"))
                }
                "ns-retry-sidecars" if value_string(&row, "result") == "skip" => {
                    totals.ns_skipped += 1
                }
                "changelog-rebase-conflicts" => totals.changelog += number_value(row.get("count")),
                "guideline-ship-outcome" if value_string(&row, "result") == "pass" => {
                    totals.guideline_runs += 1;
                    match value_string(&row, "outcome").as_str() {
                        "pinned" => totals.guideline_pinned += 1,
                        "clean" => totals.guideline_clean += 1,
                        "dropped" => totals.guideline_dropped += 1,
                        _ => {}
                    }
                }
                "invariant-ship-outcome" if value_string(&row, "result") == "pass" => {
                    totals.invariant_runs += 1;
                    match value_string(&row, "outcome").as_str() {
                        "violation" => totals.invariant_violation += 1,
                        "clean" => totals.invariant_clean += 1,
                        "dropped" => totals.invariant_dropped += 1,
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }
    let values = [
        ("SCAN_FILES_FOUND", files),
        ("EXON_MISCLASSIFICATIONS", prior.exon + totals.exon),
        ("EXON_DELTA", totals.exon),
        ("OOS_CATEGORIES_MANGLED", prior.mangled + totals.mangled),
        ("OOS_MANGLED_DELTA", totals.mangled),
        ("OOS_CATEGORIES_CLEAN", prior.clean + totals.clean),
        ("OOS_CLEAN_DELTA", totals.clean),
        ("OOS_CATEGORIES_BLANK", prior.blank + totals.blank),
        ("OOS_BLANK_DELTA", totals.blank),
        ("NS_RETRIES_CURSOR_SPECIALIST", prior.ns + totals.ns),
        ("NS_RETRIES_DELTA", totals.ns),
        ("NS_RETRIES_SKIPPED_RUNS", totals.ns_skipped),
        (
            "CHANGELOG_REBASE_CONFLICTS",
            prior.changelog + totals.changelog,
        ),
        ("CHANGELOG_DELTA", totals.changelog),
        ("GUIDELINE_OUTCOME_RUNS", totals.guideline_runs),
        ("GUIDELINE_OUTCOME_PINNED", totals.guideline_pinned),
        ("GUIDELINE_OUTCOME_CLEAN", totals.guideline_clean),
        ("GUIDELINE_OUTCOME_DROPPED", totals.guideline_dropped),
        (
            "GUIDELINE_DROP_RATE_BPS",
            if totals.guideline_runs == 0 {
                0
            } else {
                totals.guideline_dropped * 10_000 / totals.guideline_runs
            },
        ),
        ("INVARIANT_OUTCOME_RUNS", totals.invariant_runs),
        ("INVARIANT_OUTCOME_VIOLATION", totals.invariant_violation),
        ("INVARIANT_OUTCOME_CLEAN", totals.invariant_clean),
        ("INVARIANT_OUTCOME_DROPPED", totals.invariant_dropped),
    ];
    for (key, value) in values {
        println!("{key}={value}");
    }
    println!("CATEGORY_STATS_PARTIAL={partial}");
    ExitCode::SUCCESS
}

#[derive(Default)]
struct CounterValues {
    exon: u64,
    mangled: u64,
    clean: u64,
    blank: u64,
    ns: u64,
    ns_skipped: u64,
    changelog: u64,
    guideline_runs: u64,
    guideline_pinned: u64,
    guideline_clean: u64,
    guideline_dropped: u64,
    invariant_runs: u64,
    invariant_violation: u64,
    invariant_clean: u64,
    invariant_dropped: u64,
}

fn prior_value(text: &str, key: &str) -> u64 {
    Regex::new(&format!(r"(?m)^\s*{}:\s*([0-9]+)\s*$", regex::escape(key)))
        .expect("prior counter regex")
        .captures(text)
        .and_then(|found| found[1].parse().ok())
        .unwrap_or(0)
}

fn top_frontmatter(text: &str) -> &str {
    let mut offset = 0;
    let mut frontmatter_start = None;
    for line in text.split_inclusive('\n') {
        let line_start = offset;
        offset += line.len();
        if frontmatter_start.is_none() {
            if line.trim() != "---" {
                return "";
            }
            frontmatter_start = Some(offset);
            continue;
        }
        if line.trim() == "---" {
            let content = &text[frontmatter_start.expect("set after opening marker")..line_start];
            let content = content.strip_suffix('\n').unwrap_or(content);
            return content.strip_suffix('\r').unwrap_or(content);
        }
    }
    ""
}

fn number_value(value: Option<&Value>) -> u64 {
    value
        .and_then(|value| match value {
            Value::Number(number) => number.as_u64(),
            Value::String(text) => text.trim().parse().ok(),
            _ => None,
        })
        .unwrap_or(0)
}

/// Scan one archived run and emit one compact JSON object per requested scan.
#[must_use]
pub fn scan_run(arguments: &[OsString]) -> ExitCode {
    if wants_help(arguments) {
        return audit_help("scan-run");
    }
    const USAGE: &str = "usage: cli.py audit-runs scan-run [-h] --skill SKILL [--run-dir RUN_DIR] --pr PR --scans-tsv SCANS_TSV [--required-files-tsv REQUIRED_FILES_TSV] [--current-version CURRENT_VERSION]";
    let parsed = match parsed_or_usage(
        arguments,
        &[
            "--skill",
            "--run-dir",
            "--pr",
            "--scans-tsv",
            "--required-files-tsv",
            "--current-version",
        ],
        &[],
        USAGE,
        "cli.py audit-runs scan-run",
    ) {
        Ok(parsed) => parsed,
        Err(code) => return code,
    };
    let required = [
        ("--skill", parsed.value("--skill").is_some()),
        ("--pr", parsed.value("--pr").is_some()),
        ("--scans-tsv", parsed.value("--scans-tsv").is_some()),
    ];
    if required.iter().any(|(_, present)| !present) {
        return usage_error(USAGE, "cli.py audit-runs scan-run", &missing(&required), 2);
    }
    let skill = string_option(&parsed, "--skill");
    if !valid_skill(&skill, "audit-runs scan-run") {
        return ExitCode::FAILURE;
    }
    let pr_text = string_option(&parsed, "--pr");
    let Some(pr) = parse_uint(&pr_text) else {
        emit(
            json!({"scan":"audit-scan-run-args","pr":Value::Null,"result":"error","detail":format!("--pr must be a non-empty decimal integer: {pr_text}")}),
        );
        return ExitCode::FAILURE;
    };
    let run_text = string_option(&parsed, "--run-dir");
    if run_text.is_empty() || !Path::new(&run_text).is_dir() {
        emit(
            json!({"scan":"run-dir-missing","pr":pr,"incomplete":true,"result":"error","detail":format!("run-dir not found: {run_text}")}),
        );
        return ExitCode::FAILURE;
    }
    let run_dir = match fs::canonicalize(&run_text) {
        Ok(path) => path,
        Err(_) => {
            emit(
                json!({"scan":"run-dir-missing","pr":pr,"incomplete":true,"result":"error","detail":format!("run-dir not found: {run_text}")}),
            );
            return ExitCode::FAILURE;
        }
    };
    if let Some(detail) = invalid_run_directory(&run_dir, &skill) {
        emit(
            json!({"scan":"run-dir-invalid","pr":pr,"incomplete":true,"result":"error","detail":detail}),
        );
        return ExitCode::FAILURE;
    }
    let scans_path = PathBuf::from(string_option(&parsed, "--scans-tsv"));
    let Some(scans_text) = read_text(&scans_path) else {
        emit(
            json!({"scan":"scans-registry","pr":pr,"result":"error","detail":format!("scans-tsv not found: {}", scans_path.display())}),
        );
        return ExitCode::FAILURE;
    };
    let names = scans_text
        .lines()
        .filter_map(|line| {
            (!line.is_empty() && !line.starts_with('#') && !line.starts_with("name"))
                .then(|| line.split('\t').next().unwrap_or_default().to_owned())
        })
        .collect::<Vec<_>>();
    let required_path = string_option(&parsed, "--required-files-tsv");
    let current_version = string_option(&parsed, "--current-version");
    let (raw_rows, jsonl_error) = ndjson_rows(&run_dir.join("review-findings-full.jsonl"));
    let review_present = run_dir.join("review-findings-full.jsonl").is_file();
    let rows = effective_review_rows(&run_dir, &skill, raw_rows, jsonl_error);
    let has_review_rows = !rows.is_empty();
    let (signals_present, signals) = round_signals(&run_dir);
    let mut mangled: Option<Vec<&Value>> = None;
    let mut exit = ExitCode::SUCCESS;
    for name in names {
        let result = match name.as_str() {
            "required-file-presence" => required_file_scan(
                &run_dir,
                pr,
                if required_path.is_empty() {
                    None
                } else {
                    Some(Path::new(&required_path))
                },
            ),
            "exon-misclassification" => exon_scan(&run_dir, pr),
            "oos-category-mangle" => {
                if !review_present && !has_review_rows {
                    json!({"scan":name,"pr":pr,"result":"skip","detail":"review-findings-full.jsonl not found"})
                } else if jsonl_error {
                    json!({"scan":name,"pr":pr,"result":"error","detail":"jq failed (oos-category-mangle): parse error"})
                } else {
                    let bad = mangled.get_or_insert_with(|| mangled_review_rows(&rows));
                    let mut value = json!({"scan":name,"pr":pr,"result":if bad.is_empty() { "pass" } else { "fail" },"count":bad.len()});
                    if !bad.is_empty() {
                        value["detail"] = Value::String(format!(
                            "{} plan-review accepted rows with prose category (not canonical)",
                            bad.len()
                        ));
                    }
                    value
                }
            }
            "rej-category-blank" if !review_present && !has_review_rows => {
                json!({"scan":name,"pr":pr,"result":"skip","detail":"review-findings-full.jsonl not found"})
            }
            "rej-category-blank" => rejected_blank_category_scan(&rows, pr, &name),
            "ns-retry-sidecars" => ns_retry_scan(&run_dir, pr, &name, signals_present, &signals),
            "cursor-ci-stall-causes" => cursor_stall_scan(&run_dir, pr, &name),
            "codex-round1-adherence" => codex_adherence_scan(&run_dir, pr, &name),
            "codex-generalist-waste" => codex_waste_scan(&run_dir, pr, &name),
            "execution-issues-categories" => execution_categories_scan(&run_dir, pr, &name),
            "cache-freshness" => cache_freshness_scan(&run_dir, pr, &name, &current_version),
            "changelog-rebase-conflicts" => changelog_scan(&run_dir, pr, &name),
            "coder-tool" => coder_tool_scan(&run_dir, pr, &name),
            "trailing-content-no-issues-found" => {
                trailing_content_scan(pr, &name, signals_present, &signals)
            }
            "oos-silent-drop" => oos_silent_drop_scan(&run_dir, pr, &name),
            "guideline-assessment" => assessment_scan(
                &run_dir,
                pr,
                &name,
                AssessmentKind::Guidelines.design_assessment_filename(),
                AssessmentKind::Guidelines.clean_presentation_note(),
                "clean",
                "deviation",
                "guideline",
            ),
            "invariant-assessment" => assessment_scan(
                &run_dir,
                pr,
                &name,
                AssessmentKind::Invariants.design_assessment_filename(),
                AssessmentKind::Invariants.clean_presentation_note(),
                "clean",
                "violation",
                "invariant",
            ),
            "guideline-ship-outcome" => outcome_scan(
                &run_dir,
                pr,
                &name,
                AssessmentKind::Guidelines.ship_outcome_sidecar_filename(),
                AssessmentKind::Guidelines,
            ),
            "invariant-ship-outcome" => outcome_scan(
                &run_dir,
                pr,
                &name,
                AssessmentKind::Invariants.ship_outcome_sidecar_filename(),
                AssessmentKind::Invariants,
            ),
            _ => {
                emit(
                    json!({"scan":name,"pr":pr,"result":"error","detail":"unknown scan name in scans registry (registry drift vs audit-runs scan-run)"}),
                );
                return ExitCode::FAILURE;
            }
        };
        if result.get("result") == Some(&Value::String("error".to_owned()))
            && result.get("scan") == Some(&Value::String("required-file-presence".to_owned()))
        {
            exit = ExitCode::FAILURE;
        }
        emit(result);
    }
    category_stats_scan(
        &rows,
        review_present,
        has_review_rows,
        jsonl_error,
        pr,
        &skill,
        mangled.as_deref(),
    );
    cross_cutting_scan(&run_dir, pr);
    exit
}

fn emit(value: Value) {
    println!(
        "{}",
        serde_json::to_string(&value).expect("JSON scan output serializes")
    );
}

fn invalid_run_directory(path: &Path, skill: &str) -> Option<String> {
    let parts = path
        .components()
        .map(|part| part.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    for (index, part) in parts.iter().enumerate() {
        if part != "larch-logs" {
            continue;
        }
        let Some(found) = parts.get(index + 1) else {
            return Some(format!(
                "run-dir must live under larch-logs/{skill}: {}",
                path.display()
            ));
        };
        if matches!(found.as_str(), "design" | "implement") {
            if found != skill {
                return Some(format!(
                    "run-dir must live under larch-logs/{skill} for --skill={skill}: {}",
                    path.display()
                ));
            }
            if index + 2 == parts.len() {
                return Some(format!(
                    "run-dir resolves to skill log root instead of a specific run: {}",
                    path.display()
                ));
            }
            return None;
        }
    }
    None
}

fn round_directories(run_dir: &Path) -> Vec<PathBuf> {
    let mut directories = fs::read_dir(run_dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            (path.is_dir() && name.starts_with("round-")).then_some(path)
        })
        .collect::<Vec<_>>();
    directories.sort();
    directories
}

fn read_text(path: &Path) -> Option<String> {
    fs::read(path)
        .ok()
        .map(|bytes| String::from_utf8_lossy(&bytes).replace("\r\n", "\n"))
}

fn manifest_json(path: &Path) -> Option<Value> {
    read_text(path).and_then(|text| serde_json::from_str(&text).ok())
}

fn ndjson_rows(path: &Path) -> (Vec<Value>, bool) {
    let Some(text) = read_text(path) else {
        return (Vec::new(), false);
    };
    let mut rows = Vec::new();
    let mut malformed = false;
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        match serde_json::from_str::<Value>(line) {
            Ok(Value::Object(object)) => rows.push(Value::Object(object)),
            Ok(_) => {}
            Err(_) => malformed = true,
        }
    }
    (rows, malformed)
}

fn value_string(value: &Value, key: &str) -> String {
    match value.get(key) {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Number(number)) => number.to_string(),
        Some(Value::Bool(true)) => "true".to_owned(),
        Some(Value::Bool(false)) => "false".to_owned(),
        _ => String::new(),
    }
}

fn effective_review_rows(
    run_dir: &Path,
    skill: &str,
    mut rows: Vec<Value>,
    malformed: bool,
) -> Vec<Value> {
    if skill != "implement" {
        return rows;
    }
    rows.retain(|row| {
        row.get("phase").and_then(Value::as_str) != Some("retroactive-backfill")
            && json_truthy(row.get("outcome"))
    });
    if malformed || !rows.is_empty() {
        return rows;
    }
    let Some(tally) = manifest_json(&run_dir.join("code-review-tally.json")) else {
        return rows;
    };
    if value_string(&tally, "mode") != "self-review" {
        return rows;
    }
    let mut result = Vec::new();
    for (outcome, key, prefix) in [
        ("accepted", "accepted_count", "SELF_REVIEW_ACCEPTED"),
        ("rejected", "rejected_count", "SELF_REVIEW_REJECTED"),
    ] {
        for index in 1..=number_value(tally.get(key)) {
            result.push(json!({"id":format!("{prefix}_{index}"),"source":"committed-self-review-tally","phase":"code-review","outcome":outcome,"category":"","severity":"(none)","body_severity":"","focus_area":""}));
        }
    }
    result
}

fn required_file_scan(run_dir: &Path, pr: u64, required: Option<&Path>) -> Value {
    let Some(path) = required.filter(|path| path.is_file()) else {
        return json!({"scan":"required-file-presence","pr":pr,"result":"skip","detail":"required-files-tsv not provided"});
    };
    let manifest = manifest_json(&run_dir.join("manifest.json")).unwrap_or(Value::Null);
    let context =
        ReachabilityContext::with_audit_pr(run_dir, &manifest, i64::try_from(pr).unwrap_or(0));
    let text = read_text(path).unwrap_or_default();
    match scan_required_files(&context, &text, |pattern| {
        required_glob_hit(run_dir, pattern)
    }) {
        CompletenessOutcome::Complete => {
            json!({"scan":"required-file-presence","pr":pr,"result":"pass","count":0})
        }
        CompletenessOutcome::Missing(missing) => {
            json!({"scan":"required-file-presence","pr":pr,"result":"fail","missing":missing})
        }
        CompletenessOutcome::Invalid(detail) => {
            json!({"scan":"required-file-presence","pr":pr,"result":"error","detail":detail.replace("verify-completeness: unsupported manifest condition: ", "unsupported required-files condition (registry drift): ")})
        }
    }
}

fn required_glob_hit(run_dir: &Path, pattern: &str) -> bool {
    if !pattern.contains('/') {
        return fs::read_dir(run_dir)
            .into_iter()
            .flatten()
            .flatten()
            .any(|entry| {
                entry.path().is_file()
                    && glob_matches(&entry.file_name().to_string_lossy(), pattern)
            });
    }
    round_directories(run_dir).iter().any(|round| {
        fs::read_dir(round)
            .into_iter()
            .flatten()
            .flatten()
            .any(|entry| {
                let relative = format!(
                    "{}/{}",
                    run_name(round),
                    entry.file_name().to_string_lossy()
                );
                entry.path().is_file() && glob_matches(&relative, pattern)
            })
    })
}

fn exon_scan(run_dir: &Path, pr: u64) -> Value {
    let count = round_directories(run_dir)
        .iter()
        .filter_map(|round| read_text(&round.join("voting-tally.md")))
        .map(|text| EXON_RE.find_iter(&text).count())
        .sum::<usize>();
    json!({"scan":"exon-misclassification","pr":pr,"result":if count == 0 { "pass" } else { "fail" },"count":count})
}

fn category(row: &Value) -> String {
    match row.get("category") {
        Some(Value::String(value)) => value.clone(),
        Some(Value::Number(value)) => value.to_string(),
        Some(Value::Bool(value)) => value.to_string(),
        _ => String::new(),
    }
}

fn mangled_review_rows(rows: &[Value]) -> Vec<&Value> {
    rows.iter()
        .filter(|row| {
            value_string(row, "outcome") == "accepted"
                && value_string(row, "phase") == "plan-review"
                && !category(row).is_empty()
                && !FOCUS_AREAS.contains(&category(row).as_str())
        })
        .collect()
}

fn rejected_blank_category_scan(rows: &[Value], pr: u64, name: &str) -> Value {
    let count = rows
        .iter()
        .filter(|row| {
            value_string(row, "id").starts_with("REJ_")
                && !json_truthy(row.get("category"))
                && FINDING_CATEGORY_RE.is_match(&value_string(row, "prose_body"))
        })
        .count();
    let mut value = json!({"scan":name,"pr":pr,"result":if count == 0 { "pass" } else { "fail" },"count":count});
    if count > 0 {
        value["rej_blank_with_cat_in_prose"] = json!(count);
    }
    value
}

fn round_signals(run_dir: &Path) -> (bool, Vec<Value>) {
    let mut found = false;
    let mut values = Vec::new();
    for round in round_directories(run_dir) {
        let Some(meta) = manifest_json(&round.join("round-meta.json")) else {
            continue;
        };
        let Some(signals) = meta.get("reviewer_signals").and_then(Value::as_array) else {
            continue;
        };
        found = true;
        values.extend(signals.iter().filter(|signal| signal.is_object()).cloned());
    }
    (found, values)
}

fn json_truthy(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null) => false,
        Some(Value::Bool(value)) => *value,
        Some(Value::Number(value)) => value.as_f64().is_none_or(|value| value != 0.0),
        Some(Value::String(value)) => !value.is_empty(),
        Some(Value::Array(value)) => !value.is_empty(),
        Some(Value::Object(value)) => !value.is_empty(),
    }
}

fn ns_retry_scan(
    run_dir: &Path,
    pr: u64,
    name: &str,
    signals_present: bool,
    signals: &[Value],
) -> Value {
    let mut reasons = Vec::new();
    if signals_present {
        let mut signaled = BTreeSet::new();
        for signal in signals {
            let reason = value_string(signal, "ns_retry_reason");
            if !reason.is_empty() {
                reasons.push(
                    if matches!(
                        reason.as_str(),
                        "NO_ISSUES_FOUND_TOO_THIN" | "OUTPUT_EMPTY" | "JSON_PARSE_FAIL" | "UNKNOWN"
                    ) {
                        reason
                    } else {
                        "UNKNOWN".to_owned()
                    },
                );
                let basename = value_string(signal, "output_basename");
                if !basename.is_empty() {
                    signaled.insert(basename);
                }
            }
        }
        for round in round_directories(run_dir) {
            for entry in fs::read_dir(round).into_iter().flatten().flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                let basename = if name.ends_with("-ns-retry.txt") {
                    format!("{}.txt", name.trim_end_matches("-ns-retry.txt"))
                } else {
                    format!("{name}.txt")
                };
                if name.contains("-ns-retry")
                    && is_txt_file_name(&name)
                    && !signaled.contains(&basename)
                {
                    reasons.push("UNKNOWN".to_owned());
                }
            }
        }
    } else {
        for round in round_directories(run_dir) {
            reasons.extend(
                fs::read_dir(round)
                    .into_iter()
                    .flatten()
                    .flatten()
                    .filter_map(|entry| {
                        entry
                            .file_name()
                            .to_str()
                            .is_some_and(|name| {
                                name.contains("-ns-retry") && is_txt_file_name(name)
                            })
                            .then_some("UNKNOWN".to_owned())
                    }),
            );
        }
        if reasons.is_empty() {
            return json!({"scan":name,"pr":pr,"result":"skip","detail":"reviewer_signals signal unavailable"});
        }
        return json!({"scan":name,"pr":pr,"result":"fail","count":reasons.len(),"reasons":histogram(&reasons),"detail":"legacy sidecar fallback (reviewer_signals unavailable)"});
    }
    json!({"scan":name,"pr":pr,"result":if reasons.is_empty() { "pass" } else { "fail" },"count":reasons.len(),"reasons":histogram(&reasons)})
}

fn is_txt_file_name(name: &str) -> bool {
    Path::new(name)
        .extension()
        .is_some_and(|extension| extension == "txt")
}

fn histogram(values: &[String]) -> BTreeMap<String, usize> {
    let mut output = BTreeMap::new();
    for value in values {
        *output.entry(value.clone()).or_default() += 1;
    }
    output
}

fn cursor_stall_scan(run_dir: &Path, pr: u64, name: &str) -> Value {
    let files = round_directories(run_dir)
        .into_iter()
        .flat_map(|round| {
            fs::read_dir(round)
                .into_iter()
                .flatten()
                .flatten()
                .map(|entry| entry.path())
        })
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| glob_matches(name, "cursor-ci-stall-*.json"))
        })
        .collect::<Vec<_>>();
    let mut channels = Vec::new();
    let mut parsed = 0;
    for file in &files {
        if let Some(value) = manifest_json(file).filter(Value::is_object) {
            parsed += 1;
            channels.push(nonempty_or(&value_string(&value, "channel"), "UNKNOWN"));
        } else {
            channels.push("UNKNOWN".to_owned());
        }
    }
    json!({"scan":name,"pr":pr,"result":if files.is_empty() { "pass" } else { "informational" },"count":files.len(),"parsed_files":parsed,"channels":histogram(&channels)})
}

fn codex_adherence_scan(run_dir: &Path, pr: u64, name: &str) -> Value {
    let mut violations = Vec::new();
    for round in round_directories(run_dir) {
        let round_number = run_name(&round)
            .trim_start_matches("round-")
            .parse::<u64>()
            .unwrap_or(0);
        if round_number <= 2 {
            continue;
        }
        for row in ndjson_rows(&round.join("panel-manifest.ndjson")).0 {
            let slot = value_string(&row, "slot");
            if value_string(&row, "tool") == "codex"
                && matches!(slot.as_str(), "generalist" | "codex-plan-generic")
            {
                violations.push((round_number, slot));
            }
        }
    }
    let mut output =
        json!({"scan":name,"pr":pr,"result":if violations.is_empty() { "pass" } else { "fail" }});
    if !violations.is_empty() {
        let rounds = violations
            .iter()
            .map(|(round, _)| *round)
            .collect::<BTreeSet<_>>();
        output["rounds_with_generic_codex"] = json!(rounds);
        output["violations"] = json!(
            violations
                .into_iter()
                .map(|(round, slot)| json!({"round":round,"slot":slot}))
                .collect::<Vec<_>>()
        );
    }
    output
}

fn execution_rows(run_dir: &Path) -> Option<(Vec<Value>, bool)> {
    let path = run_dir.join("execution-issues.ndjson");
    path.is_file().then(|| ndjson_rows(&path))
}

fn execution_categories_scan(run_dir: &Path, pr: u64, name: &str) -> Value {
    let Some((rows, _)) = execution_rows(run_dir) else {
        return json!({"scan":name,"pr":pr,"result":"skip","detail":"execution-issues.ndjson not found"});
    };
    let non_warnings = rows
        .iter()
        .filter(|row| {
            row.get("category")
                .and_then(Value::as_str)
                .is_some_and(|category| category != "Warnings")
        })
        .count();
    let warnings = rows
        .iter()
        .filter(|row| value_string(row, "category") == "Warnings")
        .count();
    json!({"scan":name,"pr":pr,"result":if non_warnings == 0 { "pass" } else { "fail" },"non_warnings":non_warnings,"warnings":warnings})
}

fn cache_freshness_scan(run_dir: &Path, pr: u64, name: &str, current: &str) -> Value {
    let Some(manifest) = manifest_json(&run_dir.join("manifest.json")).filter(Value::is_object)
    else {
        return json!({"scan":name,"pr":pr,"result":"skip","detail":"manifest.json not found"});
    };
    let run_version = value_string(&manifest, "larch_version");
    if current.is_empty() || current == "unknown" {
        return json!({"scan":name,"pr":pr,"result":"skip","detail":"current-version unset","run_version":run_version});
    }
    if run_version.is_empty() {
        return json!({"scan":name,"pr":pr,"result":"fail","detail":"manifest larch_version empty","current_version":current});
    }
    if run_version != current && version_numbers(&run_version) < version_numbers(current) {
        return json!({"scan":name,"pr":pr,"result":"informational","run_version":run_version,"current_version":current,"detail":"run plugin version behind current"});
    }
    json!({"scan":name,"pr":pr,"result":"pass","run_version":run_version,"current_version":current})
}

fn version_numbers(value: &str) -> Vec<u64> {
    let values = value
        .split(|character: char| !character.is_ascii_digit())
        .filter(|value| !value.is_empty())
        .filter_map(|value| value.parse::<u64>().ok())
        .take(3)
        .collect::<Vec<_>>();
    if values.is_empty() { vec![0] } else { values }
}

fn strict_version_tuple(value: &str) -> Option<(u64, u64, u64)> {
    larch_core::parse_larch_version_tuple(value)
}

fn changelog_scan(run_dir: &Path, pr: u64, name: &str) -> Value {
    let Some((rows, _)) = execution_rows(run_dir) else {
        return json!({"scan":name,"pr":pr,"result":"skip","detail":"execution-issues.ndjson not found"});
    };
    let count = rows
        .iter()
        .filter(|row| {
            let body = value_string(row, "body").to_lowercase();
            body.contains("changelog") && (body.contains("rebase") || body.contains("conflict"))
        })
        .count();
    json!({"scan":name,"pr":pr,"result":if count == 0 { "pass" } else { "fail" },"count":count})
}

fn coder_tool_scan(run_dir: &Path, pr: u64, name: &str) -> Value {
    let mut by_round = BTreeMap::new();
    for round in round_directories(run_dir) {
        let mut tool = manifest_json(&round.join("round-meta.json"))
            .and_then(|meta| meta.get("coder").cloned())
            .map(|coder| value_string(&coder, "CODER_TOOL"))
            .unwrap_or_default();
        if tool.is_empty() {
            if let Some(text) = read_text(&round.join("coder.env")) {
                tool = CODER_TOOL_RE
                    .captures(&text)
                    .map_or_else(String::new, |captures| captures[1].to_owned());
            }
        }
        if !tool.is_empty() {
            by_round.insert(run_name(&round), tool);
        }
    }
    json!({"scan":name,"pr":pr,"result":"pass","by_round":by_round})
}

fn trailing_content_scan(pr: u64, name: &str, signals_present: bool, signals: &[Value]) -> Value {
    if !signals_present {
        return json!({"scan":name,"pr":pr,"result":"skip","detail":"reviewer_signals signal unavailable"});
    }
    let count = signals
        .iter()
        .filter(|signal| signal.get("first_pass_trailing_content") == Some(&Value::Bool(true)))
        .count();
    json!({"scan":name,"pr":pr,"result":if count == 0 { "pass" } else { "fail" },"count":count})
}

fn codex_waste_scan(run_dir: &Path, pr: u64, name: &str) -> Value {
    let Some(meta) = manifest_json(&run_dir.join("round-1/round-meta.json")) else {
        return json!({"scan":name,"pr":pr,"result":"skip","detail":"reviewer_signals signal unavailable"});
    };
    let Some(signals) = meta.get("reviewer_signals").and_then(Value::as_array) else {
        return json!({"scan":name,"pr":pr,"result":"skip","detail":"reviewer_signals signal unavailable"});
    };
    let result_kind = signals
        .iter()
        .find(|signal| value_string(signal, "output_basename") == "codex-generalist-output.txt")
        .map(|signal| value_string(signal, "result_kind"))
        .unwrap_or_default();
    if result_kind.is_empty() {
        return json!({"scan":name,"pr":pr,"result":"skip","detail":"reviewer_signals signal unavailable"});
    }
    let from_wrapper = meta
        .get("wrapper_logs")
        .map(|logs| value_string(logs, "codex"))
        .unwrap_or_default();
    let elapsed = Regex::new(r"([0-9]+)s elapsed")
        .expect("elapsed regex")
        .captures_iter(&from_wrapper)
        .filter_map(|capture| capture[1].parse::<u64>().ok())
        .max()
        .unwrap_or_else(|| codex_timing_elapsed(run_dir));
    let fail = result_kind == "NO_ISSUES_FOUND" && elapsed > 120;
    let mut output = json!({"scan":name,"pr":pr,"result":if fail { "fail" } else { "pass" },"result_kind":result_kind,"elapsed_seconds":elapsed});
    if fail {
        output["detail"] =
            json!("codex-generalist returned NO_ISSUES_FOUND after more than 120 seconds");
    }
    output
}

fn codex_timing_elapsed(run_dir: &Path) -> u64 {
    let Some(report) = manifest_json(&run_dir.join("timing-report.json")) else {
        return 0;
    };
    let mut preferred = Vec::new();
    let mut fallback = Vec::new();
    for key in ["vendor_task_averages", "steps", "per_step"] {
        let Some(rows) = report.get(key).and_then(Value::as_array) else {
            continue;
        };
        for row in rows {
            let seconds = timing_seconds(row);
            let Some(seconds) = seconds else { continue };
            if key == "vendor_task_averages" {
                if value_string(row, "vendor") != "codex" {
                    continue;
                }
                match value_string(row, "task_kind").as_str() {
                    "codex-review-generic" | "codex-phase1-generic" => preferred.push(seconds),
                    "codex-review" => fallback.push(seconds),
                    _ => {}
                }
            } else {
                let text = ["vendor", "task_kind", "task", "name", "step", "label"]
                    .iter()
                    .map(|field| value_string(row, field).to_lowercase())
                    .collect::<Vec<_>>()
                    .join(" ");
                if text.contains("codex")
                    && (text.contains("generalist") || text.contains("generic"))
                {
                    preferred.push(seconds);
                } else if text.contains("step 5") && text.contains("code review") {
                    fallback.push(seconds);
                }
            }
        }
    }
    preferred.into_iter().chain(fallback).max().unwrap_or(0)
}

fn timing_seconds(row: &Value) -> Option<u64> {
    let raw = [
        "max_seconds",
        "average_seconds",
        "duration_seconds",
        "duration_s",
        "elapsed_seconds",
    ]
    .iter()
    .find_map(|field| row.get(*field))?;
    let number = match raw {
        Value::Number(value) => value.to_string().parse::<f64>().ok()?,
        Value::String(value) => value.parse::<f64>().ok()?,
        _ => return None,
    };
    (number.is_finite() && number >= 0.0).then_some(number as u64)
}

fn assessment_scan(
    run_dir: &Path,
    pr: u64,
    name: &str,
    filename: &str,
    clean_note: &str,
    clean: &str,
    nonclean: &str,
    label: &str,
) -> Value {
    let path = run_dir.join(filename);
    if !path.exists() && !path.is_symlink() {
        return json!({"scan":name,"pr":pr,"result":"informational","detail":format!("no committed {label} assessment artifact; expected for older runs or absent/invalid{}", if label == "invariant" { "/empty invariants" } else { " guidelines" })});
    }
    if path.is_symlink() || !path.is_file() {
        return json!({"scan":name,"pr":pr,"result":"fail","detail":"assessment artifact must be a regular non-symlink file"});
    }
    let Some(body) = read_text(&path) else {
        return json!({"scan":name,"pr":pr,"result":"fail","detail":"assessment artifact unreadable"});
    };
    if body.trim().is_empty() {
        return json!({"scan":name,"pr":pr,"result":"fail","detail":"assessment artifact is empty"});
    }
    json!({"scan":name,"pr":pr,"result":"pass","assessment_kind":if body.trim_end_matches('\n') == clean_note { clean } else { nonclean }})
}

fn outcome_scan(
    run_dir: &Path,
    pr: u64,
    name: &str,
    filename: &str,
    kind: AssessmentKind,
) -> Value {
    let label = if matches!(kind, AssessmentKind::Guidelines) {
        "guideline"
    } else {
        "invariant"
    };
    let path = run_dir.join(filename);
    if run_dir.join("gc-slimmed").exists() && !path.exists() && !path.is_symlink() {
        return json!({"scan":name,"pr":pr,"result":"informational","detail":format!("gc-slimmed run lacks {label} outcome artifact")});
    }
    let manifest = manifest_json(&run_dir.join("manifest.json"));
    if manifest
        .as_ref()
        .is_none_or(|manifest| !step8_reachable(run_dir, manifest, pr))
    {
        return json!({"scan":name,"pr":pr,"result":"informational","detail":"run did not reach implement Step 8"});
    }
    if !path.exists() && !path.is_symlink() {
        if manifest.as_ref().is_some_and(|manifest| {
            strict_version_tuple(&value_string(manifest, "larch_version"))
                .is_none_or(|version| version < larch_core::SHIP_OUTCOME_CUTOVER_VERSION)
        }) {
            return json!({"scan":name,"pr":pr,"result":"informational","detail":format!("pre-cutover run lacks {label} outcome artifact")});
        }
        return json!({"scan":name,"pr":pr,"result":"fail","detail":format!("missing {label} outcome artifact")});
    }
    if path.is_symlink() || !path.is_file() {
        return json!({"scan":name,"pr":pr,"result":"fail","detail":format!("{label} outcome artifact must be a regular non-symlink file")});
    }
    let Some(raw) = read_text(&path) else {
        return json!({"scan":name,"pr":pr,"result":"fail","detail":format!("{label} outcome artifact malformed")});
    };
    if raw.trim().is_empty() {
        return json!({"scan":name,"pr":pr,"result":"fail","detail":format!("{label} outcome artifact is empty")});
    }
    let Ok(data) = serde_json::from_str::<Value>(&raw) else {
        return json!({"scan":name,"pr":pr,"result":"fail","detail":format!("{label} outcome artifact malformed")});
    };
    if let Some(reason) = validate_ship_outcome_record(&data, kind) {
        return json!({"scan":name,"pr":pr,"result":"fail","detail":reason});
    }
    let status_key = if matches!(kind, AssessmentKind::Guidelines) {
        "guidelines_status"
    } else {
        "invariants_status"
    };
    let mut output = json!({"scan":name,"pr":pr,"result":"pass","outcome":value_string(&data,"outcome"),"reason":value_string(&data,"reason"),status_key:value_string(&data,status_key)});
    let assessment = value_string(&data, "assessment_kind");
    if !assessment.is_empty() {
        output["assessment_kind"] = json!(assessment);
    }
    output
}

fn step8_reachable(run_dir: &Path, manifest: &Value, pr: u64) -> bool {
    if manifest
        .get("steps_ran")
        .and_then(Value::as_object)
        .and_then(|steps| steps.get("step8"))
        == Some(&Value::Bool(false))
    {
        return false;
    }
    let context =
        ReachabilityContext::with_audit_pr(run_dir, manifest, i64::try_from(pr).unwrap_or(0));
    larch_core::condition_reached(&context, "step8").unwrap_or(false)
        || (!terminal_bail_skip(run_dir, manifest, pr)
            && run_dir.join("final-summary.md").is_file())
}

fn terminal_bail_skip(run_dir: &Path, manifest: &Value, pr: u64) -> bool {
    let pr = i64::try_from(pr).unwrap_or(0);
    if pr > 0 {
        let evidence = json!({"pr_number": pr});
        if larch_core::stale_bail_heading_with_pr_evidence(run_dir, Some(&evidence), pr) {
            return false;
        }
    }
    larch_core::terminal_bail_skip_signal(run_dir, Some(manifest), pr)
}

fn oos_silent_drop_scan(run_dir: &Path, pr: u64, name: &str) -> Value {
    let gh_host = env::var("GH_HOST").unwrap_or_else(|_| "github.com".to_owned());
    let counts = larch_core::analyze_run_dir(run_dir, &gh_host);
    if counts.non_security_oos_blocks == 0 {
        return json!({"scan":name,"pr":pr,"result":"skip","detail":"no non-security OOS blocks in canonical oos-accepted-*.md"});
    }
    if counts.ndjson_parse_error {
        return json!({"scan":name,"pr":pr,"result":"error","detail":"jq parse failure while reading oos-issues.ndjson for rejected-OOS markers"});
    }
    let ok = counts.issue_urls > 0
        || counts.inline_triage_hits >= counts.non_security_oos_blocks
        || counts.rejected_oos_markers >= counts.non_security_oos_blocks;
    let mut output = json!({"scan":name,"pr":pr,"result":if ok { "pass" } else { "fail" },"non_security_oos_blocks":counts.non_security_oos_blocks,"issue_urls":counts.issue_urls,"inline_triage_hits":counts.inline_triage_hits,"rejected_oos_markers":counts.rejected_oos_markers});
    if !ok {
        output["detail"] = json!(
            "accepted OOS blocks without filed URLs, sufficient Inline-triage breadcrumbs, or explicit rejected-OOS markers in oos-issues.ndjson"
        );
    }
    output
}

fn category_stats_scan(
    rows: &[Value],
    review_present: bool,
    has_rows: bool,
    malformed: bool,
    pr: u64,
    skill: &str,
    cached_mangled: Option<&[&Value]>,
) {
    if review_present || has_rows {
        if malformed {
            emit(
                json!({"scan":"category-stats","pr":pr,"partial_data":true,"partial_reason":"malformed_review_findings_jsonl","detail":"jq failed (category-stats): parse error","canonical":0,"blank":0,"mangled":0,"oos_blank":0,"rej_blank":0}),
            );
            return;
        }
        let mangled_count =
            cached_mangled.map_or_else(|| mangled_review_rows(rows).len(), <[&Value]>::len);
        let canonical = rows
            .iter()
            .filter(|row| FOCUS_AREAS.contains(&category(row).as_str()))
            .count();
        let blank = rows.iter().filter(|row| category(row).is_empty()).count();
        let oos_blank = rows
            .iter()
            .filter(|row| value_string(row, "id").starts_with("OOS_") && category(row).is_empty())
            .count();
        let rej_blank = rows
            .iter()
            .filter(|row| value_string(row, "id").starts_with("REJ_") && category(row).is_empty())
            .count();
        emit(
            json!({"scan":"category-stats","pr":pr,"partial_data":false,"canonical":canonical,"blank":blank,"mangled":mangled_count,"oos_blank":oos_blank,"rej_blank":rej_blank}),
        );
        return;
    }
    if skill == "design" {
        emit(
            json!({"scan":"category-stats","pr":pr,"partial_data":false,"skip_reason":"design_run_has_no_review_findings_jsonl","detail":"design runs intentionally omit review-findings-full.jsonl","canonical":0,"blank":0,"mangled":0,"oos_blank":0,"rej_blank":0}),
        );
    } else {
        emit(
            json!({"scan":"category-stats","pr":pr,"partial_data":true,"partial_reason":"missing_review_findings_jsonl","detail":"review-findings-full.jsonl not found","canonical":0,"blank":0,"mangled":0,"oos_blank":0,"rej_blank":0}),
        );
    }
}

fn cross_cutting_scan(run_dir: &Path, pr: u64) {
    let manifest = manifest_json(&run_dir.join("manifest.json"));
    let (ended, pr_null, mismatch) = if let Some(manifest) = manifest.filter(Value::is_object) {
        let schema_v2 = manifest
            .get("schema_version")
            .and_then(Value::as_i64)
            .is_some_and(|value| value >= 2);
        let ended = if schema_v2 {
            manifest
                .get("ended_at")
                .is_some_and(|value| value.is_null() || value.as_str().is_some_and(str::is_empty))
        } else {
            value_string(&manifest, "ended_at").is_empty()
        };
        let supplied = manifest.get("pr_number");
        let pr_null = if schema_v2 {
            matches!(supplied, Some(Value::Null))
        } else {
            supplied.is_none() || value_string(&manifest, "pr_number").is_empty()
        };
        let mismatch = !pr_null && value_string(&manifest, "pr_number") != pr.to_string();
        (ended, pr_null, mismatch)
    } else {
        (false, false, false)
    };
    emit(
        json!({"scan":"cross-cutting","pr":pr,"ended_at_null":ended,"pr_number_null":pr_null,"manifest_pr_number_mismatch_with_audited_pr":mismatch,"self_deploying_gap":mismatch}),
    );
}

fn nonempty_or(value: &str, fallback: &str) -> String {
    if value.is_empty() {
        fallback.to_owned()
    } else {
        value.to_owned()
    }
}

fn clean_controls(value: &str) -> String {
    value
        .chars()
        .filter(|character| !character.is_control())
        .collect::<String>()
        .trim()
        .to_owned()
}

fn clean_reason(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .collect::<String>()
        .trim()
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::{
        AuditPullRequest, RunLogCorpus, design_run_id, filtered_prs, frontmatter_last_pr,
        manifest_fields, pacific_offset_hours, parent_issue_candidates, render_pacific_timestamp,
        step8_reachable, timing_seconds, top_frontmatter, valid_full_instant, version_numbers,
    };
    use chrono::{TimeZone, Utc};
    use serde_json::json;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn pacific_offset_uses_the_utc_dst_boundaries() {
        assert_eq!(
            pacific_offset_hours(Utc.with_ymd_and_hms(2026, 3, 8, 9, 59, 0).unwrap()),
            -8
        );
        assert_eq!(
            pacific_offset_hours(Utc.with_ymd_and_hms(2026, 3, 8, 10, 0, 0).unwrap()),
            -7
        );
        assert_eq!(
            pacific_offset_hours(Utc.with_ymd_and_hms(2026, 11, 1, 8, 59, 0).unwrap()),
            -7
        );
        assert_eq!(
            pacific_offset_hours(Utc.with_ymd_and_hms(2026, 11, 1, 9, 0, 0).unwrap()),
            -8
        );
        assert_eq!(
            render_pacific_timestamp(Utc.with_ymd_and_hms(2026, 3, 8, 10, 0, 0).unwrap()),
            "2026-03-08T03:00-07:00"
        );
    }

    #[test]
    fn prior_audit_range_and_iso_input_are_bounded() {
        assert_eq!(
            frontmatter_last_pr("---\naudited_pr_range:\n  last: '42'\n---\n"),
            Some(42)
        );
        assert_eq!(
            top_frontmatter("  ---  \nanswer: 42\n\n---\nignored: true\n"),
            "answer: 42\n"
        );
        assert!(valid_full_instant("2026-08-01T12:00:00Z"));
        assert!(!valid_full_instant("2026-08-01"));
    }

    #[test]
    fn mapping_and_resolution_helpers_preserve_the_legacy_selection_rules() {
        let root = tempdir().expect("temporary run corpus");
        let selected = root.path().join("selected");
        let other = root.path().join("other");
        fs::create_dir_all(&selected).expect("selected run");
        fs::create_dir_all(&other).expect("other run");
        fs::write(selected.join("parent-issue.md"), "ISSUE_NUMBER=12\n").expect("parent issue");
        fs::write(other.join("parent-issue.md"), "ISSUE_NUMBER=123\n")
            .expect("different parent issue");
        fs::write(
            selected.join("manifest.json"),
            r#"{"started_at":"2026-08-09T12:00:00Z","larch_version":"56.2.2"}"#,
        )
        .expect("manifest");

        let runs = RunLogCorpus::new(root.path()).safe_child_run_directories();
        assert_eq!(
            parent_issue_candidates(&runs, "12"),
            std::slice::from_ref(&selected)
        );
        assert_eq!(
            manifest_fields(&selected.join("manifest.json")),
            (
                "2026-08-09T12:00:00Z".to_owned(),
                "56.2.2".to_owned(),
                String::new()
            )
        );
        assert_eq!(
            design_run_id("chore(larch-logs): design run ABCDEF01-2345-6789-ABCD-EF0123456789"),
            Some("ABCDEF01-2345-6789-ABCD-EF0123456789".to_owned())
        );

        let pulls = [
            AuditPullRequest {
                number: 2,
                title: "ordinary implementation".to_owned(),
                body: String::new(),
                base_ref: "main".to_owned(),
                merged_at: Some("2026-08-09T11:00:00Z".to_owned()),
            },
            AuditPullRequest {
                number: 3,
                title: "chore(larch-logs): design run ABCDEF01-2345-6789-ABCD-EF0123456789"
                    .to_owned(),
                body: String::new(),
                base_ref: "main".to_owned(),
                merged_at: Some("2026-08-09T13:00:00Z".to_owned()),
            },
        ];
        assert_eq!(
            filtered_prs("design", &pulls, Some("2026-08-09T12:00:00Z")),
            [3]
        );
    }

    #[test]
    fn explicit_step8_false_keeps_outcome_scans_informational() {
        let root = tempdir().expect("temporary run");
        fs::write(root.path().join("final-summary.md"), "summary\n").expect("summary should write");
        assert!(!step8_reachable(
            root.path(),
            &json!({"steps_ran": {"step8": false}}),
            7,
        ));
    }

    #[test]
    fn cache_and_timing_compatibility_keep_python_value_boundaries() {
        assert!(version_numbers("1") < version_numbers("1.0"));
        assert_eq!(version_numbers("no version"), [0]);
        assert_eq!(
            timing_seconds(&json!({"max_seconds": null, "average_seconds": 5})),
            None
        );
        assert_eq!(
            timing_seconds(&json!({"average_seconds": "12.9"})),
            Some(12)
        );
    }
}

#[cfg(test)]
#[path = "../tests/support/audit_runs_commands_coverage.rs"]
mod coverage_tests;
