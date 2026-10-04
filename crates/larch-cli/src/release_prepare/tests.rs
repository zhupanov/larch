#[cfg(test)]
mod release_prepare_tests {
    use super::super::{
        BumpType, Cancellation, GixRepository, LarchRuntime, PrepareArguments,
        ReconcileNotesArguments, ReleasePlanningService, ReleasePullRequest, apply_bump, classify,
        companion_title, flag_tokens, frontmatter, frontmatter_field, idempotency_subject,
        is_bump_subject, is_log_housekeeping, is_release_subject, pr_suffix, prepare_out_dir,
        prepare_with_service, public_surface, read_prepared_pr_numbers,
        reconcile_notes_with_service, release_already_cut, release_window_base, resolve,
        select_pull_requests, semver, skill_path, strict_plugin_version_bytes, tsv,
        verify_main_worktree, verify_origin,
    };
    use crate::github_repository_resolution::parse_github_remote_url;
    use larch_adapters::{TemporaryRoot, github::GitHubOperationError};
    use larch_core::{GitHubRepositoryRef, ProcessCancellation, RepositoryRead};
    use larch_test_support::{GitFixture, GitRepository};
    use std::{
        collections::{BTreeMap, BTreeSet},
        fs,
        path::Path,
    };

    #[derive(Default)]
    struct FakeReleaseService {
        latest: Option<String>,
        open: Vec<ReleasePullRequest>,
        pulls: BTreeMap<u64, ReleasePullRequest>,
        failed_pulls: BTreeSet<u64>,
        associated: Vec<ReleasePullRequest>,
        issues: BTreeMap<u64, String>,
        fail_associated: bool,
        fail_latest: bool,
        fail_open: bool,
    }

    impl ReleasePlanningService for FakeReleaseService {
        async fn latest_release_tag(
            &self,
            _cancellation: &dyn ProcessCancellation,
            _owner: &str,
            _repo: &str,
        ) -> Result<Option<String>, GitHubOperationError> {
            if self.fail_latest {
                Err(GitHubOperationError::Malformed("latest release"))
            } else {
                Ok(self.latest.clone())
            }
        }

        async fn list_open_pull_requests(
            &self,
            _cancellation: &dyn ProcessCancellation,
            _owner: &str,
            _repo: &str,
        ) -> Result<Vec<ReleasePullRequest>, GitHubOperationError> {
            if self.fail_open {
                Err(GitHubOperationError::Malformed("open pull requests"))
            } else {
                Ok(self.open.clone())
            }
        }

        async fn pull_request(
            &self,
            _cancellation: &dyn ProcessCancellation,
            _owner: &str,
            _repo: &str,
            number: u64,
        ) -> Result<ReleasePullRequest, GitHubOperationError> {
            if self.failed_pulls.contains(&number) {
                return Err(GitHubOperationError::Malformed("missing pull request"));
            }
            self.pulls
                .get(&number)
                .cloned()
                .ok_or(GitHubOperationError::Malformed("missing pull request"))
        }

        async fn commit_pull_requests(
            &self,
            _cancellation: &dyn ProcessCancellation,
            _owner: &str,
            _repo: &str,
            _commit: &str,
        ) -> Result<Vec<ReleasePullRequest>, GitHubOperationError> {
            if self.fail_associated {
                Err(GitHubOperationError::Malformed("associated lookup failed"))
            } else {
                Ok(self.associated.clone())
            }
        }

        async fn issue_title(
            &self,
            _cancellation: &dyn ProcessCancellation,
            _owner: &str,
            _repo: &str,
            number: u64,
        ) -> Result<String, GitHubOperationError> {
            self.issues
                .get(&number)
                .cloned()
                .ok_or(GitHubOperationError::Malformed("missing issue"))
        }
    }

    fn pull_request(number: u64, title: &str) -> ReleasePullRequest {
        ReleasePullRequest {
            number,
            title: title.to_owned(),
            labels: vec!["release-note".to_owned()],
            author: "author".to_owned(),
            url: format!("https://github.com/o/r/pull/{number}"),
            head_ref: format!("feature-{number}"),
        }
    }

    fn checked_git<const N: usize>(repository: &GitRepository, arguments: [&str; N]) {
        let output = repository.git(arguments).expect("run fixture Git");
        assert!(
            output.success(),
            "fixture Git failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn release_repository() -> GitRepository {
        let repository = GitRepository::builder(GitFixture::Unborn)
            .build()
            .expect("release repository");
        repository
            .write(".claude-plugin/plugin.json", br#"{"version":"1.2.3"}"#)
            .expect("plugin manifest");
        repository
            .write(
                "skills/base/SKILL.md",
                b"---\nname: base\nargument-hint: [--old]\n---\n",
            )
            .expect("base skill");
        repository
            .write(".gitattributes", b"* text=auto eol=lf\n")
            .expect("conversion attributes");
        repository.write("README.md", b"base\n").expect("readme");
        checked_git(&repository, ["add", "-A"]);
        checked_git(&repository, ["commit", "--quiet", "-m", "base"]);
        checked_git(&repository, ["tag", "v1.2.3"]);
        repository
            .write("README.md", b"release change\n")
            .expect("release change");
        checked_git(&repository, ["add", "-A"]);
        checked_git(&repository, ["commit", "--quiet", "-m", "Feature (#42)"]);
        checked_git(
            &repository,
            ["remote", "add", "origin", "https://github.com/o/r.git"],
        );
        checked_git(
            &repository,
            ["update-ref", "refs/remotes/origin/main", "HEAD"],
        );
        repository
    }

    fn projection_baseline_repository() -> GitRepository {
        let repository = release_repository();
        checked_git(&repository, ["branch", "stable", "v1.2.3"]);
        checked_git(&repository, ["switch", "stable"]);
        repository
            .write("stable-only.txt", b"prior release pin\n")
            .expect("stable file");
        checked_git(&repository, ["add", "-A"]);
        checked_git(
            &repository,
            ["commit", "--quiet", "-m", "prior stable release"],
        );
        checked_git(&repository, ["switch", "--detach", "v1.2.3"]);
        checked_git(
            &repository,
            [
                "merge",
                "--quiet",
                "--no-ff",
                "-s",
                "ours",
                "stable",
                "-m",
                "Release v1.2.3 plugin projection",
            ],
        );
        repository
            .write(
                "plugin/.claude-plugin/plugin.json",
                br#"{"version":"1.2.3"}"#,
            )
            .expect("projected plugin manifest");
        checked_git(&repository, ["add", "plugin"]);
        checked_git(&repository, ["commit", "--quiet", "--amend", "--no-edit"]);
        checked_git(&repository, ["tag", "--force", "v1.2.3"]);
        checked_git(&repository, ["switch", "main"]);
        repository
    }

    fn one_parent_projection_baseline_repository() -> GitRepository {
        let repository = release_repository();
        checked_git(&repository, ["switch", "--detach", "v1.2.3"]);
        repository
            .write(
                "plugin/.claude-plugin/plugin.json",
                br#"{"version":"1.2.3"}"#,
            )
            .expect("projected plugin manifest");
        checked_git(&repository, ["add", "plugin"]);
        checked_git(
            &repository,
            [
                "commit",
                "--quiet",
                "-m",
                "Release v1.2.3 plugin projection",
            ],
        );
        checked_git(&repository, ["tag", "--force", "v1.2.3"]);
        checked_git(&repository, ["switch", "main"]);
        repository
    }

    fn merge_release_repository() -> GitRepository {
        let repository = GitRepository::builder(GitFixture::Unborn)
            .build()
            .expect("release repository");
        repository
            .write(".claude-plugin/plugin.json", br#"{"version":"1.2.2"}"#)
            .expect("plugin manifest");
        repository.write("README.md", b"base\n").expect("readme");
        checked_git(&repository, ["add", "-A"]);
        checked_git(&repository, ["commit", "--quiet", "-m", "base"]);
        checked_git(&repository, ["switch", "-c", "release/v1.2.3"]);
        repository
            .write(".claude-plugin/plugin.json", br#"{"version":"1.2.3"}"#)
            .expect("release plugin manifest");
        checked_git(&repository, ["add", "-A"]);
        checked_git(
            &repository,
            ["commit", "--quiet", "-m", "Bump version to 1.2.3"],
        );
        checked_git(&repository, ["tag", "v1.2.3"]);
        checked_git(&repository, ["switch", "main"]);
        checked_git(
            &repository,
            [
                "merge",
                "--quiet",
                "--no-ff",
                "release/v1.2.3",
                "-m",
                "Merge pull request #7719 from o/release/v1.2.3",
            ],
        );
        repository
            .write("README.md", b"feature\n")
            .expect("feature change");
        checked_git(&repository, ["add", "-A"]);
        checked_git(&repository, ["commit", "--quiet", "-m", "Feature (#42)"]);
        checked_git(
            &repository,
            ["remote", "add", "origin", "https://github.com/o/r.git"],
        );
        checked_git(
            &repository,
            ["update-ref", "refs/remotes/origin/main", "HEAD"],
        );
        repository
    }

    #[test]
    fn release_subject_and_frontmatter_parsers_require_exact_delimiters() {
        assert!(is_release_subject("Release v1.2.3"));
        assert!(is_release_subject("Release v1.2.3 (#42)"));
        assert!(!is_release_subject("Release v1.2.3 trailing"));
        assert!(!is_release_subject("Release v1.2.3 (#bad)"));
        assert_eq!(frontmatter("---\nname: demo\n---"), "name: demo");
        assert!(frontmatter("---\nname: demo").is_empty());
    }

    #[test]
    fn release_note_rows_exclude_housekeeping_and_flatten_untrusted_fields() {
        assert!(is_log_housekeeping("chore(larch-logs): flush run"));
        assert!(!is_log_housekeeping("Feature"));
        assert_eq!(tsv("title\twith\r\ncontrols"), "title with  controls");
    }

    #[test]
    fn scalar_parsers_cover_valid_and_hostile_boundaries() {
        assert_eq!(semver("1.2.3"), Some((1, 2, 3)));
        for invalid in ["1.2", "1.2.3.4", "v1.2.3", "1.02.x", ""] {
            assert_eq!(semver(invalid), None, "{invalid}");
        }
        assert_eq!(apply_bump("1.2.3", BumpType::Major), Ok("2.0.0".into()));
        assert_eq!(apply_bump("1.2.3", BumpType::Minor), Ok("1.3.0".into()));
        assert_eq!(apply_bump("1.2.3", BumpType::Patch), Ok("1.2.4".into()));
        assert_eq!(apply_bump("1.2.3", BumpType::None), Ok("1.2.3".into()));
        assert!(apply_bump("bad", BumpType::Patch).is_err());
        assert!(apply_bump("18446744073709551615.0.0", BumpType::Major).is_err());
        assert!(apply_bump("0.18446744073709551615.0", BumpType::Minor).is_err());
        assert!(apply_bump("0.0.18446744073709551615", BumpType::Patch).is_err());
        assert_eq!(
            strict_plugin_version_bytes(br#"{"version":"4.5.6"}"#, "fixture"),
            Ok("4.5.6".into())
        );
        assert!(strict_plugin_version_bytes(b"{}", "fixture").is_err());
        assert!(strict_plugin_version_bytes(b"not json", "fixture").is_err());

        for remote in [
            "https://github.com/o/r.git",
            "ssh://git@github.com/o/r",
            "git@github.com:o/r.git",
        ] {
            assert_eq!(parse_github_remote_url(remote).as_deref(), Some("o/r"));
        }
        for remote in [
            "https://example.com/o/r",
            "https://github.com/o",
            "git@github.com:o/r/x",
        ] {
            assert_eq!(parse_github_remote_url(remote), None);
        }
        assert_eq!(pr_suffix("Feature (#42)"), Some(42));
        assert_eq!(pr_suffix("Feature #42"), None);
    }

    #[test]
    fn bump_subject_parser_accepts_canonical_and_legacy_shapes() {
        assert!(is_bump_subject("Release v1.2.3"));
        assert!(is_bump_subject("Bump version to 1.2.3"));
        assert!(!is_bump_subject("Release v1.2.3 (#42)"));
        assert!(!is_bump_subject("Release vbad"));
        assert!(!is_bump_subject("Bump version to bad"));
    }

    #[test]
    fn public_surface_and_frontmatter_helpers_cover_exact_shapes() {
        assert!(skill_path("skills/demo/SKILL.md"));
        assert!(public_surface("agents/demo.md"));
        for path in [
            "skills/nested/demo/SKILL.md",
            "skills/demo/readme.md",
            "agents/a/b.md",
        ] {
            assert!(!public_surface(path), "{path}");
        }
        let metadata = frontmatter("---\nname: demo\nargument-hint: [--one, --two]\n---\nbody");
        assert_eq!(frontmatter_field(&metadata, "name"), "demo");
        assert_eq!(frontmatter_field(&metadata, "missing"), "");
        assert_eq!(
            flag_tokens(frontmatter_field(&metadata, "argument-hint")),
            BTreeSet::from(["--one".to_owned(), "--two".to_owned()])
        );
        assert!(flag_tokens("words -x --").is_empty());
    }

    #[test]
    fn successful_prepare_uses_typed_service_and_writes_companion_title() {
        let fixture = release_repository();
        let repository = GixRepository::open(fixture.root()).expect("open repository");
        verify_main_worktree(&repository).expect("clean main worktree");
        let repo = GitHubRepositoryRef::new("o", "r").expect("repository reference");
        verify_origin(&repository, &repo).expect("matching origin");
        assert_eq!(
            verify_origin(
                &repository,
                &GitHubRepositoryRef::new("other", "repo").expect("other repository")
            )
            .expect_err("origin mismatch")
            .token,
            "origin-repo-mismatch"
        );

        let output = tempfile::tempdir().expect("output directory");
        let output_root = TemporaryRoot::resolve(Some(output.path())).expect("temporary root");
        let mut service = FakeReleaseService {
            latest: Some("v1.2.3".to_owned()),
            ..FakeReleaseService::default()
        };
        service
            .pulls
            .insert(42, pull_request(42, "Fixes #7: feature"));
        service.issues.insert(7, "Companion issue title".to_owned());
        let mut arguments = PrepareArguments {
            repository: repo,
            bump: None,
            out_dir: output.path().to_path_buf(),
            no_fetch: false,
        };
        let runtime = LarchRuntime::current_thread().expect("test runtime");
        runtime
            .block_on(prepare_with_service(
                &arguments,
                &output_root,
                fixture.root(),
                &repository,
                &service,
                &Cancellation::new(),
            ))
            .expect("prepare release");
        let rows =
            std::fs::read_to_string(output.path().join("pr-list.tsv")).expect("release rows");
        assert_eq!(
            rows,
            "42\tCompanion issue title\trelease-note\tauthor\thttps://github.com/o/r/pull/42\n"
        );
        arguments.bump = Some(BumpType::Major);
        runtime
            .block_on(prepare_with_service(
                &arguments,
                &output_root,
                fixture.root(),
                &repository,
                &service,
                &Cancellation::new(),
            ))
            .expect("prepare release with explicit bump");
    }

    fn assert_prepare_uses_projection_tags_first_parent(fixture: &GitRepository) {
        let repository = GixRepository::open(fixture.root()).expect("open repository");
        let baseline = resolve(&repository, "v1.2.3").expect("projection baseline");
        let main = resolve(&repository, "origin/main").expect("origin/main");
        let window_base = release_window_base(&repository, &baseline, &main)
            .expect("release window lookup")
            .expect("projection baseline is accepted");
        assert_eq!(
            window_base,
            resolve(&repository, "v1.2.3^1").expect("projection first parent")
        );

        let output = tempfile::tempdir().expect("output directory");
        let output_root = TemporaryRoot::resolve(Some(output.path())).expect("temporary root");
        let service = FakeReleaseService {
            latest: Some("v1.2.3".to_owned()),
            pulls: BTreeMap::from([(42, pull_request(42, "Feature"))]),
            ..FakeReleaseService::default()
        };
        let arguments = PrepareArguments {
            repository: GitHubRepositoryRef::new("o", "r").expect("repository reference"),
            bump: Some(BumpType::Patch),
            out_dir: output.path().to_path_buf(),
            no_fetch: false,
        };
        LarchRuntime::current_thread()
            .expect("test runtime")
            .block_on(prepare_with_service(
                &arguments,
                &output_root,
                fixture.root(),
                &repository,
                &service,
                &Cancellation::new(),
            ))
            .expect("prepare from projection tag");

        assert_eq!(
            fs::read_to_string(output.path().join("pr-list.tsv")).expect("release rows"),
            "42\tFeature\trelease-note\tauthor\thttps://github.com/o/r/pull/42\n"
        );
    }

    #[test]
    fn prepare_uses_a_two_parent_projection_tags_first_parent() {
        assert_prepare_uses_projection_tags_first_parent(&projection_baseline_repository());
    }

    #[test]
    fn prepare_uses_a_one_parent_projection_tags_first_parent() {
        assert_prepare_uses_projection_tags_first_parent(
            &one_parent_projection_baseline_repository(),
        );
    }

    #[test]
    fn prepare_excludes_the_previous_merge_landed_release_pull_request() {
        let fixture = merge_release_repository();
        let repository = GixRepository::open(fixture.root()).expect("open repository");
        let output = tempfile::tempdir().expect("output directory");
        let output_root = TemporaryRoot::resolve(Some(output.path())).expect("temporary root");
        let service = FakeReleaseService {
            latest: Some("v1.2.3".to_owned()),
            pulls: BTreeMap::from([(42, pull_request(42, "Feature"))]),
            associated: vec![pull_request(7719, "Release v1.2.3")],
            ..FakeReleaseService::default()
        };
        let arguments = PrepareArguments {
            repository: GitHubRepositoryRef::new("o", "r").expect("repository reference"),
            bump: Some(BumpType::Minor),
            out_dir: output.path().to_path_buf(),
            no_fetch: false,
        };
        let runtime = LarchRuntime::current_thread().expect("test runtime");

        let baseline = resolve(&repository, "v1.2.3").expect("baseline");
        let head = resolve(&repository, "origin/main").expect("head");
        let commits = repository
            .walk_commits_range(&baseline, &head, 10)
            .expect("release commits");
        let selection = runtime
            .block_on(select_pull_requests(
                &service,
                &Cancellation::new(),
                "o",
                "r",
                &commits,
            ))
            .expect("select release pull requests");
        assert_eq!(selection.written, BTreeSet::from([42]));
        assert!(selection.ignored.is_empty());

        runtime
            .block_on(prepare_with_service(
                &arguments,
                &output_root,
                fixture.root(),
                &repository,
                &service,
                &Cancellation::new(),
            ))
            .expect("prepare release");

        let rows = fs::read_to_string(output.path().join("pr-list.tsv"))
            .expect("release pull request rows");
        assert_eq!(
            rows,
            "42\tFeature\trelease-note\tauthor\thttps://github.com/o/r/pull/42\n"
        );
    }

    #[test]
    fn clean_main_distinguishes_status_probe_failure_from_dirty_state() {
        let fixture = release_repository();
        let index = fixture.root().join(".git/index");
        fs::remove_file(&index).expect("remove fixture index");
        fs::create_dir(&index).expect("replace index with directory");
        let repository = GixRepository::open(fixture.root()).expect("open repository");

        assert_eq!(
            verify_main_worktree(&repository)
                .expect_err("status probe must fail")
                .token,
            "main-status-failed"
        );
    }

    #[test]
    fn main_worktree_allows_local_main_behind_origin_tip() {
        let fixture = release_repository();
        let repository = GixRepository::open(fixture.root()).expect("open repository");
        let tip = resolve(&repository, "HEAD").expect("head");
        fixture
            .write("README.md", b"mid-run merge\n")
            .expect("readme");
        checked_git(&fixture, ["add", "-A"]);
        checked_git(&fixture, ["commit", "--quiet", "-m", "Mid-run PR (#99)"]);
        checked_git(&fixture, ["update-ref", "refs/remotes/origin/main", "HEAD"]);
        checked_git(&fixture, ["update-ref", "refs/heads/main", &tip.to_hex()]);
        checked_git(&fixture, ["reset", "--hard", "--quiet", "main"]);
        let behind = GixRepository::open(fixture.root()).expect("behind repository");
        verify_main_worktree(&behind).expect("behind local main is allowed");
        let origin = resolve(&behind, "origin/main").expect("origin tip");
        let main = resolve(&behind, "main").expect("local main");
        assert_ne!(origin, main);
    }

    #[test]
    fn prepare_rejects_invalid_baseline_and_open_release_branch() {
        let fixture = release_repository();
        let repository = GixRepository::open(fixture.root()).expect("open repository");
        let output = tempfile::tempdir().expect("output directory");
        let output_root = TemporaryRoot::resolve(Some(output.path())).expect("temporary root");
        let arguments = PrepareArguments {
            repository: GitHubRepositoryRef::new("o", "r").expect("repository reference"),
            bump: Some(BumpType::Major),
            out_dir: output.path().to_path_buf(),
            no_fetch: false,
        };
        let runtime = LarchRuntime::current_thread().expect("test runtime");
        let invalid = FakeReleaseService {
            latest: Some("latest".to_owned()),
            ..FakeReleaseService::default()
        };
        assert_eq!(
            runtime
                .block_on(prepare_with_service(
                    &arguments,
                    &output_root,
                    fixture.root(),
                    &repository,
                    &invalid,
                    &Cancellation::new(),
                ))
                .expect_err("invalid baseline")
                .token,
            "invalid-baseline-tag"
        );

        let release_branch = FakeReleaseService {
            latest: Some("v1.2.3".to_owned()),
            open: vec![ReleasePullRequest {
                head_ref: "release/v2.0.0".to_owned(),
                ..pull_request(99, "Release v2.0.0")
            }],
            ..FakeReleaseService::default()
        };
        assert_eq!(
            runtime
                .block_on(prepare_with_service(
                    &arguments,
                    &output_root,
                    fixture.root(),
                    &repository,
                    &release_branch,
                    &Cancellation::new(),
                ))
                .expect_err("release in progress")
                .token,
            "release-cut-in-progress"
        );
    }

    #[test]
    fn prepare_maps_service_absence_lookup_and_baseline_failures() {
        let fixture = release_repository();
        let repository = GixRepository::open(fixture.root()).expect("open repository");
        let output = tempfile::tempdir().expect("output directory");
        let output_root = TemporaryRoot::resolve(Some(output.path())).expect("temporary root");
        let arguments = PrepareArguments {
            repository: GitHubRepositoryRef::new("o", "r").expect("repository reference"),
            bump: Some(BumpType::Minor),
            out_dir: output.path().to_path_buf(),
            no_fetch: false,
        };
        let runtime = LarchRuntime::current_thread().expect("test runtime");
        let cases = [
            (
                FakeReleaseService {
                    fail_latest: true,
                    ..FakeReleaseService::default()
                },
                "gh-release-list-failed",
            ),
            (FakeReleaseService::default(), "no-unique-latest-release"),
            (
                FakeReleaseService {
                    latest: Some("v9.9.9".to_owned()),
                    ..FakeReleaseService::default()
                },
                "baseline-tag-unresolvable",
            ),
            (
                FakeReleaseService {
                    latest: Some("v1.2.3".to_owned()),
                    fail_open: true,
                    ..FakeReleaseService::default()
                },
                "release-pr-list-failed",
            ),
        ];
        for (service, token) in cases {
            assert_eq!(
                runtime
                    .block_on(prepare_with_service(
                        &arguments,
                        &output_root,
                        fixture.root(),
                        &repository,
                        &service,
                        &Cancellation::new(),
                    ))
                    .expect_err(token)
                    .token,
                token
            );
        }
    }

    #[test]
    fn pull_selection_falls_back_to_commit_associations_and_fails_closed() {
        let fixture = release_repository();
        let repository = GixRepository::open(fixture.root()).expect("open repository");
        let baseline = resolve(&repository, "v1.2.3").expect("baseline");
        let head = resolve(&repository, "origin/main").expect("head");
        let commits = repository
            .walk_commits_range(&baseline, &head, 10)
            .expect("commits");
        let runtime = LarchRuntime::current_thread().expect("test runtime");
        let fallback = FakeReleaseService {
            failed_pulls: BTreeSet::from([42]),
            associated: vec![pull_request(84, "Fallback feature")],
            ..FakeReleaseService::default()
        };
        let selection = runtime
            .block_on(select_pull_requests(
                &fallback,
                &Cancellation::new(),
                "o",
                "r",
                &commits,
            ))
            .expect("fallback selection");
        assert_eq!(selection.written, BTreeSet::from([84]));

        let housekeeping = FakeReleaseService {
            pulls: BTreeMap::from([(42, pull_request(42, "chore(larch-logs): flush"))]),
            ..FakeReleaseService::default()
        };
        let selection = runtime
            .block_on(select_pull_requests(
                &housekeeping,
                &Cancellation::new(),
                "o",
                "r",
                &commits,
            ))
            .expect("housekeeping selection");
        assert_eq!(selection.ignored, BTreeSet::from([42]));

        let associated_housekeeping = FakeReleaseService {
            failed_pulls: BTreeSet::from([42]),
            associated: vec![pull_request(42, "chore(larch-logs): associated flush")],
            ..FakeReleaseService::default()
        };
        let selection = runtime
            .block_on(select_pull_requests(
                &associated_housekeeping,
                &Cancellation::new(),
                "o",
                "r",
                &commits,
            ))
            .expect("associated housekeeping selection");
        assert_eq!(selection.ignored, BTreeSet::from([42]));

        let orphan = FakeReleaseService {
            failed_pulls: BTreeSet::from([42]),
            ..FakeReleaseService::default()
        };
        assert_eq!(
            runtime
                .block_on(select_pull_requests(
                    &orphan,
                    &Cancellation::new(),
                    "o",
                    "r",
                    &commits,
                ))
                .expect_err("orphan commit")
                .token,
            "unmatched-commits"
        );
        let failed_lookup = FakeReleaseService {
            failed_pulls: BTreeSet::from([42]),
            fail_associated: true,
            ..FakeReleaseService::default()
        };
        assert_eq!(
            runtime
                .block_on(select_pull_requests(
                    &failed_lookup,
                    &Cancellation::new(),
                    "o",
                    "r",
                    &commits,
                ))
                .expect_err("failed association lookup")
                .token,
            "pr-metadata-incomplete"
        );
    }

    #[test]
    fn pull_selection_excludes_release_pull_request_suffixes() {
        let fixture = release_repository();
        let repository = GixRepository::open(fixture.root()).expect("open repository");
        let baseline = resolve(&repository, "v1.2.3").expect("baseline");
        let head = resolve(&repository, "origin/main").expect("head");
        let commits = repository
            .walk_commits_range(&baseline, &head, 10)
            .expect("commits");
        let service = FakeReleaseService {
            pulls: BTreeMap::from([(42, pull_request(42, "Release v1.2.3"))]),
            ..FakeReleaseService::default()
        };
        let runtime = LarchRuntime::current_thread().expect("test runtime");

        let selection = runtime
            .block_on(select_pull_requests(
                &service,
                &Cancellation::new(),
                "o",
                "r",
                &commits,
            ))
            .expect("release selection");

        assert!(selection.written.is_empty());
        assert!(selection.ignored.is_empty());
    }

    #[test]
    fn classification_covers_frontmatter_and_transparent_commit_paths() {
        let fixture = release_repository();
        let repository = GixRepository::open(fixture.root()).expect("open repository");
        fixture
            .write(
                "skills/base/SKILL.md",
                b"---\nname: renamed\nargument-hint: [--new]\n---\n",
            )
            .expect("changed skill");
        checked_git(&fixture, ["add", "-A"]);
        checked_git(
            &fixture,
            ["commit", "--quiet", "-m", "Change skill metadata"],
        );
        let classification = classify(
            fixture.root(),
            &repository,
            Some("v1.2.3"),
            Some("HEAD"),
            true,
        )
        .expect("classification");
        assert_eq!(classification.bump, BumpType::Major);
        assert!(classification.reasoning.contains("Renamed `name:`"));
        assert!(classification.reasoning.contains("Removed `--old`"));
        assert!(classification.reasoning.contains("Added `--new`"));

        fixture
            .write("CHANGELOG.md", b"release notes\n")
            .expect("changelog");
        checked_git(&fixture, ["add", "-A"]);
        checked_git(
            &fixture,
            ["commit", "--quiet", "-m", "Update CHANGELOG for v1.2.4"],
        );
        let head = resolve(&repository, "HEAD").expect("head");
        assert_eq!(
            idempotency_subject(&repository, &head).expect("idempotency subject"),
            Some("Change skill metadata".to_owned())
        );
        assert_eq!(head.to_hex().len(), head.digest().len() * 2);
    }

    #[test]
    fn repository_and_classification_guards_cover_non_main_and_renames() {
        let fixture = release_repository();
        checked_git(&fixture, ["checkout", "--quiet", "--detach", "HEAD"]);
        let detached = GixRepository::open(fixture.root()).expect("detached repository");
        assert_eq!(
            verify_main_worktree(&detached)
                .expect_err("detached head")
                .token,
            "not-on-main"
        );
        checked_git(&fixture, ["checkout", "--quiet", "-b", "topic"]);
        let topic = GixRepository::open(fixture.root()).expect("topic repository");
        assert_eq!(
            verify_main_worktree(&topic).expect_err("topic head").token,
            "not-on-main"
        );

        checked_git(&fixture, ["checkout", "--quiet", "main"]);
        checked_git(&fixture, ["mv", "skills/base", "skills/renamed"]);
        checked_git(&fixture, ["commit", "--quiet", "-m", "Rename skill"]);
        let repository = GixRepository::open(fixture.root()).expect("renamed repository");
        let classification = classify(
            fixture.root(),
            &repository,
            Some("v1.2.3"),
            Some("HEAD"),
            true,
        )
        .expect("rename classification");
        assert!(classification.reasoning.contains("Renamed skill"));

        fixture
            .write("larch-logs/run/report.md", b"report\n")
            .expect("run log");
        checked_git(&fixture, ["add", "-A"]);
        checked_git(
            &fixture,
            ["commit", "--quiet", "-m", "chore(larch-logs): flush run"],
        );
        let head = resolve(&repository, "HEAD").expect("head");
        let commit = repository
            .walk_commits(&head, 1)
            .expect("head commit")
            .remove(0);
        let parent = repository
            .walk_commits(commit.parents.first().expect("parent"), 1)
            .expect("parent commit")
            .remove(0);
        let paths = repository
            .tree_changes(&parent.tree, &commit.tree)
            .expect("transparent changes")
            .entries()
            .iter()
            .map(|change| String::from_utf8_lossy(change.path.as_bytes()).into_owned())
            .collect::<Vec<_>>();
        assert_eq!(
            paths,
            ["larch-logs", "larch-logs/run", "larch-logs/run/report.md"]
        );
        assert_eq!(
            idempotency_subject(&repository, &head).expect("idempotency subject"),
            Some("Rename skill".to_owned())
        );
    }

    #[test]
    fn release_cut_and_companion_fallbacks_are_exact() {
        let fixture = release_repository();
        fixture
            .write(".claude-plugin/plugin.json", br#"{"version":"1.2.4"}"#)
            .expect("bumped manifest");
        checked_git(&fixture, ["add", "-A"]);
        checked_git(&fixture, ["commit", "--quiet", "-m", "Release v1.2.4"]);
        checked_git(&fixture, ["update-ref", "refs/remotes/origin/main", "HEAD"]);
        let repository = GixRepository::open(fixture.root()).expect("open repository");
        let baseline = resolve(&repository, "v1.2.3").expect("baseline");
        let head = resolve(&repository, "origin/main").expect("head");
        let commits = repository
            .walk_commits_range(&baseline, &head, 10)
            .expect("commits");
        assert!(
            release_already_cut(&repository, &head, "v1.2.3", &commits).expect("release cut check")
        );

        let output = tempfile::tempdir().expect("output directory");
        let output_root = TemporaryRoot::resolve(Some(output.path())).expect("temporary root");
        let release_service = FakeReleaseService {
            latest: Some("v1.2.3".to_owned()),
            ..FakeReleaseService::default()
        };
        let arguments = PrepareArguments {
            repository: GitHubRepositoryRef::new("o", "r").expect("repository reference"),
            bump: None,
            out_dir: output.path().to_path_buf(),
            no_fetch: false,
        };
        let runtime = LarchRuntime::current_thread().expect("test runtime");
        assert_eq!(
            runtime
                .block_on(prepare_with_service(
                    &arguments,
                    &output_root,
                    fixture.root(),
                    &repository,
                    &release_service,
                    &Cancellation::new(),
                ))
                .expect_err("release already cut")
                .token,
            "release-already-cut"
        );

        let service = FakeReleaseService::default();
        let ordinary = pull_request(1, "Ordinary title");
        assert_eq!(
            runtime.block_on(companion_title(
                &service,
                &Cancellation::new(),
                "o",
                "r",
                &ordinary,
            )),
            "Ordinary title"
        );
        let missing = pull_request(2, "Fixes #9: missing companion");
        assert_eq!(
            runtime.block_on(companion_title(
                &service,
                &Cancellation::new(),
                "o",
                "r",
                &missing,
            )),
            missing.title
        );
    }

    #[cfg(unix)]
    #[test]
    fn prepare_out_dir_rejects_a_symlink() {
        use std::os::unix::fs::symlink;

        let parent = tempfile::tempdir().expect("parent");
        let target = tempfile::tempdir().expect("target");
        let link = parent.path().join("out");
        symlink(target.path(), &link).expect("symlink");
        assert_eq!(
            prepare_out_dir(&link).expect_err("symlink must fail").token,
            "invalid-args"
        );
    }

    #[test]
    fn reconcile_notes_surfaces_mid_run_additions_and_excludes() {
        let fixture = projection_baseline_repository();
        fixture
            .write("README.md", b"mid-run merge\n")
            .expect("readme");
        checked_git(&fixture, ["add", "-A"]);
        checked_git(&fixture, ["commit", "--quiet", "-m", "Mid-run PR (#99)"]);
        checked_git(&fixture, ["update-ref", "refs/remotes/origin/main", "HEAD"]);
        let repository = GixRepository::open(fixture.root()).expect("open repository");
        let source = resolve(&repository, "HEAD").expect("source").to_hex();

        let prepare_list = tempfile::NamedTempFile::new().expect("prepare list");
        fs::write(
            prepare_list.path(),
            "42\tFeature\trelease-note\tauthor\thttps://github.com/o/r/pull/42\n",
        )
        .expect("write prepare list");
        let output = tempfile::tempdir().expect("output directory");
        let output_root = TemporaryRoot::resolve(Some(output.path())).expect("temporary root");
        let service = FakeReleaseService {
            pulls: BTreeMap::from([
                (42, pull_request(42, "Feature")),
                (99, pull_request(99, "Mid-run")),
            ]),
            ..FakeReleaseService::default()
        };
        let arguments = ReconcileNotesArguments {
            repository: GitHubRepositoryRef::new("o", "r").expect("repository reference"),
            baseline_tag: "v1.2.3".to_owned(),
            source_commit: source,
            pr_list: prepare_list.path().to_path_buf(),
            exclude_pr: 0,
            out_dir: output.path().to_path_buf(),
        };
        let runtime = LarchRuntime::current_thread().expect("test runtime");
        runtime
            .block_on(reconcile_notes_with_service(
                &arguments,
                &output_root,
                &repository,
                &service,
                &Cancellation::new(),
            ))
            .expect("reconcile notes");
        let full = fs::read_to_string(output.path().join("pr-list.tsv")).expect("full list");
        let added =
            fs::read_to_string(output.path().join("added-pr-list.tsv")).expect("added list");
        assert!(full.contains("42\t"));
        assert!(full.contains("99\t"));
        assert!(!added.contains("42\t"));
        assert_eq!(
            added,
            "99\tMid-run\trelease-note\tauthor\thttps://github.com/o/r/pull/99\n"
        );

        let exclude_out = tempfile::tempdir().expect("exclude output");
        let exclude_root =
            TemporaryRoot::resolve(Some(exclude_out.path())).expect("exclude temporary root");
        let exclude_arguments = ReconcileNotesArguments {
            exclude_pr: 99,
            out_dir: exclude_out.path().to_path_buf(),
            ..arguments
        };
        runtime
            .block_on(reconcile_notes_with_service(
                &exclude_arguments,
                &exclude_root,
                &repository,
                &service,
                &Cancellation::new(),
            ))
            .expect("reconcile with exclude");
        let excluded_full =
            fs::read_to_string(exclude_out.path().join("pr-list.tsv")).expect("excluded full");
        let excluded_added = fs::read_to_string(exclude_out.path().join("added-pr-list.tsv"))
            .expect("excluded added");
        assert!(excluded_full.contains("42\t"));
        assert!(!excluded_full.contains("99\t"));
        assert!(excluded_added.is_empty());
    }

    #[test]
    fn reconcile_notes_accepts_a_one_parent_projection_baseline() {
        let fixture = one_parent_projection_baseline_repository();
        let repository = GixRepository::open(fixture.root()).expect("open repository");
        let source = resolve(&repository, "HEAD").expect("source").to_hex();
        let prepare_list = tempfile::NamedTempFile::new().expect("prepare list");
        let output = tempfile::tempdir().expect("output directory");
        let output_root = TemporaryRoot::resolve(Some(output.path())).expect("temporary root");
        let service = FakeReleaseService {
            pulls: BTreeMap::from([(42, pull_request(42, "Feature"))]),
            ..FakeReleaseService::default()
        };
        let arguments = ReconcileNotesArguments {
            repository: GitHubRepositoryRef::new("o", "r").expect("repository reference"),
            baseline_tag: "v1.2.3".to_owned(),
            source_commit: source,
            pr_list: prepare_list.path().to_path_buf(),
            exclude_pr: 0,
            out_dir: output.path().to_path_buf(),
        };

        LarchRuntime::current_thread()
            .expect("test runtime")
            .block_on(reconcile_notes_with_service(
                &arguments,
                &output_root,
                &repository,
                &service,
                &Cancellation::new(),
            ))
            .expect("reconcile from one-parent projection tag");

        let expected = "42\tFeature\trelease-note\tauthor\thttps://github.com/o/r/pull/42\n";
        assert_eq!(
            fs::read_to_string(output.path().join("pr-list.tsv")).expect("full list"),
            expected
        );
        assert_eq!(
            fs::read_to_string(output.path().join("added-pr-list.tsv")).expect("added list"),
            expected
        );
    }

    #[test]
    fn reconcile_notes_maps_resolve_failures() {
        let fixture = release_repository();
        let repository = GixRepository::open(fixture.root()).expect("open repository");
        let prepare_list = tempfile::NamedTempFile::new().expect("prepare list");
        fs::write(prepare_list.path(), "42\tFeature\n").expect("write prepare list");
        let output = tempfile::tempdir().expect("output directory");
        let output_root = TemporaryRoot::resolve(Some(output.path())).expect("temporary root");
        let service = FakeReleaseService::default();
        let runtime = LarchRuntime::current_thread().expect("test runtime");
        let base = ReconcileNotesArguments {
            repository: GitHubRepositoryRef::new("o", "r").expect("repository reference"),
            baseline_tag: "v1.2.3".to_owned(),
            source_commit: resolve(&repository, "HEAD").expect("source").to_hex(),
            pr_list: prepare_list.path().to_path_buf(),
            exclude_pr: 0,
            out_dir: output.path().to_path_buf(),
        };
        let cases = [
            (
                ReconcileNotesArguments {
                    baseline_tag: "latest".to_owned(),
                    ..base.clone()
                },
                "invalid-baseline-tag",
            ),
            (
                ReconcileNotesArguments {
                    baseline_tag: "v9.9.9".to_owned(),
                    ..base.clone()
                },
                "baseline-tag-unresolvable",
            ),
            (
                ReconcileNotesArguments {
                    source_commit: "deadbeefdeadbeefdeadbeefdeadbeefdeadbeef".to_owned(),
                    ..base.clone()
                },
                "source-commit-unresolvable",
            ),
        ];
        for (arguments, token) in cases {
            assert_eq!(
                runtime
                    .block_on(reconcile_notes_with_service(
                        &arguments,
                        &output_root,
                        &repository,
                        &service,
                        &Cancellation::new(),
                    ))
                    .expect_err(token)
                    .token,
                token
            );
        }

        fixture
            .write("side-branch.txt", b"not a projection\n")
            .expect("side-branch change");
        checked_git(&fixture, ["add", "side-branch.txt"]);
        checked_git(
            &fixture,
            ["commit", "--quiet", "-m", "non-projection baseline"],
        );
        checked_git(&fixture, ["tag", "v0.0.1"]);
        checked_git(&fixture, ["reset", "--hard", "--quiet", "HEAD~1"]);
        let disconnected = GixRepository::open(fixture.root()).expect("reopen");
        assert_eq!(
            runtime
                .block_on(reconcile_notes_with_service(
                    &ReconcileNotesArguments {
                        baseline_tag: "v0.0.1".to_owned(),
                        source_commit: resolve(&disconnected, "HEAD").expect("head").to_hex(),
                        ..base
                    },
                    &output_root,
                    &disconnected,
                    &service,
                    &Cancellation::new(),
                ))
                .expect_err("baseline not ancestor")
                .token,
            "baseline-not-on-main"
        );
    }

    #[test]
    fn read_prepared_pr_numbers_rejects_bad_rows() {
        let prepare_list = tempfile::NamedTempFile::new().expect("prepare list");
        fs::write(prepare_list.path(), "42\tFeature\n").expect("write prepare list");
        assert_eq!(
            read_prepared_pr_numbers(prepare_list.path())
                .expect("valid prepared list")
                .into_iter()
                .collect::<Vec<_>>(),
            vec![42]
        );
        let bad_list = tempfile::NamedTempFile::new().expect("bad list");
        fs::write(bad_list.path(), "not-a-number\trow\n").expect("write bad list");
        assert_eq!(
            read_prepared_pr_numbers(bad_list.path())
                .expect_err("invalid prepared list")
                .token,
            "pr-list-read-failed"
        );
        assert_eq!(
            read_prepared_pr_numbers(Path::new("/tmp/larch-missing-pr-list.tsv"))
                .expect_err("missing prepared list")
                .token,
            "pr-list-read-failed"
        );
    }
}
