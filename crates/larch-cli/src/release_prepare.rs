//! Read-only release version, bump-classification, and preparation commands.

use std::{
    collections::BTreeSet,
    env,
    fmt::Write as _,
    fs,
    fs::OpenOptions,
    future::Future,
    io::Write as _,
    path::{Path, PathBuf},
    process::ExitCode,
    time::Duration,
};

use larch_adapters::{
    GixRepository, PathIntent, RepositoryRoot, TemporaryRoot, TokioProcessRunner,
    atomic_write_utf8,
    github::{
        GitHubOperationError, OctocrabGitHubService, ReleasePlanningService, ReleasePullRequest,
    },
    runtime::{Cancellation, LarchRuntime},
};

use crate::git_command_runtime::GitCommandRuntime;
use crate::release_common::{self, semver};
use larch_core::{
    ChangeKind, GitPath, Head, ObjectId, RepositoryRead, Revision, SafeText, StatusOptions, emit_kv,
};
use serde_json::Value;

use crate::github_repository_resolution::{parse_github_remote_url, validate_repo_slug};

const PLUGIN_JSON: &str = ".claude-plugin/plugin.json";
const TRANSPARENT_LOG_PREFIX: &str = "chore(larch-logs): ";
const TRANSPARENT_CHANGELOG_PREFIX: &str = "Update CHANGELOG for ";
const MAX_RELEASE_COMMITS: usize = 10_000;
const IDEMPOTENCY_DEPTH: usize = 3;

/// Attempts a single read-only GitHub call makes across transient failures.
const TRANSIENT_READ_ATTEMPTS: u32 = 4;

/// Pre-retry backoff before the 2nd, 3rd, and later attempts.
const fn transient_read_backoff(attempt: u32) -> Duration {
    let seconds = match attempt {
        1 => 1,
        2 => 2,
        _ => 5,
    };
    Duration::from_secs(seconds)
}

/// Retry a read-only GitHub call across transient transport and rate-limit
/// failures.
///
/// Release preparation issues one call per commit across a window that can hold
/// hundreds of commits, so a single transient transport blip must not abort the
/// whole read-only preparation. Each closure invocation builds a fresh request
/// future, and every wrapped call is an idempotent read, so retrying converges
/// without side effects. Non-transient failures return immediately.
async fn retry_transient<F, Fut, T>(mut call: F) -> Result<T, GitHubOperationError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, GitHubOperationError>>,
{
    let mut attempt: u32 = 1;
    loop {
        match call().await {
            Ok(value) => return Ok(value),
            Err(error) => {
                let transient = matches!(
                    error,
                    GitHubOperationError::Transport(_) | GitHubOperationError::RateLimited
                );
                if !transient || attempt >= TRANSIENT_READ_ATTEMPTS {
                    return Err(error);
                }
                tokio::time::sleep(transient_read_backoff(attempt)).await;
                attempt += 1;
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BumpType {
    Major,
    Minor,
    Patch,
    None,
}

impl BumpType {
    const fn machine(self) -> &'static str {
        match self {
            Self::Major => "MAJOR",
            Self::Minor => "MINOR",
            Self::Patch => "PATCH",
            Self::None => "NONE",
        }
    }
}

pub struct ClassifyArguments {
    pub base: Option<String>,
    pub head: Option<String>,
}

pub struct PrepareArguments {
    pub repository: larch_core::GitHubRepositoryRef,
    pub bump: Option<BumpType>,
    pub out_dir: PathBuf,
    pub no_fetch: bool,
}

struct Classification {
    current_version: String,
    new_version: String,
    bump: BumpType,
    reasoning: String,
}

#[derive(Debug)]
struct ReleaseSelection {
    selected: Vec<ReleasePullRequest>,
    written: BTreeSet<u64>,
    ignored: BTreeSet<u64>,
}

pub fn read_plugin_version(arguments: &[String]) -> ExitCode {
    if !arguments.is_empty() {
        eprintln!("Usage: larch plugin read-version");
        return ExitCode::from(2);
    }
    let root = env::var_os("CLAUDE_PLUGIN_ROOT").map_or_else(
        || env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        PathBuf::from,
    );
    let version = fs::read_to_string(root.join(PLUGIN_JSON))
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .and_then(|value| value.get("version").cloned())
        .and_then(|value| match value {
            Value::Null => None,
            Value::String(value) => Some(value),
            other => Some(other.to_string()),
        })
        .and_then(|value| value.lines().next().map(str::to_owned))
        .map(|value| value.trim_end_matches('\r').to_owned())
        .filter(|value| !value.is_empty() && value != "null")
        .unwrap_or_else(|| "unknown".to_owned());
    emit_kv("LARCH_PLUGIN_VERSION", &version);
    ExitCode::SUCCESS
}

pub fn resolve_plugin_repository(arguments: &[String]) -> ExitCode {
    if !arguments.is_empty() {
        eprintln!("Usage: larch plugin resolve-repository");
        return ExitCode::from(2);
    }
    let root = env::var_os("CLAUDE_PLUGIN_ROOT").map_or_else(
        || env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        PathBuf::from,
    );
    let repository = fs::read_to_string(root.join(PLUGIN_JSON))
        .map_err(|_| "could not read plugin metadata")
        .and_then(|text| {
            serde_json::from_str::<Value>(&text).map_err(|_| "could not read plugin metadata")
        })
        .and_then(|value| {
            value
                .get("repository")
                .and_then(Value::as_str)
                .filter(|repository| !repository.trim().is_empty())
                .map(str::to_owned)
                .ok_or("repository metadata missing")
        })
        .and_then(|repository| normalize_plugin_repository(&repository));
    match repository {
        Ok(repository) => {
            println!("{repository}");
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("resolve-upstream-larch-repo: {message}");
            ExitCode::from(1)
        }
    }
}

fn normalize_plugin_repository(repository: &str) -> Result<String, &'static str> {
    if repository.contains(['\r', '\n', '\t']) {
        return Err("repository metadata must be single-value");
    }
    let repository = repository.trim();
    if repository.contains("..") || repository.starts_with('/') {
        return Err("repository metadata is malformed");
    }
    let plain = repository.strip_suffix(".git").unwrap_or(repository);
    if validate_repo_slug(plain) {
        return Ok(plain.to_owned());
    }

    let candidate = repository.strip_prefix("git+").unwrap_or(repository);
    let owner_repo = if let Some(path) = candidate.strip_prefix("git@github.com:") {
        path.to_owned()
    } else {
        let (scheme, rest) = candidate
            .split_once("://")
            .ok_or("repository must be a GitHub URL or OWNER/REPO")?;
        if !["https", "ssh", "git"]
            .iter()
            .any(|allowed| scheme.eq_ignore_ascii_case(allowed))
        {
            return Err("repository must be a GitHub URL or OWNER/REPO");
        }
        let (authority, path) = rest
            .split_once('/')
            .ok_or("repository owner/name is malformed")?;
        let host = authority
            .rsplit_once('@')
            .map_or(authority, |(_, host)| host);
        if !host.eq_ignore_ascii_case("github.com") {
            return Err("repository must be a GitHub URL or OWNER/REPO");
        }
        path.split(['?', '#'])
            .next()
            .unwrap_or_default()
            .trim_start_matches('/')
            .to_owned()
    };
    let owner_repo = owner_repo.strip_suffix(".git").unwrap_or(&owner_repo);
    if validate_repo_slug(owner_repo) {
        Ok(owner_repo.to_owned())
    } else {
        Err("repository owner/name is malformed")
    }
}

pub fn classify_bump(arguments: &ClassifyArguments) -> ExitCode {
    match open_repository().and_then(|(root, repository)| {
        classify(
            &root,
            &repository,
            arguments.base.as_deref(),
            arguments.head.as_deref(),
            true,
        )
    }) {
        Ok(classification) => match reasoning_file(&classification.reasoning) {
            Ok(path) => {
                emit_kv("CURRENT_VERSION", &classification.current_version);
                emit_kv("NEW_VERSION", &classification.new_version);
                emit_kv("BUMP_TYPE", classification.bump.machine());
                emit_kv("REASONING_FILE", &path.to_string_lossy());
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("ERROR: {error}");
                ExitCode::FAILURE
            }
        },
        Err(error) => {
            eprintln!("ERROR: {error}");
            ExitCode::FAILURE
        }
    }
}

pub fn prepare(arguments: &PrepareArguments) -> ExitCode {
    match prepare_inner(arguments) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            emit_kv("ERROR", error.token);
            for row in error.rows {
                println!("{row}");
            }
            if !error.detail.is_empty() {
                eprintln!("{}", SafeText::diagnostic(error.detail));
            }
            ExitCode::FAILURE
        }
    }
}

#[derive(Clone)]
pub struct ReconcileNotesArguments {
    pub repository: larch_core::GitHubRepositoryRef,
    pub baseline_tag: String,
    pub source_commit: String,
    pub pr_list: PathBuf,
    pub exclude_pr: u64,
    pub out_dir: PathBuf,
}

/// Recompute the release PR window over `baseline..SOURCE_COMMIT` and surface
/// mid-run additions relative to prepare's `pr-list.tsv`.
pub fn reconcile_notes(arguments: &ReconcileNotesArguments) -> ExitCode {
    match reconcile_notes_inner(arguments) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            emit_kv("ERROR", error.token);
            for row in error.rows {
                println!("{row}");
            }
            if !error.detail.is_empty() {
                eprintln!("{}", SafeText::diagnostic(error.detail));
            }
            ExitCode::FAILURE
        }
    }
}

fn reconcile_notes_inner(arguments: &ReconcileNotesArguments) -> Result<(), PrepareError> {
    let out_dir = prepare_out_dir(&arguments.out_dir)?;
    let (root, repository) =
        open_repository().map_err(|error| PrepareError::new("not-on-main", error))?;
    verify_origin(&repository, &arguments.repository)?;
    fetch_origin_main(&root)?;
    let repository = GixRepository::open(&root)
        .map_err(|error| PrepareError::new("origin-main-unresolvable", error.to_string()))?;

    let runtime = LarchRuntime::new()
        .map_err(|error| PrepareError::new("gh-release-list-failed", error.to_string()))?;
    runtime.block_on(async {
        let cancellation = Cancellation::new();
        let runner = TokioProcessRunner::default();
        let service = OctocrabGitHubService::from_gh(&runner, &root, &cancellation)
            .await
            .map_err(|error| PrepareError::new("gh-release-list-failed", error.to_string()))?;
        reconcile_notes_with_service(arguments, &out_dir, &repository, &service, &cancellation)
            .await
    })
}

async fn reconcile_notes_with_service<S: ReleasePlanningService + ?Sized>(
    arguments: &ReconcileNotesArguments,
    out_dir: &TemporaryRoot,
    repository: &GixRepository,
    service: &S,
    cancellation: &Cancellation,
) -> Result<(), PrepareError> {
    let owner = arguments.repository.owner();
    let name = arguments.repository.name();
    let baseline = arguments.baseline_tag.as_str();
    if semver(baseline.strip_prefix('v').unwrap_or_default()).is_none() {
        return Err(PrepareError::new(
            "invalid-baseline-tag",
            format!("baseline tag has invalid format: {baseline}"),
        ));
    }
    let baseline_id = resolve(repository, baseline).map_err(|_| {
        PrepareError::new(
            "baseline-tag-unresolvable",
            format!("baseline tag not resolvable: {baseline}"),
        )
    })?;
    let source_id = resolve(repository, &arguments.source_commit).map_err(|_| {
        PrepareError::new(
            "source-commit-unresolvable",
            format!("source commit not resolvable: {}", arguments.source_commit),
        )
    })?;
    let window_base = release_window_base(repository, &baseline_id, &source_id)
        .map_err(|error| PrepareError::new("baseline-not-on-main", error))?
        .ok_or_else(|| {
            PrepareError::new(
                "baseline-not-on-main",
                format!("baseline tag {baseline} is not an ancestor of the source commit"),
            )
        })?;

    let prepared = read_prepared_pr_numbers(&arguments.pr_list)?;
    let mut commits = repository
        .walk_commits_range(&window_base, &source_id, MAX_RELEASE_COMMITS + 1)
        .map_err(|error| PrepareError::new("release-history-failed", error.to_string()))?;
    if commits.len() > MAX_RELEASE_COMMITS {
        return Err(PrepareError::new(
            "release-history-too-large",
            "release commit window exceeds the bounded limit",
        ));
    }
    let selection = select_pull_requests(service, cancellation, owner, name, &commits).await?;
    let mut selected = selection
        .selected
        .into_iter()
        .filter(|pull_request| pull_request.number != arguments.exclude_pr)
        .collect::<Vec<_>>();
    selected.sort_by_key(|pull_request| pull_request.number);
    let added = selected
        .iter()
        .filter(|pull_request| !prepared.contains(&pull_request.number))
        .cloned()
        .collect::<Vec<_>>();
    write_pr_list(out_dir, service, cancellation, owner, name, &selected).await?;
    write_named_pr_list(
        out_dir,
        service,
        cancellation,
        owner,
        name,
        &added,
        "added-pr-list.tsv",
    )
    .await?;
    emit_kv("BASELINE_TAG", baseline);
    emit_kv("SOURCE_COMMIT", &arguments.source_commit);
    emit_kv("PR_COUNT", &selected.len().to_string());
    emit_kv("ADDED_PR_COUNT", &added.len().to_string());
    emit_kv(
        "PR_LIST_FILE",
        &arguments.out_dir.join("pr-list.tsv").to_string_lossy(),
    );
    emit_kv(
        "ADDED_PR_LIST_FILE",
        &arguments
            .out_dir
            .join("added-pr-list.tsv")
            .to_string_lossy(),
    );
    commits.clear();
    Ok(())
}

fn read_prepared_pr_numbers(path: &Path) -> Result<BTreeSet<u64>, PrepareError> {
    let text = fs::read_to_string(path).map_err(|error| {
        PrepareError::new(
            "pr-list-read-failed",
            format!("could not read prepare pr-list: {error}"),
        )
    })?;
    let mut numbers = BTreeSet::new();
    for (index, line) in text.lines().enumerate() {
        if line.is_empty() {
            continue;
        }
        let number = line
            .split('\t')
            .next()
            .and_then(|value| value.parse::<u64>().ok())
            .ok_or_else(|| {
                PrepareError::new(
                    "pr-list-read-failed",
                    format!("invalid pr-list row {}: {line}", index + 1),
                )
            })?;
        numbers.insert(number);
    }
    Ok(numbers)
}

async fn write_named_pr_list<S: ReleasePlanningService + ?Sized>(
    out_dir: &TemporaryRoot,
    service: &S,
    cancellation: &Cancellation,
    owner: &str,
    name: &str,
    selected: &[ReleasePullRequest],
    file_name: &str,
) -> Result<(), PrepareError> {
    let pr_list = out_dir
        .confine(file_name, PathIntent::Write)
        .map_err(|error| PrepareError::new("pr-list-write-failed", error.to_string()))?;
    let mut rows = String::new();
    for pull_request in selected {
        let title = companion_title(service, cancellation, owner, name, pull_request).await;
        let row = [
            pull_request.number.to_string(),
            tsv(&title),
            tsv(&pull_request.labels.join(",")),
            tsv(&pull_request.author),
            tsv(&pull_request.url),
        ];
        rows.push_str(&row.join("\t"));
        rows.push('\n');
    }
    atomic_write_utf8(&pr_list, &rows, 0o600)
        .map_err(|error| PrepareError::new("pr-list-write-failed", error.to_string()))
}

#[derive(Debug)]
struct PrepareError {
    token: &'static str,
    detail: String,
    rows: Vec<String>,
}

impl PrepareError {
    fn new(token: &'static str, detail: impl Into<String>) -> Self {
        Self {
            token,
            detail: detail.into(),
            rows: Vec::new(),
        }
    }

    fn with_row(mut self, row: impl Into<String>) -> Self {
        self.rows.push(row.into());
        self
    }
}

fn prepare_inner(arguments: &PrepareArguments) -> Result<(), PrepareError> {
    let out_dir = prepare_out_dir(&arguments.out_dir)?;
    let (root, repository) =
        open_repository().map_err(|error| PrepareError::new("not-on-main", error))?;
    verify_origin(&repository, &arguments.repository)?;
    verify_main_worktree(&repository)?;
    if !arguments.no_fetch {
        fetch_origin_main(&root)?;
    }
    let repository = GixRepository::open(&root)
        .map_err(|error| PrepareError::new("origin-main-unresolvable", error.to_string()))?;

    let runtime = LarchRuntime::new()
        .map_err(|error| PrepareError::new("gh-release-list-failed", error.to_string()))?;
    runtime.block_on(async {
        let cancellation = Cancellation::new();
        let runner = TokioProcessRunner::default();
        let service = OctocrabGitHubService::from_gh(&runner, &root, &cancellation)
            .await
            .map_err(|error| PrepareError::new("gh-release-list-failed", error.to_string()))?;
        prepare_with_service(
            arguments,
            &out_dir,
            &root,
            &repository,
            &service,
            &cancellation,
        )
        .await
    })
}

async fn prepare_with_service<S: ReleasePlanningService + ?Sized>(
    arguments: &PrepareArguments,
    out_dir: &TemporaryRoot,
    root: &Path,
    repository: &GixRepository,
    service: &S,
    cancellation: &Cancellation,
) -> Result<(), PrepareError> {
    let owner = arguments.repository.owner();
    let name = arguments.repository.name();
    let baseline = retry_transient(|| service.latest_release_tag(cancellation, owner, name))
        .await
        .map_err(|error| PrepareError::new("gh-release-list-failed", error.to_string()))?
        .ok_or_else(|| {
            PrepareError::new("no-unique-latest-release", "no Latest release found")
                .with_row("LATEST_COUNT=0")
        })?;
    if semver(baseline.strip_prefix('v').unwrap_or_default()).is_none() {
        return Err(PrepareError::new(
            "invalid-baseline-tag",
            format!("baseline tag has invalid format: {baseline}"),
        ));
    }
    let baseline_id = resolve(repository, &baseline).map_err(|_| {
        PrepareError::new(
            "baseline-tag-unresolvable",
            format!("baseline tag not resolvable: {baseline}"),
        )
    })?;
    let release_id = resolve(repository, "origin/main")
        .map_err(|_| PrepareError::new("origin-main-unresolvable", "origin/main not resolvable"))?;
    let release_sha = release_id.to_hex();
    let window_base = release_window_base(repository, &baseline_id, &release_id)
        .map_err(|error| PrepareError::new("baseline-not-on-main", error))?
        .ok_or_else(|| {
            PrepareError::new(
                "baseline-not-on-main",
                format!("baseline tag {baseline} is not an ancestor of origin/main"),
            )
        })?;

    let open = retry_transient(|| service.list_open_pull_requests(cancellation, owner, name))
        .await
        .map_err(|error| PrepareError::new("release-pr-list-failed", error.to_string()))?;
    if open.iter().any(|pull_request| {
        pull_request
            .head_ref
            .strip_prefix("release/v")
            .is_some_and(|version| semver(version).is_some())
    }) {
        return Err(PrepareError::new(
            "release-cut-in-progress",
            format!("open release/v* PR exists on {owner}/{name}"),
        ));
    }

    let mut commits = repository
        .walk_commits_range(&window_base, &release_id, MAX_RELEASE_COMMITS + 1)
        .map_err(|error| PrepareError::new("release-history-failed", error.to_string()))?;
    if commits.len() > MAX_RELEASE_COMMITS {
        return Err(PrepareError::new(
            "release-history-too-large",
            "release commit window exceeds the bounded limit",
        ));
    }
    if release_already_cut(repository, &release_id, &baseline, &commits)? {
        return Err(PrepareError::new(
            "release-already-cut",
            "origin/main plugin version is ahead of the Latest release with a Release commit",
        ));
    }

    let selection = select_pull_requests(service, cancellation, owner, name, &commits).await?;
    let mut selected = selection.selected;
    selected.sort_by_key(|pull_request| pull_request.number);
    write_pr_list(out_dir, service, cancellation, owner, name, &selected).await?;

    let window_base_hex = window_base.to_hex();
    let classification = classify(
        root,
        repository,
        Some(&window_base_hex),
        Some(&release_sha),
        false,
    )
    .map_err(|error| PrepareError::new("classify-bump-failed", error))?;
    let (bump, new_version) = match arguments.bump {
        Some(bump) => (
            bump,
            apply_bump(&classification.current_version, bump)
                .map_err(|error| PrepareError::new("classify-bump-failed", error))?,
        ),
        None => (classification.bump, classification.new_version),
    };
    emit_kv("BASELINE_TAG", &baseline);
    emit_kv("RELEASE_SHA", &release_sha);
    emit_kv("CURRENT_VERSION", &classification.current_version);
    emit_kv("NEW_VERSION", &new_version);
    emit_kv("BUMP_TYPE", bump.machine());
    emit_kv("PR_COUNT", &selection.written.len().to_string());
    emit_kv(
        "IGNORED_LARCHLOG_PR_COUNT",
        &selection.ignored.len().to_string(),
    );
    emit_kv(
        "PR_LIST_FILE",
        &arguments.out_dir.join("pr-list.tsv").to_string_lossy(),
    );
    commits.clear();
    Ok(())
}

async fn write_pr_list<S: ReleasePlanningService + ?Sized>(
    out_dir: &TemporaryRoot,
    service: &S,
    cancellation: &Cancellation,
    owner: &str,
    name: &str,
    selected: &[ReleasePullRequest],
) -> Result<(), PrepareError> {
    write_named_pr_list(
        out_dir,
        service,
        cancellation,
        owner,
        name,
        selected,
        "pr-list.tsv",
    )
    .await
}

async fn select_pull_requests<S: ReleasePlanningService + ?Sized>(
    service: &S,
    cancellation: &Cancellation,
    owner: &str,
    name: &str,
    commits: &[larch_core::Commit],
) -> Result<ReleaseSelection, PrepareError> {
    let suffix_numbers = commits
        .iter()
        .filter_map(|commit| pr_suffix(&String::from_utf8_lossy(&commit.subject)))
        .collect::<BTreeSet<_>>();
    let mut selected = Vec::new();
    let mut written = BTreeSet::new();
    let mut ignored = BTreeSet::new();
    let mut release_pull_requests = BTreeSet::new();
    let mut unresolved = BTreeSet::new();
    for number in suffix_numbers {
        match retry_transient(|| service.pull_request(cancellation, owner, name, number)).await {
            Ok(pull_request) if is_release_subject(&pull_request.title) => {
                release_pull_requests.insert(pull_request.number);
            }
            Ok(pull_request) if is_log_housekeeping(&pull_request.title) => {
                ignored.insert(pull_request.number);
            }
            Ok(pull_request) => {
                written.insert(pull_request.number);
                selected.push(pull_request);
            }
            Err(_) => {
                unresolved.insert(number);
            }
        }
    }

    let mut orphans = Vec::new();
    for commit in commits {
        let subject = String::from_utf8_lossy(&commit.subject);
        if pr_suffix(&subject).is_some_and(|number| !unresolved.contains(&number)) {
            continue;
        }
        let sha = commit.id.to_hex();
        let pulls =
            retry_transient(|| service.commit_pull_requests(cancellation, owner, name, &sha))
                .await
                .map_err(|error| {
                    PrepareError::new(
                        "pr-metadata-incomplete",
                        format!("commits-to-pulls lookup failed for {sha}: {error}"),
                    )
                })?;
        let Some(pull_request) = pulls.into_iter().next() else {
            eprintln!(
                "WARN: commit {sha} has no associated pull request: {}",
                SafeText::diagnostic(&subject)
            );
            orphans.push(sha);
            continue;
        };
        let number = pull_request.number;
        if written.contains(&number)
            || ignored.contains(&number)
            || release_pull_requests.contains(&number)
        {
            continue;
        }
        if is_release_subject(&pull_request.title) {
            release_pull_requests.insert(number);
            continue;
        }
        if is_log_housekeeping(&pull_request.title) {
            ignored.insert(number);
            continue;
        }
        eprintln!(
            "NOTE: commit {sha} resolved to PR #{number} via GitHub API ({})",
            SafeText::diagnostic(&subject)
        );
        written.insert(number);
        selected.push(pull_request);
    }
    if !orphans.is_empty() {
        let csv = orphans.join(",");
        println!("UNMATCHED_COMMITS={csv}");
        return Err(PrepareError::new(
            "unmatched-commits",
            format!("commits with no associated pull request: {csv}"),
        ));
    }

    Ok(ReleaseSelection {
        selected,
        written,
        ignored,
    })
}

fn open_repository() -> Result<(PathBuf, GixRepository), String> {
    let current = env::current_dir().map_err(|_| "cannot resolve current directory".to_owned())?;
    let root = RepositoryRoot::resolve(Some(&current)).map_err(|error| error.to_string())?;
    let repository = GixRepository::open(root.path()).map_err(|error| error.to_string())?;
    Ok((root.path().to_owned(), repository))
}

fn verify_origin(
    repository: &GixRepository,
    expected: &larch_core::GitHubRepositoryRef,
) -> Result<(), PrepareError> {
    let remotes = repository
        .remotes()
        .map_err(|error| PrepareError::new("origin-repo-mismatch", error.to_string()))?;
    let actual = remotes
        .iter()
        .find(|remote| remote.name == b"origin")
        .and_then(|remote| remote.fetch_url.as_deref())
        .and_then(|url| std::str::from_utf8(url).ok())
        .and_then(parse_github_remote_url);
    if actual.as_deref() == Some(&format!("{}/{}", expected.owner(), expected.name())) {
        Ok(())
    } else {
        Err(PrepareError::new(
            "origin-repo-mismatch",
            format!(
                "origin does not match --repo ({}/{})",
                expected.owner(),
                expected.name()
            ),
        ))
    }
}

/// Require a clean checkout on local `main` without chasing `origin/main`.
///
/// The release window is pinned to the fetched `origin/main` tip (`RELEASE_SHA`),
/// so local `main` may sit behind or ahead of that tip. Tip equality is not a
/// prepare precondition.
fn verify_main_worktree(repository: &GixRepository) -> Result<(), PrepareError> {
    let head = repository
        .head()
        .map_err(|error| PrepareError::new("not-on-main", error.to_string()))?;
    let Head::Symbolic { name, target } = head else {
        return Err(PrepareError::new("not-on-main", "HEAD is not local main"));
    };
    if name.as_bytes() != b"refs/heads/main" {
        return Err(PrepareError::new("not-on-main", "HEAD is not local main"));
    }
    let main = resolve(repository, "main")
        .map_err(|_| PrepareError::new("not-on-main", "main is not resolvable"))?;
    if target != main {
        return Err(PrepareError::new(
            "not-on-main",
            "HEAD does not identify local main",
        ));
    }
    let status = repository
        .local_status(&StatusOptions::default())
        .map_err(|error| PrepareError::new("main-status-failed", error.to_string()))?;
    if status.is_dirty() {
        return Err(PrepareError::new("dirty-main", "main worktree is dirty"));
    }
    Ok(())
}

fn fetch_origin_main(root: &Path) -> Result<(), PrepareError> {
    let git = GitCommandRuntime::for_repository(root)
        .map_err(|error| PrepareError::new("fetch-origin-main-failed", error))?;
    let request = release_common::main_fetch_request()
        .map_err(|error| PrepareError::new("fetch-origin-main-failed", error))?;
    git.runtime
        .block_on(git.git_cli().fetch(request, &git.cancellation))
        .map(|_| ())
        .map_err(|error| PrepareError::new("fetch-origin-main-failed", error.to_string()))
}

fn classify(
    root: &Path,
    repository: &GixRepository,
    base_ref: Option<&str>,
    head_ref: Option<&str>,
    require_worktree_match: bool,
) -> Result<Classification, String> {
    let worktree_version = strict_plugin_version(&root.join(PLUGIN_JSON))?;
    let compare_ref = head_ref.unwrap_or("HEAD");
    let compare = resolve(repository, compare_ref)
        .map_err(|_| format!("could not resolve --head ref: {compare_ref}"))?;
    let current_version = if head_ref.is_some() {
        let bytes = repository
            .blob_at_commit(&compare, &GitPath::new(PLUGIN_JSON))
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "could not read plugin.json at --head ref".to_owned())?;
        let version = strict_plugin_version_bytes(&bytes, "plugin.json at --head ref")?;
        if require_worktree_match && version != worktree_version {
            return Err(format!(
                "worktree plugin.json version ({worktree_version}) != --head ref ({version})"
            ));
        }
        version
    } else {
        worktree_version
    };
    let (base, skip_idempotency) = if let Some(base_ref) = base_ref {
        (
            resolve(repository, base_ref)
                .map_err(|_| format!("could not resolve --base ref: {base_ref}"))?,
            true,
        )
    } else {
        let base = resolve(repository, "origin/main")
            .ok()
            .and_then(|main| repository.merge_base(&main, &compare).ok())
            .or_else(|| {
                resolve(repository, "main")
                    .ok()
                    .and_then(|main| repository.merge_base(&main, &compare).ok())
            })
            .ok_or_else(|| "could not resolve merge-base against origin/main or main".to_owned())?;
        (base, false)
    };

    if !skip_idempotency
        && idempotency_subject(repository, &compare)?
            .is_some_and(|subject| is_bump_subject(&subject))
    {
        return Ok(classification_result(
            &base,
            current_version.clone(),
            current_version,
            BumpType::None,
            &[],
            &[],
        ));
    }

    let (major, minor) = classify_changes(repository, &base, &compare)?;
    let bump = if !major.is_empty() {
        BumpType::Major
    } else if !minor.is_empty() {
        BumpType::Minor
    } else {
        BumpType::Patch
    };
    let new_version = apply_bump(&current_version, bump)?;
    Ok(classification_result(
        &base,
        current_version,
        new_version,
        bump,
        &major,
        &minor,
    ))
}

fn classify_changes(
    repository: &GixRepository,
    base: &ObjectId,
    compare: &ObjectId,
) -> Result<(Vec<String>, Vec<String>), String> {
    let base_commit = repository
        .walk_commits(base, 1)
        .map_err(|error| error.to_string())?
        .into_iter()
        .next()
        .ok_or_else(|| "base commit is missing".to_owned())?;
    let head_commit = repository
        .walk_commits(compare, 1)
        .map_err(|error| error.to_string())?
        .into_iter()
        .next()
        .ok_or_else(|| "head commit is missing".to_owned())?;
    let changes = repository
        .tree_changes(&base_commit.tree, &head_commit.tree)
        .map_err(|error| error.to_string())?;
    let mut major = Vec::new();
    let mut minor = Vec::new();
    for change in changes.entries() {
        let path = String::from_utf8_lossy(change.path.as_bytes());
        let source = change
            .source_path
            .as_ref()
            .map(|value| String::from_utf8_lossy(value.as_bytes()));
        match change.kind {
            ChangeKind::Deleted if public_surface(&path) => {
                major.push(format!("Deleted `{path}`"));
            }
            ChangeKind::Added if public_surface(&path) => {
                minor.push(format!("Added `{path}`"));
            }
            ChangeKind::Renamed => {
                let old = source.as_deref().unwrap_or_default();
                if skill_path(old) {
                    major.push(format!("Renamed skill `{old}` → `{path}`"));
                } else if agent_path(old) {
                    major.push(format!("Renamed agent `{old}` → `{path}`"));
                }
            }
            ChangeKind::Modified if public_surface(&path) => {
                classify_frontmatter_change(
                    repository,
                    base,
                    compare,
                    change.path.as_bytes(),
                    &mut major,
                    &mut minor,
                )?;
            }
            _ => {}
        }
    }
    Ok((major, minor))
}

fn classify_frontmatter_change(
    repository: &GixRepository,
    base: &ObjectId,
    head: &ObjectId,
    path: &[u8],
    major: &mut Vec<String>,
    minor: &mut Vec<String>,
) -> Result<(), String> {
    let path_value = GitPath::new(path);
    let old = repository
        .blob_at_commit(base, &path_value)
        .map_err(|error| error.to_string())?;
    let new = repository
        .blob_at_commit(head, &path_value)
        .map_err(|error| error.to_string())?;
    let (Some(old), Some(new)) = (old, new) else {
        return Ok(());
    };
    let display = String::from_utf8_lossy(path);
    let old = String::from_utf8_lossy(&old);
    let new = String::from_utf8_lossy(&new);
    let old_frontmatter = frontmatter(&old);
    let new_frontmatter = frontmatter(&new);
    let old_name = frontmatter_field(&old_frontmatter, "name");
    let new_name = frontmatter_field(&new_frontmatter, "name");
    if !old_name.is_empty() && new_name.is_empty() {
        major.push(format!("Removed `name:` frontmatter from `{display}`"));
    } else if !old_name.is_empty() && old_name != new_name {
        major.push(format!(
            "Renamed `name:` frontmatter in `{display}` ({old_name} → {new_name})"
        ));
    }
    let old_flags = flag_tokens(frontmatter_field(&old_frontmatter, "argument-hint"));
    let new_flags = flag_tokens(frontmatter_field(&new_frontmatter, "argument-hint"));
    for flag in old_flags.difference(&new_flags) {
        major.push(format!(
            "Removed `{flag}` from argument-hint in `{display}`"
        ));
    }
    for flag in new_flags.difference(&old_flags) {
        minor.push(format!("Added `{flag}` to argument-hint in `{display}`"));
    }
    Ok(())
}

fn classification_result(
    base: &ObjectId,
    current_version: String,
    new_version: String,
    bump: BumpType,
    major: &[String],
    minor: &[String],
) -> Classification {
    let base_hex = base.to_hex();
    let mut reasoning = format!(
        "# Version Bump Reasoning\n\n- **Base commit**: `{}`\n- **Current version**: `{current_version}`\n- **Classification scope**: `skills/**` and `agents/**` only (public plugin surface).\n\n## Result: {}\n\n- **New version**: `{new_version}`\n",
        &base_hex[..7.min(base_hex.len())],
        bump.machine()
    );
    if !major.is_empty() {
        reasoning.push_str("\n### MAJOR evidence\n");
        for item in major {
            let _ = writeln!(reasoning, "- {item}");
        }
    }
    if !minor.is_empty() {
        reasoning.push_str("\n### MINOR evidence\n");
        for item in minor {
            let _ = writeln!(reasoning, "- {item}");
        }
    }
    if bump == BumpType::Patch {
        reasoning.push_str("\n### PATCH rationale\n\nNo MAJOR or MINOR evidence found in the public plugin surface. Defaulting to PATCH for this release classification.");
    }
    Classification {
        current_version,
        new_version,
        bump,
        reasoning: SafeText::from_untrusted(reasoning).to_string(),
    }
}

fn idempotency_subject(
    repository: &GixRepository,
    head: &ObjectId,
) -> Result<Option<String>, String> {
    let commits = repository
        .walk_commits(head, IDEMPOTENCY_DEPTH + 1)
        .map_err(|error| error.to_string())?;
    for commit in commits {
        let subject = String::from_utf8_lossy(&commit.subject).into_owned();
        let expected = if subject.starts_with(TRANSPARENT_CHANGELOG_PREFIX) {
            Some("CHANGELOG.md")
        } else if subject.starts_with(TRANSPARENT_LOG_PREFIX) {
            Some("larch-logs/")
        } else {
            None
        };
        let Some(expected) = expected else {
            return Ok(Some(subject));
        };
        let Some(parent) = commit.parents.first() else {
            return Ok(Some(subject));
        };
        let parent = repository
            .walk_commits(parent, 1)
            .map_err(|error| error.to_string())?
            .into_iter()
            .next()
            .ok_or_else(|| "transparent commit parent is missing".to_owned())?;
        let changes = repository
            .tree_changes(&parent.tree, &commit.tree)
            .map_err(|error| error.to_string())?;
        let valid = !changes.is_empty()
            && changes.entries().iter().all(|change| {
                let path = change.path.as_bytes();
                expected == "CHANGELOG.md" && path == b"CHANGELOG.md"
                    || expected == "larch-logs/"
                        && (path == b"larch-logs" || path.starts_with(b"larch-logs/"))
            });
        if !valid {
            return Ok(Some(subject));
        }
    }
    Ok(None)
}

fn release_already_cut(
    repository: &GixRepository,
    origin_main: &ObjectId,
    baseline: &str,
    commits: &[larch_core::Commit],
) -> Result<bool, PrepareError> {
    let bytes = repository
        .blob_at_commit(origin_main, &GitPath::new(PLUGIN_JSON))
        .map_err(|error| PrepareError::new("release-history-failed", error.to_string()))?
        .ok_or_else(|| PrepareError::new("release-history-failed", "plugin.json is missing"))?;
    let origin_version = strict_plugin_version_bytes(&bytes, "origin/main plugin.json")
        .map_err(|error| PrepareError::new("release-history-failed", error))?;
    let baseline_version = baseline.strip_prefix('v').unwrap_or_default();
    Ok(semver(&origin_version) > semver(baseline_version)
        && commits.iter().any(|commit| {
            let subject = String::from_utf8_lossy(&commit.subject);
            is_release_subject(&subject)
        }))
}

async fn companion_title<S: ReleasePlanningService + ?Sized>(
    service: &S,
    cancellation: &Cancellation,
    owner: &str,
    repo: &str,
    pull_request: &ReleasePullRequest,
) -> String {
    let Some(number) = pull_request
        .title
        .strip_prefix("Fixes #")
        .and_then(|tail| tail.split_once(':'))
        .and_then(|(number, _)| number.parse::<u64>().ok())
    else {
        return pull_request.title.clone();
    };
    retry_transient(|| service.issue_title(cancellation, owner, repo, number))
        .await
        .ok()
        .filter(|title| !title.trim().is_empty())
        .unwrap_or_else(|| pull_request.title.clone())
}

fn prepare_out_dir(path: &Path) -> Result<TemporaryRoot, PrepareError> {
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        env::current_dir()
            .map_err(|error| PrepareError::new("invalid-args", error.to_string()))?
            .join(path)
    };
    fs::create_dir_all(&path)
        .map_err(|error| PrepareError::new("invalid-args", error.to_string()))?;
    TemporaryRoot::resolve(Some(&path))
        .map_err(|error| PrepareError::new("invalid-args", error.to_string()))
}

fn reasoning_file(content: &str) -> Result<PathBuf, String> {
    if let Some(directory) = env::var_os("IMPLEMENT_TMPDIR").map(PathBuf::from)
        && let Ok(root) = prepare_reasoning_root(&directory)
    {
        let path = root
            .confine("bump-version-reasoning.md", PathIntent::Write)
            .map_err(|error| error.to_string())?;
        if atomic_write_utf8(&path, content, 0o600).is_ok() {
            return Ok(path.path().to_path_buf());
        }
    }
    let directory = env::var_os("TMPDIR").map_or_else(env::temp_dir, PathBuf::from);
    let root = prepare_reasoning_root(&directory)?;
    for attempt in 0..100_u32 {
        let path = root.path().join(format!(
            "bump-version-reasoning.{}.{}",
            std::process::id(),
            attempt
        ));
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => {
                file.write_all(content.as_bytes())
                    .map_err(|error| error.to_string())?;
                return Ok(path);
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error.to_string()),
        }
    }
    Err("could not allocate reasoning file".to_owned())
}

fn prepare_reasoning_root(path: &Path) -> Result<TemporaryRoot, String> {
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        env::current_dir()
            .map_err(|error| error.to_string())?
            .join(path)
    };
    fs::create_dir_all(&path).map_err(|error| error.to_string())?;
    TemporaryRoot::resolve(Some(&path)).map_err(|error| error.to_string())
}

fn strict_plugin_version(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|_| format!("{PLUGIN_JSON} not found"))?;
    strict_plugin_version_bytes(&bytes, PLUGIN_JSON)
}

fn strict_plugin_version_bytes(bytes: &[u8], source: &str) -> Result<String, String> {
    let value: Value =
        serde_json::from_slice(bytes).map_err(|_| format!("{source} is not valid JSON"))?;
    let version = value
        .get("version")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{source} missing .version field"))?;
    if semver(version).is_none() {
        return Err(format!(
            "version {version:?} is not semver (expected X.Y.Z)"
        ));
    }
    Ok(version.to_owned())
}

fn apply_bump(version: &str, bump: BumpType) -> Result<String, String> {
    let (major, minor, patch) = semver(version).ok_or_else(|| "invalid version".to_owned())?;
    match bump {
        BumpType::Major => major
            .checked_add(1)
            .map(|major| format!("{major}.0.0"))
            .ok_or_else(|| "major version overflow".to_owned()),
        BumpType::Minor => minor
            .checked_add(1)
            .map(|minor| format!("{major}.{minor}.0"))
            .ok_or_else(|| "minor version overflow".to_owned()),
        BumpType::Patch => patch
            .checked_add(1)
            .map(|patch| format!("{major}.{minor}.{patch}"))
            .ok_or_else(|| "patch version overflow".to_owned()),
        BumpType::None => Ok(version.to_owned()),
    }
}

fn resolve(repository: &GixRepository, revision: &str) -> Result<ObjectId, String> {
    repository
        .resolve_revision(&Revision::new(revision))
        .map_err(|error| error.to_string())
}

/// Resolve the commit that starts a release-history window.
///
/// Legacy release tags name a commit on `main` and remain their own window
/// base. Projection tags name a one- or two-parent child of `main`; their first
/// parent is the merged release commit and becomes the window base. A
/// projection is accepted only when that parent is on the source history and
/// every tree change from it is confined to `plugin/`.
fn release_window_base(
    repository: &GixRepository,
    baseline: &ObjectId,
    source: &ObjectId,
) -> Result<Option<ObjectId>, String> {
    if repository
        .is_ancestor(baseline, source)
        .map_err(|error| error.to_string())?
    {
        return Ok(Some(baseline.to_owned()));
    }
    let commit = repository
        .walk_commits(baseline, 1)
        .map_err(|error| error.to_string())?
        .into_iter()
        .next()
        .ok_or_else(|| "baseline tag commit is missing".to_owned())?;
    if !(1..=2).contains(&commit.parents.len()) {
        return Ok(None);
    }
    let first_parent = commit
        .parents
        .first()
        .expect("one- or two-parent projection commit has a first parent");
    if !repository
        .is_ancestor(first_parent, source)
        .map_err(|error| error.to_string())?
    {
        return Ok(None);
    }
    let parent = repository
        .walk_commits(first_parent, 1)
        .map_err(|error| error.to_string())?
        .into_iter()
        .next()
        .ok_or_else(|| "projection first parent commit is missing".to_owned())?;
    let changes = repository
        .tree_changes(&parent.tree, &commit.tree)
        .map_err(|error| error.to_string())?;
    let is_plugin_path = |path: &[u8]| path == b"plugin" || path.starts_with(b"plugin/");
    let changes_only_plugin = changes.entries().iter().all(|change| {
        is_plugin_path(change.path.as_bytes())
            && (change.kind != ChangeKind::Renamed
                || change
                    .source_path
                    .as_ref()
                    .is_some_and(|path| is_plugin_path(path.as_bytes())))
    });
    Ok(changes_only_plugin.then(|| first_parent.to_owned()))
}

fn pr_suffix(subject: &str) -> Option<u64> {
    let suffix = subject.strip_suffix(')')?.rsplit_once(" (#")?.1;
    suffix.parse().ok()
}

fn is_release_subject(subject: &str) -> bool {
    let Some(tail) = subject.strip_prefix("Release v") else {
        return false;
    };
    if semver(tail).is_some() {
        return true;
    }
    let Some((version, number)) = tail
        .strip_suffix(')')
        .and_then(|tail| tail.split_once(" (#"))
    else {
        return false;
    };
    semver(version).is_some()
        && !number.is_empty()
        && number.bytes().all(|byte| byte.is_ascii_digit())
}

fn is_bump_subject(subject: &str) -> bool {
    subject
        .strip_prefix("Release v")
        .or_else(|| subject.strip_prefix("Bump version to "))
        .is_some_and(|version| semver(version).is_some())
}

fn is_log_housekeeping(title: &str) -> bool {
    title.starts_with(TRANSPARENT_LOG_PREFIX)
}

fn skill_path(path: &str) -> bool {
    path.strip_prefix("skills/")
        .and_then(|tail| tail.strip_suffix("/SKILL.md"))
        .is_some_and(|name| !name.is_empty() && !name.contains('/'))
}

fn agent_path(path: &str) -> bool {
    path.strip_prefix("agents/")
        .and_then(|tail| tail.strip_suffix(".md"))
        .is_some_and(|name| !name.is_empty() && !name.contains('/'))
}

fn public_surface(path: &str) -> bool {
    skill_path(path) || agent_path(path)
}

fn frontmatter(text: &str) -> String {
    let mut lines = text.lines();
    if lines.next() != Some("---") {
        return String::new();
    }
    let mut body = Vec::new();
    for line in lines {
        if line == "---" {
            return body.join("\n");
        }
        body.push(line);
    }
    String::new()
}

fn frontmatter_field<'a>(frontmatter: &'a str, field: &str) -> &'a str {
    let prefix = format!("{field}: ");
    frontmatter
        .lines()
        .find_map(|line| line.strip_prefix(&prefix))
        .map(str::trim)
        .unwrap_or_default()
}

fn flag_tokens(text: &str) -> BTreeSet<String> {
    text.split(|character: char| !(character.is_ascii_alphanumeric() || "_-".contains(character)))
        .filter(|token| token.starts_with("--") && token.len() > 2)
        .map(str::to_owned)
        .collect()
}

fn tsv(value: &str) -> String {
    value.replace(['\t', '\r', '\n'], " ")
}

#[cfg(test)]
#[path = "release_prepare/tests.rs"]
mod tests;
