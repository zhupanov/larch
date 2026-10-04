# GitHub Service Inventory

This inventory records every GitHub service operation, its adapter method, its
current command owner, and the issues that established the boundary. Historical
completion columns remain in the linted matrix for audit continuity. Exact
command and implementation-leaf ownership lives in the command registry.

## Checked scope

The scan covers production Rust, skills, agents, hooks, scripts, and CI
configuration. It excludes documentation, fixtures, historical run logs, and the
release-generated `plugin/` projection, which is absent from `main`.
`service-ownership` in `crates/larch-lint` mechanically holds the boundary this
inventory records: concrete clients, service request surfaces, and `gcloud`
stay inside `crates/larch-adapters`.
The #7843 refresh ran after the Actions-log, pull-request merge,
issue-dependency, and credential-contract repairs. It explicitly covered `gh`,
`gh api`, `gh auth token --hostname github.com`, GitHub and Google service
hosts, GraphQL documents, concrete clients, `gcloud`, and service-credential
propagation.

## Concrete client owner

`crates/larch-core/src/github_auth.rs` owns the single typed
`gh auth token --hostname github.com` credential lookup.
`crates/larch-adapters/src/github/mod.rs` is the single concrete GitHub client
owner. `OctocrabGitHubService::from_gh` builds the one private Octocrab client
from the core-owned result and pins the
`api.github.com` and `github.com` host allowlist. It verifies that pinned
Octocrab supplies one API version header. Other adapters layer typed operations
over that client and hide REST URLs, GraphQL documents, and the client. Only
`crates/larch-adapters` imports Octocrab or names GitHub hosts and GraphQL.

## Adapter operation ownership

The tab-separated matrix below is the linted ownership contract. Operation
groups are unique. Adapter paths must exist, planning owners must be concrete
roadmap or completed-leaf issues, and every listed command must match its final
recorded owner and completed historical milestones. A command may
appear in more than one row when it consumes several typed adapter operations.
Issue-dependency adapter parity landed in #7841, and sub-issue adapter parity
in #8164. The `issue-reads` row records the #7682 command cutovers: the
issue query verbs moved to Rust in #8167, and `issue list-issues` plus
`issue fetch-issue-details` followed in #8168. The shared typed list operation
returns a bounded result that separates returned issue rows from raw REST rows
scanned and reports truncation, so each caller declares exhaustive or
bounded-partial intent rather than treating every page-bound refusal as a
transport failure; the contract is canonical in
[`supply-chain-credentials-and-services.md`](security/supply-chain-credentials-and-services.md). The `issue-creation` row records
the #8169 cutover of `issue create-one`, `issue write-sentinel`, and
`issue cleanup-failed`, plus the #8946 `issue create-batch` composition owner;
its writes run through the shared issue-mutation owner,
and `write-sentinel` is grouped with them because it is the receipt a completed
filing run publishes, not because it reaches GitHub. The `issue-dependencies`,
`issue-sub-issues`, and `label-dependency-mutations` rows record the #8170
cutover of `issue add-blocked-by`, `issue add-sub-issue`, and both
`/block-issue` dependency mutations; all four now drive the typed issue-graph
adapter operations rather than raw `gh api` REST and GraphQL calls, which is
why they no longer appear in the `comments`, `issues`, and `labels` rows. The
`issue-body-blocks` row records the #8171 cutover of `named-block write`,
`plan-block read`, and `plan-block write`: the two writers drive the shared
issue-mutation owner and the reader drives the typed issue read, so the
`/design` to `/implement` plan handoff no longer reaches GitHub through raw
`gh` calls. The same leaf moved `issue insert-signal-marker`,
`issue title-archival-jq`, and `issue title-eligibility` to Rust; they reach no
GitHub service at all, so they left the `comments`, `issues`, and `labels` rows
without joining another. The `umbrella-conversion` row records the #8174
cutover of `umbrella mutate`, the one live write `/umbrella` performs: it
drives the shared issue-mutation owner's field-scoped compare-and-swap. Its
managed conversion and record-less umbrella adoption modes each have a
shape-restricted field that the owner validates. The same leaf moved `umbrella verify` and
`umbrella verify-completion` to Rust; both prove a completed run entirely from
recorded artifacts and reach no GitHub service, so neither joins a row.

Issue #8932 added `issue search-implementing` to the `issue-reads` row. It uses
the typed, bounded issue search as a recall filter, then applies the exact path
and lifecycle-title predicates locally before returning a match.

The `design-issue-read` row records the #8578 cutover of `design step0-route`.
Following the #7672 canonical GitHub-service result, its Step 0b issue read moved
from the retired Python `gh issue view` to the typed `OctocrabGitHubService`
adapter (`OctocrabGitHubOperations::issue_read`), which returns one bounded,
redacted title, body, and label-name set. The other eight leaf #8578 verbs
(`step0-parse`, `step0-session`, `step0-init`, `step0-clarify-hard-halt`,
`step0-abort-cleanup`, `step0-ap-continue`, `step0c`, `settle-next-action`)
reach no GitHub service, so they left the `pull-requests` row without joining
another.

The `labels` and `design-oos-label-mutations` rows record the #8590 cutover of
`design file-oos-annotate`. Following the #7672 canonical result, label
provisioning uses the typed label service and issue-label application uses the
shared mutation owner with read-back. `design file-oos-prepare` reaches no
GitHub service. Both commands left the broad pending `pull-requests` row.

The three `tracking-issue-*` rows record the corrected atomic cutover in #8346
of the six tracking-issue lifecycle verbs. `tracking-issue-comment-reads`
covers the three verbs that list comments: `read` renders the issue and its
human comments into an untrusted-input task file, `append-comment` checks for
an idempotent replay, and `upsert-summary` resolves the comment its marker owns.
`tracking-issue-comment-mutations` covers the latter two verbs' verified
comment creation, replacement, and deletion.
`tracking-issue-lifecycle` covers the three that change issue identity:
`create-issue` files one through the mutation owner's redacting create, and
`rename` and `mark-false-positive` apply a title as a freshness-checked
compare-and-swap. `rename --run-id` and the `upsert-summary` lease heartbeat
also refresh the implementation lease, which the same owner binds to the run
that already holds it. Lease initialization binds the preflight title, body,
and admission-relevant label hashes, timestamp lower bound, base-target SHA,
plan receipt, active title, and lease body in one mutation. A metadata comment
may advance `updatedAt`, but the command admits that drift only while those
preflight issue fields remain exact.
Issue #8789 moved `tracking post-issue` to Rust. It composes the implementation
metadata locally, then calls the same in-process marker-keyed upsert owner. It
therefore shares the bounded comment read, mutation, redaction, read-back, and
lease-refresh contracts instead of introducing another GitHub client path.
Issue #8837 moved `diagrams upsert` to Rust. It uses the typed comment list and
the same marker-keyed mutation owner, including authorization and exact
read-back, so it also shares the comment read and mutation rows.
Comment create and edit operations use the same mutation owner and verify both
their mutation echo and a same-surface comment-list read-back; deletion verifies
absence from that list. Issue creation verifies its response with an exact
same-issue GET and names an unverified orphan for best-effort closure. External
command consumers enter through `scripts/larch.sh`; `final-report write` calls
the same Rust tracking owner in process so its own output envelope stays
unpolluted.

The three `issue-backlog-*` rows record the #8183 cutover of
`analyze-issues fetch` and `analyze-issues run`. They read bounded issue and
comment DTOs through the typed REST adapter, while the fixed closure-reference
GraphQL operation stays inside the operations adapter. The offline `analyze`
verb reads only its supplied snapshot and therefore has no GitHub-service row.

`audit-report-issues` records #8189's bounded audit advisory and prior-report
closure cutover. The advisory is read-only; prior closure uses the shared typed
issue-mutation owner for authorization and close read-back.

The #8673 cutover completed the audit-runs GitHub surface with five Rust-born
verbs in `crates/larch-cli/src/audit_runs_commands.rs`: `issue-search`
(typed issue search with local audit-title exclusion), `fix-merge` (typed
issue read plus the bounded merged-main history from the operations adapter),
`label-check` (typed label list with an exact local match), `version-window`
(local typed Git history only; no GitHub service), and `comment` (the shared
issue-mutation owner's authorized, redacted, read-back-verified comment
publication). These verbs were born Rust-owned with no predecessor and remain
recorded in the final command registry. Their adapter owners are the
already-listed
`crates/larch-adapters/src/github_rest.rs`,
`crates/larch-adapters/src/github/operations.rs`, and
`crates/larch-adapters/src/github/issue_mutation.rs` rows.

The #8577 cutover moved `design parse-flags`, `design route`, and
`design init-runparams` to Rust. Parse-flags reaches no GitHub service. Route's
title predicates run in-process and its pause bridge delegates to the
Rust-owned `design pause-load`, whose typed marker read joined `issue-reads` in
issue #8589. `design init-runparams` drives the shared tracking rename in
process, so it joined the `tracking-issue-lifecycle` row.

The #8592 cutover moved `design log-publish` to Rust. It publishes sanitized
design session archives through the shared run-log lifecycle (object storage /
cache) and no longer creates a GitHub pull request, so it left the
`pull-requests` row without joining another GitHub-service row.

The #8591 cutover moved `design publish` to Rust. It reaches the pull-request
service only through the Rust `design log-publish` bridge, and its own direct
mutation is the published plan receipt through the shared issue-mutation owner,
so it moved from `pull-requests` to the `pull-request-design-migrated` row.

Issue #8622 moved the four ship routing and pre-fix commands to Rust.
`ship pre-fix-rebase` resolves the checked-out repository through the typed
`gh resolve-repo` owner, so it joins `repository-metadata`. The other three
commands reach no GitHub service. Issue #8628 moved `ship pr` and
`ship reconcile-manual-merge` to the typed pull-request service.

Issue #8798 moved `forked-repo setup` to Rust. Its fork existence and immediate
parent check use the typed repository-metadata read, whose bounded repository
DTO now retains the validated parent slug. Git transport remains separately
owned by the closed Git adapter.

Issue #8629 moved `complete-umbrella ship-leaf` to Rust. Pull-request reads and
creation joined the typed ship service, while exact leaf-title transitions and
their freshness/read-back checks use the shared issue-mutation owner. The
Rust owner has no direct raw `gh` boundary; it composes the separately owned
merge commands through the reviewed process seam.

Issue #8788 moved `merge pr` and `merge wait` to Rust. The command owner uses
the typed pull-request, Actions, direct-merge, and fixed merge-queue operations.
Every ship and release consumer now reaches that owner through verified larch
dispatch. The retired registrations and module were removed atomically.

`release stage` gives the `releases` operation row the tag and target commit for
the synthetic projection commit, not the merged `main` commit. The Git adapter
creates the projection and tag before the GitHub release adapter creates or
updates the draft. The canonical identity and pin rules live in
[Release content pin](security/supply-chain-credentials-and-services.md#release-content-pin).

Issue #8797 moved `token check-budget`, `token compute-pr-line-counts`, and its
`token compute-pr-lines` alias to Rust. Line counting uses the typed, bounded
pull-request files operation and aggregates only filenames, additions, and
deletions. The final-report and launcher consumers call the Rust owners in
process; external callers continue through `scripts/larch.sh`.

Issue #8928 moved `stall-recovery file-report` to Rust and removed its Bash
owner. The verb reads one bounded newest-first issue page through the typed
issue service. It performs issue creation and duplicate comments through
`IssueMutationOwner`, including authorization, redaction, identity checks, and
exact read-back. Callers inside the reporting runtime invoke the owner in
process. External callers enter through `scripts/larch.sh`.

<!-- markdownlint-disable MD010 -->
<!-- github-service-ownership:start -->
```text
operation	adapter_owner	current_owner	planning_issues	implementation_parity	consumer_cutover	python_removal	commands
actions	crates/larch-adapters/src/github_actions.rs	rust	#7676,#7685,#8362,#8862	complete	complete	complete	ci-timing harness,ci-timing jobs,ci-timing rust-jobs,ci-timing merge-group-source,gh run-logs,gh workflow-path,rebalance-tests run
attestations	crates/larch-adapters/src/github/attestation.rs	rust	#7674	complete	complete	complete	release validate-assets
comments	crates/larch-adapters/src/github_rest.rs	rust	#7680	complete	complete	complete	clarify *
dependency-consumers	crates/larch-adapters/src/github/operations.rs	rust	#7682	complete	complete	complete	deps *
issue-dependencies	crates/larch-adapters/src/github/operations.rs	rust	#7682,#7685,#8946	complete	complete	complete	block-issue *,issue add-blocked-by,issue create-batch,issue migration-audit
issue-sub-issues	crates/larch-adapters/src/github/operations.rs	rust	#7682	complete	complete	complete	issue add-sub-issue
issue-creation	crates/larch-adapters/src/github/issue_mutation.rs	rust	#7682,#8946	complete	complete	complete	issue cleanup-failed,issue create-batch,issue create-one,issue write-sentinel
issue-body-blocks	crates/larch-adapters/src/github/issue_mutation.rs	rust	#7680,#7682	complete	complete	complete	named-block write,plan-block read,plan-block write
issue-reads	crates/larch-adapters/src/github_rest.rs	rust	#7680,#7682,#7685,#8927	complete	complete	complete	design pause-load,design pause-save,gh agnix-issue,issue context,issue fetch-issue-details,issue info,issue list-issues,issue search-implementing,issue state,umbrella prepare
stall-report-reads	crates/larch-adapters/src/github_rest.rs	rust	#7677,#7680,#8928	complete	complete	complete	stall-recovery file-report
stall-report-mutations	crates/larch-adapters/src/github/issue_mutation.rs	rust	#7677,#7680,#8928	complete	complete	complete	stall-recovery file-report
design-issue-read	crates/larch-adapters/src/github/operations.rs	rust	#7680	complete	complete	complete	design step0-route
issue-backlog-reads	crates/larch-adapters/src/github_rest.rs	rust	#7682	complete	complete	complete	analyze-issues fetch,analyze-issues run
issue-backlog-comments	crates/larch-adapters/src/github_rest.rs	rust	#7682	complete	complete	complete	analyze-issues run
issue-backlog-closure-references	crates/larch-adapters/src/github/operations.rs	rust	#7682	complete	complete	complete	analyze-issues fetch,analyze-issues run
issues	crates/larch-adapters/src/github_rest.rs	rust	#7685,#7684	complete	complete	complete	issue migration-audit,rejected-analysis prepare
audit-report-issues	crates/larch-adapters/src/github_rest.rs	rust	#7682	complete	complete	complete	audit-runs bugs-backlog-nudge,audit-runs close-priors
audit-pull-requests	crates/larch-adapters/src/github/operations.rs	rust	#7682	complete	complete	complete	audit-runs map-runs,audit-runs preflight,audit-runs resolve-prs
rebalance-pull-requests	crates/larch-adapters/src/github/operations.rs	rust	#7685	complete	complete	complete	issue migration-audit,rebalance-tests run
combine-issues	crates/larch-adapters/src/github/issue_mutation.rs	rust	#7682	complete	complete	complete	combine-issues *
complete-umbrella-leaf-lifecycle	crates/larch-adapters/src/github/issue_mutation.rs	rust	#8282,#8629	complete	complete	complete	complete-umbrella ship-leaf
label-dependency-mutations	crates/larch-adapters/src/github_rest.rs	rust	#7682	complete	complete	complete	block-issue *
labels	crates/larch-adapters/src/github_rest.rs	rust	#7680	complete	complete	complete	clarify label,design file-oos-annotate
design-oos-label-mutations	crates/larch-adapters/src/github/issue_mutation.rs	rust	#7680	complete	complete	complete	design file-oos-annotate
agnix-label-provision	crates/larch-adapters/src/github_rest.rs	rust	#7685	complete	complete	complete	gh agnix-ensure-label
pull-request-implement	crates/larch-adapters/src/github/operations.rs	rust	#7681	complete	complete	complete	implement cleanup,implement-finalize postbump,implement-finalize postmerge,implement-finalize teardown
pull-request-implement-dispatch	crates/larch-adapters/src/github/operations.rs	rust	#7681	complete	complete	complete	implement checks-commit-route,implement commit,implement commit-route,implement kill-active-leg,implement recovery-paths,implement run-dispatch,implement run-step-checks,implement step-2-post-dispatch,implement step2-dispatch
pull-request-implement-migrated	crates/larch-adapters/src/github/operations.rs	rust	#7681	complete	complete	complete	implement checks-result-identity,implement checks-step5-resume,implement clone-tag,implement normalize-coder-scout,implement preflight,implement step-0-bootstrap,implement step-0-degraded-gate,implement step-16,implement step-16-16a,implement step-16-17,implement step-17,implement step-18,implement step-5-resume,implement step-5-review,implement step-6-entry,implement step-7a,implement step-8-oos-checkpoint,implement step-8-seed-initial,implement step-8-ship
pull-request-implement-retired	crates/larch-adapters/src/github/operations.rs	retired	#7681	not-applicable	complete	complete	implement step-18-gate-finalize
pull-request-implement-terminal	crates/larch-adapters/src/github/operations.rs	rust	#7995	complete	complete	complete	implement step-18-gate-logs-flush,implement step-19
pull-request-ci-monitor	crates/larch-adapters/src/github_actions.rs	rust	#7681	complete	complete	complete	ci behind-count,ci decide,ci distill-log,ci failed-jobs,ci main-health,ci rerun-failed,ci status,ci wait
pull-request-design-migrated	crates/larch-adapters/src/github/operations.rs	rust	#7680	complete	complete	complete	design dialectic-clear-stale,design dialectic-gatec,design dialectic-manual,design dialectic-promote-candidates,design dialectic-validate-candidates,design dialectic-write-candidates,design driver,design failure-report,design file-oos-annotate,design file-oos-prepare,design pause-load,design pause-save,design prelude,design publish,design read-result-env,design render-final-summary,design render-gate,design stage-terminal-state,design step-final-summary,design step1d5,design step1d7,design step1e-reentry,design step2b5,design step35-settle,design step3-continuation-entry,design step5b-annotate,design step5b-prepare,design step5c,design step6,design step6-cleanup,design step6-prelude
pull-request-line-counts	crates/larch-adapters/src/github/operations.rs	rust	#7681,#8797	complete	complete	complete	token compute-pr-line-counts,token compute-pr-lines
pull-request-merge	crates/larch-adapters/src/github/operations.rs	rust	#7681,#8788	complete	complete	complete	merge pr,merge wait
pull-request-ship-pr	crates/larch-adapters/src/github/operations.rs	rust	#7681,#8282,#8626,#8628,#8629	complete	complete	complete	complete-umbrella ship-leaf,ship pr,ship reconcile-manual-merge
pull-requests	crates/larch-adapters/src/github/operations.rs	rust	#7681,#8790	complete	complete	complete	pr body-update,pr checks,pr closes-issue,pr create,pr create-branch
releases	crates/larch-adapters/src/github/release.rs	rust	#7674	complete	complete	complete	release *
repository-metadata	crates/larch-adapters/src/github/mod.rs	rust	#7676,#7681,#8798	complete	complete	complete	forked-repo setup,gh remote-repo,gh resolve-repo,ship pre-fix-rebase
tracking-issue-comment-reads	crates/larch-adapters/src/github_rest.rs	rust	#7680,#7681,#7682,#8789,#8837	complete	complete	complete	diagrams upsert,tracking post-issue,tracking-issue append-comment,tracking-issue read,tracking-issue upsert-summary
tracking-issue-comment-mutations	crates/larch-adapters/src/github/issue_mutation.rs	rust	#7680,#7681,#7682,#8789,#8837	complete	complete	complete	diagrams upsert,tracking post-issue,tracking-issue append-comment,tracking-issue upsert-summary
tracking-issue-lifecycle	crates/larch-adapters/src/github/issue_mutation.rs	rust	#7680,#7682	complete	complete	complete	design init-runparams,tracking-issue create-issue,tracking-issue mark-false-positive,tracking-issue rename
umbrella-conversion	crates/larch-adapters/src/github/issue_mutation.rs	rust	#7682	complete	complete	complete	umbrella mutate
```
<!-- github-service-ownership:end -->
<!-- markdownlint-enable MD010 -->

`crates/larch-lint/data/command-registry.toml` is the authoritative per-command
ledger. Its required `planning_issue` records historical roadmap placement and
`migration_issue` records the exact executable leaf that established the final
owner. The `command-registry` rule requires every live caller to match a Rust
row and clean-install fixture, and rejects any retired row with a live caller.

## Completed shared cutovers

`gh remote-repo`, `gh resolve-repo` (#7764), `gh run-logs`, `gh workflow-path`
(#7765), and `ci-timing harness`, `ci-timing jobs` (#8098) are Rust-owned. Their
callers enter the single larch executable through `scripts/larch.sh`; the
subcommands use typed Rust adapters rather than GitHub CLI API shell-outs.
`ci-timing rust-jobs` (#8862) extends that same typed jobs owner.
`rebalance-tests run` (#8343) likewise uses the typed Actions and pull-request
owners for its complete Rust-only workflow.

## CLI independence and the bootstrap exception

The only production Rust invocation of `gh` is the core-owned, fixed
`gh auth token --hostname github.com` credential lookup. Rust performs GitHub
API operations only through the authenticated Octocrab adapter, never through
`gh api`; `gcloud` is never a runtime service fallback. The `gh-argv-literal`
rule keeps raw `gh` construction inside approved wrappers. The clean-install `gh` usage in
`scripts/larch.sh` downloads and verifies the release binary before runtime.

## Redaction and diagnostics

The credential is held by a non-`Debug` wrapper and omitted from the typed child
environment allowlist. Authorization diagnostics pass through an
invocation-owned redactor, and errors retain only stable failure classes.
Diagnostics, session files, published logs, and snapshots contain no tokens,
authorization headers, or access tokens.
