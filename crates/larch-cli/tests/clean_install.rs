use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    time::{Duration, SystemTime},
};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt as _;

use larch_core::shell_quote;
use sha2::{Digest as _, Sha256};
use tempfile::TempDir;

#[derive(Clone, Copy)]
struct CleanInstallCase {
    id: &'static str,
    domain: &'static str,
    verb: &'static str,
}

impl CleanInstallCase {
    const fn new(id: &'static str, domain: &'static str, verb: &'static str) -> Self {
        Self { id, domain, verb }
    }

    /// Exit status a clean dispatch of this case produces.
    ///
    /// A verb whose only offline-deterministic invocation is a refusal still
    /// proves the dispatch reached it; the case pins that refusal's code rather
    /// than reaching the network or mutating a repository to reach `0`.
    #[allow(clippy::too_many_lines)] // One comment-rich clean-install dispatch table.
    fn expected_exit(self) -> i32 {
        match self.id {
            // The two issue-graph verbs now publish their complete authorization
            // grammar for `--help` and exit 0 through the default arm below.
            // `issue state` refuses its own missing-value line. Neither
            // `parse-input` nor `fetch-issue-details` has a `--help` action, so
            // the clean-install token reads as an unknown option and each
            // refuses too. The other two issue-input verbs accept that token and
            // exit 0: `allocate-candidates` prints its usage, and `list-issues`
            // reports its fail-open envelope.
            // `issue state` refuses its own missing-value line, and neither
            // `create-one` nor `write-sentinel` has a `--help` action, so the
            // clean-install token reads as an unknown option there too.
            "clean-install-alias-generate"
            | "clean-install-alias-resolve-target"
            | "clean-install-issue-create-one"
            | "clean-install-issue-fetch-issue-details"
            | "clean-install-issue-parse-input"
            | "clean-install-issue-state"
            | "clean-install-issue-write-sentinel"
            // The six tracking-issue verbs declare no `--help` action either,
            // so the clean-install token reads as an unrecognized argument and
            // each refuses with the `argparse` usage exit code.
            | "clean-install-tracking-issue-append-comment"
            | "clean-install-tracking-issue-create-issue"
            | "clean-install-tracking-issue-mark-false-positive"
            | "clean-install-tracking-issue-read"
            | "clean-install-tracking-issue-rename"
            | "clean-install-tracking-issue-upsert-summary"
            // `oos file-conflict-deps` parses its own option line and reports
            // its own usage exit rather than the `argparse` one.
            | "clean-install-oos-file-conflict-deps"
            // Pacific timestamp treats every argument, including `--help`, as
            // its legacy unexpected-argument refusal. `local-cleanup` keeps
            // its historical raw compatibility parser, so the same token is a
            // deterministic usage refusal that proves verified dispatch.
            | "clean-install-audit-runs-pacific-timestamp"
            // `token claude-source` resolves no Claude project directory under the
            // fixture home, so it reports its `STATUS=unavailable` exit after
            // proving verified dispatch.
            | "clean-install-token-claude-source"
            // `render lane-status` parses with `add_help=False`, so the
            // clean-install `--help` token reads as an invalid flag and the verb
            // reports its own breadcrumb refusal exit.
            | "clean-install-render-lane-status"
            | "clean-install-diagrams-upsert"
            | "clean-install-session-local-cleanup"
            | "clean-install-ci-decide"
            | "clean-install-ci-wait"
            // `plan validator-autofix` refuses missing `DESIGN_TMPDIR` before
            // `--help`, matching the frozen Python wrapper order.
            | "clean-install-plan-validator-autofix"
            // `design read-result-env` parses with `add_help=False`; the
            // clean-install `--help` token is an extra argument, so the verb
            // prints its usage and exits 1 like the frozen Python parser.
            | "clean-install-design-read-result-env"
            // The Step 2b drafter wrapper ignores the unbound `--help` token and
            // then refuses on the missing `DESIGN_TMPDIR`, exiting 1.
            | "clean-install-design-step2b-drafter"
            | "clean-install-design-step3-continuation-entry"
            | "clean-install-design-step5c"
            // `ci rerun-failed` prints its help and then exits with its own
            // usage code, exactly as the retired `argparse` owner did.
            | "clean-install-ci-rerun-failed"
            | "clean-install-design-step5b-prepare"
            | "clean-install-design-step5b-annotate"
            // Manual-merge recovery catches argparse's help SystemExit and
            // publishes its frozen failed reconciliation envelope with exit 1.
            | "clean-install-ship-reconcile-manual-merge"
            | "clean-install-ship-seed-initial-state"
            | "clean-install-merge-pr"
            | "clean-install-merge-wait"
            | "clean-install-pr-create"
            | "clean-install-pr-checks"
            | "clean-install-pr-closes-issue" => 1,
            // `design parse-flags` owns the frozen Step 0-pre grammar: the
            // clean-install `--help` token is an unrecognized public flag and
            // refuses with the Python validation exit code, matching the
            // admission preflight's own refusal exit.
            "clean-install-admission-preflight" | "clean-install-design-parse-flags" => 3,
            "clean-install-token-measure-cache-efficiency"
            | "clean-install-token-measure-checks-digest-savings"
            | "clean-install-token-measure-panel-cost"
            | "clean-install-token-measure-realized-cost"
            | "clean-install-token-measure-references-heatmap" => 4,
            "clean-install-session-check-live-mutation-auth" => 5,
            // Neither `/block-issue` verb has a `--help` action either, so the
            // clean-install token reads as an unknown flag and each refuses
            // with its own usage exit code, which is the same `2` the terminal
            // snapshot reports for its missing session directory, the three
            // title verbs and four untrusted verbs report for the token they
            // cannot use, and each write verb reports for its missing required
            // option.
            "clean-install-block-issue-add-blocked-by"
            | "clean-install-implement-finalize-postbump"
            | "clean-install-implement-finalize-postmerge"
            | "clean-install-implement-finalize-teardown"
            // The four remaining CI failure verbs print their help and then
            // exit with the retired `argparse` usage code.
            | "clean-install-ci-behind-count"
            | "clean-install-ci-failed-jobs"
            | "clean-install-ci-main-health"
            | "clean-install-block-issue-remove-blocked-by"
            | "clean-install-issue-insert-signal-marker"
            | "clean-install-issue-title-archival-jq"
            | "clean-install-issue-title-eligibility"
            | "clean-install-named-block-write"
            | "clean-install-plan-block-read"
            | "clean-install-plan-block-write"
            | "clean-install-run-log-prepare-terminal-snapshot"
            | "clean-install-untrusted-file-block"
            | "clean-install-triage-apply"
            | "clean-install-triage-inspect"
            | "clean-install-triage-probe"
            // All four redaction verbs retain raw compatibility parsing. The
            // clean-install `--help` token is therefore their usage refusal.
            | "clean-install-redact-scrub-log-secrets"
            | "clean-install-redact-scrub-submodule-paths"
            | "clean-install-redact-secrets"
            | "clean-install-redact-tmpdir-paths"
            | "clean-install-untrusted-redact-stream"
            | "clean-install-untrusted-xml-escape-attr"
            // The `design stage-terminal-state`/`failure-report` wrappers
            // validate `--design-tmpdir` before `--help`, and
            // `step-final-summary` refuses the unknown wrapper argument, so each
            // exits 2 like the frozen Python owner.
            | "clean-install-design-stage-terminal-state"
            | "clean-install-design-failure-report"
            | "clean-install-design-step-final-summary"
            // `render-final-summary` ignores the unbound `--help` token in its
            // manual parser and then refuses on the unset `DESIGN_TMPDIR`,
            // exiting 2 like the frozen Python owner. (`render-gate` owns a real
            // `-h`/`--help` action and exits 0 by default.)
            | "clean-install-design-render-final-summary"
            // Neither final-report verb declares a `--help` action either, so
            // the clean-install token reads as an unrecognized argument and each
            // refuses for its missing `--implement-tmpdir`.
            | "clean-install-final-report-step18b"
            | "clean-install-final-report-write"
            // None of the four execution-issue verbs declares a `--help`
            // action, so the clean-install token reads as an unrecognized
            // argument and each refuses with the same usage exit code.
            | "clean-install-execution-issues-append"
            | "clean-install-execution-issues-flush"
            | "clean-install-execution-issues-flush-safety-net"
            | "clean-install-execution-issues-refresh"
            // No `oos` verb declares a `--help` action either: the two hand
            // rolled option lines report their own usage exit `1`, and the
            // three `argparse`-shaped ones refuse the token with `2`.
            | "clean-install-oos-materialize-manifest"
            | "clean-install-oos-issue-cap"
            | "clean-install-oos-disposition-gate"
            | "clean-install-oos-disposition-checkpoint"
            | "clean-install-oos-file"
            // The combine-issues compatibility verbs receive the fixture's
            // `--help` token as a raw argument, so their argparse boundary
            // proves dispatch by refusing it with the standard usage code.
            | "clean-install-combine-issues-apply"
            | "clean-install-combine-issues-close-eligible"
            | "clean-install-combine-issues-close-sources"
            | "clean-install-combine-issues-close-stale"
            | "clean-install-combine-issues-fetch"
            | "clean-install-combine-issues-fetch-deps"
            | "clean-install-combine-issues-list-open"
            | "clean-install-combine-issues-plan-audit"
            | "clean-install-combine-issues-plan-inherited"
            | "clean-install-combine-issues-prose-audit"
            // `generate` keeps its raw compatibility boundary, so `--help`
            // proves that the verified wrapper reaches each selector while the
            // selector rejects its unsupported extra argument.
            | "clean-install-generate-check"
            | "clean-install-generate-code-reviewer-agent"
            | "clean-install-generate-codex-implementer"
            | "clean-install-generate-cursor-implementer"
            | "clean-install-generate-pre-rendered-reviewer-prompts"
            | "clean-install-generate-reviewer-code-robustness-agent"
            | "clean-install-generate-reviewer-correctness-agent"
            | "clean-install-generate-reviewer-edge-cases-agent"
            | "clean-install-generate-reviewer-plan-fidelity-agent"
            | "clean-install-generate-reviewer-security-agent"
            | "clean-install-generate-reviewer-security-structure-tests-agent"
            | "clean-install-generate-reviewer-structure-agent"
            | "clean-install-generate-reviewer-testing-agent"
            | "clean-install-voting-code-review-classification-header"
            | "clean-install-voting-compose-tally-record"
            | "clean-install-voting-findings-classification-header"
            | "clean-install-voting-degraded-warning"
            | "clean-install-voting-voter-status-block"
            | "clean-install-plan-review-drift-baseline"
            | "clean-install-plan-review-step3-review"
            | "clean-install-plan-review-step3b-tail"
            // Both prompt renderers preserve the legacy `add_help=False`
            // contract, so the clean-install `--help` token is a usage error.
            | "clean-install-render-reviewer"
            | "clean-install-render-specialist"
            // The five scope-anchor verbs also preserve `add_help=False`, so
            // the clean-install token proves Rust dispatch through exit 2.
            | "clean-install-render-scope-anchor"
            | "clean-install-scope-anchor-design-handoff"
            | "clean-install-scope-anchor-relay-allowed"
            | "clean-install-scope-anchor-retally-handoff"
            | "clean-install-scope-anchor-validate"
            | "clean-install-render-plan-review"
            | "clean-install-render-voter"
            | "clean-install-mermaid-sanitize"
            // Neither debate verb declares a `--help` action: the Rust owner
            // treats the clean-install `--help` token as an unknown flag and
            // emits its validation envelope with the argparse usage exit code.
            | "clean-install-debate-abort"
            | "clean-install-debate-adjudicate"
            | "clean-install-debate-adjudication-preview"
            | "clean-install-debate-init"
            | "clean-install-debate-issue-prepare"
            | "clean-install-debate-publish-prepare"
            | "clean-install-debate-record-turn"
            | "clean-install-debate-round-prep"
            | "clean-install-debate-synthesize"
            | "clean-install-voting-write-tally"
            // `implement step-7a` catches its argparse SystemExit — including the
            // `--help` action — and emits the seven-key bail envelope with the
            // argparse usage exit code instead of printing help.
            | "clean-install-implement-step-7a"
            // The eight `design step0-*` verbs and both Step 3.5 settlement
            // selectors reject the clean-install `--help` token with exit 2;
            // `settle-next-action` instead owns a real help action and exits 0.
            // Both decompose owner verbs parse with `add_help=False`, so the
            // clean-install `--help` token reads as an unknown flag and each
            // refuses with the argparse usage exit after proving dispatch.
            | "clean-install-decompose-prepare"
            | "clean-install-decompose-panel-dispatch"
            | "clean-install-design-driver"
            | "clean-install-design-step0-parse"
            | "clean-install-design-step0-session"
            | "clean-install-design-step0-route"
            | "clean-install-design-step0-clarify-hard-halt"
            | "clean-install-design-step0-init"
            | "clean-install-design-step0-abort-cleanup"
            | "clean-install-design-step0-ap-continue"
            | "clean-install-design-step0c"
            | "clean-install-design-step35-settle"
            | "clean-install-design-compose-plan-md"
            | "clean-install-design-step2b5"
            | "clean-install-plan-review-step35-settle"
            | "clean-install-design-file-oos-prepare"
            | "clean-install-design-file-oos-annotate"
            // The three scout verbs declared `add_help=False`, so the
            // clean-install `--help` token reaches each parser's own
            // required-argument refusal and exits 2.
            | "clean-install-scout-dynamic-archetypes"
            | "clean-install-scout-filter-manifest"
            | "clean-install-scout-plan-archetypes"
            | "clean-install-pr-create-branch"
            | "clean-install-pr-body-update"
            // `step3b-entry` requires `--mode finalize|diagram`; the clean-install
            // `--help` token carries no mode, so the entry refuses with exit 2.
            // (`postplan-emit` and `step2b-postplan` own real `--help` actions and
            // exit 0 by default; the Step 2b drafter joins the `=> 1` arm above.)
            | "clean-install-design-step3b-entry"
            | "clean-install-design-gate-b"
            | "clean-install-design-step3-entry"
            | "clean-install-design-step4-tail" => 2,
            // The three remaining publication verbs mirror the retired Python
            // module, which caught the argparse `SystemExit` and emitted each
            // verb's own publication-failure envelope, so the clean-install
            // `--help` token surfaces as that class's exit `10`.
            "clean-install-debate-title-transition"
            | "clean-install-debate-proposal-link"
            | "clean-install-debate-comment-verify" => 10,
            // Every umbrella verb owns a real help action, so the default
            // clean-install `--help` probe succeeds.
            _ => 0,
        }
    }

    #[allow(clippy::too_many_lines)] // One comment-rich clean-install argument dispatch table.
    fn arguments(self) -> &'static [&'static str] {
        if let Some(arguments) = phase_detail_clean_install_arguments(self.id) {
            return arguments;
        }
        if let Some(arguments) = admission_clean_install_arguments(self.id) {
            return arguments;
        }
        match self.id {
            "clean-install-kv-get" => &["--key", "MISSING", "--default", "clean-install"],
            "clean-install-session-read-key" => &[
                "--file",
                "/larch-clean-install-read-key-missing",
                "--key",
                "KEY",
                "--default",
                "clean-install",
            ],
            "clean-install-session-read-keys" => &[
                "--file",
                "/larch-clean-install-read-keys-missing",
                "--key",
                "KEY=clean-install",
            ],
            "clean-install-session-cleanup-tmpdir" => {
                &["--dir", "/tmp/larch-clean-install-session-missing"]
            }
            "clean-install-session-setup" => &[
                "--prefix",
                "clean-install",
                "--skip-preflight",
                "--skip-repo-check",
            ],
            // `require-plugin-root` rejects every argument, and the three
            // progress stdin readers see an empty payload, so all four dispatch
            // with no arguments at all.
            "clean-install-session-require-plugin-root"
            | "clean-install-progress-statusline"
            | "clean-install-progress-session-reset"
            | "clean-install-progress-install-statusline" => &[],
            "clean-install-session-resolve-implement-tmpdir" => {
                &["--cwd", "/larch-clean-install-clone-missing"]
            }
            "clean-install-session-validate-design-tmpdir" => {
                &["/tmp/larch-clean-install-design-tmpdir-missing"]
            }
            "clean-install-design-step2b5" => &["--claude-pid"],
            id if id.starts_with("clean-install-run-log-") => run_log_arguments(id),
            id if id.starts_with("clean-install-timing-") => timing_arguments(id),
            id if id.starts_with("clean-install-token-") => token_arguments(id),
            "clean-install-progress-activate" | "clean-install-progress-deactivate" => &[
                "--repo-root",
                "/larch-clean-install-clone-missing",
                "--run-id",
                "clean-install",
            ],
            "clean-install-progress-cleanup" => &["--retention-days", "7"],
            "clean-install-progress-clear" => {
                &["--repo-root", "/larch-clean-install-clone-missing"]
            }
            "clean-install-progress-note" => &[
                "--repo-root",
                "/larch-clean-install-clone-missing",
                "--skill",
                "clean",
                "--step",
                "install",
                "dispatch",
            ],
            // Every writer below runs against the fixture's seeded session
            // directory, so a clean install proves the whole route, not just
            // the argument rejection in front of it.
            "clean-install-session-write-env" => &[
                "--output",
                "%SESSION%/session-env.sh",
                "--repo-unavailable",
                "false",
            ],
            "clean-install-session-write-id" => &["--output", "%SESSION%/session-id"],
            "clean-install-session-write-design-env" => &[
                "--output",
                "%SESSION%/source-env.sh",
                "--design-tmpdir",
                "%SESSION%",
                "--session-id",
                "clean-install",
            ],
            "clean-install-session-write-implement-env" => &[
                "--claude-pid",
                "4242",
                "--implement-tmpdir",
                "%SESSION%",
                "--cwd",
                "%SESSION%",
            ],
            "clean-install-session-clear-implement-pointer" => &["--claude-pid", "4242"],
            "clean-install-session-persist-run-flags" => {
                &["--implement-tmpdir", "%SESSION%", "--no-issues", "false"]
            }
            "clean-install-session-restore-finalize-state" => &["--implement-tmpdir", "%SESSION%"],
            "clean-install-session-write-run-params" => &["--output", "%SESSION%/run-params.json"],
            "clean-install-session-resolve-trusted-design-env" => &[
                "--session-env-path",
                "%HOME%/.cache/larch/sessions/current-design-env-4242.sh",
                "--claude-pid",
                "4242",
            ],
            _ => &["--help"],
        }
    }
}

/// Arguments for the `/implement` admission, gate, and blocker verbs.
///
/// A free helper for the same reason `phase_detail_clean_install_arguments` is
/// one: it keeps `arguments` inside the per-function line cap.
fn admission_clean_install_arguments(id: &str) -> Option<&'static [&'static str]> {
    match id {
        "clean-install-admission-preflight" => Some(&["--larch-clean-install-probe"]),
        "clean-install-session-check-live-mutation-auth" => Some(&[
            "--context-file",
            "/larch-clean-install-context-missing",
            "--run-id",
            "clean-install",
            "--trusted-root",
            "/larch-clean-install-root-missing",
        ]),
        "clean-install-session-entry-gate" => Some(&[
            "--mode",
            "implement",
            "--current-branch",
            "main",
            "--is-main",
            "true",
            "--is-user-branch",
            "false",
            "--user-prefix",
            "clean-install",
        ]),
        // `all-open` needs no arguments to reach its empty-result path.
        "clean-install-blocker-all-open" => Some(&[]),
        // Neither issue verb has a `--help` action. `state` proves dispatch
        // through its argument refusal, and `info` through the empty value it
        // reports for a field it does not serve; neither reaches the network.
        "clean-install-issue-state" => Some(&["--issue"]),
        "clean-install-issue-info" => Some(&["--issue", "1", "--field", "title"]),
        // An all-removed path proves the search dispatch through its
        // deterministic `STATUS=invalid_path` result without reaching GitHub.
        "clean-install-issue-search-implementing" => Some(&["--file-path", "!!!"]),
        // `content-block` and `scope-paths` print their `argparse` help and
        // exit `0`; `strip-body` routes its help through the diagnostic writer
        // and also exits `0`. The rest refuse the clean-install token, so each
        // is given the exact line that proves dispatch without a GitHub read.
        "clean-install-issue-insert-signal-marker"
        | "clean-install-issue-title-archival-jq"
        | "clean-install-issue-title-eligibility"
        | "clean-install-untrusted-redact-stream"
        | "clean-install-untrusted-xml-escape-attr" => Some(&["--clean-install"]),
        "clean-install-untrusted-file-block" => Some(&["clean-install"]),
        "clean-install-named-block-write" | "clean-install-plan-block-write" => Some(&["--delete"]),
        "clean-install-plan-block-read" => Some(&["--issue", "1"]),
        _ => None,
    }
}

fn phase_detail_clean_install_arguments(id: &str) -> Option<&'static [&'static str]> {
    match id {
        "clean-install-progress-render-phase-detail" => Some(&[
            "--rounds-root",
            "/larch-clean-install-rounds-missing",
            "--no-gantt",
        ]),
        "clean-install-progress-write-design-round-meta"
        | "clean-install-progress-write-implement-round-meta" => {
            Some(&["--round-dir", "/larch-clean-install-round-missing"])
        }
        _ => None,
    }
}

/// Argument sets for every Rust-owned `timing` clean-install case.
///
/// A clean install names no session temporary directory, so every verb resolves
/// no ledger: each case proves the whole dispatch route and still writes nothing.
#[rustfmt::skip]
fn timing_arguments(id: &str) -> &'static [&'static str] {
    match id {
        "clean-install-timing-mark" => &["clean-install"],
        "clean-install-timing-report" => &["--summary"],
        "clean-install-timing-record-round" => &[
            "--skill", "implement", "--step", "clean-install", "--round", "1",
            "--start-s", "0", "--end-s", "1", "--accepted", "0", "--rejected", "0",
        ],
        "clean-install-timing-record-vendor-task" => &[
            "--vendor", "codex", "--task-kind", "codex-review",
            "--start-s", "0", "--end-s", "1", "--output", "clean-install.log",
        ],
        "clean-install-timing-harness-mark" => &["--label", "clean-install", "--", "/usr/bin/true"],
        _ => &[],
    }
}

/// Argument sets for every Rust-owned `token` clean-install case.
///
/// A clean install names no session temporary directory, so recording verbs
/// resolve no ledger and still succeed after proving the dispatch route.
#[rustfmt::skip]
fn token_arguments(id: &str) -> &'static [&'static str] {
    match id {
        "clean-install-token-check-budget" => &["--cap", "1"],
        "clean-install-token-mark" => &["clean-install"],
        "clean-install-token-record-vendor" => &[
            "codex", "input=1", "output=0", "cache_read=0", "cache_create=0", "total=1", "raw=clean-install",
        ],
        "clean-install-token-record-vendor-sidecar" => &["--input", "/larch-clean-install-token-sidecar-missing"],
        "clean-install-token-append-record" => &[
            "--tmpdir", "/tmp", "--input", "/larch-clean-install-token-sidecar-missing",
        ],
        "clean-install-token-lane-write" => &[
            "--dir", "/tmp", "--phase", "research", "--lane", "clean-install",
            "--tool", "claude", "--total-tokens", "1",
        ],
        "clean-install-token-lane-report" => &["--dir", "/tmp"],
        "clean-install-token-render-cost-line" => &["--quiet-on-empty"],
        // cost, dump, report, and any unknown id prove dispatch with zero args.
        _ => &[],
    }
}

/// Argument sets for every Rust-owned `run-log` clean-install case.
///
/// The entry-write verbs run against the fixture's seeded session inputs, so a
/// clean install proves each whole route rather than only the argument
/// rejection in front of it. Split out of `CleanInstallCase::arguments` so that
/// matcher stays readable.
#[rustfmt::skip]
fn run_log_arguments(id: &str) -> &'static [&'static str] {
    match id {
        "clean-install-run-log-manifest" => &[
            "--log-root", "manifest-logs", "--skill", "clean",
            "--run-id", "clean-install", "--field", "steps_ran.install=true",
        ],
        "clean-install-run-log-validate-run-id" => &["--run-id", "clean-install"],
        "clean-install-run-log-init" => &[
            "--log-root", "%SESSION%/larch-logs", "--skill", "clean",
            "--run-id", "clean-install",
        ],
        "clean-install-run-log-write" => &[
            "--log-root",
            "%SESSION%/larch-logs",
            "--skill",
            "clean",
            "--run-id",
            "clean-install",
            "--batch",
            "review-context",
            "--input-file",
            "%SESSION%/payload.md",
        ],
        "clean-install-run-log-write-round" => &[
            "--log-root",
            "%SESSION%/larch-logs",
            "--skill",
            "clean",
            "--run-id",
            "clean-install",
            "--round",
            "1",
            "--source-dir",
            "%SESSION%/round-src",
        ],
        "clean-install-run-log-append" => &[
            "--log-root",
            "%SESSION%/larch-logs",
            "--skill",
            "clean",
            "--run-id",
            "clean-install",
            "--batch",
            "execution-issues",
            "--record-file",
            "%SESSION%/record.ndjson",
        ],
        "clean-install-run-log-exists" => &[
            "--log-root",
            "%SESSION%/larch-logs",
            "--skill",
            "clean",
            "--run-id",
            "clean-install",
            "--batch",
            "run-statistics",
        ],
        "clean-install-run-log-append-entry" => &[
            "--log",
            "%SESSION%/execution-issues.md",
            "--category",
            "Warnings",
            "--entry",
            "clean-install",
        ],
        "clean-install-run-log-append-failure" => &[
            "--log",
            "%SESSION%/execution-issues.md",
            "--site",
            "clean",
            "--tool",
            "install",
            "--exit-code",
            "0",
            "--category",
            "Warnings",
            "--output-file",
            "%SESSION%/payload.md",
        ],
        "clean-install-run-log-verify-completeness" => &["%SESSION%/verify-run"],
        "clean-install-run-log-publish-breadcrumbs" => &[
            "--source-dir",
            "%SESSION%/breadcrumbs",
            "--dest-dir",
            "%SESSION%/larch-logs/clean/clean-install/breadcrumbs",
        ],
        "clean-install-run-log-checkpoint" => &[],
        "clean-install-run-log-capture-transcript" => &[
            "--log-root", "%SESSION%/larch-logs", "--skill", "implement",
            "--run-id", "clean-install", "--source-file", "%SESSION%/missing-source.env",
        ],
        "clean-install-run-log-refresh" => &["--implement-tmpdir", "%SESSION%"],
        "clean-install-run-log-prepare-terminal-snapshot" => &[
            "--implement-tmpdir", "/larch-clean-install-session-missing",
            "--run-id", "clean-install",
        ],
        _ => &["--help"],
    }
}

#[rustfmt::skip]
const CLEAN_INSTALL_CASES: &[CleanInstallCase] = &[
    CleanInstallCase::new("clean-install-admission-fork-env", "admission", "fork-env"),
    CleanInstallCase::new("clean-install-admission-gate", "admission", "gate"),
    CleanInstallCase::new(
        "clean-install-admission-preflight",
        "admission",
        "preflight",
    ),
    CleanInstallCase::new("clean-install-alias-generate", "alias", "generate"),
    CleanInstallCase::new(
        "clean-install-alias-resolve-target",
        "alias",
        "resolve-target",
    ),
    CleanInstallCase::new(
        "clean-install-architectural-assessment-materialize",
        "architectural-assessment",
        "materialize",
    ),
    CleanInstallCase::new(
        "clean-install-architectural-assessment-submit",
        "architectural-assessment",
        "submit",
    ),
    CleanInstallCase::new(
        "clean-install-architectural-guidelines-materialize-diff",
        "architectural-guidelines",
        "materialize-diff",
    ),
    CleanInstallCase::new(
        "clean-install-architectural-guidelines-persist-design-assessment",
        "architectural-guidelines",
        "persist-design-assessment",
    ),
    CleanInstallCase::new(
        "clean-install-architectural-guidelines-prepare",
        "architectural-guidelines",
        "prepare",
    ),
    CleanInstallCase::new(
        "clean-install-architectural-guidelines-prepare-compose",
        "architectural-guidelines",
        "prepare-compose",
    ),
    CleanInstallCase::new(
        "clean-install-architectural-guidelines-append-deviation-note",
        "architectural-guidelines",
        "append-deviation-note",
    ),
    CleanInstallCase::new(
        "clean-install-architectural-guidelines-invalidate",
        "architectural-guidelines",
        "invalidate",
    ),
    CleanInstallCase::new(
        "clean-install-architectural-guidelines-pin-note-from-staged",
        "architectural-guidelines",
        "pin-note-from-staged",
    ),
    CleanInstallCase::new(
        "clean-install-architectural-guidelines-present-note",
        "architectural-guidelines",
        "present-note",
    ),
    CleanInstallCase::new(
        "clean-install-architectural-guidelines-read",
        "architectural-guidelines",
        "read",
    ),
    CleanInstallCase::new(
        "clean-install-architectural-guidelines-write-compose-assessment",
        "architectural-guidelines",
        "write-compose-assessment",
    ),
    CleanInstallCase::new(
        "clean-install-architectural-guidelines-write-staged-assessment",
        "architectural-guidelines",
        "write-staged-assessment",
    ),
    CleanInstallCase::new(
        "clean-install-architectural-invariants-materialize-diff",
        "architectural-invariants",
        "materialize-diff",
    ),
    CleanInstallCase::new(
        "clean-install-architectural-invariants-persist-design-assessment",
        "architectural-invariants",
        "persist-design-assessment",
    ),
    CleanInstallCase::new(
        "clean-install-architectural-invariants-prepare",
        "architectural-invariants",
        "prepare",
    ),
    CleanInstallCase::new(
        "clean-install-architectural-invariants-prepare-compose",
        "architectural-invariants",
        "prepare-compose",
    ),
    CleanInstallCase::new(
        "clean-install-architectural-invariants-append-deviation-note",
        "architectural-invariants",
        "append-deviation-note",
    ),
    CleanInstallCase::new(
        "clean-install-architectural-invariants-invalidate",
        "architectural-invariants",
        "invalidate",
    ),
    CleanInstallCase::new(
        "clean-install-architectural-invariants-pin-note-from-staged",
        "architectural-invariants",
        "pin-note-from-staged",
    ),
    CleanInstallCase::new(
        "clean-install-architectural-invariants-present-note",
        "architectural-invariants",
        "present-note",
    ),
    CleanInstallCase::new(
        "clean-install-architectural-invariants-read",
        "architectural-invariants",
        "read",
    ),
    CleanInstallCase::new(
        "clean-install-architectural-invariants-write-compose-assessment",
        "architectural-invariants",
        "write-compose-assessment",
    ),
    CleanInstallCase::new(
        "clean-install-architectural-invariants-write-staged-assessment",
        "architectural-invariants",
        "write-staged-assessment",
    ),
    CleanInstallCase::new(
        "clean-install-analyze-bugs-ledger",
        "analyze-bugs",
        "ledger",
    ),
    CleanInstallCase::new(
        "clean-install-analyze-bugs-prefetch",
        "analyze-bugs",
        "prefetch",
    ),
    CleanInstallCase::new(
        "clean-install-analyze-bugs-report",
        "analyze-bugs",
        "report",
    ),
    CleanInstallCase::new(
        "clean-install-analyze-bugs-runtime",
        "analyze-bugs",
        "runtime",
    ),
    CleanInstallCase::new(
        "clean-install-rejected-analysis-finalize",
        "rejected-analysis",
        "finalize",
    ),
    CleanInstallCase::new(
        "clean-install-rejected-analysis-ingest-verdict",
        "rejected-analysis",
        "ingest-verdict",
    ),
    CleanInstallCase::new(
        "clean-install-rejected-analysis-prepare",
        "rejected-analysis",
        "prepare",
    ),
    CleanInstallCase::new(
        "clean-install-rejected-analysis-record",
        "rejected-analysis",
        "record",
    ),
    CleanInstallCase::new(
        "clean-install-learn-from-bugs-check-proposals",
        "learn-from-bugs",
        "check-proposals",
    ),
    CleanInstallCase::new(
        "clean-install-learn-from-bugs-coverage-index",
        "learn-from-bugs",
        "coverage-index",
    ),
    CleanInstallCase::new(
        "clean-install-learn-from-bugs-filing-deps",
        "learn-from-bugs",
        "filing-deps",
    ),
    CleanInstallCase::new(
        "clean-install-learn-from-bugs-prepare",
        "learn-from-bugs",
        "prepare",
    ),
    CleanInstallCase::new(
        "clean-install-learn-from-bugs-read-state",
        "learn-from-bugs",
        "read-state",
    ),
    CleanInstallCase::new(
        "clean-install-learn-from-bugs-resolve-zones",
        "learn-from-bugs",
        "resolve-zones",
    ),
    CleanInstallCase::new(
        "clean-install-learn-from-bugs-state-publish",
        "learn-from-bugs",
        "state-publish",
    ),
    CleanInstallCase::new(
        "clean-install-learn-from-bugs-validate-report",
        "learn-from-bugs",
        "validate-report",
    ),
    CleanInstallCase::new(
        "clean-install-learn-from-bugs-verify-origin",
        "learn-from-bugs",
        "verify-origin",
    ),
    CleanInstallCase::new(
        "clean-install-learn-from-bugs-write-state",
        "learn-from-bugs",
        "write-state",
    ),
    CleanInstallCase::new(
        "clean-install-validate-merged-ingest-finder",
        "validate-merged",
        "ingest-finder",
    ),
    CleanInstallCase::new(
        "clean-install-validate-merged-ingest-refuter",
        "validate-merged",
        "ingest-refuter",
    ),
    CleanInstallCase::new(
        "clean-install-validate-merged-prepare",
        "validate-merged",
        "prepare",
    ),
    CleanInstallCase::new(
        "clean-install-validate-merged-report",
        "validate-merged",
        "report",
    ),
    CleanInstallCase::new(
        "clean-install-validate-merged-write-state",
        "validate-merged",
        "write-state",
    ),
    CleanInstallCase::new(
        "clean-install-analyze-issues-analyze",
        "analyze-issues",
        "analyze",
    ),
    CleanInstallCase::new(
        "clean-install-analyze-issues-fetch",
        "analyze-issues",
        "fetch",
    ),
    CleanInstallCase::new(
        "clean-install-analyze-issues-run",
        "analyze-issues",
        "run",
    ),
    CleanInstallCase::new(
        "clean-install-audit-runs-bugs-backlog-nudge",
        "audit-runs",
        "bugs-backlog-nudge",
    ),
    CleanInstallCase::new(
        "clean-install-audit-runs-close-priors",
        "audit-runs",
        "close-priors",
    ),
    CleanInstallCase::new("clean-install-audit-runs-compute-counters", "audit-runs", "compute-counters"),
    CleanInstallCase::new("clean-install-audit-runs-map-runs", "audit-runs", "map-runs"),
    CleanInstallCase::new("clean-install-audit-runs-pacific-timestamp", "audit-runs", "pacific-timestamp"),
    CleanInstallCase::new("clean-install-audit-runs-preflight", "audit-runs", "preflight"),
    CleanInstallCase::new("clean-install-audit-runs-resolve-prs", "audit-runs", "resolve-prs"),
    CleanInstallCase::new("clean-install-audit-runs-scan-run", "audit-runs", "scan-run"),
    CleanInstallCase::new("clean-install-audit-runs-title", "audit-runs", "title"),
    CleanInstallCase::new(
        "clean-install-audit-runs-title-match",
        "audit-runs",
        "title-match",
    ),
    CleanInstallCase::new("clean-install-blocker-all-open", "blocker", "all-open"),
    CleanInstallCase::new("clean-install-bootstrap-invoke", "bootstrap", "invoke"),
    CleanInstallCase::new(
        "clean-install-bootstrap-parse-routing",
        "bootstrap",
        "parse-routing",
    ),
    CleanInstallCase::new(
        "clean-install-bootstrap-resolve-non-interactive",
        "bootstrap",
        "resolve-non-interactive",
    ),
    CleanInstallCase::new("clean-install-clarify-state", "clarify", "state"),
    CleanInstallCase::new("clean-install-clarify-comment-fetch", "clarify", "comment-fetch"),
    CleanInstallCase::new("clean-install-clarify-comment-post", "clarify", "comment-post"),
    CleanInstallCase::new("clean-install-clarify-label", "clarify", "label"),
    CleanInstallCase::new("clean-install-design-clarify", "design", "clarify"),
    CleanInstallCase::new("clean-install-design-publish", "design", "publish"),
    CleanInstallCase::new(
        "clean-install-design-file-oos-prepare",
        "design",
        "file-oos-prepare",
    ),
    CleanInstallCase::new(
        "clean-install-design-file-oos-annotate",
        "design",
        "file-oos-annotate",
    ),
    CleanInstallCase::new(
        "clean-install-design-render-final-summary",
        "design",
        "render-final-summary",
    ),
    CleanInstallCase::new("clean-install-design-render-gate", "design", "render-gate"),
    CleanInstallCase::new("clean-install-design-step2b-drafter", "design", "step2b-drafter"),
    CleanInstallCase::new("clean-install-design-step2b-postplan", "design", "step2b-postplan"),
    CleanInstallCase::new("clean-install-design-postplan-emit", "design", "postplan-emit"),
    CleanInstallCase::new("clean-install-design-step3b-entry", "design", "step3b-entry"),
    CleanInstallCase::new("clean-install-cleanup-run", "cleanup", "run"),
    CleanInstallCase::new("clean-install-combine-issues-apply", "combine-issues", "apply"),
    CleanInstallCase::new(
        "clean-install-combine-issues-close-eligible",
        "combine-issues",
        "close-eligible",
    ),
    CleanInstallCase::new(
        "clean-install-combine-issues-close-sources",
        "combine-issues",
        "close-sources",
    ),
    CleanInstallCase::new(
        "clean-install-combine-issues-close-stale",
        "combine-issues",
        "close-stale",
    ),
    CleanInstallCase::new("clean-install-combine-issues-fetch", "combine-issues", "fetch"),
    CleanInstallCase::new(
        "clean-install-combine-issues-fetch-deps",
        "combine-issues",
        "fetch-deps",
    ),
    CleanInstallCase::new(
        "clean-install-combine-issues-list-open",
        "combine-issues",
        "list-open",
    ),
    CleanInstallCase::new(
        "clean-install-combine-issues-plan-audit",
        "combine-issues",
        "plan-audit",
    ),
    CleanInstallCase::new(
        "clean-install-combine-issues-plan-inherited",
        "combine-issues",
        "plan-inherited",
    ),
    CleanInstallCase::new(
        "clean-install-combine-issues-prose-audit",
        "combine-issues",
        "prose-audit",
    ),
    CleanInstallCase::new(
        "clean-install-complete-umbrella-bootstrap",
        "complete-umbrella",
        "bootstrap",
    ),
    CleanInstallCase::new(
        "clean-install-complete-umbrella-ship-leaf",
        "complete-umbrella",
        "ship-leaf",
    ),
    CleanInstallCase::new("clean-install-debate-abort", "debate", "abort"),
    CleanInstallCase::new("clean-install-debate-adjudicate", "debate", "adjudicate"),
    CleanInstallCase::new(
        "clean-install-debate-adjudication-preview",
        "debate",
        "adjudication-preview",
    ),
    CleanInstallCase::new(
        "clean-install-debate-comment-verify",
        "debate",
        "comment-verify",
    ),
    CleanInstallCase::new("clean-install-debate-init", "debate", "init"),
    CleanInstallCase::new(
        "clean-install-debate-issue-prepare",
        "debate",
        "issue-prepare",
    ),
    CleanInstallCase::new(
        "clean-install-debate-proposal-link",
        "debate",
        "proposal-link",
    ),
    CleanInstallCase::new(
        "clean-install-debate-publish-prepare",
        "debate",
        "publish-prepare",
    ),
    CleanInstallCase::new("clean-install-debate-record-turn", "debate", "record-turn"),
    CleanInstallCase::new("clean-install-debate-round-prep", "debate", "round-prep"),
    CleanInstallCase::new("clean-install-debate-synthesize", "debate", "synthesize"),
    CleanInstallCase::new(
        "clean-install-debate-title-transition",
        "debate",
        "title-transition",
    ),
    CleanInstallCase::new("clean-install-deps-apply", "deps", "apply"),
    CleanInstallCase::new("clean-install-decompose-prepare", "decompose", "prepare"),
    CleanInstallCase::new(
        "clean-install-decompose-panel-dispatch",
        "decompose",
        "panel-dispatch",
    ),
    CleanInstallCase::new(
        "clean-install-design-init-runparams",
        "design",
        "init-runparams",
    ),
    CleanInstallCase::new("clean-install-design-driver", "design", "driver"),
    CleanInstallCase::new("clean-install-design-prelude", "design", "prelude"),
    CleanInstallCase::new(
        "clean-install-design-dialectic-clear-stale",
        "design",
        "dialectic-clear-stale",
    ),
    CleanInstallCase::new(
        "clean-install-design-dialectic-gatec",
        "design",
        "dialectic-gatec",
    ),
    CleanInstallCase::new(
        "clean-install-design-dialectic-manual",
        "design",
        "dialectic-manual",
    ),
    CleanInstallCase::new(
        "clean-install-design-dialectic-promote-candidates",
        "design",
        "dialectic-promote-candidates",
    ),
    CleanInstallCase::new(
        "clean-install-design-dialectic-validate-candidates",
        "design",
        "dialectic-validate-candidates",
    ),
    CleanInstallCase::new(
        "clean-install-design-dialectic-write-candidates",
        "design",
        "dialectic-write-candidates",
    ),
    CleanInstallCase::new("clean-install-design-log-publish", "design", "log-publish"),
    CleanInstallCase::new("clean-install-design-pause-load", "design", "pause-load"),
    CleanInstallCase::new("clean-install-design-pause-save", "design", "pause-save"),
    CleanInstallCase::new("clean-install-design-parse-flags", "design", "parse-flags"),
    CleanInstallCase::new("clean-install-design-route", "design", "route"),
    CleanInstallCase::new("clean-install-design-step0-parse", "design", "step0-parse"),
    CleanInstallCase::new("clean-install-design-step0-session", "design", "step0-session"),
    CleanInstallCase::new("clean-install-design-step0-route", "design", "step0-route"),
    CleanInstallCase::new(
        "clean-install-design-step0-clarify-hard-halt",
        "design",
        "step0-clarify-hard-halt",
    ),
    CleanInstallCase::new("clean-install-design-step0-init", "design", "step0-init"),
    CleanInstallCase::new(
        "clean-install-design-step0-abort-cleanup",
        "design",
        "step0-abort-cleanup",
    ),
    CleanInstallCase::new(
        "clean-install-design-step0-ap-continue",
        "design",
        "step0-ap-continue",
    ),
    CleanInstallCase::new("clean-install-design-step0c", "design", "step0c"),
    CleanInstallCase::new(
        "clean-install-design-step3-continuation-entry",
        "design",
        "step3-continuation-entry",
    ),
    CleanInstallCase::new("clean-install-design-gate-b", "design", "gate-b"),
    CleanInstallCase::new("clean-install-design-step3-entry", "design", "step3-entry"),
    CleanInstallCase::new("clean-install-design-step4-tail", "design", "step4-tail"),
    CleanInstallCase::new(
        "clean-install-design-step35-settle",
        "design",
        "step35-settle",
    ),
    CleanInstallCase::new("clean-install-design-compose-plan-md", "design", "compose-plan-md"),
    CleanInstallCase::new("clean-install-design-step2b5", "design", "step2b5"),
    CleanInstallCase::new("clean-install-design-step5c", "design", "step5c"),
    CleanInstallCase::new("clean-install-design-step6", "design", "step6"),
    CleanInstallCase::new("clean-install-design-step6-cleanup", "design", "step6-cleanup"),
    CleanInstallCase::new("clean-install-design-step6-prelude", "design", "step6-prelude"),
    CleanInstallCase::new(
        "clean-install-design-step5b-annotate",
        "design",
        "step5b-annotate",
    ),
    CleanInstallCase::new(
        "clean-install-design-step5b-prepare",
        "design",
        "step5b-prepare",
    ),
    CleanInstallCase::new(
        "clean-install-design-settle-next-action",
        "design",
        "settle-next-action",
    ),
    CleanInstallCase::new(
        "clean-install-plan-review-step35-settle",
        "plan-review",
        "step35-settle",
    ),
    CleanInstallCase::new(
        "clean-install-design-read-result-env",
        "design",
        "read-result-env",
    ),
    CleanInstallCase::new(
        "clean-install-design-stage-terminal-state",
        "design",
        "stage-terminal-state",
    ),
    CleanInstallCase::new(
        "clean-install-design-failure-report",
        "design",
        "failure-report",
    ),
    CleanInstallCase::new(
        "clean-install-design-step-final-summary",
        "design",
        "step-final-summary",
    ),
    CleanInstallCase::new("clean-install-deps-explicit-refs", "deps", "explicit-refs"),
    CleanInstallCase::new("clean-install-deps-fetch", "deps", "fetch"),
    CleanInstallCase::new("clean-install-deps-plan", "deps", "plan"),
    CleanInstallCase::new("clean-install-deps-resolve-repo", "deps", "resolve-repo"),
    CleanInstallCase::new("clean-install-deps-write-proposals", "deps", "write-proposals"),
    CleanInstallCase::new("clean-install-generate-check", "generate", "check"),
    CleanInstallCase::new(
        "clean-install-generate-code-reviewer-agent",
        "generate",
        "code-reviewer-agent",
    ),
    CleanInstallCase::new(
        "clean-install-generate-codex-implementer",
        "generate",
        "codex-implementer",
    ),
    CleanInstallCase::new(
        "clean-install-generate-cursor-implementer",
        "generate",
        "cursor-implementer",
    ),
    CleanInstallCase::new(
        "clean-install-generate-pre-rendered-reviewer-prompts",
        "generate",
        "pre-rendered-reviewer-prompts",
    ),
    CleanInstallCase::new(
        "clean-install-generate-reviewer-code-robustness-agent",
        "generate",
        "reviewer-code-robustness-agent",
    ),
    CleanInstallCase::new(
        "clean-install-generate-reviewer-correctness-agent",
        "generate",
        "reviewer-correctness-agent",
    ),
    CleanInstallCase::new(
        "clean-install-generate-reviewer-edge-cases-agent",
        "generate",
        "reviewer-edge-cases-agent",
    ),
    CleanInstallCase::new(
        "clean-install-generate-reviewer-plan-fidelity-agent",
        "generate",
        "reviewer-plan-fidelity-agent",
    ),
    CleanInstallCase::new(
        "clean-install-generate-reviewer-security-agent",
        "generate",
        "reviewer-security-agent",
    ),
    CleanInstallCase::new(
        "clean-install-generate-reviewer-security-structure-tests-agent",
        "generate",
        "reviewer-security-structure-tests-agent",
    ),
    CleanInstallCase::new(
        "clean-install-generate-reviewer-structure-agent",
        "generate",
        "reviewer-structure-agent",
    ),
    CleanInstallCase::new(
        "clean-install-generate-reviewer-testing-agent",
        "generate",
        "reviewer-testing-agent",
    ),
    CleanInstallCase::new(
        "clean-install-block-issue-add-blocked-by",
        "block-issue",
        "add-blocked-by",
    ),
    CleanInstallCase::new(
        "clean-install-block-issue-remove-blocked-by",
        "block-issue",
        "remove-blocked-by",
    ),
    CleanInstallCase::new("clean-install-implement-cleanup", "implement", "cleanup"),
    CleanInstallCase::new("clean-install-implement-clone-tag", "implement", "clone-tag"),
    CleanInstallCase::new("clean-install-implement-commit", "implement", "commit"),
    CleanInstallCase::new(
        "clean-install-implement-commit-route",
        "implement",
        "commit-route",
    ),
    CleanInstallCase::new(
        "clean-install-implement-checks-commit-route",
        "implement",
        "checks-commit-route",
    ),
    CleanInstallCase::new(
        "clean-install-implement-kill-active-leg",
        "implement",
        "kill-active-leg",
    ),
    CleanInstallCase::new(
        "clean-install-implement-normalize-coder-scout",
        "implement",
        "normalize-coder-scout",
    ),
    CleanInstallCase::new("clean-install-implement-preflight", "implement", "preflight"),
    CleanInstallCase::new(
        "clean-install-implement-run-dispatch",
        "implement",
        "run-dispatch",
    ),
    CleanInstallCase::new(
        "clean-install-implement-scope-disposition",
        "implement",
        "scope-disposition",
    ),
    CleanInstallCase::new(
        "clean-install-implement-step-2-post-dispatch",
        "implement",
        "step-2-post-dispatch",
    ),
    CleanInstallCase::new(
        "clean-install-implement-step2-dispatch",
        "implement",
        "step2-dispatch",
    ),
    CleanInstallCase::new(
        "clean-install-implement-step-0-bootstrap",
        "implement",
        "step-0-bootstrap",
    ),
    CleanInstallCase::new(
        "clean-install-implement-step-0-degraded-gate",
        "implement",
        "step-0-degraded-gate",
    ),
    CleanInstallCase::new(
        "clean-install-issue-add-blocked-by",
        "issue",
        "add-blocked-by",
    ),
    CleanInstallCase::new("clean-install-issue-add-sub-issue", "issue", "add-sub-issue"),
    CleanInstallCase::new(
        "clean-install-issue-allocate-candidates",
        "issue",
        "allocate-candidates",
    ),
    CleanInstallCase::new(
        "clean-install-issue-cleanup-failed",
        "issue",
        "cleanup-failed",
    ),
    CleanInstallCase::new(
        "clean-install-issue-create-batch",
        "issue",
        "create-batch",
    ),
    CleanInstallCase::new("clean-install-issue-create-one", "issue", "create-one"),
    CleanInstallCase::new(
        "clean-install-issue-fetch-issue-details",
        "issue",
        "fetch-issue-details",
    ),
    CleanInstallCase::new("clean-install-issue-info", "issue", "info"),
    CleanInstallCase::new(
        "clean-install-issue-insert-signal-marker",
        "issue",
        "insert-signal-marker",
    ),
    CleanInstallCase::new(
        "clean-install-issue-title-archival-jq",
        "issue",
        "title-archival-jq",
    ),
    CleanInstallCase::new(
        "clean-install-issue-title-eligibility",
        "issue",
        "title-eligibility",
    ),
    CleanInstallCase::new("clean-install-named-block-write", "named-block", "write"),
    CleanInstallCase::new(
        "clean-install-scout-dynamic-archetypes",
        "scout",
        "dynamic-archetypes",
    ),
    CleanInstallCase::new(
        "clean-install-scout-filter-manifest",
        "scout",
        "filter-manifest",
    ),
    CleanInstallCase::new(
        "clean-install-scout-plan-archetypes",
        "scout",
        "plan-archetypes",
    ),
    CleanInstallCase::new("clean-install-plan-scope-paths", "plan", "scope-paths"),
    CleanInstallCase::new("clean-install-plan-auto-fix-commands", "plan", "auto-fix-commands"),
    CleanInstallCase::new("clean-install-plan-check-size", "plan", "check-size"),
    CleanInstallCase::new("clean-install-plan-compose-goals-test", "plan", "compose-goals-test"),
    CleanInstallCase::new("clean-install-plan-optional-trailers", "plan", "optional-trailers"),
    CleanInstallCase::new("clean-install-plan-parse-commands", "plan", "parse-commands"),
    CleanInstallCase::new("clean-install-plan-revise-waterfall", "plan", "revise-waterfall"),
    CleanInstallCase::new("clean-install-plan-set-oversize-override", "plan", "set-oversize-override"),
    CleanInstallCase::new("clean-install-plan-validate", "plan", "validate"),
    CleanInstallCase::new("clean-install-plan-validate-commands", "plan", "validate-commands"),
    CleanInstallCase::new("clean-install-pr-compose-summary", "pr", "compose-summary"),
    CleanInstallCase::new("clean-install-pr-create-branch", "pr", "create-branch"),
    CleanInstallCase::new("clean-install-pr-create", "pr", "create"),
    CleanInstallCase::new("clean-install-pr-body-update", "pr", "body-update"),
    CleanInstallCase::new("clean-install-pr-checks", "pr", "checks"),
    CleanInstallCase::new("clean-install-pr-closes-issue", "pr", "closes-issue"),
    CleanInstallCase::new("clean-install-plan-validator-autofix", "plan", "validator-autofix"),
    CleanInstallCase::new(
        "clean-install-plan-review-panel-dispatch",
        "plan-review",
        "panel-dispatch",
    ),
    CleanInstallCase::new(
        "clean-install-plan-review-voter-dispatch",
        "plan-review",
        "voter-dispatch",
    ),
    CleanInstallCase::new("clean-install-plan-review-await-loop-identity", "plan-review", "await-loop-identity"),
    CleanInstallCase::new("clean-install-plan-review-continuation", "plan-review", "continuation"),
    CleanInstallCase::new("clean-install-plan-review-drift-baseline", "plan-review", "drift-baseline"),
    CleanInstallCase::new("clean-install-plan-review-emit", "plan-review", "emit"),
    CleanInstallCase::new("clean-install-plan-review-emit-rejected", "plan-review", "emit-rejected"),
    CleanInstallCase::new("clean-install-plan-review-filter-gate-b-skipped", "plan-review", "filter-gate-b-skipped"),
    CleanInstallCase::new("clean-install-plan-review-finalize", "plan-review", "finalize"),
    CleanInstallCase::new("clean-install-plan-review-gate-b-counts", "plan-review", "gate-b-counts"),
    CleanInstallCase::new("clean-install-plan-review-gate-b-dedup", "plan-review", "gate-b-dedup"),
    CleanInstallCase::new("clean-install-plan-review-gate-b-finding-line", "plan-review", "gate-b-finding-line"),
    CleanInstallCase::new("clean-install-plan-review-json-get-bool", "plan-review", "json-get-bool"),
    CleanInstallCase::new("clean-install-plan-review-normalize-status", "plan-review", "normalize-status"),
    CleanInstallCase::new("clean-install-plan-review-persist-accepted-audit", "plan-review", "persist-accepted-audit"),
    CleanInstallCase::new("clean-install-plan-review-persist-retally-env", "plan-review", "persist-retally-env"),
    CleanInstallCase::new("clean-install-plan-review-persist-round-start-s", "plan-review", "persist-round-start-s"),
    CleanInstallCase::new("clean-install-plan-review-prelaunch-failure", "plan-review", "prelaunch-failure"),
    CleanInstallCase::new("clean-install-plan-review-preview", "plan-review", "preview"),
    CleanInstallCase::new("clean-install-plan-review-resume-state", "plan-review", "resume-state"),
    CleanInstallCase::new("clean-install-plan-review-round-artifact-included", "plan-review", "round-artifact-included"),
    CleanInstallCase::new("clean-install-plan-review-round-revise-artifact-excluded", "plan-review", "round-revise-artifact-excluded"),
    CleanInstallCase::new("clean-install-plan-review-round-revise-artifact-included", "plan-review", "round-revise-artifact-included"),
    CleanInstallCase::new("clean-install-plan-review-run", "plan-review", "run"),
    CleanInstallCase::new("clean-install-plan-review-snapshot-pre-review", "plan-review", "snapshot-pre-review"),
    CleanInstallCase::new("clean-install-plan-review-step3-entry", "plan-review", "step3-entry"),
    CleanInstallCase::new("clean-install-plan-review-step3-entry-preview", "plan-review", "step3-entry-preview"),
    CleanInstallCase::new("clean-install-plan-review-step3-entry-state", "plan-review", "step3-entry-state"),
    CleanInstallCase::new("clean-install-plan-review-step3-gate-b-bypass", "plan-review", "step3-gate-b-bypass"),
    CleanInstallCase::new("clean-install-plan-review-step3-mav", "plan-review", "step3-mav"),
    CleanInstallCase::new("clean-install-plan-review-step3-review", "plan-review", "step3-review"),
    CleanInstallCase::new("clean-install-plan-review-step3-state", "plan-review", "step3-state"),
    CleanInstallCase::new("clean-install-plan-review-step35", "plan-review", "step35"),
    CleanInstallCase::new("clean-install-plan-review-step3b-tail", "plan-review", "step3b-tail"),
    CleanInstallCase::new("clean-install-plan-review-tally", "plan-review", "tally"),
    CleanInstallCase::new("clean-install-plan-review-teardown-loop-identity", "plan-review", "teardown-loop-identity"),
    CleanInstallCase::new("clean-install-plan-review-write-loop-identity", "plan-review", "write-loop-identity"),
    CleanInstallCase::new("clean-install-status-check", "status", "check"),
    CleanInstallCase::new("clean-install-plan-block-read", "plan-block", "read"),
    CleanInstallCase::new(
        "clean-install-plan-block-strip-body",
        "plan-block",
        "strip-body",
    ),
    CleanInstallCase::new("clean-install-plan-block-write", "plan-block", "write"),
    CleanInstallCase::new("clean-install-tracking-post-issue", "tracking", "post-issue"),
    CleanInstallCase::new(
        "clean-install-tracking-issue-append-comment",
        "tracking-issue",
        "append-comment",
    ),
    CleanInstallCase::new(
        "clean-install-tracking-issue-create-issue",
        "tracking-issue",
        "create-issue",
    ),
    CleanInstallCase::new(
        "clean-install-tracking-issue-mark-false-positive",
        "tracking-issue",
        "mark-false-positive",
    ),
    CleanInstallCase::new("clean-install-tracking-issue-read", "tracking-issue", "read"),
    CleanInstallCase::new(
        "clean-install-tracking-issue-rename",
        "tracking-issue",
        "rename",
    ),
    CleanInstallCase::new(
        "clean-install-tracking-issue-upsert-summary",
        "tracking-issue",
        "upsert-summary",
    ),
    CleanInstallCase::new("clean-install-triage-apply", "triage", "apply"),
    CleanInstallCase::new("clean-install-triage-inspect", "triage", "inspect"),
    CleanInstallCase::new("clean-install-triage-probe", "triage", "probe"),
    CleanInstallCase::new(
        "clean-install-umbrella-mark-in-flight",
        "umbrella",
        "mark-in-flight",
    ),
    CleanInstallCase::new(
        "clean-install-umbrella-persist-proposal",
        "umbrella",
        "persist-proposal",
    ),
    CleanInstallCase::new("clean-install-umbrella-prepare", "umbrella", "prepare"),
    CleanInstallCase::new(
        "clean-install-umbrella-reconcile-in-flight",
        "umbrella",
        "reconcile-in-flight",
    ),
    CleanInstallCase::new(
        "clean-install-umbrella-record-resolved",
        "umbrella",
        "record-resolved",
    ),
    CleanInstallCase::new("clean-install-umbrella-mutate", "umbrella", "mutate"),
    CleanInstallCase::new("clean-install-umbrella-verify", "umbrella", "verify"),
    CleanInstallCase::new(
        "clean-install-umbrella-verify-completion",
        "umbrella",
        "verify-completion",
    ),
    CleanInstallCase::new(
        "clean-install-untrusted-content-block",
        "untrusted",
        "content-block",
    ),
    CleanInstallCase::new(
        "clean-install-untrusted-file-block",
        "untrusted",
        "file-block",
    ),
    CleanInstallCase::new(
        "clean-install-untrusted-redact-stream",
        "untrusted",
        "redact-stream",
    ),
    CleanInstallCase::new(
        "clean-install-untrusted-xml-escape-attr",
        "untrusted",
        "xml-escape-attr",
    ),
    CleanInstallCase::new("clean-install-issue-list-issues", "issue", "list-issues"),
    CleanInstallCase::new(
        "clean-install-issue-migration-audit",
        "issue",
        "migration-audit",
    ),
    CleanInstallCase::new(
        "clean-install-issue-governance-gate",
        "issue",
        "governance-gate",
    ),
    CleanInstallCase::new(
        "clean-install-plan-receipt-refresh",
        "plan-receipt",
        "refresh",
    ),
    CleanInstallCase::new("clean-install-issue-parse-input", "issue", "parse-input"),
    CleanInstallCase::new(
        "clean-install-issue-search-implementing",
        "issue",
        "search-implementing",
    ),
    CleanInstallCase::new("clean-install-issue-state", "issue", "state"),
    CleanInstallCase::new(
        "clean-install-issue-write-sentinel",
        "issue",
        "write-sentinel",
    ),
    CleanInstallCase::new(
        "clean-install-session-check-live-mutation-auth",
        "session",
        "check-live-mutation-auth",
    ),
    CleanInstallCase::new("clean-install-session-entry-gate", "session", "entry-gate"),
    CleanInstallCase::new(
        "clean-install-agent-classify-diff",
        "agent",
        "classify-diff",
    ),
    CleanInstallCase::new(
        "clean-install-agent-check-reviewers",
        "agent",
        "check-reviewers",
    ),
    CleanInstallCase::new(
        "clean-install-agent-compose-collector-failure-log",
        "agent",
        "compose-collector-failure-log",
    ),
    CleanInstallCase::new(
        "clean-install-agent-cursor-auth-preflight",
        "agent",
        "cursor-auth-preflight",
    ),
    CleanInstallCase::new(
        "clean-install-agent-cursor-wrap-prompt",
        "agent",
        "cursor-wrap-prompt",
    ),
    CleanInstallCase::new(
        "clean-install-agent-degraded-tools-gate",
        "agent",
        "degraded-tools-gate",
    ),
    CleanInstallCase::new(
        "clean-install-agent-external-tool-registry",
        "agent",
        "external-tool-registry",
    ),
    CleanInstallCase::new(
        "clean-install-agent-launch-review",
        "agent",
        "launch-review",
    ),
    CleanInstallCase::new(
        "clean-install-agent-launch-codex-ci",
        "agent",
        "launch-codex-ci",
    ),
    CleanInstallCase::new(
        "clean-install-agent-launch-cursor-ci",
        "agent",
        "launch-cursor-ci",
    ),
    CleanInstallCase::new(
        "clean-install-agent-launch-claude-ci",
        "agent",
        "launch-claude-ci",
    ),
    CleanInstallCase::new(
        "clean-install-agent-launch-codex-implement",
        "agent",
        "launch-codex-implement",
    ),
    CleanInstallCase::new(
        "clean-install-agent-launch-cursor-implement",
        "agent",
        "launch-cursor-implement",
    ),
    CleanInstallCase::new(
        "clean-install-agent-launch-claude-lint-fix",
        "agent",
        "launch-claude-lint-fix",
    ),
    CleanInstallCase::new(
        "clean-install-agent-launch-claude-review-fix",
        "agent",
        "launch-claude-review-fix",
    ),
    CleanInstallCase::new(
        "clean-install-agent-collect-results",
        "agent",
        "collect-results",
    ),
    CleanInstallCase::new(
        "clean-install-agent-dispatch-waterfall",
        "agent",
        "dispatch-waterfall",
    ),
    CleanInstallCase::new(
        "clean-install-agent-dispatch-voters",
        "agent",
        "dispatch-voters",
    ),
    CleanInstallCase::new("clean-install-agent-model-args", "agent", "model-args"),
    CleanInstallCase::new(
        "clean-install-agent-read-claude-model",
        "agent",
        "read-claude-model",
    ),
    CleanInstallCase::new(
        "clean-install-agent-launch-claude-review",
        "agent",
        "launch-claude-review",
    ),
    CleanInstallCase::new(
        "clean-install-agent-launch-claude-subprocess",
        "agent",
        "launch-claude-subprocess",
    ),
    CleanInstallCase::new(
        "clean-install-agent-resolve-model-pins",
        "agent",
        "resolve-model-pins",
    ),
    CleanInstallCase::new(
        "clean-install-execution-issues-append",
        "execution-issues",
        "append",
    ),
    CleanInstallCase::new(
        "clean-install-execution-issues-flush",
        "execution-issues",
        "flush",
    ),
    CleanInstallCase::new(
        "clean-install-execution-issues-flush-safety-net",
        "execution-issues",
        "flush-safety-net",
    ),
    CleanInstallCase::new(
        "clean-install-execution-issues-refresh",
        "execution-issues",
        "refresh",
    ),
    CleanInstallCase::new(
        "clean-install-oos-materialize-manifest",
        "oos",
        "materialize-manifest",
    ),
    CleanInstallCase::new("clean-install-oos-issue-cap", "oos", "issue-cap"),
    CleanInstallCase::new(
        "clean-install-oos-file-conflict-deps",
        "oos",
        "file-conflict-deps",
    ),
    CleanInstallCase::new(
        "clean-install-oos-disposition-gate",
        "oos",
        "disposition-gate",
    ),
    CleanInstallCase::new(
        "clean-install-oos-disposition-checkpoint",
        "oos",
        "disposition-checkpoint",
    ),
    CleanInstallCase::new("clean-install-oos-file", "oos", "file"),
    CleanInstallCase::new(
        "clean-install-external-defaults-docs",
        "external-defaults",
        "docs",
    ),
    CleanInstallCase::new(
        "clean-install-external-defaults-resolve-vendor",
        "external-defaults",
        "resolve-vendor",
    ),
    CleanInstallCase::new(
        "clean-install-external-defaults-role",
        "external-defaults",
        "role",
    ),
    CleanInstallCase::new(
        "clean-install-slack-issue-announce",
        "slack",
        "issue-announce",
    ),
    CleanInstallCase::new(
        "clean-install-agent-gather-branch-context",
        "agent",
        "gather-branch-context",
    ),
    CleanInstallCase::new(
        "clean-install-review-gather-context",
        "review",
        "gather-context",
    ),
    CleanInstallCase::new(
        "clean-install-review-core",
        "review",
        "core",
    ),
    CleanInstallCase::new(
        "clean-install-review-compose-findings",
        "review",
        "compose-findings",
    ),
    CleanInstallCase::new(
        "clean-install-review-dispatch-panel",
        "review",
        "dispatch-panel",
    ),
    CleanInstallCase::new(
        "clean-install-review-collect-findings",
        "review",
        "collect-findings",
    ),
    CleanInstallCase::new(
        "clean-install-review-check-reviewer-failure-threshold",
        "review",
        "check-reviewer-failure-threshold",
    ),
    CleanInstallCase::new(
        "clean-install-review-aggregate-findings",
        "review",
        "aggregate-findings",
    ),
    CleanInstallCase::new(
        "clean-install-review-prune-nit-findings",
        "review",
        "prune-nit-findings",
    ),
    CleanInstallCase::new(
        "clean-install-review-reviewer-prune",
        "review",
        "reviewer-prune",
    ),
    CleanInstallCase::new(
        "clean-install-review-emit-tally",
        "review",
        "emit-tally",
    ),
    CleanInstallCase::new(
        "clean-install-review-log-phase",
        "review",
        "log-phase",
    ),
    CleanInstallCase::new(
        "clean-install-review-tally-code-votes",
        "review",
        "tally-code-votes",
    ),
    CleanInstallCase::new(
        "clean-install-review-and-fix-apply-findings",
        "review-and-fix",
        "apply-findings",
    ),
    CleanInstallCase::new(
        "clean-install-review-and-fix-await-loop-identity",
        "review-and-fix",
        "await-loop-identity",
    ),
    CleanInstallCase::new(
        "clean-install-review-and-fix-check-changes",
        "review-and-fix",
        "check-changes",
    ),
    CleanInstallCase::new(
        "clean-install-review-and-fix-commit-fixes",
        "review-and-fix",
        "commit-fixes",
    ),
    CleanInstallCase::new(
        "clean-install-review-and-fix-normalize-status",
        "review-and-fix",
        "normalize-status",
    ),
    CleanInstallCase::new(
        "clean-install-review-and-fix-step5",
        "review-and-fix",
        "step5",
    ),
    CleanInstallCase::new(
        "clean-install-review-and-fix-teardown-loop-identity",
        "review-and-fix",
        "teardown-loop-identity",
    ),
    CleanInstallCase::new(
        "clean-install-review-and-fix-write-loop-identity",
        "review-and-fix",
        "write-loop-identity",
    ),
    CleanInstallCase::new(
        "clean-install-review-and-fix-write-pre-self-review-snapshot",
        "review-and-fix",
        "write-pre-self-review-snapshot",
    ),
    CleanInstallCase::new(
        "clean-install-review-and-fix-write-rejected",
        "review-and-fix",
        "write-rejected",
    ),
    CleanInstallCase::new(
        "clean-install-review-and-fix-write-self-review-tally",
        "review-and-fix",
        "write-self-review-tally",
    ),
    CleanInstallCase::new(
        "clean-install-agent-parse-codex-usage",
        "agent",
        "parse-codex-usage",
    ),
    CleanInstallCase::new(
        "clean-install-agent-launch-claude-drafter",
        "agent",
        "launch-claude-drafter",
    ),
    CleanInstallCase::new(
        "clean-install-agent-launch-codex-drafter",
        "agent",
        "launch-codex-drafter",
    ),
    CleanInstallCase::new(
        "clean-install-agent-launch-codex-exec",
        "agent",
        "launch-codex-exec",
    ),
    CleanInstallCase::new(
        "clean-install-agent-run-negotiation-round",
        "agent",
        "run-negotiation-round",
    ),
    CleanInstallCase::new(
        "clean-install-agent-run-external-agent",
        "agent",
        "run-external-agent",
    ),
    CleanInstallCase::new(
        "clean-install-agent-wait-reviewers",
        "agent",
        "wait-reviewers",
    ),
    CleanInstallCase::new("clean-install-lint-gitleaks", "lint", "gitleaks"),
    CleanInstallCase::new("clean-install-bgjob-adapt", "bgjob", "adapt"),
    CleanInstallCase::new("clean-install-bgjob-reap", "bgjob", "reap"),
    CleanInstallCase::new("clean-install-bgjob-start", "bgjob", "start"),
    CleanInstallCase::new("clean-install-bgjob-status", "bgjob", "status"),
    CleanInstallCase::new("clean-install-bgjob-wait", "bgjob", "wait"),
    CleanInstallCase::new(
        "clean-install-bgjob-write-merge-result-env",
        "bgjob",
        "write-merge-result-env",
    ),
    CleanInstallCase::new("clean-install-kv-get", "kv", "get"),
    CleanInstallCase::new(
        "clean-install-session-cleanup-tmpdir",
        "session",
        "cleanup-tmpdir",
    ),
    CleanInstallCase::new(
        "clean-install-session-local-cleanup",
        "session",
        "local-cleanup",
    ),
    CleanInstallCase::new("clean-install-session-setup", "session", "setup"),
    CleanInstallCase::new("clean-install-session-read-key", "session", "read-key"),
    CleanInstallCase::new("clean-install-session-read-keys", "session", "read-keys"),
    CleanInstallCase::new(
        "clean-install-session-kill-background-processes",
        "session",
        "kill-background-processes",
    ),
    CleanInstallCase::new(
        "clean-install-session-require-plugin-root",
        "session",
        "require-plugin-root",
    ),
    CleanInstallCase::new(
        "clean-install-session-resolve-implement-tmpdir",
        "session",
        "resolve-implement-tmpdir",
    ),
    CleanInstallCase::new(
        "clean-install-session-validate-design-tmpdir",
        "session",
        "validate-design-tmpdir",
    ),
    CleanInstallCase::new("clean-install-session-write-env", "session", "write-env"),
    CleanInstallCase::new("clean-install-session-write-id", "session", "write-id"),
    CleanInstallCase::new(
        "clean-install-session-write-design-env",
        "session",
        "write-design-env",
    ),
    CleanInstallCase::new(
        "clean-install-session-write-implement-env",
        "session",
        "write-implement-env",
    ),
    CleanInstallCase::new(
        "clean-install-session-clear-implement-pointer",
        "session",
        "clear-implement-pointer",
    ),
    CleanInstallCase::new(
        "clean-install-session-persist-run-flags",
        "session",
        "persist-run-flags",
    ),
    CleanInstallCase::new(
        "clean-install-session-write-run-params",
        "session",
        "write-run-params",
    ),
    CleanInstallCase::new(
        "clean-install-session-restore-finalize-state",
        "session",
        "restore-finalize-state",
    ),
    CleanInstallCase::new(
        "clean-install-session-resolve-trusted-design-env",
        "session",
        "resolve-trusted-design-env",
    ),
    CleanInstallCase::new(
        "clean-install-ship-governance-refresh",
        "ship",
        "governance-refresh",
    ),
    CleanInstallCase::new(
        "clean-install-ship-normalize-assessment-handoff",
        "ship",
        "normalize-assessment-handoff",
    ),
    CleanInstallCase::new("clean-install-ship-pr", "ship", "pr"),
    CleanInstallCase::new("clean-install-ship-pre-driver", "ship", "pre-driver"),
    CleanInstallCase::new(
        "clean-install-ship-pre-fix-rebase",
        "ship",
        "pre-fix-rebase",
    ),
    CleanInstallCase::new(
        "clean-install-ship-reconcile-manual-merge",
        "ship",
        "reconcile-manual-merge",
    ),
    CleanInstallCase::new("clean-install-ship-route-exit", "ship", "route-exit"),
    CleanInstallCase::new("clean-install-merge-pr", "merge", "pr"),
    CleanInstallCase::new("clean-install-merge-wait", "merge", "wait"),
    CleanInstallCase::new("clean-install-ci-timing-harness", "ci-timing", "harness"),
    CleanInstallCase::new("clean-install-ci-timing-jobs", "ci-timing", "jobs"),
    CleanInstallCase::new(
        "clean-install-ci-timing-rust-jobs",
        "ci-timing",
        "rust-jobs",
    ),
    CleanInstallCase::new(
        "clean-install-ci-timing-merge-group-source",
        "ci-timing",
        "merge-group-source",
    ),
    CleanInstallCase::new("clean-install-ci-gitleaks-base", "ci", "gitleaks-base"),
    CleanInstallCase::new("clean-install-ci-behind-count", "ci", "behind-count"),
    CleanInstallCase::new("clean-install-ci-distill-log", "ci", "distill-log"),
    CleanInstallCase::new("clean-install-ci-failed-jobs", "ci", "failed-jobs"),
    CleanInstallCase::new("clean-install-ci-main-health", "ci", "main-health"),
    CleanInstallCase::new("clean-install-ci-rerun-failed", "ci", "rerun-failed"),
    CleanInstallCase::new("clean-install-ci-decide", "ci", "decide"),
    CleanInstallCase::new("clean-install-ci-status", "ci", "status"),
    CleanInstallCase::new("clean-install-ci-wait", "ci", "wait"),
    CleanInstallCase::new(
        "clean-install-ci-prepare-rust-integration-artifact",
        "ci",
        "prepare-rust-integration-artifact",
    ),
    CleanInstallCase::new(
        "clean-install-ci-promote-rust-policy-candidate",
        "ci",
        "promote-rust-policy-candidate",
    ),
    CleanInstallCase::new(
        "clean-install-ci-stage-rust-policy-candidate",
        "ci",
        "stage-rust-policy-candidate",
    ),
    CleanInstallCase::new("clean-install-ci-rust-select", "ci", "rust-select"),
    CleanInstallCase::new(
        "clean-install-ci-rust-select-summary",
        "ci",
        "rust-select-summary",
    ),
    CleanInstallCase::new(
        "clean-install-ci-stage-main-cache-candidate",
        "ci",
        "stage-main-cache-candidate",
    ),
    CleanInstallCase::new(
        "clean-install-ci-verify-main-cache-candidate",
        "ci",
        "verify-main-cache-candidate",
    ),
    CleanInstallCase::new("clean-install-rebalance-tests-run", "rebalance-tests", "run"),
    CleanInstallCase::new(
        "clean-install-redact-scrub-log-secrets",
        "redact",
        "scrub-log-secrets",
    ),
    CleanInstallCase::new(
        "clean-install-redact-scrub-submodule-paths",
        "redact",
        "scrub-submodule-paths",
    ),
    CleanInstallCase::new("clean-install-redact-secrets", "redact", "secrets"),
    CleanInstallCase::new(
        "clean-install-redact-tmpdir-paths",
        "redact",
        "tmpdir-paths",
    ),
    CleanInstallCase::new(
        "clean-install-report-tokens-analyze",
        "report-tokens",
        "analyze",
    ),
    CleanInstallCase::new("clean-install-repo-size", "repo", "size"),
    CleanInstallCase::new("clean-install-research-banner", "research", "banner"),
    CleanInstallCase::new(
        "clean-install-research-render-findings-batch",
        "research",
        "render-findings-batch",
    ),
    CleanInstallCase::new("clean-install-research-run-planner", "research", "run-planner"),
    CleanInstallCase::new(
        "clean-install-research-validate-citations",
        "research",
        "validate-citations",
    ),
    CleanInstallCase::new("clean-install-eval-research", "eval", "research"),
    CleanInstallCase::new(
        "clean-install-eval-validate-research-output",
        "eval",
        "validate-research-output",
    ),
    CleanInstallCase::new(
        "clean-install-residual-bash-paths",
        "residual-bash",
        "paths",
    ),
    CleanInstallCase::new("clean-install-final-report-write", "final-report", "write"),
    CleanInstallCase::new(
        "clean-install-final-report-step18b",
        "final-report",
        "step18b",
    ),
    CleanInstallCase::new("clean-install-timing-dump", "timing", "dump"),
    CleanInstallCase::new(
        "clean-install-timing-harness-mark",
        "timing",
        "harness-mark",
    ),
    CleanInstallCase::new("clean-install-timing-mark", "timing", "mark"),
    CleanInstallCase::new("clean-install-timing-record-round", "timing", "record-round"),
    CleanInstallCase::new(
        "clean-install-timing-record-vendor-task",
        "timing",
        "record-vendor-task",
    ),
    CleanInstallCase::new("clean-install-timing-report", "timing", "report"),
    CleanInstallCase::new("clean-install-timing-task-kinds", "timing", "task-kinds"),
    CleanInstallCase::new(
        "clean-install-timing-telemetry-mark",
        "timing",
        "telemetry-mark",
    ),
    CleanInstallCase::new("clean-install-token-append-record", "token", "append-record"),
    CleanInstallCase::new("clean-install-token-check-budget", "token", "check-budget"),
    CleanInstallCase::new("clean-install-token-claude-source", "token", "claude-source"),
    CleanInstallCase::new(
        "clean-install-token-compute-pr-line-counts",
        "token",
        "compute-pr-line-counts",
    ),
    CleanInstallCase::new(
        "clean-install-token-compute-pr-lines",
        "token",
        "compute-pr-lines",
    ),
    CleanInstallCase::new("clean-install-token-cost", "token", "cost"),
    CleanInstallCase::new("clean-install-token-dump", "token", "dump"),
    CleanInstallCase::new("clean-install-token-lane-report", "token", "lane-report"),
    CleanInstallCase::new("clean-install-token-lane-write", "token", "lane-write"),
    CleanInstallCase::new("clean-install-token-mark", "token", "mark"),
    CleanInstallCase::new(
        "clean-install-token-measure-cache-efficiency",
        "token",
        "measure-cache-efficiency",
    ),
    CleanInstallCase::new(
        "clean-install-token-measure-checks-digest-savings",
        "token",
        "measure-checks-digest-savings",
    ),
    CleanInstallCase::new(
        "clean-install-token-measure-md-cost",
        "token",
        "measure-md-cost",
    ),
    CleanInstallCase::new(
        "clean-install-token-measure-ngram-duplication",
        "token",
        "measure-ngram-duplication",
    ),
    CleanInstallCase::new(
        "clean-install-token-measure-panel-cost",
        "token",
        "measure-panel-cost",
    ),
    CleanInstallCase::new(
        "clean-install-token-measure-realized-cost",
        "token",
        "measure-realized-cost",
    ),
    CleanInstallCase::new(
        "clean-install-token-measure-references-heatmap",
        "token",
        "measure-references-heatmap",
    ),
    CleanInstallCase::new("clean-install-token-record-vendor", "token", "record-vendor"),
    CleanInstallCase::new(
        "clean-install-token-record-vendor-sidecar",
        "token",
        "record-vendor-sidecar",
    ),
    CleanInstallCase::new(
        "clean-install-token-render-cost-line",
        "token",
        "render-cost-line",
    ),
    CleanInstallCase::new("clean-install-token-report", "token", "report"),
    CleanInstallCase::new("clean-install-test-shard-pack", "test-shard", "pack"),
    CleanInstallCase::new(
        "clean-install-test-shard-read-makefile",
        "test-shard",
        "read-makefile",
    ),
    CleanInstallCase::new(
        "clean-install-test-shard-write-makefile",
        "test-shard",
        "write-makefile",
    ),
    CleanInstallCase::new(
        "clean-install-difficulty-extract-plan-metadata",
        "difficulty",
        "extract-plan-metadata",
    ),
    CleanInstallCase::new(
        "clean-install-difficulty-render-line",
        "difficulty",
        "render-line",
    ),
    CleanInstallCase::new("clean-install-difficulty-render-rubric", "difficulty", "render-rubric"),
    CleanInstallCase::new(
        "clean-install-difficulty-resolve-panel",
        "difficulty",
        "resolve-panel",
    ),
    CleanInstallCase::new("clean-install-difficulty-sync-labels", "difficulty", "sync-labels"),
    CleanInstallCase::new(
        "clean-install-difficulty-validate-rating",
        "difficulty",
        "validate-rating",
    ),
    CleanInstallCase::new(
        "clean-install-difficulty-write-record",
        "difficulty",
        "write-record",
    ),
    CleanInstallCase::new(
        "clean-install-difficulty-calibration-analyze",
        "difficulty-calibration",
        "analyze",
    ),
    CleanInstallCase::new(
        "clean-install-fluff-analysis-analyze",
        "fluff-analysis",
        "analyze",
    ),
    CleanInstallCase::new("clean-install-forked-repo-setup", "forked-repo", "setup"),
    CleanInstallCase::new("clean-install-diagram-code-flow", "diagram", "code-flow"),
    CleanInstallCase::new(
        "clean-install-render-findings-view",
        "render",
        "findings-view",
    ),
    CleanInstallCase::new("clean-install-render-lane-status", "render", "lane-status"),
    CleanInstallCase::new("clean-install-render-reviewer", "render", "reviewer"),
    CleanInstallCase::new("clean-install-render-run-summary", "render", "run-summary"),
    CleanInstallCase::new(
        "clean-install-render-scope-anchor",
        "render",
        "scope-anchor",
    ),
    CleanInstallCase::new("clean-install-render-specialist", "render", "specialist"),
    CleanInstallCase::new("clean-install-render-plan-review", "render", "plan-review"),
    CleanInstallCase::new("clean-install-render-voter", "render", "voter"),
    CleanInstallCase::new("clean-install-mermaid-sanitize", "mermaid", "sanitize"),
    CleanInstallCase::new("clean-install-diagrams-upsert", "diagrams", "upsert"),
    CleanInstallCase::new(
        "clean-install-scope-anchor-design-handoff",
        "scope-anchor",
        "design-handoff",
    ),
    CleanInstallCase::new(
        "clean-install-scope-anchor-relay-allowed",
        "scope-anchor",
        "relay-allowed",
    ),
    CleanInstallCase::new(
        "clean-install-scope-anchor-retally-handoff",
        "scope-anchor",
        "retally-handoff",
    ),
    CleanInstallCase::new(
        "clean-install-scope-anchor-validate",
        "scope-anchor",
        "validate",
    ),
    CleanInstallCase::new(
        "clean-install-dirty-tree-baseline",
        "dirty-tree",
        "baseline",
    ),
    CleanInstallCase::new(
        "clean-install-dirty-tree-checkpoint",
        "dirty-tree",
        "checkpoint",
    ),
    CleanInstallCase::new(
        "clean-install-dirty-tree-scope-check",
        "dirty-tree",
        "scope-check",
    ),
    CleanInstallCase::new(
        "clean-install-dirty-tree-scope-marker",
        "dirty-tree",
        "scope-marker",
    ),
    CleanInstallCase::new(
        "clean-install-implement-recovery-paths",
        "implement",
        "recovery-paths",
    ),
    CleanInstallCase::new(
        "clean-install-implement-run-step-checks",
        "implement",
        "run-step-checks",
    ),
    CleanInstallCase::new(
        "clean-install-implement-checks-step5-resume",
        "implement",
        "checks-step5-resume",
    ),
    CleanInstallCase::new(
        "clean-install-implement-code-flow-diagram",
        "implement",
        "code-flow-diagram",
    ),
    CleanInstallCase::new(
        "clean-install-implement-step-5-resume",
        "implement",
        "step-5-resume",
    ),
    CleanInstallCase::new(
        "clean-install-implement-step-5-review",
        "implement",
        "step-5-review",
    ),
    CleanInstallCase::new(
        "clean-install-implement-step-6-entry",
        "implement",
        "step-6-entry",
    ),
    CleanInstallCase::new(
        "clean-install-implement-step-7a",
        "implement",
        "step-7a",
    ),
    CleanInstallCase::new(
        "clean-install-implement-step-8-oos-checkpoint",
        "implement",
        "step-8-oos-checkpoint",
    ),
    CleanInstallCase::new(
        "clean-install-implement-step-8-seed-initial",
        "implement",
        "step-8-seed-initial",
    ),
    CleanInstallCase::new(
        "clean-install-implement-step-8-ship",
        "implement",
        "step-8-ship",
    ),
    CleanInstallCase::new(
        "clean-install-ship-seed-initial-state",
        "ship",
        "seed-initial-state",
    ),
    CleanInstallCase::new(
        "clean-install-ship-write-result-env",
        "ship",
        "write-result-env",
    ),
    CleanInstallCase::new(
        "clean-install-implement-checks-result-identity",
        "implement",
        "checks-result-identity",
    ),
    CleanInstallCase::new("clean-install-implement-step-16", "implement", "step-16"),
    CleanInstallCase::new(
        "clean-install-implement-step-16-16a",
        "implement",
        "step-16-16a",
    ),
    CleanInstallCase::new(
        "clean-install-implement-step-16-17",
        "implement",
        "step-16-17",
    ),
    CleanInstallCase::new("clean-install-implement-step-17", "implement", "step-17"),
    CleanInstallCase::new("clean-install-implement-step-18", "implement", "step-18"),
    CleanInstallCase::new(
        "clean-install-implement-step-18-gate-logs-flush",
        "implement",
        "step-18-gate-logs-flush",
    ),
    CleanInstallCase::new("clean-install-implement-step-19", "implement", "step-19"),
    CleanInstallCase::new(
        "clean-install-implement-finalize-postbump",
        "implement-finalize",
        "postbump",
    ),
    CleanInstallCase::new(
        "clean-install-implement-finalize-postmerge",
        "implement-finalize",
        "postmerge",
    ),
    CleanInstallCase::new(
        "clean-install-implement-finalize-teardown",
        "implement-finalize",
        "teardown",
    ),
    CleanInstallCase::new("clean-install-checks-rust-clippy", "checks", "rust-clippy"),
    CleanInstallCase::new(
        "clean-install-checks-self-edit-log",
        "checks",
        "self-edit-log",
    ),
    CleanInstallCase::new(
        "clean-install-checks-run-relevant",
        "checks",
        "run-relevant",
    ),
    CleanInstallCase::new(
        "clean-install-checks-contains-pins",
        "checks",
        "contains-pins",
    ),
    CleanInstallCase::new(
        "clean-install-checks-fixer-evidence",
        "checks",
        "fixer-evidence",
    ),
    CleanInstallCase::new("clean-install-checks-lint-fix", "checks", "lint-fix"),
    CleanInstallCase::new(
        "clean-install-checks-repair-loop",
        "checks",
        "repair-loop",
    ),
    CleanInstallCase::new(
        "clean-install-gh-agnix-ensure-label",
        "gh",
        "agnix-ensure-label",
    ),
    CleanInstallCase::new("clean-install-gh-agnix-issue", "gh", "agnix-issue"),
    CleanInstallCase::new("clean-install-gh-remote-repo", "gh", "remote-repo"),
    CleanInstallCase::new("clean-install-gh-resolve-repo", "gh", "resolve-repo"),
    CleanInstallCase::new("clean-install-gh-run-logs", "gh", "run-logs"),
    CleanInstallCase::new("clean-install-gh-workflow-path", "gh", "workflow-path"),
    CleanInstallCase::new("clean-install-git-amend-add", "git", "amend-add"),
    CleanInstallCase::new("clean-install-git-branch-info", "git", "branch-info"),
    CleanInstallCase::new(
        "clean-install-git-check-main-sync",
        "git",
        "check-main-sync",
    ),
    CleanInstallCase::new(
        "clean-install-git-check-phantom-dirty",
        "git",
        "check-phantom-dirty",
    ),
    CleanInstallCase::new(
        "clean-install-git-check-remote-branch",
        "git",
        "check-remote-branch",
    ),
    CleanInstallCase::new("clean-install-git-checkout-ours", "git", "checkout-ours"),
    CleanInstallCase::new("clean-install-git-clean-tree", "git", "clean-tree"),
    CleanInstallCase::new("clean-install-git-commit", "git", "commit"),
    CleanInstallCase::new("clean-install-git-conflict-files", "git", "conflict-files"),
    CleanInstallCase::new("clean-install-git-count-commits", "git", "count-commits"),
    CleanInstallCase::new("clean-install-git-current-branch", "git", "current-branch"),
    CleanInstallCase::new("clean-install-git-phantom-probe", "git", "phantom-probe"),
    CleanInstallCase::new("clean-install-git-rebase-abort", "git", "rebase-abort"),
    CleanInstallCase::new("clean-install-git-rebase-skip", "git", "rebase-skip"),
    CleanInstallCase::new("clean-install-git-show-stage", "git", "show-stage"),
    CleanInstallCase::new(
        "clean-install-git-snapshot-untracked",
        "git",
        "snapshot-untracked",
    ),
    CleanInstallCase::new("clean-install-git-stage", "git", "stage"),
    CleanInstallCase::new(
        "clean-install-git-sync-local-main",
        "git",
        "sync-local-main",
    ),
    CleanInstallCase::new(
        "clean-install-hook-anti-read-poll",
        "hook",
        "anti-read-poll",
    ),
    CleanInstallCase::new(
        "clean-install-hook-audit-edit-write",
        "hook",
        "audit-edit-write",
    ),
    CleanInstallCase::new(
        "clean-install-hook-block-submodule-edit",
        "hook",
        "block-submodule-edit",
    ),
    CleanInstallCase::new(
        "clean-install-hook-cleanup-sessionstart",
        "hook",
        "cleanup-sessionstart",
    ),
    CleanInstallCase::new(
        "clean-install-hook-deny-edit-write",
        "hook",
        "deny-edit-write",
    ),
    CleanInstallCase::new(
        "clean-install-hook-deny-run-in-background",
        "hook",
        "deny-run-in-background",
    ),
    CleanInstallCase::new(
        "clean-install-hook-sessionstart-health",
        "hook",
        "sessionstart-health",
    ),
    CleanInstallCase::new(
        "clean-install-hook-sessionstart-statusline",
        "hook",
        "sessionstart-statusline",
    ),
    CleanInstallCase::new(
        "clean-install-hook-stop-fail-close",
        "hook",
        "stop-fail-close",
    ),
    CleanInstallCase::new(
        "clean-install-plugin-read-version",
        "plugin",
        "read-version",
    ),
    CleanInstallCase::new(
        "clean-install-plugin-resolve-repository",
        "plugin",
        "resolve-repository",
    ),
    CleanInstallCase::new("clean-install-object-store-gcs", "object-store", "gcs"),
    CleanInstallCase::new("clean-install-push-branch", "push", "branch"),
    CleanInstallCase::new(
        "clean-install-push-checkpoint-probe",
        "push",
        "checkpoint-probe",
    ),
    CleanInstallCase::new("clean-install-push-force", "push", "force"),
    CleanInstallCase::new("clean-install-push-rebase", "push", "rebase"),
    CleanInstallCase::new(
        "clean-install-release-asset-candidate",
        "release",
        "asset-candidate",
    ),
    CleanInstallCase::new(
        "clean-install-release-classify-bump",
        "release",
        "classify-bump",
    ),
    CleanInstallCase::new(
        "clean-install-release-collect-assets",
        "release",
        "collect-assets",
    ),
    CleanInstallCase::new(
        "clean-install-release-package-asset",
        "release",
        "package-asset",
    ),
    CleanInstallCase::new(
        "clean-install-release-plugin-runtime",
        "release",
        "plugin-runtime",
    ),
    CleanInstallCase::new("clean-install-release-prepare", "release", "prepare"),
    CleanInstallCase::new(
        "clean-install-release-reconcile-notes",
        "release",
        "reconcile-notes",
    ),
    CleanInstallCase::new(
        "clean-install-release-set-version",
        "release",
        "set-version",
    ),
    CleanInstallCase::new(
        "clean-install-release-validate-assets",
        "release",
        "validate-assets",
    ),
    CleanInstallCase::new("clean-install-run-log-archive", "run-log", "archive"),
    CleanInstallCase::new("clean-install-run-log-manifest", "run-log", "manifest"),
    CleanInstallCase::new("clean-install-run-log-publish", "run-log", "publish"),
    CleanInstallCase::new(
        "clean-install-run-log-lifecycle-cancel",
        "run-log",
        "lifecycle-cancel",
    ),
    CleanInstallCase::new(
        "clean-install-run-log-lifecycle-early-return",
        "run-log",
        "lifecycle-early-return",
    ),
    CleanInstallCase::new(
        "clean-install-run-log-lifecycle-failure",
        "run-log",
        "lifecycle-failure",
    ),
    CleanInstallCase::new(
        "clean-install-run-log-lifecycle-finalize",
        "run-log",
        "lifecycle-finalize",
    ),
    CleanInstallCase::new(
        "clean-install-run-log-lifecycle-start",
        "run-log",
        "lifecycle-start",
    ),
    CleanInstallCase::new(
        "clean-install-run-log-materialize",
        "run-log",
        "materialize",
    ),
    CleanInstallCase::new(
        "clean-install-run-log-cleanup-implement-logs",
        "run-log",
        "cleanup-implement-logs",
    ),
    CleanInstallCase::new(
        "clean-install-run-log-migrate-layout",
        "run-log",
        "migrate-layout",
    ),
    CleanInstallCase::new(
        "clean-install-run-log-retro-fix-cursor",
        "run-log",
        "retro-fix-cursor",
    ),
    CleanInstallCase::new(
        "clean-install-run-log-retro-v3-sweep",
        "run-log",
        "retro-v3-sweep",
    ),
    CleanInstallCase::new("clean-install-run-log-sync", "run-log", "sync"),
    CleanInstallCase::new(
        "clean-install-run-log-validate-run-id",
        "run-log",
        "validate-run-id",
    ),
    CleanInstallCase::new(
        "clean-install-stall-recovery-chat-print",
        "stall-recovery",
        "chat-print",
    ),
    CleanInstallCase::new(
        "clean-install-stall-recovery-clear-stall",
        "stall-recovery",
        "clear-stall",
    ),
    CleanInstallCase::new(
        "clean-install-stall-recovery-comment-url-from-response",
        "stall-recovery",
        "comment-url-from-response",
    ),
    CleanInstallCase::new("clean-install-stall-recovery-classify", "stall-recovery", "classify"),
    CleanInstallCase::new("clean-install-stall-recovery-init-attempts", "stall-recovery", "init-attempts"),
    CleanInstallCase::new("clean-install-stall-recovery-normalize-file-failure-report-env", "stall-recovery", "normalize-file-failure-report-env"),
    CleanInstallCase::new("clean-install-stall-recovery-normalize-issue-env", "stall-recovery", "normalize-issue-env"),
    CleanInstallCase::new("clean-install-stall-recovery-normalize-outcome", "stall-recovery", "normalize-outcome"),
    CleanInstallCase::new("clean-install-stall-recovery-record-attempt", "stall-recovery", "record-attempt"),
    CleanInstallCase::new("clean-install-stall-recovery-record-escalation", "stall-recovery", "record-escalation"),
    CleanInstallCase::new("clean-install-stall-recovery-retry-policy", "stall-recovery", "retry-policy"),
    CleanInstallCase::new(
        "clean-install-stall-recovery-rewind-public-fd",
        "stall-recovery",
        "rewind-public-fd",
    ),
    CleanInstallCase::new(
        "clean-install-stall-recovery-compose-report",
        "stall-recovery",
        "compose-report",
    ),
    CleanInstallCase::new(
        "clean-install-stall-recovery-compose-comment-request",
        "stall-recovery",
        "compose-comment-request",
    ),
    CleanInstallCase::new(
        "clean-install-stall-recovery-dedup-tier-a-report",
        "stall-recovery",
        "dedup-tier-a-report",
    ),
    CleanInstallCase::new(
        "clean-install-stall-recovery-file-report",
        "stall-recovery",
        "file-report",
    ),
    CleanInstallCase::new(
        "clean-install-stall-recovery-is-larch-dev-clone",
        "stall-recovery",
        "is-larch-dev-clone",
    ),
    CleanInstallCase::new(
        "clean-install-stall-recovery-find-open-stall-issue",
        "stall-recovery",
        "find-open-stall-issue",
    ),
    CleanInstallCase::new("clean-install-stall-recovery-lint", "stall-recovery", "lint"),
    CleanInstallCase::new(
        "clean-install-stall-recovery-populate-sensitive-corpus",
        "stall-recovery",
        "populate-sensitive-corpus",
    ),
    CleanInstallCase::new(
        "clean-install-stall-recovery-seed-terminal-state",
        "stall-recovery",
        "seed-terminal-state",
    ),
    CleanInstallCase::new(
        "clean-install-stall-recovery-validate-terminal-state",
        "stall-recovery",
        "validate-terminal-state",
    ),
    CleanInstallCase::new(
        "clean-install-stall-recovery-validate-tier-b-public-file",
        "stall-recovery",
        "validate-tier-b-public-file",
    ),
    CleanInstallCase::new(
        "clean-install-stall-recovery-validate-token",
        "stall-recovery",
        "validate-token",
    ),
    CleanInstallCase::new("clean-install-run-log-init", "run-log", "init"),
    CleanInstallCase::new("clean-install-run-log-write", "run-log", "write"),
    CleanInstallCase::new(
        "clean-install-run-log-write-round",
        "run-log",
        "write-round",
    ),
    CleanInstallCase::new("clean-install-run-log-append", "run-log", "append"),
    CleanInstallCase::new("clean-install-run-log-exists", "run-log", "exists"),
    CleanInstallCase::new(
        "clean-install-run-log-append-entry",
        "run-log",
        "append-entry",
    ),
    CleanInstallCase::new(
        "clean-install-run-log-append-failure",
        "run-log",
        "append-failure",
    ),
    CleanInstallCase::new(
        "clean-install-run-log-verify-completeness",
        "run-log",
        "verify-completeness",
    ),
    CleanInstallCase::new(
        "clean-install-run-log-publish-breadcrumbs",
        "run-log",
        "publish-breadcrumbs",
    ),
    CleanInstallCase::new(
        "clean-install-run-log-capture-transcript",
        "run-log",
        "capture-transcript",
    ),
    CleanInstallCase::new("clean-install-run-log-checkpoint", "run-log", "checkpoint"),
    CleanInstallCase::new(
        "clean-install-run-log-prepare-terminal-snapshot",
        "run-log",
        "prepare-terminal-snapshot",
    ),
    CleanInstallCase::new("clean-install-run-log-refresh", "run-log", "refresh"),
    CleanInstallCase::new(
        "clean-install-run-log-render-session-transcript",
        "run-log",
        "render-session-transcript",
    ),
    CleanInstallCase::new("clean-install-progress-activate", "progress", "activate"),
    CleanInstallCase::new("clean-install-progress-cleanup", "progress", "cleanup"),
    CleanInstallCase::new("clean-install-progress-clear", "progress", "clear"),
    CleanInstallCase::new(
        "clean-install-progress-deactivate",
        "progress",
        "deactivate",
    ),
    CleanInstallCase::new(
        "clean-install-progress-install-statusline",
        "progress",
        "install-statusline",
    ),
    CleanInstallCase::new("clean-install-progress-note", "progress", "note"),
    CleanInstallCase::new(
        "clean-install-progress-render-phase-detail",
        "progress",
        "render-phase-detail",
    ),
    CleanInstallCase::new(
        "clean-install-progress-session-reset",
        "progress",
        "session-reset",
    ),
    CleanInstallCase::new(
        "clean-install-progress-statusline",
        "progress",
        "statusline",
    ),
    CleanInstallCase::new(
        "clean-install-progress-write-design-round-meta",
        "progress",
        "write-design-round-meta",
    ),
    CleanInstallCase::new(
        "clean-install-progress-write-implement-round-meta",
        "progress",
        "write-implement-round-meta",
    ),
    CleanInstallCase::new(
        "clean-install-run-log-storage-preflight",
        "run-log",
        "storage-preflight",
    ),
    CleanInstallCase::new(
        "clean-install-upgrade-larch-release-step7-root",
        "upgrade-larch",
        "release-step7-root",
    ),
    CleanInstallCase::new("clean-install-upgrade-larch-run", "upgrade-larch", "run"),
    CleanInstallCase::new(
        "clean-install-upgrade-larch-sparse-dirs",
        "upgrade-larch",
        "sparse-dirs",
    ),
    CleanInstallCase::new(
        "clean-install-verify-skill-called",
        "verify",
        "skill-called",
    ),
    CleanInstallCase::new(
        "clean-install-voting-code-review-classification-header",
        "voting",
        "code-review-classification-header",
    ),
    CleanInstallCase::new(
        "clean-install-voting-findings-classification-header",
        "voting",
        "findings-classification-header",
    ),
    CleanInstallCase::new(
        "clean-install-calibration-replay-rebuild-ballot",
        "calibration-replay",
        "rebuild-ballot",
    ),
    CleanInstallCase::new(
        "clean-install-calibration-replay-run-replay",
        "calibration-replay",
        "run-replay",
    ),
    CleanInstallCase::new(
        "clean-install-calibration-replay-validate-manifest",
        "calibration-replay",
        "validate-manifest",
    ),
    CleanInstallCase::new(
        "clean-install-voter-calibration-snapshot",
        "voter-calibration",
        "snapshot",
    ),
    CleanInstallCase::new(
        "clean-install-voter-calibration-analyze",
        "voter-calibration",
        "analyze",
    ),
    CleanInstallCase::new(
        "clean-install-voting-compose-tally-record",
        "voting",
        "compose-tally-record",
    ),
    CleanInstallCase::new(
        "clean-install-voting-degraded-warning",
        "voting",
        "degraded-warning",
    ),
    CleanInstallCase::new(
        "clean-install-voting-effective-judges",
        "voting",
        "effective-judges",
    ),
    CleanInstallCase::new(
        "clean-install-voting-parse-rate-check",
        "voting",
        "parse-rate-check",
    ),
    CleanInstallCase::new(
        "clean-install-voting-parse-rate-retry",
        "voting",
        "parse-rate-retry",
    ),
    CleanInstallCase::new(
        "clean-install-voting-scoreboard",
        "voting",
        "scoreboard",
    ),
    CleanInstallCase::new(
        "clean-install-voting-tally-vote",
        "voting",
        "tally-vote",
    ),
    CleanInstallCase::new(
        "clean-install-voting-voter-status-block",
        "voting",
        "voter-status-block",
    ),
    CleanInstallCase::new(
        "clean-install-voting-write-tally",
        "voting",
        "write-tally",
    ),
];

#[test]
fn release_publication_commands_are_exposed_by_the_rust_binary() {
    for command in ["finish", "promote", "promote-latest"] {
        let output = Command::new(env!("CARGO_BIN_EXE_larch"))
            .args(["release", command, "--help"])
            .output()
            .unwrap_or_else(|error| panic!("launch release {command}: {error}"));

        assert!(
            output.status.success(),
            "release {command} --help failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            String::from_utf8_lossy(&output.stdout)
                .contains(&format!("Usage: larch release {command}")),
            "release {command} did not enter the Rust CLI"
        );
    }
}

const CLEAN_INSTALL_PARTITION_COUNT: usize = 12;

#[test]
fn rust_owned_selector_matrix_partition_0_enters_through_verified_clean_install_script() {
    assert_clean_install_partition(0);
}

#[test]
fn rust_owned_selector_matrix_partition_1_enters_through_verified_clean_install_script() {
    assert_clean_install_partition(1);
}

#[test]
fn rust_owned_selector_matrix_partition_2_enters_through_verified_clean_install_script() {
    assert_clean_install_partition(2);
}

#[test]
fn rust_owned_selector_matrix_partition_3_enters_through_verified_clean_install_script() {
    assert_clean_install_partition(3);
}

#[test]
fn rust_owned_selector_matrix_partition_4_enters_through_verified_clean_install_script() {
    assert_clean_install_partition(4);
}

#[test]
fn rust_owned_selector_matrix_partition_5_enters_through_verified_clean_install_script() {
    assert_clean_install_partition(5);
}

#[test]
fn rust_owned_selector_matrix_partition_6_enters_through_verified_clean_install_script() {
    assert_clean_install_partition(6);
}

#[test]
fn rust_owned_selector_matrix_partition_7_enters_through_verified_clean_install_script() {
    assert_clean_install_partition(7);
}

#[test]
fn rust_owned_selector_matrix_partition_8_enters_through_verified_clean_install_script() {
    assert_clean_install_partition(8);
}

#[test]
fn rust_owned_selector_matrix_partition_9_enters_through_verified_clean_install_script() {
    assert_clean_install_partition(9);
}

#[test]
fn rust_owned_selector_matrix_partition_10_enters_through_verified_clean_install_script() {
    assert_clean_install_partition(10);
}

#[test]
fn rust_owned_selector_matrix_partition_11_enters_through_verified_clean_install_script() {
    assert_clean_install_partition(11);
}

fn assert_clean_install_partition(partition: usize) {
    let fixture = clean_install_fixture();
    for (index, case) in CLEAN_INSTALL_CASES.iter().copied().enumerate() {
        if index % CLEAN_INSTALL_PARTITION_COUNT != partition {
            continue;
        }
        fs::write(&fixture.events, b"").expect("clear clean-install event log");
        let output = run_clean_install_case(&fixture, case, None);
        assert_eq!(
            output.status.code(),
            Some(case.expected_exit()),
            "{} failed: {}",
            case.id,
            String::from_utf8_lossy(&output.stderr)
        );
        let events = fs::read_to_string(&fixture.events).expect("read clean-install events");
        let lines: Vec<&str> = events.lines().collect();
        let expected_dispatch = clean_install_dispatch(&fixture, case);
        assert_eq!(lines.first(), Some(&"--version"), "{}", case.id);
        assert_eq!(lines.get(1), Some(&"bootstrap self-check"), "{}", case.id);
        assert_eq!(
            lines.get(2),
            Some(&expected_dispatch.as_str()),
            "{}",
            case.id
        );
        assert_eq!(lines.len(), 3, "{}", case.id);
        assert!(!fixture.root.join("bin/larch").exists(), "{}", case.id);
    }
}

#[test]
fn clean_install_validation_failures_precede_selector_dispatch() {
    let fixture = clean_install_fixture();
    let case = CLEAN_INSTALL_CASES[0];
    for failure in ["version", "target", "bootstrap"] {
        fs::write(&fixture.events, b"").expect("clear clean-install event log");
        let output = run_clean_install_case(&fixture, case, Some(failure));
        assert!(
            !output.status.success(),
            "{failure} unexpectedly dispatched"
        );
        let events = fs::read_to_string(&fixture.events).expect("read clean-install events");
        assert!(
            !events
                .lines()
                .any(|line| line == clean_install_dispatch(&fixture, case)),
            "{failure} reached selector dispatch"
        );
    }
}

#[cfg(unix)]
#[test]
fn hook_no_install_mode_never_bootstraps_and_still_uses_verified_binaries() {
    let fixture = clean_install_fixture();
    let entrypoint = fixture.root.join("scripts/larch.sh");
    let unavailable = Command::new("/bin/bash")
        .arg(&entrypoint)
        .args(["hook", "block-submodule-edit"])
        .env("HOME", &fixture.home)
        .env("CLAUDE_PLUGIN_ROOT", &fixture.root)
        .env("LARCH_BOOTSTRAP_NO_INSTALL", "1")
        .env_remove("LARCH_BINARY")
        .env_remove("CLAUDE_PLUGIN_DATA")
        .output()
        .expect("run unavailable no-install hook");
    assert_eq!(unavailable.status.code(), Some(97));
    assert!(!fixture.root.join("bin").exists());

    let invalid_override = Command::new("/bin/bash")
        .arg(&entrypoint)
        .args(["hook", "block-submodule-edit"])
        .env("HOME", &fixture.home)
        .env("CLAUDE_PLUGIN_ROOT", &fixture.root)
        .env("LARCH_BOOTSTRAP_NO_INSTALL", "1")
        .env("LARCH_BINARY", "relative/larch")
        .env_remove("CLAUDE_PLUGIN_DATA")
        .output()
        .expect("run invalid override in no-install mode");
    assert_eq!(invalid_override.status.code(), Some(97));
    assert!(!fixture.root.join("bin").exists());

    fs::write(&fixture.events, b"").expect("clear override event log");
    let override_output = Command::new("/bin/bash")
        .arg(&entrypoint)
        .args(["hook", "anti-read-poll"])
        .env("HOME", &fixture.home)
        .env("CLAUDE_PLUGIN_ROOT", &fixture.root)
        .env("LARCH_BOOTSTRAP_NO_INSTALL", "1")
        .env("LARCH_BINARY", &fixture.wrapper)
        .env("REAL_LARCH", &fixture.binary)
        .env("CLEAN_INSTALL_EVENTS", &fixture.events)
        .output()
        .expect("run verified override in no-install mode");
    assert!(
        override_output.status.success(),
        "{}",
        String::from_utf8_lossy(&override_output.stderr)
    );
    assert!(
        fs::read_to_string(&fixture.events)
            .expect("override events")
            .lines()
            .any(|line| line == "hook anti-read-poll")
    );

    let installed = fixture.root.join("bin/larch");
    fs::create_dir_all(installed.parent().expect("installed binary parent"))
        .expect("create installed bin directory");
    fs::copy(&fixture.wrapper, &installed).expect("install verified fixture binary");
    let mut permissions = fs::metadata(&installed)
        .expect("installed binary metadata")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&installed, permissions).expect("make installed binary executable");
    fs::write(&fixture.events, b"").expect("clear installed event log");
    let installed_output = Command::new("/bin/bash")
        .arg(&entrypoint)
        .args(["hook", "anti-read-poll"])
        .env("HOME", &fixture.home)
        .env("CLAUDE_PLUGIN_ROOT", &fixture.root)
        .env("LARCH_BOOTSTRAP_NO_INSTALL", "1")
        .env_remove("LARCH_BINARY")
        .env("REAL_LARCH", &fixture.binary)
        .env("CLEAN_INSTALL_EVENTS", &fixture.events)
        .output()
        .expect("run verified installed binary in no-install mode");
    assert!(
        installed_output.status.success(),
        "{}",
        String::from_utf8_lossy(&installed_output.stderr)
    );
    assert!(
        fs::read_to_string(&fixture.events)
            .expect("installed events")
            .lines()
            .any(|line| line == "hook anti-read-poll")
    );
}

#[cfg(unix)]
#[test]
fn hook_shims_emit_static_denies_when_no_verified_binary_is_available() {
    let fixture = clean_install_fixture();
    let manifest = fixture.root.join(".claude-plugin/plugin.json");
    let valid_manifest = fs::read(&manifest).expect("read clean-install manifest");
    let repair = format!(
        "CLAUDE_PLUGIN_ROOT={root} CLAUDE_PLUGIN_DATA=<absolute-dir> {root}/scripts/larch.sh --version",
        root = fixture.root.display()
    );
    for (script, unavailable_fragment) in [
        (
            "block-submodule-edit.sh",
            "larch hook unavailable, blocking as precaution",
        ),
        (
            "deny-edit-write.sh",
            "Edit/Write/NotebookEdit outside /tmp",
        ),
        (
            "hook-deny-run-in-background.sh",
            "run_in_background denied: larch hook unavailable",
        ),
    ] {
        let source = repo_root().join("scripts").join(script);
        let destination = fixture.root.join("scripts").join(script);
        fs::copy(source, &destination).expect("copy hook shim");
        let mut permissions = fs::metadata(&destination)
            .expect("hook shim metadata")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&destination, permissions).expect("make hook shim executable");

        // A valid root with no `bin/larch` is the exit-97 shape an interrupted
        // upgrade leaves behind (#9097): still a deny, but one that names the
        // one-command repair so a bricked session can report it.
        fs::write(&manifest, &valid_manifest).expect("restore clean-install manifest");
        let reason = shim_deny_reason(&fixture, &destination, script);
        assert!(
            reason.contains("no verified larch executable for this plugin version (exit 97)")
                && reason.contains(&repair),
            "{script}: {reason}"
        );
        assert!(!fixture.root.join("bin").exists(), "{script}");

        // A root with a JSON-significant or shell-hostile character is never
        // interpolated; the reason falls back to the placeholder.
        let hostile_root = fixture.root.with_file_name("plu\"g in");
        fs::create_dir_all(hostile_root.join("scripts")).expect("hostile root scripts");
        fs::create_dir_all(hostile_root.join(".claude-plugin")).expect("hostile root manifest");
        for relative in ["scripts/larch.sh", ".claude-plugin/plugin.json"] {
            fs::copy(fixture.root.join(relative), hostile_root.join(relative))
                .expect("copy into hostile root");
        }
        let hostile_shim = hostile_root.join("scripts").join(script);
        fs::copy(&destination, &hostile_shim).expect("copy shim into hostile root");
        let reason = shim_deny_reason_at(&fixture, &hostile_root, &hostile_shim, script);
        assert!(
            reason.contains("CLAUDE_PLUGIN_ROOT=<CLAUDE_PLUGIN_ROOT> CLAUDE_PLUGIN_DATA=<absolute-dir> <CLAUDE_PLUGIN_ROOT>/scripts/larch.sh --version"),
            "{script}: {reason}"
        );

        // Any other bootstrap failure keeps the fixed static deny.
        fs::write(&manifest, b"not a manifest\n").expect("corrupt clean-install manifest");
        let reason = shim_deny_reason(&fixture, &destination, script);
        assert!(
            reason.contains(unavailable_fragment),
            "{script}: {reason}"
        );
        assert!(!fixture.root.join("bin").exists(), "{script}");
    }
}

#[cfg(unix)]
fn shim_deny_reason(fixture: &CleanInstallFixture, shim: &Path, script: &str) -> String {
    shim_deny_reason_at(fixture, &fixture.root, shim, script)
}

#[cfg(unix)]
fn shim_deny_reason_at(
    fixture: &CleanInstallFixture,
    root: &Path,
    shim: &Path,
    script: &str,
) -> String {
    let output = Command::new("/bin/bash")
        .arg(shim)
        .env("HOME", &fixture.home)
        .env("CLAUDE_PLUGIN_ROOT", root)
        .env_remove("LARCH_BINARY")
        .env_remove("CLAUDE_PLUGIN_DATA")
        .stdin(Stdio::null())
        .output()
        .expect("run hook shim without binary");
    assert!(
        output.status.success(),
        "{script}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let payload: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("static deny JSON");
    assert_eq!(
        payload["hookSpecificOutput"]["permissionDecision"],
        "deny",
        "{script}"
    );
    payload["hookSpecificOutput"]["permissionDecisionReason"]
        .as_str()
        .unwrap_or_else(|| panic!("{script}: deny reason missing"))
        .to_owned()
}

#[cfg(unix)]
#[test]
fn advisory_hook_shims_fail_open_when_no_verified_binary_is_available() {
    let fixture = clean_install_fixture();
    for relative in [
        "scripts/audit-edit-write.sh",
        "scripts/cleanup-sessionstart.sh",
        "scripts/sessionstart-statusline.sh",
        "skills/implement/scripts/hook-stop-fail-close.sh",
    ] {
        let source = repo_root().join(relative);
        let destination = fixture.root.join(relative);
        fs::create_dir_all(destination.parent().expect("hook shim parent"))
            .expect("create hook shim parent");
        fs::copy(source, &destination).expect("copy advisory hook shim");
        let mut permissions = fs::metadata(&destination)
            .expect("advisory hook shim metadata")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&destination, permissions)
            .expect("make advisory hook shim executable");

        let output = Command::new("/bin/bash")
            .arg(&destination)
            .env("HOME", &fixture.home)
            .env("CLAUDE_PLUGIN_ROOT", &fixture.root)
            .env_remove("LARCH_BINARY")
            .env_remove("CLAUDE_PLUGIN_DATA")
            .stdin(Stdio::null())
            .output()
            .expect("run advisory hook shim without binary");
        assert!(
            output.status.success(),
            "{relative}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stdout.is_empty(), "{relative}");
        assert!(output.stderr.is_empty(), "{relative}");
        assert!(!fixture.root.join("bin").exists(), "{relative}");
    }
}

#[cfg(unix)]
#[test]
fn sessionstart_health_shim_keeps_only_fixed_stripped_path_fallbacks() {
    let fixture = clean_install_fixture();
    let source = repo_root().join("scripts/sessionstart-health.sh");
    let destination = fixture.root.join("scripts/sessionstart-health.sh");
    fs::copy(source, &destination).expect("copy health hook shim");
    let mut permissions = fs::metadata(&destination)
        .expect("health hook shim metadata")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&destination, permissions).expect("make health hook shim executable");

    let both_missing = Command::new("/bin/bash")
        .arg(&destination)
        .env("PATH", "")
        .env("HOME", &fixture.home)
        .env("CLAUDE_PLUGIN_ROOT", &fixture.root)
        .env_remove("LARCH_BINARY")
        .env_remove("CLAUDE_PLUGIN_DATA")
        .stdin(Stdio::null())
        .output()
        .expect("run health hook with stripped PATH");
    assert!(both_missing.status.success());
    assert!(both_missing.stderr.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&both_missing.stdout),
        "{\"hookSpecificOutput\":{\"hookEventName\":\"SessionStart\",\"additionalContext\":\"larch hook preflight: jq not on PATH and git not on PATH; install jq and git for advisory hook output.\"}}\n"
    );

    let path = fixture.root.join("health-path");
    fs::create_dir(&path).expect("create health PATH");
    let git = path.join("git");
    fs::write(&git, b"").expect("write git marker");
    let mut permissions = fs::metadata(&git).expect("git marker metadata").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&git, permissions).expect("make git marker executable");
    let jq_missing = Command::new("/bin/bash")
        .arg(&destination)
        .env("PATH", &path)
        .env("HOME", &fixture.home)
        .env("CLAUDE_PLUGIN_ROOT", &fixture.root)
        .env_remove("LARCH_BINARY")
        .env_remove("CLAUDE_PLUGIN_DATA")
        .stdin(Stdio::null())
        .output()
        .expect("run health hook with only git on PATH");
    assert!(jq_missing.status.success());
    assert!(jq_missing.stderr.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&jq_missing.stdout),
        "{\"hookSpecificOutput\":{\"hookEventName\":\"SessionStart\",\"additionalContext\":\"larch hook preflight: jq not on PATH (install jq for advisory hook output).\"}}\n"
    );

    let jq = path.join("jq");
    fs::write(&jq, b"").expect("write jq marker");
    let mut permissions = fs::metadata(&jq).expect("jq marker metadata").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&jq, permissions).expect("make jq marker executable");
    let tools_present = Command::new("/bin/bash")
        .arg(&destination)
        .env("PATH", &path)
        .env("HOME", &fixture.home)
        .env("CLAUDE_PLUGIN_ROOT", &fixture.root)
        .env_remove("LARCH_BINARY")
        .env_remove("CLAUDE_PLUGIN_DATA")
        .stdin(Stdio::null())
        .output()
        .expect("run unavailable health hook with tools on PATH");
    assert!(tools_present.status.success());
    assert!(tools_present.stdout.is_empty());
    assert!(tools_present.stderr.is_empty());
    assert!(!fixture.root.join("bin").exists());
}

#[cfg(unix)]
#[test]
fn no_install_mode_does_not_intercept_bootstrap_metadata_actions() {
    let fixture = clean_install_fixture();
    let entrypoint = fixture.root.join("scripts/larch.sh");
    let invalid_preflight = Command::new("/bin/bash")
        .arg(&entrypoint)
        .args(["--preflight-release", "not-a-version"])
        .env("LARCH_BOOTSTRAP_NO_INSTALL", "1")
        .output()
        .expect("run preflight validation");
    assert_eq!(invalid_preflight.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&invalid_preflight.stderr)
            .contains("requested release version is not a semantic version")
    );

    let bin = fixture.root.join("metadata-bin");
    fs::create_dir(&bin).expect("create metadata bin");
    let gh = bin.join("gh");
    fs::write(
        &gh,
        "#!/bin/sh\nif [ \"$1\" = api ] && [ \"$2\" = --help ]; then exit 0; fi\nif [ \"$1\" = api ]; then printf 'v9.8.7\\n'; exit 0; fi\nexit 1\n",
    )
    .expect("write gh stub");
    let mut permissions = fs::metadata(&gh).expect("gh metadata").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&gh, permissions).expect("make gh stub executable");
    let mut path_entries = vec![bin];
    path_entries.extend(env::split_paths(&env::var_os("PATH").unwrap_or_default()));
    let path = env::join_paths(path_entries).expect("metadata PATH");
    let latest = Command::new("/bin/bash")
        .arg(&entrypoint)
        .arg("--latest-stable-version")
        .env("PATH", path)
        .env("LARCH_BOOTSTRAP_NO_INSTALL", "1")
        .output()
        .expect("run latest stable lookup");
    assert!(latest.status.success(), "{}", String::from_utf8_lossy(&latest.stderr));
    assert_eq!(String::from_utf8_lossy(&latest.stdout), "LARCH_STABLE_VERSION=9.8.7\n");
}

/// Pin the public Rust-owned bootstrap envelope on both paths. The verified
/// wrapper supplies deterministic session setup without a secondary runtime.
#[cfg(unix)]
#[test]
fn bootstrap_invoke_stdout_is_pinned_for_fresh_and_resume_paths() {
    let fixture = clean_install_fixture();
    let bin = fixture.root.join("bin");
    fs::create_dir_all(&bin).expect("create fixture binary directory");
    let fixture_binary = bin.join("larch");
    fs::copy(&fixture.binary, &fixture_binary).expect("copy fixture binary");
    let mut permissions = fs::metadata(&fixture_binary)
        .expect("read fixture binary metadata")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&fixture_binary, permissions).expect("make fixture binary executable");

    let bootstrap_session = fixture.root.join("bootstrap-session");
    let fresh = run_bootstrap_invoke(&fixture, &bootstrap_session, "initial");
    assert!(
        fresh.status.success(),
        "fresh bootstrap failed: {}",
        String::from_utf8_lossy(&fresh.stderr)
    );
    let routing_target = bootstrap_session.join("routing-target.env");
    fs::write(&routing_target, "prior\n").expect("write routing target");
    let routing_file = bootstrap_session.join("bootstrap-routing.env");
    fs::remove_file(&routing_file).expect("remove fresh routing envelope");
    std::os::unix::fs::symlink(&routing_target, &routing_file).expect("symlink routing envelope");
    let resume = run_bootstrap_invoke(&fixture, &bootstrap_session, "resume");
    assert!(
        resume.status.success(),
        "resume bootstrap failed: {}",
        String::from_utf8_lossy(&resume.stderr)
    );
    assert!(
        String::from_utf8_lossy(&resume.stderr)
            .contains("refusing to overwrite symlinked bootstrap-routing.env"),
        "resume must retain a hostile routing-file target: {:?}",
        String::from_utf8_lossy(&resume.stderr)
    );
    assert_eq!(
        fs::read_to_string(&routing_target).expect("read routing target"),
        "prior\n"
    );
    let token_ledger = fs::read_dir(&bootstrap_session)
        .expect("read bootstrap session")
        .flatten()
        .map(|entry| entry.path())
        .find(|path| {
            path.file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with("larch-tokens-"))
        })
        .expect("Step 0 bootstrap token ledger");
    let token_rows = fs::read_to_string(token_ledger).expect("read Step 0 token ledger");
    assert!(
        token_rows.contains(r#""step":"Step 0 \u2014 preflight""#),
        "bootstrap must mark the preflight token boundary; rows: {token_rows:?}"
    );

    let expected = concat!(
        "IMPLEMENT_TMPDIR={SESSION}\n",
        "STALL_TRACKING=false\n",
        "REPO_UNAVAILABLE=true\n",
        "DEFERRED=true\n",
        "REPO_ROOT={REPO_ROOT}\n",
        "CODEX_BINARY_FOUND=false\n",
        "CURSOR_BINARY_FOUND=false\n",
        "codex_available=false\n",
        "cursor_available=false\n",
        "RUN_ID=bootstrap-session\n",
        "SELF_REVIEW_REQUESTED=true\n",
        "SELF_IMPLEMENT_REQUESTED=true\n",
        "BOOTSTRAP_NEXT=cleanup\n",
    );
    let normalize = |output: &[u8]| {
        String::from_utf8_lossy(output)
            .replace(&bootstrap_session.display().to_string(), "{SESSION}")
            .replace(
                &fixture.root.join("nested-repo").display().to_string(),
                "{REPO_ROOT}",
            )
    };
    assert_eq!(normalize(&fresh.stdout), expected);
    assert_eq!(normalize(&resume.stdout), expected);
}

/// Exercise the native continuation's successful plan, coder, and routing
/// path through the verified clean-install entrypoint.  The fixture is a
/// forked target so it does not mutate a real tracking issue or branch.
#[cfg(unix)]
#[test]
fn bootstrap_invoke_clean_install_runs_native_plan_coder_and_tail() {
    let fixture = clean_install_fixture();
    let bin = fixture.root.join("bin");
    fs::create_dir_all(&bin).expect("create fixture binary directory");
    let fixture_binary = bin.join("larch");
    fs::copy(&fixture.binary, &fixture_binary).expect("copy fixture binary");
    let mut permissions = fs::metadata(&fixture_binary)
        .expect("read fixture binary metadata")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&fixture_binary, permissions).expect("make fixture binary executable");

    let session = fixture.root.join("bootstrap-full-session");
    fs::create_dir_all(&session).expect("create bootstrap session");
    fs::write(session.join(".bootstrap-test-repo-available"), "")
        .expect("mark bootstrap fixture repository available");
    let preflight = fixture.root.join("bootstrap-full-preflight");
    fs::create_dir_all(&preflight).expect("create preflight directory");
    fs::write(
        preflight.join("plan-from-issue.txt"),
        concat!(
            "## Implementation Plan\n",
            "Move the continuation into Rust.\n\n",
            "## Test Plan\n",
            "- Exercise the public bootstrap command.\n\n",
            "review_status: approved\n",
            "rounds_completed: 1\n",
            "difficulty: MODERATE\n",
            "diff_lines: 1\n",
        ),
    )
    .expect("write preflight plan");

    let output = run_bootstrap_forked_invoke(&fixture, &session, &preflight);
    assert!(
        output.status.success(),
        "full bootstrap failed: {}\nevents: {}",
        String::from_utf8_lossy(&output.stderr),
        fs::read_to_string(&fixture.events).unwrap_or_default(),
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    for expected in [
        format!("IMPLEMENT_TMPDIR={}", session.display()),
        format!("PLAN_FILE={}", session.join("plan.txt").display()),
        "ISSUE_NUMBER=8358".to_owned(),
        "REPO=character-ai/larch".to_owned(),
        "REPO_UNAVAILABLE=false".to_owned(),
        "DEFERRED=true".to_owned(),
        "coder=claude".to_owned(),
        "ROUTE=continue".to_owned(),
        "CHECKPOINT_NEXT=continue".to_owned(),
        "REBASE_RC=0".to_owned(),
        "BOOTSTRAP_NEXT=step2".to_owned(),
    ] {
        assert!(
            stdout.contains(&format!("{expected}\n")),
            "stdout: {stdout}"
        );
    }
    assert_eq!(
        fs::read_to_string(session.join("plan.txt")).expect("read materialized plan"),
        concat!(
            "## Implementation Plan\n",
            "Move the continuation into Rust.\n\n",
            "## Test Plan\n",
            "- Exercise the public bootstrap command.\n\n",
            "diff_lines: 1\n",
        )
    );
    assert_eq!(
        fs::read_to_string(session.join("feature-description.txt"))
            .expect("read materialized feature description"),
        "Issue 8358 title\n\nIssue 8358 body"
    );
    assert_eq!(
        fs::read_to_string(session.join("bootstrap-routing.env"))
            .expect("read durable routing envelope"),
        stdout
    );
}

/// Exercise the non-forked Step 0 transaction through a local repository and
/// verified-entrypoint fixture. The in-process typed GitHub owner is
/// deliberately unavailable here, so publication defers and the adoption
/// sentinel must remain absent.
#[cfg(unix)]
#[test]
fn bootstrap_invoke_tracking_path_defers_unpublished_sentinel_and_activates_lease() {
    let tracking = tracking_bootstrap_fixture();
    let output =
        invoke_tracking_bootstrap(&tracking, "tracking-run-8358", "true", "true", None, false);

    assert!(
        output.status.success(),
        "tracking bootstrap failed: {}",
        String::from_utf8_lossy(&output.stderr),
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    for expected in [
        "ISSUE_NUMBER=8358",
        "RUN_ID=tracking-run-8358",
        "BRANCH_ACTION=created",
        "coder=claude",
        "ROUTE=continue",
        "DEFERRED=true",
        "BOOTSTRAP_NEXT=step2",
    ] {
        assert!(stdout.contains(expected), "stdout: {stdout}");
    }
    let branch = git_output(&tracking.repository, &["branch", "--show-current"]);
    assert_ne!(branch, "main");
    assert!(
        branch.starts_with("test-user/issue-8358-title-8358"),
        "{branch}"
    );
    assert!(
        !tracking.session.join("parent-issue.md").exists(),
        "deferred publication must not create the adoption sentinel"
    );
}

/// A dirty-tree resume with no prior coder selection must select one before
/// running the absorbed continuation tail.
#[cfg(unix)]
#[test]
fn bootstrap_resume_after_dirty_tree_selects_coder_and_routes_step2() {
    let tracking = tracking_bootstrap_fixture();
    let dirty_marker = tracking.session.join(".bootstrap-test-dirty-plan");
    fs::write(&dirty_marker, "").expect("mark plan checkpoint dirty");

    let initial =
        invoke_tracking_bootstrap(&tracking, "dirty-run-8358", "true", "true", None, false);
    assert!(
        initial.status.success(),
        "initial bootstrap failed: {}",
        String::from_utf8_lossy(&initial.stderr),
    );
    let initial_stdout = String::from_utf8_lossy(&initial.stdout);
    assert!(
        initial_stdout.contains("IMPLEMENT_BAIL_REASON=dirty-tree\n"),
        "stdout: {initial_stdout}"
    );
    assert!(
        initial_stdout.contains("BOOTSTRAP_NEXT=dirty-recovery\n"),
        "stdout: {initial_stdout}"
    );
    assert!(
        !initial_stdout.contains("coder="),
        "initial dirty-tree pass must stop before coder selection: {initial_stdout}"
    );

    fs::remove_file(dirty_marker).expect("restore clean plan checkpoint");
    let resume = invoke_tracking_bootstrap_mode(
        &tracking,
        "resume",
        "dirty-run-8358",
        "true",
        "true",
        None,
        false,
    );
    assert!(
        resume.status.success(),
        "resume bootstrap failed: {}",
        String::from_utf8_lossy(&resume.stderr),
    );
    let resume_stdout = String::from_utf8_lossy(&resume.stdout);
    for expected in ["coder=claude", "ROUTE=continue", "BOOTSTRAP_NEXT=step2"] {
        assert!(
            resume_stdout.contains(&format!("{expected}\n")),
            "stdout: {resume_stdout}"
        );
    }
}

/// Failed lease activation must preserve both captured child streams in the
/// redacted tracking diagnostic.
#[cfg(unix)]
#[test]
fn bootstrap_invoke_tracking_path_preserves_rename_failure_detail() {
    assert_tracking_child_failure_detail(
        ".bootstrap-test-rename-failure",
        "rename-failure-8358",
        "tracking-issue rename failed: FAILED=true",
        "ERROR=implementation-lease-base-mismatch",
    );
}

/// The sibling post-admission read branch must preserve captured child output
/// through the same redacted tracking diagnostic.
#[cfg(unix)]
#[test]
fn bootstrap_invoke_tracking_path_preserves_post_admission_read_failure_detail() {
    assert_tracking_child_failure_detail(
        ".bootstrap-test-post-admission-read-failure",
        "post-admission-failure-8358",
        "tracking-issue post-admission read failed: FAILED=true",
        "ERROR=post-admission-read-failed",
    );
}

#[cfg(unix)]
fn assert_tracking_child_failure_detail(
    marker: &str,
    run_id: &str,
    summary: &str,
    error: &str,
) {
    let tracking = tracking_bootstrap_fixture();
    fs::write(tracking.session.join(marker), "").expect("mark fixture child failure");

    let output = invoke_tracking_bootstrap(&tracking, run_id, "true", "true", None, false);

    assert!(
        output.status.success(),
        "tracking bootstrap failed: {}",
        String::from_utf8_lossy(&output.stderr),
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("IMPLEMENT_BAIL_REASON=tracking-init-failed\n"),
        "stdout: {stdout}"
    );
    let diagnostic = fs::read_to_string(tracking.session.join("tracking-init-failed.stderr.log"))
        .expect("read tracking failure diagnostic");
    assert!(
        diagnostic.contains(summary),
        "diagnostic: {diagnostic}"
    );
    assert!(diagnostic.contains(error), "diagnostic: {diagnostic}");
}

/// A closed issue is not adopted or mutated, but still gets a durable cleanup
/// route for the caller.
#[cfg(unix)]
#[test]
fn bootstrap_invoke_tracking_path_stops_for_closed_issue() {
    let tracking = tracking_bootstrap_fixture();
    fs::write(tracking.session.join(".bootstrap-test-issue-closed"), "")
        .expect("mark fixture issue closed");

    let output =
        invoke_tracking_bootstrap(&tracking, "closed-run-8358", "true", "true", None, false);

    assert!(
        output.status.success(),
        "closed tracking bootstrap failed: {}",
        String::from_utf8_lossy(&output.stderr),
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("IMPLEMENT_BAIL_REASON=adopted-issue-closed\n"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("BOOTSTRAP_NEXT=cleanup\n"),
        "stdout: {stdout}"
    );
    assert!(!tracking.session.join("parent-issue.md").exists());
}

/// The native continuation must stop with the documented failure when neither
/// external coder can pass the refreshed health gate.
#[cfg(unix)]
#[test]
fn bootstrap_invoke_tracking_path_stops_when_external_coders_are_unavailable() {
    let tracking = tracking_bootstrap_fixture();
    let output =
        invoke_tracking_bootstrap(&tracking, "degraded-run-8358", "true", "false", None, true);

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(2),
        "stdout: {}\nstderr: {stderr}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        stderr.contains("STEP_FAILED=degraded-both-down-hard-fail"),
        "stderr: {stderr}"
    );
    assert!(
        stderr.contains("both Codex and Cursor are unavailable after health probes"),
        "stderr: {stderr}"
    );
}

#[cfg(unix)]
struct TrackingBootstrapFixture {
    fixture: CleanInstallFixture,
    repository: PathBuf,
    session: PathBuf,
    preflight: PathBuf,
    fake_bin: PathBuf,
}

#[cfg(unix)]
fn tracking_bootstrap_fixture() -> TrackingBootstrapFixture {
    let fixture = clean_install_fixture();

    let session = fixture.root.join("bootstrap-tracking-session");
    fs::create_dir_all(&session).expect("create tracking session");
    fs::write(session.join(".bootstrap-test-repo-available"), "")
        .expect("mark bootstrap fixture repository available");
    fs::write(session.join(".bootstrap-test-tracking"), "")
        .expect("enable tracking fixture responses");

    let repository = create_bootstrap_tracking_repository(&fixture.root);
    let base_sha = git_output(&repository, &["rev-parse", "HEAD"]);
    let issue_body = tracking_issue_body(&base_sha);
    fs::write(session.join("fixture-issue-body.md"), &issue_body)
        .expect("write post-admission issue body");

    let preflight = fixture.root.join("bootstrap-tracking-preflight");
    fs::create_dir_all(&preflight).expect("create tracking preflight");
    fs::write(
        preflight.join("plan-from-issue.txt"),
        concat!(
            "## Implementation Plan\n",
            "Exercise the tracking transaction.\n\n",
            "## Test Plan\n",
            "- Run the native Step 0 path.\n\n",
            "review_status: approved\n",
            "rounds_completed: 1\n",
            "difficulty: MODERATE\n",
            "diff_lines: 1\n",
        ),
    )
    .expect("write tracking preflight plan");
    fs::write(
        preflight.join("issue.json"),
        serde_json::to_string(&serde_json::json!({
            "updatedAt": "2026-08-10T00:00:00Z",
            "body": issue_body,
            "title": "Issue 8358 title",
            "labels": [],
        }))
        .expect("serialize issue snapshot"),
    )
    .expect("write issue snapshot");

    let fake_bin = fixture.root.join("tracking-bin");
    fs::create_dir_all(&fake_bin).expect("create fake gh directory");
    let fake_gh = fake_bin.join("gh");
    write_test_executable(
        &fake_gh,
        concat!(
            "#!/bin/sh\n",
            "set -eu\n",
            "case \"${1:-}:${2:-}\" in\n",
            "  api:*dependencies/blocked_by) printf '%s\\n' '[]' ;;\n",
            "  *) printf 'unexpected gh invocation: %s\\n' \"$*\" >&2; exit 64 ;;\n",
            "esac\n",
        ),
    );
    write_test_executable(
        &fake_bin.join("python3"),
        "#!/bin/sh\nprintf '%s\\n' 'unexpected Python invocation' >&2\nexit 99\n",
    );

    TrackingBootstrapFixture {
        fixture,
        repository,
        session,
        preflight,
        fake_bin,
    }
}

#[cfg(unix)]
#[allow(clippy::too_many_arguments)]
fn invoke_tracking_bootstrap(
    tracking: &TrackingBootstrapFixture,
    run_id: &str,
    self_review_requested: &str,
    self_implement_requested: &str,
    coder: Option<&str>,
    isolate_external_coders: bool,
) -> Output {
    invoke_tracking_bootstrap_mode(
        tracking,
        "initial",
        run_id,
        self_review_requested,
        self_implement_requested,
        coder,
        isolate_external_coders,
    )
}

#[cfg(unix)]
fn invoke_tracking_bootstrap_mode(
    tracking: &TrackingBootstrapFixture,
    mode: &str,
    run_id: &str,
    self_review_requested: &str,
    self_implement_requested: &str,
    coder: Option<&str>,
    isolate_external_coders: bool,
) -> Output {
    let inherited_path = if isolate_external_coders {
        std::ffi::OsString::from("/usr/bin:/bin")
    } else {
        env::var_os("PATH").expect("test process should have PATH")
    };
    let path = env::join_paths(
        std::iter::once(tracking.fake_bin.clone()).chain(env::split_paths(&inherited_path)),
    )
    .expect("join fixture PATH");
    let mut command = Command::new(tracking.fixture.root.join("scripts/larch.sh"));
    command
        .args([
            "bootstrap",
            "invoke",
            "--mode",
            mode,
            "--issue-number",
            "8358",
            "--run-id",
        ])
        .arg(run_id)
        .args(["--preflight-tmpdir"])
        .arg(path_text(&tracking.preflight))
        .args([
            "--self-review-requested",
            self_review_requested,
            "--self-implement-requested",
            self_implement_requested,
            "--difficulty",
            "HARD",
        ]);
    if let Some(coder) = coder {
        command.args(["--coder", coder]);
    }
    command
        .current_dir(&tracking.repository)
        .env("HOME", &tracking.fixture.home)
        .env("TMPDIR", &tracking.fixture.session)
        .env("CLAUDE_PLUGIN_ROOT", &tracking.fixture.root)
        .env("LARCH_BINARY", &tracking.fixture.wrapper)
        .env("IMPLEMENT_TMPDIR", &tracking.session)
        .env("LARCH_CLAUDE_PID", "4242")
        .env("REPO_ROOT", &tracking.repository)
        .env("CLAUDE_PROJECT_DIR", &tracking.repository)
        .env("PATH", path)
        .env("LARCH_TEST_CACHE_HOME", &tracking.fixture.root)
        .env("LARCH_STATUSLINE_DISABLE", "1");
    command.output().expect("run tracking bootstrap invoke")
}

#[cfg(unix)]
fn write_test_executable(path: &Path, contents: &str) {
    fs::write(path, contents).expect("write test executable");
    let mut permissions = fs::metadata(path)
        .expect("read test executable metadata")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).expect("make test executable");
}

#[cfg(unix)]
fn create_bootstrap_tracking_repository(root: &Path) -> PathBuf {
    let origin = root.join("bootstrap-tracking-origin.git");
    let repository = root.join("bootstrap-tracking-repository");
    git_success(root, &["init", "--bare", path_text(&origin)]);
    git_success(
        root,
        &["init", "--initial-branch=main", path_text(&repository)],
    );
    git_success(
        &repository,
        &["config", "user.email", "test@example.invalid"],
    );
    git_success(&repository, &["config", "user.name", "Test User"]);
    fs::write(repository.join("README.md"), "tracking fixture\n").expect("write tracked fixture");
    git_success(&repository, &["add", "README.md"]);
    git_success(&repository, &["commit", "-m", "initial"]);
    git_success(
        &repository,
        &["remote", "add", "origin", path_text(&origin)],
    );
    git_success(&repository, &["push", "--set-upstream", "origin", "main"]);
    git_success(&repository, &["fetch", "origin", "main"]);
    repository
}

#[cfg(unix)]
fn git_success(directory: &Path, arguments: &[&str]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(directory)
        .args(arguments)
        .output()
        .expect("launch git fixture command");
    assert!(
        output.status.success(),
        "git {:?} failed: {}",
        arguments,
        String::from_utf8_lossy(&output.stderr)
    );
}

#[cfg(unix)]
fn git_output(directory: &Path, arguments: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(directory)
        .args(arguments)
        .output()
        .expect("launch git fixture query");
    assert!(
        output.status.success(),
        "git {:?} failed: {}",
        arguments,
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("git output should be UTF-8")
        .trim()
        .to_owned()
}

#[cfg(unix)]
fn tracking_issue_body(base_sha: &str) -> String {
    let plan = "## Plan\nNo shared owner changes.\n";
    let plan_sha = format!("{:x}", Sha256::digest(plan.as_bytes()));
    let empty_sha = format!("{:x}", Sha256::digest(b""));
    format!(
        "<!-- larch:plan:start -->\n{plan}<!-- larch:plan:end -->\n<!-- larch:plan-receipt v1 plan_sha256={plan_sha} base_sha={base_sha} blockers_sha256={empty_sha} owners_sha256={empty_sha} -->\n"
    )
}

/// The public command must preserve the live session named by its environment,
/// even when the age gate would otherwise remove it from the cache root.
#[cfg(unix)]
#[test]
fn cleanup_run_preserves_live_session_directory() {
    let fixture = clean_install_fixture();
    let live = fixture.home.join(".cache/larch/sessions/live-session");
    fs::create_dir_all(&live).expect("create live session");
    let old = SystemTime::now()
        .checked_sub(Duration::from_secs(2 * 86_400))
        .expect("old timestamp");
    fs::File::open(&live)
        .expect("open live session")
        .set_times(fs::FileTimes::new().set_modified(old))
        .expect("age live session");

    let output = Command::new("/bin/bash")
        .arg(fixture.root.join("scripts/larch.sh"))
        .args(["cleanup", "run"])
        .env("HOME", &fixture.home)
        .env_remove("XDG_CACHE_HOME")
        .env("TMPDIR", &fixture.session)
        .env("IMPLEMENT_TMPDIR", &live)
        .env("LARCH_CLEANUP_RETENTION_DAYS", "1")
        .env("CLAUDE_PLUGIN_ROOT", &fixture.root)
        .env("LARCH_BINARY", &fixture.wrapper)
        .env("REAL_LARCH", &fixture.binary)
        .env("CLEAN_INSTALL_EVENTS", &fixture.events)
        .env("CLEAN_INSTALL_FAILURE", "")
        .output()
        .expect("run cleanup");

    assert!(output.status.success(), "{output:?}");
    assert!(live.is_dir(), "cleanup removed a live session");
    assert!(String::from_utf8_lossy(&output.stdout).contains("CACHE_REMOVED=0"));
}

#[cfg(unix)]
#[allow(clippy::literal_string_with_formatting_args)]
#[test]
fn status_check_preserves_the_health_envelope() {
    let fixture = clean_install_fixture();
    let entrypoint = fixture.root.join("scripts/larch.sh");
    fs::write(
        &entrypoint,
        r#"#!/bin/sh
case "${1:-}:${2:-}" in
  agent:check-reviewers)
    printf '%s\n' 'CODEX_BINARY_FOUND=true' 'CURSOR_BINARY_FOUND=true' 'CODEX_PRESENT=false' 'CURSOR_PRESENT=true' 'CODEX_PROBE_DETAIL=update Codex'
    ;;
  agent:degraded-tools-gate)
    printf '%s\n' 'CODEX_STATE=probe-failed' 'CURSOR_STATE=ok' 'DEGRADED=true'
    ;;
  agent:resolve-model-pins)
    printf '%s\n' 'CURSOR_MODEL_PINS=unknown-id' 'CURSOR_MODEL_PIN_DETAIL=CURSOR_MODEL=missing' 'CODEX_MODEL_PINS=skipped' 'CODEX_MODEL_PIN_DETAIL=vendor probe not ok'
    ;;
  *) exit 1 ;;
esac
"#,
    )
    .expect("write fake larch entrypoint");
    let mut permissions = fs::metadata(&entrypoint)
        .expect("read entrypoint permissions")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&entrypoint, permissions).expect("make entrypoint executable");

    let output = Command::new(&fixture.binary)
        .arg("status")
        .env("CLAUDE_PLUGIN_ROOT", &fixture.root)
        .output()
        .expect("run status");
    let explicit = Command::new(&fixture.binary)
        .args(["status", "check"])
        .env("CLAUDE_PLUGIN_ROOT", &fixture.root)
        .output()
        .expect("run explicit status check");

    assert!(output.status.success(), "{output:?}");
    assert!(explicit.status.success(), "{explicit:?}");
    assert_eq!(output.stdout, explicit.stdout);
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        format!(
            concat!(
                "LARCH_PLUGIN_VERSION={}\n",
                "CODEX_BINARY_FOUND=true\n",
                "CURSOR_BINARY_FOUND=true\n",
                "CODEX_PRESENT=false\n",
                "CURSOR_PRESENT=true\n",
                "CODEX_STATE=probe-failed\n",
                "CURSOR_STATE=ok\n",
                "DEGRADED=true\n",
                "CODEX_PROBE_DETAIL=update Codex\n",
                "CURSOR_MODEL_PINS=unknown-id\n",
                "CURSOR_MODEL_PIN_DETAIL=CURSOR_MODEL=missing\n",
                "CODEX_MODEL_PINS=skipped\n",
                "CODEX_MODEL_PIN_DETAIL=vendor probe not ok\n",
            ),
            env!("CARGO_PKG_VERSION"),
        )
    );
}

#[cfg(unix)]
fn run_bootstrap_invoke(
    fixture: &CleanInstallFixture,
    session: &Path,
    mode: &str,
) -> std::process::Output {
    let session_hint = fixture.root.join(".bootstrap-test-session");
    fs::write(&session_hint, format!("{}\n", session.display()))
        .expect("write bootstrap session hint");
    let repo_root = if mode == "resume" {
        fixture.root.join("unexpected-resume-root")
    } else {
        fixture.root.join("nested-repo")
    };
    let mut command = Command::new("/bin/bash");
    command
        .arg(fixture.root.join("scripts/larch.sh"))
        .args([
            "bootstrap",
            "invoke",
            "--mode",
            mode,
            "--self-review-requested",
            "true",
            "--self-implement-requested",
            "true",
        ])
        .env("HOME", &fixture.home)
        .env("TMPDIR", &fixture.session)
        .env("CLAUDE_PLUGIN_ROOT", &fixture.root)
        .env("LARCH_BINARY", &fixture.wrapper)
        .env("REAL_LARCH", &fixture.binary)
        .env("CLEAN_INSTALL_EVENTS", &fixture.events)
        .env("CLEAN_INSTALL_FAILURE", "")
        .env("LARCH_CLAUDE_PID", "4242")
        .env("LARCH_TEST_CACHE_HOME", &fixture.root)
        .env("XDG_CACHE_HOME", fixture.root.join("nested-cache"))
        .env("REPO_ROOT", repo_root)
        .env("LARCH_STATUSLINE_DISABLE", "1")
        .env("CLAUDE_PLUGIN_OPTION_CODEX_EFFORT", "medium")
        .env("CLAUDE_PLUGIN_OPTION_CODEX_MODEL", "plugin-codex")
        .env("CLAUDE_PLUGIN_OPTION_CURSOR_MODEL", "plugin-cursor")
        .env("LARCH_CODEX_EFFORT", "high")
        .env("LARCH_CODEX_FIX_MODEL", "fix-codex")
        .env("LARCH_CODEX_MODEL", "impl-codex")
        .env("LARCH_CODEX_REVIEW_MODEL", "review-codex")
        .env("LARCH_CODEX_VOTE_MODEL", "vote-codex")
        .env("LARCH_CURSOR_MODEL", "cursor-model")
        .env("LARCH_EXTERNAL_AUTH_RETRIES", "2")
        .env("LARCH_EXTERNAL_HEALTH_CHECK_TIMEOUT", "17")
        .env("LARCH_PROBE_NEGATIVE_TTL_SECONDS", "3")
        .env("LARCH_PROBE_RETRIES", "4")
        .env("LARCH_PROBE_TIMEOUT_RETRIES", "5")
        .env("LARCH_PROBE_TIMEOUT_SECONDS", "6")
        .env("LARCH_PROBE_TTL_SECONDS", "7")
        .env("IMPLEMENT_TMPDIR", session);
    let output = command.output().expect("run bootstrap invoke");
    fs::remove_file(session_hint).expect("remove bootstrap session hint");
    output
}

#[cfg(unix)]
fn run_bootstrap_forked_invoke(
    fixture: &CleanInstallFixture,
    session: &Path,
    preflight: &Path,
) -> std::process::Output {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("canonical test repository root");
    Command::new("/bin/bash")
        .arg(fixture.root.join("scripts/larch.sh"))
        .args([
            "bootstrap",
            "invoke",
            "--mode",
            "initial",
            "--issue-number",
            "8358",
            "--forked-target",
            "true",
            "--upstream-repo",
            "character-ai/larch",
            "--preflight-tmpdir",
            &preflight.display().to_string(),
            "--self-review-requested",
            "true",
            "--self-implement-requested",
            "true",
        ])
        .current_dir(&fixture.root)
        .env("HOME", &fixture.home)
        .env("TMPDIR", &fixture.session)
        .env("CLAUDE_PLUGIN_ROOT", &fixture.root)
        .env("CLAUDE_PROJECT_DIR", repo_root)
        .env("LARCH_BINARY", &fixture.wrapper)
        .env("REAL_LARCH", &fixture.binary)
        .env("CLEAN_INSTALL_EVENTS", &fixture.events)
        .env("CLEAN_INSTALL_FAILURE", "")
        .env("BOOTSTRAP_TEST_SESSION", session)
        .env("IMPLEMENT_TMPDIR", session)
        .env("LARCH_CLAUDE_PID", "4242")
        .env("LARCH_TEST_CACHE_HOME", &fixture.root)
        .output()
        .expect("run forked bootstrap invoke")
}

/// Argument placeholder each clean-install case expands to the seeded session.
const CLEAN_INSTALL_SESSION_TOKEN: &str = "%SESSION%";
/// Argument placeholder each clean-install case expands to the isolated home.
const CLEAN_INSTALL_HOME_TOKEN: &str = "%HOME%";

struct CleanInstallFixture {
    _temporary: TempDir,
    root: PathBuf,
    /// Isolated home, so a verb that publishes a PID-keyed pointer stays contained.
    home: PathBuf,
    /// Seeded session directory every writer verb targets.
    session: PathBuf,
    wrapper: PathBuf,
    events: PathBuf,
    binary: PathBuf,
}

#[allow(clippy::literal_string_with_formatting_args, clippy::too_many_lines)]
fn clean_install_fixture() -> CleanInstallFixture {
    let temporary = tempfile::tempdir().expect("clean-install tempdir");
    let temporary_root = fs::canonicalize(temporary.path()).expect("canonical clean-install root");
    let root = temporary_root.join("plugin");
    let scripts = root.join("scripts");
    let manifest_directory = root.join(".claude-plugin");
    fs::create_dir_all(&scripts).expect("create clean-install scripts directory");
    fs::create_dir_all(&manifest_directory).expect("create clean-install manifest directory");
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/larch.sh"),
        scripts.join("larch.sh"),
    )
    .expect("copy verified bootstrap script");
    #[cfg(unix)]
    {
        let script = scripts.join("larch.sh");
        let mut permissions = fs::metadata(&script)
            .expect("read clean-install script metadata")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(script, permissions).expect("make clean-install script executable");
    }
    fs::write(
        manifest_directory.join("plugin.json"),
        format!(
            "{{\n  \"name\": \"larch\",\n  \"version\": \"{}\"\n}}\n",
            env!("CARGO_PKG_VERSION")
        ),
    )
    .expect("write clean-install plugin manifest");
    seed_clean_install_stall_recovery_contract(&root);
    let wrapper = temporary_root.join("verified-larch");
    let wrapper_source = r#"#!/bin/sh
set -eu
if [ -n "${CLEAN_INSTALL_EVENTS:-}" ]; then
  printf '%s\n' "$*" >> "$CLEAN_INSTALL_EVENTS"
fi
bootstrap_session=${BOOTSTRAP_TEST_SESSION:-${IMPLEMENT_TMPDIR:-}}
if [ -z "$bootstrap_session" ] \
  && [ -n "${CLAUDE_PLUGIN_ROOT:-}" ] \
  && [ -r "$CLAUDE_PLUGIN_ROOT/.bootstrap-test-session" ]; then
  IFS= read -r bootstrap_session < "$CLAUDE_PLUGIN_ROOT/.bootstrap-test-session"
fi
bootstrap_repo_available=false
if [ "${BOOTSTRAP_TEST_REPO_AVAILABLE:-}" = true ] \
  || { [ -n "$bootstrap_session" ] && [ -f "$bootstrap_session/.bootstrap-test-repo-available" ]; }; then
  bootstrap_repo_available=true
fi
bootstrap_tracking=false
if [ -n "$bootstrap_session" ] && [ -f "$bootstrap_session/.bootstrap-test-tracking" ]; then
  bootstrap_tracking=true
fi
if [ -n "$bootstrap_session" ]; then
  case "${1:-}:${2:-}" in
    git:current-branch)
      printf '%s\n' 'BRANCH=bootstrap-fixture'
      exit 0
      ;;
    session:setup)
      if [ "$bootstrap_repo_available" != true ]; then
        [ "${LARCH_CLAUDE_PID:-}" = 4242 ] || exit 78
        [ "${XDG_CACHE_HOME:-}" = "$CLAUDE_PLUGIN_ROOT/nested-cache" ] || exit 78
        [ "${REPO_ROOT:-}" = "$CLAUDE_PLUGIN_ROOT/nested-repo" ] || exit 78
        [ "${LARCH_STATUSLINE_DISABLE:-}" = 1 ] || exit 78
        [ "${CLAUDE_PLUGIN_OPTION_CODEX_EFFORT:-}" = medium ] || exit 78
        [ "${CLAUDE_PLUGIN_OPTION_CODEX_MODEL:-}" = plugin-codex ] || exit 78
        [ "${CLAUDE_PLUGIN_OPTION_CURSOR_MODEL:-}" = plugin-cursor ] || exit 78
        [ "${LARCH_CODEX_EFFORT:-}" = high ] || exit 78
        [ "${LARCH_CODEX_FIX_MODEL:-}" = fix-codex ] || exit 78
        [ "${LARCH_CODEX_MODEL:-}" = impl-codex ] || exit 78
        [ "${LARCH_CODEX_REVIEW_MODEL:-}" = review-codex ] || exit 78
        [ "${LARCH_CODEX_VOTE_MODEL:-}" = vote-codex ] || exit 78
        [ "${LARCH_CURSOR_MODEL:-}" = cursor-model ] || exit 78
        [ "${LARCH_EXTERNAL_AUTH_RETRIES:-}" = 2 ] || exit 78
        [ "${LARCH_EXTERNAL_HEALTH_CHECK_TIMEOUT:-}" = 17 ] || exit 78
        [ "${LARCH_PROBE_NEGATIVE_TTL_SECONDS:-}" = 3 ] || exit 78
        [ "${LARCH_PROBE_RETRIES:-}" = 4 ] || exit 78
        [ "${LARCH_PROBE_TIMEOUT_RETRIES:-}" = 5 ] || exit 78
        [ "${LARCH_PROBE_TIMEOUT_SECONDS:-}" = 6 ] || exit 78
        [ "${LARCH_PROBE_TTL_SECONDS:-}" = 7 ] || exit 78
      fi
      mkdir -p "$bootstrap_session"
      printf '%s\n' 'bootstrap-session' > "$bootstrap_session/session-id"
      if [ "$bootstrap_repo_available" = true ]; then
        printf '%s\n' \
          "SESSION_TMPDIR=$bootstrap_session" \
          'SESSION_ID=bootstrap-session' \
          'REPO=character-ai/larch' \
          'REPO_UNAVAILABLE=false' \
          'CLAUDE_BINARY_FOUND=false' \
          'CODEX_BINARY_FOUND=false' \
          'CURSOR_BINARY_FOUND=false'
      else
        printf '%s\n' \
          "SESSION_TMPDIR=$bootstrap_session" \
          'SESSION_ID=bootstrap-session' \
          'REPO=' \
          'REPO_UNAVAILABLE=true' \
          'CLAUDE_BINARY_FOUND=false' \
          'CODEX_BINARY_FOUND=false' \
          'CURSOR_BINARY_FOUND=false'
      fi
      exit 0
      ;;
    issue:context)
      if [ "$bootstrap_repo_available" = true ]; then
        printf '%s' 'Issue 8358 title' > "$bootstrap_session/upstream-issue-title.txt"
        printf '%s' 'Issue 8358 body' > "$bootstrap_session/upstream-issue-body.txt"
        printf '%s\n' \
          "TITLE_FILE=$bootstrap_session/upstream-issue-title.txt" \
          "BODY_FILE=$bootstrap_session/upstream-issue-body.txt"
        exit 0
      fi
      ;;
    issue:state)
      if [ "$bootstrap_tracking" = true ]; then
        if [ -f "$bootstrap_session/.bootstrap-test-issue-closed" ]; then
          printf '%s\n' 'STATE=CLOSED' 'IS_PR=false'
        else
          printf '%s\n' 'STATE=OPEN' 'IS_PR=false'
        fi
        exit 0
      fi
      ;;
    issue:governance-gate)
      if [ "$bootstrap_tracking" = true ]; then
        printf '%s\n' 'GOVERNANCE_OK=true'
        exit 0
      fi
      ;;
    dirty-tree:checkpoint)
      if [ "$bootstrap_repo_available" = true ]; then
        if [ -f "$bootstrap_session/.bootstrap-test-dirty-plan" ]; then
          if [ -f "$bootstrap_session/.bootstrap-test-dirty-check-seen" ]; then
            printf '%s\n' 'STATUS=dirty'
          else
            : > "$bootstrap_session/.bootstrap-test-dirty-check-seen"
            printf '%s\n' 'STATUS=clean'
          fi
        else
          printf '%s\n' 'STATUS=clean'
        fi
        exit 0
      fi
      ;;
    push:checkpoint-probe)
      if [ "$bootstrap_repo_available" = true ]; then
        printf '%s\n' 'ROUTE=continue' 'CHECKPOINT_NEXT=continue' 'REBASE_OUTCOME=clean'
        exit 0
      fi
      ;;
    session:persist-run-flags)
      if [ "$bootstrap_tracking" = true ]; then
        printf '%s\n' 'DIFFICULTY_OVERRIDE=HARD' > "$bootstrap_session/run-flags.sh"
        exit 0
      fi
      ;;
    tracking-issue:rename)
      if [ "$bootstrap_tracking" = true ]; then
        if [ -f "$bootstrap_session/.bootstrap-test-rename-failure" ]; then
          printf '%s\n' 'FAILED=true'
          printf '%s\n' 'ERROR=implementation-lease-base-mismatch' >&2
          exit 64
        fi
        exit 0
      fi
      ;;
    tracking-issue:read)
      if [ "$bootstrap_tracking" = true ]; then
        if [ -f "$bootstrap_session/.bootstrap-test-post-admission-read-failure" ]; then
          printf '%s\n' 'FAILED=true'
          printf '%s\n' 'ERROR=post-admission-read-failed' >&2
          exit 64
        fi
        body_out=''
        shift 2
        while [ "$#" -gt 0 ]; do
          if [ "$1" = '--body-out' ]; then
            body_out="${2:-}"
            break
          fi
          shift
        done
        [ -n "$body_out" ] && [ -f "$body_out" ] && [ -f "$bootstrap_session/fixture-issue-body.md" ] || exit 64
        cp "$bootstrap_session/fixture-issue-body.md" "$body_out"
        exit 0
      fi
      ;;
    progress:install-statusline)
      if [ "$bootstrap_tracking" = true ]; then
        exit 0
      fi
      ;;
    run-log:init|run-log:write|run-log:append-failure|run-log:append-entry|run-log:manifest|tracking-issue:upsert-summary)
      if [ "$bootstrap_repo_available" = true ]; then
        exit 0
      fi
      ;;
  esac
fi
case "${CLEAN_INSTALL_FAILURE:-}" in
  version)
    if [ "$1" = --version ]; then printf '%s\n' 'larch 0.0.0'; exit 0; fi
    ;;
  target)
    if [ "$1" = bootstrap ]; then
      if [ "$2" = self-check ]; then
        "$REAL_LARCH" "$@" | sed 's/"target":"[^"]*"/"target":"wrong-target"/'
        exit "$?"
      fi
    fi
    ;;
  bootstrap)
    if [ "$1" = bootstrap ]; then
      if [ "$2" = self-check ]; then exit 9; fi
    fi
    ;;
esac
if [ -n "${REAL_LARCH:-}" ]; then
  real_larch=$REAL_LARCH
else
  real_larch=__REAL_LARCH__
fi
exec "$real_larch" "$@"
"#;
    let real_larch = shell_quote(path_text(Path::new(env!("CARGO_BIN_EXE_larch"))));
    fs::write(
        &wrapper,
        wrapper_source.replace("__REAL_LARCH__", &real_larch),
    )
    .expect("write verified binary wrapper");
    #[cfg(unix)]
    {
        let mut permissions = fs::metadata(&wrapper)
            .expect("read wrapper metadata")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&wrapper, permissions).expect("make wrapper executable");
    }
    let home = temporary_root.join("home");
    let session = temporary_root.join("session");
    let sessions_cache = home.join(".cache/larch/sessions");
    fs::create_dir_all(&sessions_cache).expect("create clean-install home");
    fs::create_dir_all(&session).expect("create clean-install session directory");
    // `restore-finalize-state` reports a missing durable state file as a warning
    // exit, so the seeded state is what lets a clean dispatch complete.
    fs::write(
        session.join("ship-pr-state.sh"),
        "BRANCH_NAME=clean-install\n",
    )
    .expect("seed clean-install ship-pr state");
    // `resolve-trusted-design-env` resolves an existing pointer or exits 1, so the
    // seeded link and target are what let a clean dispatch complete.
    fs::write(
        session.join("design-env.sh"),
        format!(
            "DESIGN_TMPDIR={}\nexport SESSION_ID=clean-install\n",
            session.display()
        ),
    )
    .expect("seed clean-install design env");
    #[cfg(unix)]
    std::os::unix::fs::symlink(
        session.join("design-env.sh"),
        sessions_cache.join("current-design-env-4242.sh"),
    )
    .expect("seed clean-install design pointer");
    seed_clean_install_run_log_inputs(&root, &session);
    CleanInstallFixture {
        events: temporary_root.join("events.log"),
        binary: PathBuf::from(env!("CARGO_BIN_EXE_larch")),
        _temporary: temporary,
        root,
        home,
        session,
        wrapper,
    }
}

/// Copy the contract that the installed Rust-owned lint command reads.
fn seed_clean_install_stall_recovery_contract(root: &Path) {
    let source_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for relative in [
        "docs/stall-recovery-report.md",
        "docs/stall-recovery-report-allowlists.tsv",
    ] {
        let destination = root.join(relative);
        fs::create_dir_all(
            destination
                .parent()
                .expect("stall-recovery contract parent"),
        )
        .expect("create stall-recovery contract parent");
        fs::copy(source_root.join(relative), destination).expect("copy stall-recovery contract");
    }
}

/// Seed the payloads, quiet log, round source, run directory, and required-files
/// manifest the `run-log` entry-write and breadcrumb verbs read on a clean install.
fn seed_clean_install_run_log_inputs(root: &Path, session: &Path) {
    fs::write(session.join("payload.md"), "clean-install payload\n")
        .expect("seed clean-install batch payload");
    fs::write(
        session.join("larch-quiet-clean-install.sh-1.log"),
        "clean-install breadcrumb\n",
    )
    .expect("seed clean-install quiet log");
    fs::write(session.join("record.ndjson"), "{\"clean\":\"install\"}\n")
        .expect("seed clean-install append record");
    let round_source = session.join("round-src");
    fs::create_dir_all(&round_source).expect("create clean-install round source");
    fs::write(round_source.join("coder-prompt.md"), "prompt\n")
        .expect("seed clean-install round artifact");
    let run_directory = session.join("verify-run");
    fs::create_dir_all(&run_directory).expect("create clean-install verify run directory");
    fs::write(
        run_directory.join("manifest.json"),
        "{\"schema_version\":2,\"status\":\"merged\",\"run_id\":\"clean-install\",\"steps_ran\":{}}\n",
    )
    .expect("seed clean-install verify manifest");
    let documents = root.join("docs");
    fs::create_dir_all(&documents).expect("create clean-install docs directory");
    fs::write(
        documents.join("run-logs-required-files.tsv"),
        "relative_path\tcondition\nmanifest.json\talways\n",
    )
    .expect("seed clean-install required-files manifest");
}

/// Expand one case's static arguments against the fixture's seeded session.
fn clean_install_arguments(fixture: &CleanInstallFixture, case: CleanInstallCase) -> Vec<String> {
    let session = fixture.session.to_string_lossy().into_owned();
    let home = fixture.home.to_string_lossy().into_owned();
    case.arguments()
        .iter()
        .map(|argument| {
            argument
                .replace(CLEAN_INSTALL_SESSION_TOKEN, &session)
                .replace(CLEAN_INSTALL_HOME_TOKEN, &home)
        })
        .collect()
}

/// Render the argv line the verified bootstrap wrapper records for one case.
///
/// An argument-free verb records only its domain and verb, with no trailing
/// separator, because the wrapper logs the shell's joined argument list.
fn clean_install_dispatch(fixture: &CleanInstallFixture, case: CleanInstallCase) -> String {
    std::iter::once(case.domain.to_owned())
        .chain(std::iter::once(case.verb.to_owned()))
        .chain(clean_install_arguments(fixture, case))
        .collect::<Vec<_>>()
        .join(" ")
}

fn run_clean_install_case(
    fixture: &CleanInstallFixture,
    case: CleanInstallCase,
    failure: Option<&str>,
) -> std::process::Output {
    let manifest_root = if case.id == "clean-install-run-log-manifest" {
        let path = fixture
            .root
            .join("manifest-logs/clean/clean-install/manifest.json");
        fs::create_dir_all(path.parent().expect("manifest parent"))
            .expect("create clean-install manifest parent");
        fs::write(
            &path,
            "{\"schema_version\":2,\"status\":\"partial\",\"run_id\":\"clean-install\",\"steps_ran\":{}}\n",
        )
        .expect("write clean-install manifest");
        Some(fixture.root.as_path())
    } else {
        None
    };
    let mut command = Command::new("/bin/bash");
    command
        .arg(fixture.root.join("scripts/larch.sh"))
        .args([case.domain, case.verb])
        .args(clean_install_arguments(fixture, case))
        .env("HOME", &fixture.home)
        .env("TMPDIR", &fixture.session)
        .env_remove("XDG_CACHE_HOME")
        .env("CLAUDE_PLUGIN_ROOT", &fixture.root)
        .env("LARCH_BINARY", &fixture.wrapper)
        .env("REAL_LARCH", &fixture.binary)
        .env("CLEAN_INSTALL_EVENTS", &fixture.events)
        // Progress verbs write clone-scoped cache state; confine it to the fixture.
        .env("LARCH_TEST_CACHE_HOME", &fixture.root)
        .env("CLEAN_INSTALL_FAILURE", failure.unwrap_or_default());
    if let Some(root) = manifest_root {
        command.env("IMPLEMENT_TMPDIR", root);
    }
    command.output().expect("run clean-install selector")
}

// ---------------------------------------------------------------------------
// eval validate-research-output / eval research (leaf #8500)
// ---------------------------------------------------------------------------

fn path_text(path: &Path) -> &str {
    path.to_str().expect("fixture path should be UTF-8")
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("repo root")
        .to_path_buf()
}

fn eval_command(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_larch"))
        .args(arguments)
        .output()
        .expect("run eval command")
}

fn exit_code(output: &Output) -> i32 {
    output.status.code().expect("exit code")
}

#[test]
fn eval_validate_research_output_exit_code_matrix() {
    let dir = TempDir::new().expect("tempdir");

    // Missing file -> 4.
    let missing = dir.path().join("nope.md");
    let output = eval_command(&[
        "eval",
        "validate-research-output",
        missing.to_str().expect("path"),
    ]);
    assert_eq!(exit_code(&output), 4, "missing file must exit 4");

    // Thin body -> 2.
    let thin = dir.path().join("thin.md");
    fs::write(&thin, "one two three\n").expect("write");
    assert_eq!(
        exit_code(&eval_command(&[
            "eval",
            "validate-research-output",
            thin.to_str().expect("path"),
        ])),
        2,
    );

    let body: String = std::iter::repeat_n("word", 250)
        .collect::<Vec<_>>()
        .join(" ");

    // Enough words, no provenance -> 3.
    let no_prov = dir.path().join("noprov.md");
    fs::write(&no_prov, format!("{body}\n")).expect("write");
    assert_eq!(
        exit_code(&eval_command(&[
            "eval",
            "validate-research-output",
            no_prov.to_str().expect("path"),
        ])),
        3,
    );

    // Provenance present -> 0.
    let with_prov = dir.path().join("prov.md");
    fs::write(&with_prov, format!("{body} https://example.com/x\n")).expect("write");
    assert_eq!(
        exit_code(&eval_command(&[
            "eval",
            "validate-research-output",
            with_prov.to_str().expect("path"),
        ])),
        0,
    );

    // Structured reviewer mode -> exit 5 when nothing normalizes.
    let junk = dir.path().join("junk.md");
    fs::write(&junk, "not structured at all\n").expect("write");
    assert_eq!(
        exit_code(&eval_command(&[
            "eval",
            "validate-research-output",
            "--structured-reviewer-mode",
            junk.to_str().expect("path"),
        ])),
        5,
    );

    // -h prints usage and exits 0.
    let help = eval_command(&["eval", "validate-research-output", "-h"]);
    assert_eq!(exit_code(&help), 0);
    assert!(String::from_utf8_lossy(&help.stdout).contains("Usage: validate-research-output"));
}

#[test]
fn eval_validate_research_output_validation_mode_accepts_sentinel() {
    let dir = TempDir::new().expect("tempdir");
    let sentinel = dir.path().join("s.md");
    fs::write(&sentinel, "NO_ISSUES_FOUND\n").expect("write");
    assert_eq!(
        exit_code(&eval_command(&[
            "eval",
            "validate-research-output",
            "--validation-mode",
            sentinel.to_str().expect("path"),
        ])),
        0,
    );
}

#[test]
fn eval_validate_research_output_writes_normalized_wire_file() {
    let dir = TempDir::new().expect("tempdir");
    let reviewer = dir.path().join("reviewer.tsv");
    let header = "schema_version\tscope\tseverity\tfocus_area\tlocation\twhat\tscenario_or_breakage\tsuggested_fix";
    fs::write(
        &reviewer,
        format!("{header}\n1\tin_scope\tMAJOR\tcompleteness\tsrc/a.rs:1\twhat\tscenario\tfix\n"),
    )
    .expect("write");
    let wire = dir.path().join("out.tsv");
    let output = eval_command(&[
        "eval",
        "validate-research-output",
        "--structured-reviewer-mode",
        "--write-structured",
        wire.to_str().expect("path"),
        reviewer.to_str().expect("path"),
    ]);
    assert_eq!(exit_code(&output), 0);
    // Column 1 normalizes to "1", severity lowercases, focus canonicalizes.
    assert_eq!(
        fs::read_to_string(&wire).expect("wire"),
        format!("{header}\n1\tin_scope\tmajor\tcode-quality\tsrc/a.rs:1\twhat\tscenario\tfix\n"),
    );
}

#[test]
fn eval_research_smoke_test_reports_pass() {
    let output = Command::new(env!("CARGO_BIN_EXE_larch"))
        .args(["eval", "research", "--smoke-test"])
        .env("CLAUDE_PLUGIN_ROOT", repo_root())
        .output()
        .expect("run eval research");
    assert_eq!(
        exit_code(&output),
        0,
        "smoke test stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("smoke test PASS"),
        "stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn eval_research_rejects_bad_timeout() {
    let output = eval_command(&["eval", "research", "--timeout", "0"]);
    assert_eq!(exit_code(&output), 2);
}

#[test]
fn eval_research_help_exits_zero() {
    let output = eval_command(&["eval", "research", "--help"]);
    assert_eq!(exit_code(&output), 0);
    assert!(String::from_utf8_lossy(&output.stdout).contains("Usage: eval research"));
}

#[test]
fn eval_research_missing_claude_reports_exit_three() {
    let output = Command::new(env!("CARGO_BIN_EXE_larch"))
        .args(["eval", "research"])
        .env("PATH", "")
        .env("CLAUDE_PLUGIN_ROOT", repo_root())
        .output()
        .expect("run eval research");
    assert_eq!(exit_code(&output), 3);
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("required tool missing: claude"),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
