# Artifacts, Redaction, and Publication

This document is the canonical security reference for larch artifacts,
redaction, diagnostics, secret scanning, retention, and publication. Root
[`SECURITY.md`](../../SECURITY.md) keeps the public summary. [Larch Run
Logs](../run-logs.md) owns the detailed archive-selection and batch contracts.

The rules here classify data by where it may go. Redaction and scanning are
backstops. They do not decide whether content is safe to publish.

## Confidentiality and Publication Classes

| Class | Examples | Rule |
|-------|----------|------|
| Private session state | Prompts, model output, raw transcripts and event streams, state envs, retry metadata, raw stdout and stderr, agent sidecars, caches, and temporary files | Keep under the owning session or temporary root. Do not commit or publish unless a named publisher selects and sanitizes the content. |
| Model-facing data | Repository and GitHub content, plans, findings, diagnostics, tool output, and delegated-agent input or output | Treat as untrusted and session-private by default. Model access does not make the content public or safe to publish. |
| Operator-visible diagnostics | Terminal errors, bounded stderr tails, statusline breadcrumbs, debug reports, and local fallback reports | Show only the bounded and redacted form supplied by the owning renderer. Do not paste raw diagnostics into a public issue or PR. |
| Published run logs | Registered batches, round artifacts, summaries, transcripts, and quiet-log breadcrumbs | Filter, trim, redact, and scrub before archive publication. Treat the result as sensitive and grant access only through the configured storage provider. |
| Public GitHub content | Issue and PR bodies, comments, diagrams, research reports, failure reports, and tracking summaries | Treat publication as irreversible. Apply content allowlists, bounds, and redaction, then make a separate human or workflow classification that the content is safe for the target repository. |
| Run-log archives and remote objects | Deterministic archives and object-store copies of a sanitized run tree | Preserve the sanitized run tree's confidentiality class. Archive transport does not make the content public or less sensitive. |

Session-private means private from publication, not private from the local
operating-system user. A process running as the same user may read or replace
session files, environment variables, caches, and temporary files. Larch path,
type, containment, and symlink checks reduce accidental or cross-root writes;
they do not create a same-user confidentiality boundary.

The Rust-owned Claude subprocess and review launchers validate prompt and
context inputs against their allowed roots, then supply the rendered prompt as
a confined regular-file stdin. The shared external-process adapter revalidates
those input and output files at open time and uses no-follow opens where the
platform supports them. Launcher outputs and diagnostic sidecars remain
session-private artifacts with private creation modes where supported. These checks reduce
accidental cross-root writes and symlink substitution; they do not turn vendor
input or output into a publication-safe artifact.

Review-and-fix coder prompts keep temporary helpers under the session root.
The shared repair stage selector excludes scratch-looking paths from repair
commits, records each exclusion as a warning, and leaves each excluded change
out of the repair commit. The selector recognizes `.tmp-*`, `.tmp_*`,
`*.orig`, and `*.rej` paths, including content below a scratch-named directory.

The Rust-owned `voting scoreboard` treats its caller-selected output as a
local private artifact. It rejects replaceable symlinks in the output path
before creating missing parents, confines that creation below the nearest
existing directory, and publishes the final file by private atomic replacement.
This writer does not redact scoreboard content or authorize public publication.

## Redaction and Secret Scanning

### Redaction invariants

The four public redaction commands are Rust-owned and enter through
`scripts/larch.sh redact`: `secrets`, `tmpdir-paths`, `scrub-log-secrets`, and
`scrub-submodule-paths`. Their command boundary lives in
`crates/larch-cli/src/redact_commands.rs`; pure redaction and finding filtering
live in `crates/larch-core/src/redaction.rs`. The submodule command uses only the
closed typed `GitCli::submodule(SubmoduleRequest::Foreach)` adapter, rejects a
symlinked `.gitmodules`, and ignores entries that are not safe non-empty Git
paths. `review-and-fix` reuses the same pure finding filter. These Rust modules
are the sole redaction implementation.

Redaction state, scrubbed finding output, and audit-log writes reject symlinked
or multiply linked targets and publish by confined atomic replacement. New
state and finding artifacts use private mode `0600`; in-place log scrubbing
preserves the existing ordinary permission mode. Missing streaming state starts
clean; unsafe, unreadable, or malformed existing state fails closed before any
redacted output is emitted. Directory scrubbing skips symlinks,
non-regular files, unreadable files, and non-UTF-8 files, then rescans every
changed payload before publication. Rust egress paths use `larch_core::SafeText`
for human output, machine output, breadcrumbs, and journal fields. `SafeText`
redacts known path, token, and PEM families, then withholds a value when a
recognized secret survives its rescan. Contract writers also reject line
breaks before writing line-oriented records.

Run-log batch payloads and `--redact` failure bodies are redacted by the Rust
owner, `larch_core::redact_run_log_payload`, because `run-log write`, `append`,
`write-round`, and `append-failure` are Rust-owned. It scrubs session and
operator paths and then every secret family the Rust redactor knows, which is a
superset of the families the retired Python owner scrubbed on this path: Slack,
Google API, Stripe live, and GitLab tokens are now removed from published batch
payloads in addition to the Anthropic/OpenAI, GitHub, AWS, JWT, Cursor, and PEM
families. Debate batches additionally refuse, rather than redact, a recognized
session-tmpdir pointer, including one that appears only inside decoded JSON.

Before content crosses into a committed or public artifact, the owning
publisher must:

1. Minimize the captured content and select only fields needed by the consumer.
2. Remove session and system temporary paths where the surface requires it.
3. Redact recognized secret families.
4. Fail closed when the redactor cannot run, reports an error, or cannot prove
   that a detected secret is absent. Public captured-error helpers also fail
   closed on their safety-truncation marker.
5. Warn the operator to rotate a credential that was detected and scrubbed.

A successful scrub does not prove that publication is safe. It proves only
that the configured patterns did not leave a recognized secret in the checked
content. A detected credential was already present in the session and may have
reached another local artifact before the scrub.

### Redaction limits

Pattern-based redaction may miss:

- unknown credential formats and partial token fragments;
- private infrastructure identifiers, internal hostnames, and internal URLs;
- personal data and other identifying information;
- proprietary repository content and private architecture details;
- vulnerability details and security-sensitive findings;
- domain-specific sensitive content that has no general token pattern.

It may also over-redact benign text. Minimize external text before redaction,
and review the sanitized result for the destination. Never use a clean redactor
or scanner result as a content-classification decision.

### Secret-scanning layers

Larch uses three distinct scanner layers:

1. **Manual local working-tree scan**. The manual pre-commit stage runs
   `scripts/larch.sh lint gitleaks --mode working-tree`. The Rust command
   verifies the pinned Gitleaks release archive, extracted binary, and reported
   version, then runs with the repository `.gitleaks.toml`. `--no-git` and
   `pass_filenames: false` keep the scan on the full working tree. Select it
   explicitly with `pre-commit run --hook-stage manual --all-files`; it is not
   part of the default commit hook or changed-file relevant checks.
2. **CI working-tree and history scan**. The `gitleaks` CI job uses a
   workflow-local installer for the same pinned scanner release, avoiding a
   full `larch-cli` build. It verifies the archive, extracted binary, cache
   entry, and reported version before executing the scanner by absolute path in
   a minimal credential-free environment.
   It scans the working tree with `--no-git` and the PR commit range with full
   history available. This is the enforced backstop for the opt-in local scan.
3. **CI live-credential verification**. The `trufflehog` job pins the action to
   an immutable commit and pins its Docker version. `--only-verified` reports
   credentials that authenticate against a live provider. It does not replace
   pattern scanning because revoked, synthetic, or otherwise unverifiable
   token-shaped values may still be sensitive.

The local Rust wrapper and CI installer both pin Gitleaks `v8.18.4`.
TruffleHog currently pins the action commit corresponding to `v3.82.13` and
sets `version: 3.82.13`. Scanner or checksum drift fails closed. Both Gitleaks
paths execute the verified cache file by its absolute path, rather than
resolving a scanner from `PATH`; the child retains its normal `PATH` only so
Gitleaks can invoke Git for bounded history scans. The CI installer rechecks
the cache before each scan and allows cache publication only after a successful
trusted-main run.

The `.gitleaks.toml` path allowlist still creates pattern-scan blind spots. It
covers the config itself, named residual-script and skill fixtures, one Rust
parity golden that preserves a synthetic PEM marker to verify fail-closed log
rescanning, and build output that legitimately contains synthetic token shapes.
Keep test values obviously fake.
The independent verified scan remains necessary, but it does not fill every
allowlist or pattern gap.

The ignored `target/` directory may contain dependency-owned key fixtures in
compiled metadata. Authored source and high-churn documentation are not
blanket-allowlisted; Gitleaks scans them locally and in CI. Remote archives are
not fetched into CI. Contributors should use short token prefixes such as
`ghp_`, or explicit placeholders such as `<REDACTED-TOKEN>`.

The larch-owned pre-publication scrubber is the primary recognized-secret defense for
published run logs. It covers token families, including selected Cursor key
families, that the pinned Gitleaks rules may not recognize. Gitleaks and
TruffleHog remain independent backstops, not substitutes for that scrubber.

## Private Session State and Retention

Session roots under `${XDG_CACHE_HOME:-$HOME/.cache}/larch/sessions/`, the
system temporary directory, and matching `/tmp` paths may contain prompts,
raw model and tool output, issue content, paths, credentials in child process
environments, and raw retry metadata. Examples include:

- `*.events.jsonl`, raw agent transcripts, prompts, and launch stdout or stderr;
- `.meta` files with `CMD_JSON`, JSON response envelopes, and retry sidecars;
- bgjob result envs, daemon-status sidecars, logs, registry state, and
  completion sentinels;
- in-loop token, timing, and transcript refresh sidecars;
- CI, lint, design-publish, and vendor diagnostic carriers;
- session envs, finalize state, plugin-root state, and timing ledgers.

These files support routing, retries, and operator diagnosis. They are not
stable published batches. A publisher may copy a bounded, registered,
sanitized projection into a published artifact. The raw source remains
session-private.

Stall-recovery pending-clear markers and attempt-ledger companion locks are
also private session artifacts. A pending marker is transition evidence, not a
completed-clear signal or a public wire file: classification and outcome readers
refuse it until the clear transaction verifies and removes it. Neither the
marker nor a lock may be copied into a report, run-log batch, issue, or PR.
Their only purpose is to make interrupted local recovery recognizable and to
preserve every successful private retry record.

Rust-owned background-job completion is a confined transaction, not a result
file followed by best-effort markers. Before publishing any observable output,
the daemon privately stages the exact result-envelope bytes and records a
descriptor with their digest and the complete ordered sentinel set, including
an explicit zero-sentinel declaration. Readers treat a result as terminal only
after that descriptor is committed, the digest matches, and every declared
sentinel is a confined regular non-symlink file. If publication is interrupted,
recovery first proves the worker group absent, then replays the staged bytes and
declared set; it never infers a terminal result from a partial file set. Direct
and adapted merge-result paths must be absolute, below the owning session root,
and have a regular non-symlink leaf and full parent chain at launch. The daemon
revalidates and opens the merge envelope without following symlinks when it
consumes it.

`session setup` is Rust-owned in
`crates/larch-cli/src/session_setup_commands.rs`. It reads the allowlisted
caller-env handoff before preflight, so malformed wire input cannot trigger a
Git mutation. Its session directory is created through the shared
`TemporaryRoot` and `SecureTempDir` adapters: prefixes cannot escape the
session root, creation is private where the platform supports it, and the
directory is revalidated before it becomes durable. It retains the confined
`.larch-session-setup` uncommitted marker and `SecureTempDir` cleanup ownership
until the complete ordered stdout result envelope has been written and flushed.
One atomic ownership-transfer decision races cancellation against the actual
stdout write, flush, and commit boundary. A `SIGINT` or `SIGTERM` that wins
before transfer returns the controlled cancellation result and closes the
private directory, even if it arrived while the envelope writer was active. A
signal after transfer does not retract a session whose complete envelope the
caller can already own. Setup then writes the confined keepalive, persists the
directory, and removes the marker. The marker binds `OWNER_PID` to the
normalized process start time. Recovery retains an unverifiable or matching
live owner, reclaims a missing or mismatched upgraded owner, and treats a
legacy PID-only marker as stale only when a newer process start proves PID
reuse. An uncatchable termination before transfer leaves that confined marker
for the next cleanup pass. The session identity is a confined pre-publication
write, so an identity-write failure also closes the owned temporary directory
rather than leaving a partial committed session state.

Rust-owned `bootstrap invoke` completes its Step 0 continuation in
`crates/larch-cli/src/implement_bootstrap_continuation.rs`. Continuation-owned
session and preflight inputs are opened through canonical, no-follow confined
paths; symlinked or non-regular routing, plan, sentinel, and lease-evidence
files are refused rather than followed. Tracking-lease body snapshots are
removed through the same confinement boundary after verification, including on
the failure path. The Rust `issue migration-audit` command collects read-only
evidence through the typed GitHub, Git, filesystem, and lint boundaries and has
no issue mutation owner. The Rust `issue governance-gate` machine envelope
validates its caller-supplied repository root and body-file boundary before
evaluating the canonical governance policy. Rust `plan-receipt refresh`
validates its issue, repository root, preflight plan/snapshot, prior receipt,
and base SHA; it uses the
protected issue-mutation owner and accepts a refresh only after exact receipt
read-back. Its bounded path-only scope-drift artifact JSON-quotes every
name-status row before redaction, and Step 0 validates then appends it once to
the `Warnings` ledger. The `/implement` caller may invoke that mutation only
after its bounded semantic-materiality probe clears sole scope drift.

External CLI credentials may enter a child process environment. A live
`OPENAI_API_KEY` stays out of larch-owned argv and copied config, but same-user
or host-level process inspection can still observe the child environment. On
Darwin, shared Cursor launchers may pre-read a readable keychain token and
export it to the child when `CURSOR_API_KEY` is unset, so leaving that variable
unset does not guarantee zero environment-secret propagation. Raw vendor
stderr and event streams may also contain upstream authentication diagnostics.
Operators who require no environment-secret propagation should avoid those
lanes or remove the readable credential source. See
[Configuration and Permissions](../configuration-and-permissions.md) for the
current launcher and authentication settings.

Rust-owned `cleanup run` removes stale session-cache and temporary entries by
age and bounded nested-activity checks. `LARCH_CLEANUP_RETENTION_DAYS`
controls the retention window. Before a sweep it protects directories named by
the current session environment and by current session pointers, so a live
session is retained even when its top-level timestamp is old. The authoritative
PID-keyed pointers remain under `$HOME/.cache/larch/sessions/` even when XDG
redirects session artifacts. Pointer publication, pointer reaping, and the
final recursive-removal decision share a confined advisory activity lock. At
that final decision cleanup re-reads the current pointers and revalidates the
root, target shape, age, and nested activity while holding the lease; an
unreadable pointer or unavailable lease skips age-based removal rather than
risking a concurrent activation or resume. A prior crash can also leave a
private, uncommitted setup marker: cleanup immediately recovers only a direct,
confined non-symlinked directory below an approved session root whose recorded
setup owner is proven stale. Malformed markers, uncertain owner liveness, live
setups, and committed sessions are retained. Implement-tempdir discovery also
requires a direct non-symlinked candidate and non-symlinked sentinel and
keepalive components below a supplied root; it never follows those routing
files outside the root. Cleanup may unlink only a dangling design-session
pointer, and reaps a stale implement pointer only when it is a regular file.
Matching loose temporary files do not receive nested-directory activity
protection. Deletion is permanent and does not redact first. A failed directory
activity scan skips that directory, while a failed top-level enumeration skips
that pass. See
[Configuration and Permissions](../configuration-and-permissions.md) for the
operator setting and [Larch Run Logs](../run-logs.md#retention) for run-log
retention.

The Rust-owned audit verb behind the opt-in `scripts/audit-edit-write.sh`
developer shim is intentionally unredacted. It accepts only object JSON and
refuses symlinked, multiply linked, or non-regular audit paths, but its
gitignored JSONL can still contain file paths, file contents, credentials,
personal data, and proprietary code. It has no automatic retention. Never
commit or publish it, and clear it after debugging. The Claude
`--debug-file` log can expose settings paths, plugin paths, MCP server URLs, and
permission data. Review and redact that log before sharing it. See
[Developer Hook Audit](../dev-hook-audit.md) and the
[configuration evidence-handling rules](../configuration-and-permissions.md#evidence-handling).

## Operator-Visible Diagnostics

Operator visibility does not authorize publication. Diagnostic renderers
should expose fixed classifications and bounded context instead of raw vendor,
subprocess, GitHub, or repository text.

Shell error helpers are not a general redaction boundary. Some print only
fixed maintainer-controlled text, while call sites that surface untrusted
external text must invoke the owning redactor first. Do not pass raw vendor,
GitHub, repository, or subprocess content to a plain `larch_err` wrapper.

Implement Step 0 lease failures preserve the verified-larch child's bounded
stdout and stderr in the session-private tracking diagnostic. The tracking
bail owner redacts the complete composed diagnostic before writing it under
the session root; it never writes the raw child streams directly.

Failed Codex, Cursor, and Claude launches may expose a bounded stderr tail.
`LARCH_FAILED_AGENT_STDERR_TAIL_LINES` controls the line limit and `0` disables
the tail. The default is 30 lines. After line limiting, the tail passes through
temporary-path and secret redaction, then a 5120-byte cap. Successful launches
remove stale tail sidecars. Batch collection deduplicates repeated root-cause
tails.

Bgjob DEAD recovery may expose `DAEMON_EXIT`, `DAEMON_SIGNAL`, `STDOUT_TAIL`,
and `STDERR_TAIL`. The exit and signal fields are mutually exclusive and come
from a confined supervisor sidecar whose daemon PID must match the durable
registry identity. Empty fields mean the evidence was unavailable. Each stream
tail retains at most 4096 bytes from its confined regular log. The reader may
inspect one preceding byte to prove an exact line boundary. When earlier bytes
were omitted and that boundary is not proven, the renderer drops the leading
partial line and adds an explicit omission marker. It then applies the shared
temporary-path and secret redactor and escapes line breaks before emitting the
`KEY=value` row. The raw logs and status sidecar remain private session state.

The Rust-owned `agent compose-collector-failure-log` command reads its local
reviewer and stderr sidecars through `larch_adapters::vendor_diagnostics`,
uses the same bounded-tail renderer for stderr carriers, applies the shared
core redactor to the complete composed body, and atomically writes the result
with private mode `0600`. Its output remains session-private; it is not a
public-report publisher.

Per-slot `*.failure-diag` files combine bounded diagnostic sources for local
recovery. They remain session-private. Implement Step 18 terminal snapshot
preparation redacts their selected content into
`vendor-failure-diagnostics.txt`, which becomes a published run-log batch. The
per-slot byte cap is owned by `vendor_failure_diag_byte_cap`. The composed batch
has no second aggregate cap, so the slot cap and registered-slot set bound its
inputs. Paths that do not reach terminalization and research validation runs
keep these diagnostics local.

CI and lint repair surfaces pass only bounded, redacted evidence to delegated
fixers. Design and ship diagnostics persist raw captures only in the session
root. Trusted result envs contain fixed state and classification tokens, not
tracebacks or subprocess bodies. When a public fallback report is allowed, the
report renderer applies the public field contract described below.

Clone-local statusline breadcrumbs are operator diagnostics stored under
`~/.cache/larch/progress/`. Their one-line events avoid URLs and use GitHub
numbers. They are not the published breadcrumb stream and are not public
reports. Rust exclusively owns their pointer, breadcrumb, stale-state, and
persisted-identity behavior. See [Progress reporting](../progress-reporting.md).

## Run Logs and Breadcrumbs

Published run-log archives are durable object-store artifacts. They may contain
plans, findings, summaries, transcripts, failure evidence, and model output
after sanitization. Treat them as sensitive even when every redaction and
scanner passes. Current workflows do not store run logs in Git.

Cloud run-log retention is append-only. No shipped runtime deletes or slims
remote archive content, creates a run-log Git branch, commit, push, or pull
request, or configures remote lifecycle expiration. The local,
operator-only `run-log cleanup-implement-logs` command is the narrow exception:
it may remove only redundant files inside a manifest-accepted, completed local
implement run. A present durability marker must be committed, and publication
configured runs require that marker. It never deletes a run directory, archive,
cache entry, or external path.

Calibration replay treats synchronized run logs and committed calibration
fixtures as read-only evidence. Its Rust owner validates manifest bindings and
resolved input containment before dispatch, confines ballots and reconstructed
ledgers to a separate work directory, and refuses unsafe write targets. Replay
dispatch also forces calibration feedback off so a replay cannot feed its own
result back into the synchronized corpus.

The Rust-owned, operator-only `run-log migrate-layout` command is a bounded
exception for creating the tool-first copies required by
`character-ai/larch#8081`. Live mode accepts only the issue's exact S3 roots.
It uses create-only writes, never deletes or overwrites remote objects, and
verifies every downloaded target. Its published report contains identities,
hashes, sizes, aggregate counts, and fixed status tokens. It excludes
credentials, provider diagnostics, archive contents, and local absolute paths.
The retained old prefixes remain rollback evidence until the separate cleanup
issue passes its retention gates.

The shared Rust run lifecycle, reached only through `scripts/larch.sh`, is the
sole terminal archive publisher. Specialized
design, implement, and review owners may select and stage richer artifacts, but
they hand that one staging tree to the shared terminal boundary. That boundary
enforces these security invariants:

- select only registered batches and documented directory artifacts;
- exclude raw prompt-bearing event streams, retry-only output, unregistered
  diagnostic carriers, and redundant GitHub snapshots;
- trim `CMD_JSON` from selected `.meta` files and remove the top-level
  `.result` from selected agent JSON before archiving round artifacts;
- reject unsafe roots, symlinks, hardlinks where prohibited, special files,
  path escape, and a failed trim or redaction;
- redact temporary paths and recognized secrets in every selected artifact;
- scrub the complete staged run tree immediately before each flush;
- record or fail on missing required artifacts according to the run-log
  completeness contract.

Raw Codex event streams, plan-review transcripts, render prompts, launch
stderr, collector stderr, retry carriers, and session sentinels remain private
unless the run-log selection contract names a sanitized derivative. Pause
snapshots may retain the top-level completion sentinels needed for resume
provenance. Design publication applies its allowlist by basename at every
copied depth and rejects an unsafe or untrimmed artifact instead of copying it.

The complete selection rules, per-round allowlists, batch schemas, and
retention rules live in [Larch Run Logs](../run-logs.md),
[Run-log CLI contract](../run-log-cli.md), and
[Run-log batch registry](../run-log-batches.md). Those documents describe what
is published. This document owns why every transition must stay filtered,
bounded, and fail-closed.

### Durable debate-record invariant

The four debate batches (`debate-round-ledger`, `debate-proposal`,
`debate-stalemate-tally`, `debate-participants`) reject recognized
session-tmpdir pointers before redaction or persistence. The matcher is
session-tmpdir-specific: it does not reject operator-repository paths, and it
does not replace existing redaction of valid operator-repository paths,
secrets, or other sensitive content in accepted payloads. JSON debate batches
also inspect decoded object keys and string values so escaped session paths
cannot bypass the guard. Batch contracts and write/append mechanics live in
[Run-log batch registry](../run-log-batches.md#debate-record-batches).

The debate orchestrator redacts the automated stalemate tally and synthesized
proposal before it calls the batch writer. Dispatcher paths, vendor output
paths, and publication-handoff paths remain local control data and are not
embedded in either durable batch.

The source metadata, redacted subject, per-slot turn prompts, raw vendor
ledgers, Claude input files, operator-decision TSV, and publication handoffs
remain ephemeral session artifacts. They can contain public issue text,
repository evidence, or model output that is still sensitive in aggregate.
They are never copied into a public comment or selected as a run-log batch.
Normal success removes the debate scratch tree. Failed runs retain it only for
local diagnosis and then fall under the bounded session-retention cleanup.

### Breadcrumb security invariants

Session `breadcrumbs/` directories are publication hints, not content roots.
The run-log publisher selects only session-root quiet logs with accepted
basenames. Legacy stream files and monitor sidecars stay local. Every selected
quiet log must remain within the active session root, be a regular non-symlink
file, and satisfy the hardlink and basename checks.

Each quiet log passes independently through sensitive-path and secret
redaction. The publisher concatenates the sanitized files into one
`larch-logs/<skill>/<run-id>/breadcrumbs/quiet.log` through a staging directory
and atomic promotion. Any enforced validation or redaction failure rejects the
whole publication and leaves the prior destination unchanged. Missing sources
and documented non-matching files are no-op or skip cases, not permission to
publish a broader set.

The exact source resolution, accepted basename, enforced-reject, and
silent-skip rules live under
[Larch Run Logs: breadcrumbs](../run-logs.md#breadcrumbs).

### Archives and remote copies

A run-log archive contains a deterministic representation of an already
sanitized run tree. Materialization validates identity, paths, types, sizes,
digests, expansion, and collision limits before atomic promotion. Remote writes
are create-only and identity-checked. Retained retry state and local archive
caches remain private operator state. Moving an archive to object storage does
not broaden who should receive it.

This boundary applies to every public, alias, internal child, and dev-only skill
when publication is enabled. A durable lifecycle context binds the selected
staging root to one repository, publication mode and reason, derived client
repository, skill, and run ID. Enabled contexts also pin the canonical tool
repository URI and storage-origin ID. A config, environment, Git-origin, or
storage-origin change before enabled publication fails closed. Disabled
contexts instead pin a repository-root-derived local namespace digest. They
never carry a fake provider or URI, and adding configuration later does not
enable publication for that run. Parent-child metadata identifies run
relationships but does not change the classification or publication mode.

`scripts/larch.sh run-log sync` treats the remote inventory and downloaded archives
as untrusted. It accepts only the exact `run-logs/<skill>/<run-id>.tar.gz`
layout, rejects invalid or colliding local names, checks listed and downloaded
sizes, and routes content through bounded materialization under the publisher's
per-run lock. It does not replace a valid cache entry. Repair quarantines an
invalid entry, restores it on failure, and removes stale private transfer and
repair state under the same lock. See
[Run-log archive format](../run-log-archive.md).

Normal synchronization rejects manifest-less archives and never loads a legacy
migration descriptor or inventory. The retained legacy parser is available
only to an explicit operator migration API. It does not discover normal
repository configuration or change the sync trust boundary.

Corpus-derived evidence in a public run-log audit may record only
`INVENTORY_SHA256`, aggregate sync counters, and fixed outcome tokens. It must
not record `CORPUS_ROOT`, archive names, archive contents, provider diagnostics,
or credentials.

Mutable analyzer state is not an archive and never appears under `run-logs/`.
It stays under the private, client-repository and storage-origin-bound XDG state
home with owner-scoped paths, `0600` files, atomic replacement, per-file locks,
and stale-writer detection. Provider, bucket, prefix, tool, or repository
changes cannot reuse another origin's cache, locks, pending publication, or
analyzer state. Treat its ledgers, retry bundles, and generated reports as
untrusted private operator state. See [Analyzer state](../analysis-state.md).

The Rust-owned `token measure-*` commands synchronize through the same run-log
boundary, skip symlinked corpus entries and report inputs, and publish only to
their fixed owner plus validated date filename under that analyzer-state tree.
They do not write into the synchronized corpus or the consumer repository.

`/design` and standalone `/review` resolve storage before session work.
Enabled storage runs the exact prefix-scoped preflight and preserves the
existing design allowlists, review round artifacts, breadcrumbs, completeness
checks, and secret scrub before publishing
`run-logs/<skill>/<run-id>.tar.gz`. A failure returns nonzero and retains a
content-pinned pending archive; retry publishes that exact content even if the
live staging tree changed. Enabled success requires both the immutable remote
object and the verified unpacked cache. A completeness failure leaves the
manifest non-terminal, so a corrected terminal outcome can replace only the
owner-generated provisional final report. Terminal-summary wrappers retain raw
child stderr locally and expose at most one bounded, redacted diagnostic.

A retryable `/design` Step 5c plan-block or receipt-write failure passes its
selected artifacts through the same copy-time tmpdir-path redaction and secret
scrub as terminal publication. The sanitized snapshot stays in local lifecycle
staging. This path leaves the manifest in progress, creates no remote object or
cache entry, and records no terminal outcome. A later successful tail performs
the single create-only publication for that run ID.

Disabled storage skips provider construction, archive creation, upload,
verification, cache promotion, and pending state. Local staging and
bookkeeping remain active until terminalization, which writes universal
terminal artifacts and removes the run and context. Errors retain diagnostic
state. `/design` cross-session pause and resume require a verified published
cache, so pause rejects disabled storage before it writes a GitHub marker. The
Rust pause owner resolves the marker's run identity to the provider-scoped
materialized cache and verifies its archive manifest before reading it. Resume
then stages validated regular files beneath a private directory inside the
validated design root, rejects symlink or non-regular destinations, and
installs the complete snapshot — including `.completed/` — before clearing the
marker. A missing, incomplete, or unsafe snapshot leaves the marker intact for
retry; a permanent marker or manifest binding failure clears the stale marker.
Neither mode creates log branches, commits, pushes, pull requests, or merges.

For `/implement`, intermediate writes, appends, refreshes, and checkpoints
update only the session staging tree. Step 18 closes the ledgers, rebuilds the
complete terminal snapshot, recaptures the transcript whenever a source is
configured, and performs the final execution-issues append. Enabled mode
validates completeness and scrubs the full tree before creating the archive.
Disabled mode terminalizes without archive or cache fields and reports
`RUN_LOG_PUBLICATION=skipped-disabled`; explicit `--no-logs-commit` reports
`skipped-suppressed`. A snapshot, identity, upload, or cache-verification
failure returns nonzero, does not authorize Step 19 cleanup, and retains the
session plus any content-pinned pending archive for retry. Step 19 requires the
terminalization record and performs no log writes.

## Public GitHub Publication

GitHub publication is a separate security boundary from run-log storage.
Storage-provider access determines who can read archived logs. Issue and PR
content may target another repository or a public upstream project. Never infer
that content safe for one boundary is safe for the other.

Outbound bodies should be composed from fixed templates and allowlisted fields,
then bounded, redacted for temporary paths and secrets, and validated before
the network call. Captured `gh` or delegated-helper stderr must use the
fail-closed error redactor. If the redactor is absent, fails, or emits its
safety-truncation marker, publish no original stderr bytes and return a fixed
token-free diagnostic.

### Pull requests, tracking issues, and comments

PR bodies embed only sanitized diagrams or placeholders and pass through the
PR creation redactor before the typed GitHub mutation. Tracking issues carry slim,
marker-keyed summaries that name the provider, skill, and run ID when an
archive exists. Full run payloads live in remote archives, not issue comments.
When storage is disabled, public summaries state that no archive was published
instead of naming provider `unknown` or a fake location. Tracking title, body,
and comment writes redact paths and secrets before GitHub receives them.

The `larch:diagrams` publisher accepts source files from approved temporary
roots unless the operator explicitly allows another path. It validates the
repository identifier, sanitizes new Mermaid sections, preserves existing
sections only under the documented joint-comment contract, and redacts the
composed comment. The Rust owner in
`crates/larch-cli/src/diagram_commands.rs` authorizes the mutation through the
typed GitHub service and verifies the exact comment after mutation. It has no
`gh api` fallback. Diagram labels should omit private paths, hosts, and
secret-adjacent identifiers. A public or stale marker comment can remain until
a later full replacement, so the marker is not author provenance.

Issue-anchored plan and clarification writers validate issue and repository
identifiers, redact outbound bodies, confine temporary files, and redact errors.
Fetched issue and comment content remains untrusted data. These helpers do not
replace repository permissions, branch protection, or editorial review.

### Debate proposals

`/debate` publishes only the redacted synthesized proposal, a deterministic
source backlink, a fixed round-status digest, and fixed marker-keyed source
comments. Proposal filing uses `/issue`, so title and body secret redaction and
creation read-back remain in force. Round comments may name slots and stable
drop classes but never quote ledger reasons, prompts, paths, or raw output.

The source title mutation is separate from content publication. A typed owner
re-reads the issue before every start, finish, or restore transition and
verifies the title after a write. The proposal forward link and backward link
must verify before `[DEBATED]` is applied. An aborted run upserts one fixed
sanitized comment and restores the original title only when the live title is
still the exact debate-owned value.

### Research reports

`/research` publishes the full report and token-spend metadata to a GitHub issue
after a successful user-facing run unless `--no-issue` is set. Intermediate
`scripts/larch.sh eval research` calls pass `--no-issue`. Reports can contain
private architecture, vulnerability analysis, internal infrastructure
references, personal data, or domain-specific sensitive content that pattern
redaction cannot recognize. Use `--no-issue` whenever the report has not been
classified as safe for the target repository.

### Cross-repository failure reports

Consumer and forked runs may publish failure reports to the upstream larch
repository under the operator's GitHub identity. This is a broader boundary
than a private consumer repository.

Public report renderers use bounded allowlists of closed enums, sanitized step
and exit fields, fixed templates, bounded attempts, and bounded root-cause
summaries. They exclude raw logs, stdout, stderr, plans, issue bodies, feature
descriptions, repository and branch names, local and session paths, URLs,
credentials, evidence digests, raw state, and arbitrary run identifiers. The
only run-identifier exception is the public `Run ID` field sourced from
`RUN_ID`, `LARCH_RUN_ID`, or `SESSION_ID` when it is nonempty and limited to
ASCII letters, digits, `.`, `_`, `:`, and `-`; token-session identifiers remain
sensitive. Public dedup signatures and comments use only the same bounded
public fields. The Rust-owned `stall-recovery validate-tier-b-public-file`
command rebuilds the effective sensitive corpus under the validated session
root and rejects oversized, symlinked, path-bearing, remote-bearing, or
corpus-matching public text. Its `--snapshot-fd` interface remains the
publication boundary for external descriptor consumers. It returns a private,
unlinked descriptor through `/dev/fd`.

The Rust-owned `stall-recovery file-report` verb applies the same validation in
process before deduplication, issue creation, or duplicate-comment publication.
It checks file identity before, during, and after each read. Marker lookup,
create-title derivation, and typed GitHub transport consume the frozen approved
bytes without reopening the caller-provided pathname. A source that changes
while it is read, or is missing, oversized, non-regular, or symlinked, fails
closed before any GitHub mutation. Later source replacement cannot alter the
approved transport bytes. Typed issue reads enforce the 100-record dedup bound.
Issue and comment writes pass through `IssueMutationOwner`, which applies the
live-mutation gate, outbound redaction, identity validation, and exact read-back.
A missing validator, sensitive corpus, repository resolver, network result, or
valid created URL falls back to a sanitized local report for manual filing. It
never falls back to the raw evidence.

The Rust-owned `stall-recovery compose-report`, `chat-print`,
`dedup-tier-a-report`, and `populate-sensitive-corpus` commands keep their
reporting boundary in the Rust runtime. `compose-report` renders the title,
body, and Tier A slices in memory, applies the shared `larch_core` redactor,
then verifies the redacted payload and effective sensitive corpus before any
atomic public-artifact write. Redaction and corpus failures prevent the write;
a failed read-back postcondition triggers confined cleanup of that payload.
Tier A dedup receives only that sanitized body. The shared Rust adapter
validates optional detail logs as confined, regular files no larger than 64 KiB
before reading them or selecting a ledger sidecar. The sensitive-corpus command
rebuilds its corpus from validated session-root evidence through that adapter.

Tier A failure reporting inside a larch development clone may use fuller local
context only through the normal issue publisher and same-target classification.
It still requires sanitization and must not publish sensitive content. Tier B
cross-repository reporting must use the public allowlist. Design and implement
reporters each keep their own field schema and dedup token, but both follow this
confidentiality boundary.

## Implementation and Verification Owners

Publication paths are Rust-owned. The current owner of a publication path
defines its implementation checks and the complete egress contract.

| Concern | Current owners |
|---------|----------------|
| Redaction commands | `crates/larch-cli/src/redact_commands.rs` and `crates/larch-core/src/redaction.rs`; typed Git and confined atomic filesystem effects come from `larch-adapters` |
| Review-fix scratch exclusion | `larch_core::review::select_repair_stage_paths` owns classification; `crates/larch-cli/src/review_and_fix_commands.rs` owns warning persistence and path-limited commits |
| Cross-repository failure report publication | `crates/larch-cli/src/stall_recovery_file_report.rs`, `crates/larch-adapters/src/stall_recovery.rs`, `crates/larch-adapters/src/github_rest.rs`, and `crates/larch-adapters/src/github/issue_mutation.rs` |
| Checksum-pinned scanner | Local Rust command: `crates/larch-cli/src/gitleaks.rs` and `crates/larch-adapters/src/github/release.rs`; CI verifier: `.github/workflows/ci.yaml` |
| Rust human, machine, breadcrumb, and journal redaction | `crates/larch-core/src/redaction.rs`, `crates/larch-core/src/telemetry.rs`, and `larch_core::SafeText` consumers |
| Clone-local statusline progress state | Rust owns pointer activation, compare-and-clear, breadcrumb append, stale cleanup, and persisted run-identity parsing in `crates/larch-adapters/src/progress_state.rs` and `crates/larch-cli/src/progress_commands.rs`. |
| Mutable run-log flush and transcript staging | Rust owns execution-issue append, checkpoint, refresh, terminal snapshot, transcript capture, flush ordering, manifest reconciliation, and sorted vendor-diagnostic aggregation in `crates/larch-cli/src/execution_issue_commands.rs` and `run_log_flush_commands.rs`. Category-keyed chunk deduplication, the directory lock, atomic live-ledger replacement, lock-protected compare-and-clear after flush, and atomic batch replacement and append use `crates/larch-cli/src/run_log_entry_commands.rs`. `token mark`, `token report`, `token claude-source`, and `difficulty write-record` are Rust-owned in process. `final-report write` additionally reads assessment, plan, and PR payloads. |
| Timing ledger mutation and reports | Rust exclusively owns timing marks, vendor and round records, locking, validation, and report rendering in `crates/larch-cli/src/timing_commands.rs` (#8291). |
| Run-log selection, trim, scrub, and publication | Rust owns standalone and lifecycle publication, tree redaction, durable retry, create-only remote verification, cache promotion, and breadcrumb publication through `crates/larch-adapters/src/run_lifecycle.rs` and `crates/larch-cli/src/run_log_publication_commands.rs`. Design session archives use the same lifecycle through `scripts/larch.sh design log-publish` (`crates/larch-cli/src/design_log_publish_commands.rs`, selection filter `larch_core::design::log_publish::publish_excluded`). |
| Design dialectic candidate artifacts | Rust owns candidate validation, promotion, direct write, and stale cleanup in `crates/larch-core/src/design/dialectic.rs` and `crates/larch-cli/src/design_dialectic_commands.rs` (#8584). Persisted plan, promoted-candidate, status, and generation reads are confined below a canonical non-symlink design root and opened without following symlinks; candidate publication uses private mode-0600 atomic replacement. Drafter content is an explicit untrusted input path, not a publication target. All candidate, status, digest, and request files remain private session state. |
| Design pause and resume | Rust owns marker parsing, issue identity checks, verified cache lookup, confined restore staging, and marker cleanup in `crates/larch-core/src/design/pause.rs` and `crates/larch-cli/src/design_pause_commands.rs` (#8589). |
| Run-log archive, sync, and object publication | Rust owns archive creation, materialization, standalone sync, shared lifecycle publication, cache promotion, and `run-log storage-preflight` through `crates/larch-adapters/src/run_lifecycle.rs`, `google_storage.rs`, and `s3_storage.rs`. The same provider-neutral object-store port validates pagination, names, sizes, archive materialization, and repair rollback. |
| Agent diagnostic bounds and carriers | `crates/larch-core/src/vendor_diagnostics.rs`, `crates/larch-adapters/src/vendor_diagnostics.rs`, and `crates/larch-cli/src/launcher_support.rs` |
| Bgjob DEAD diagnostics | `crates/larch-core/src/bgjob_daemon.rs` owns the status and bounded-tail codec; `crates/larch-cli/src/bgjob_commands.rs` owns supervised capture and rendering |
| Residual Bash egress call sites | Thin scripts call the Rust redaction or run-log owner through `scripts/larch.sh` before forwarding untrusted content; plain shell error helpers are not independent redactors |
| Tier B public-file validation | `crates/larch-core/src/stall_recovery.rs`, `crates/larch-adapters/src/stall_recovery.rs`, and `crates/larch-cli/src/stall_recovery_commands.rs` |
| Stall classification, normalization, attempts, and escalation ledgers | `crates/larch-core/src/stall_recovery.rs`, `crates/larch-adapters/src/stall_recovery.rs`, and `crates/larch-cli/src/stall_recovery_commands.rs` |
| Tracking, plan, PR, diagram, and public-report publication | Rust `tracking-issue upsert-summary` (#8346) owns marker-keyed comment mutation through the shared issue-mutation owner. Rust `diagrams upsert` (#8837) composes, redacts, authorizes, mutates, and exactly verifies the shared `larch:diagrams` comment through that typed owner. Rust `tracking post-issue` (#8789) composes its confined private metadata file and calls that owner in process. Rust `pr create` and `pr body-update` (#8790) use `larch_core::redact_pr_body` before the typed GitHub mutation boundary; the latter verifies the returned body. Rust `final-report write` (#8090) owns `/implement` final-summary publication. On the supported Unix runtime, marker-comment and post-admission issue-body materialization write only below a canonical non-symlink process temporary root or larch session-cache root and use no-follow reads plus private atomic writes. `render run-summary` remains a #7680 `/design` payload renderer. Rust `implement code-flow-diagram` (#8933) and compatibility entrypoint `diagram code-flow` (#8839) use one in-process generator owner. `final-report write` calls the same Rust owner in process to preserve its output envelope. |
| Runtime projection | `crates/larch-cli/src/release_plugin_runtime.rs` |

Verification includes recorded black-box contracts for all four redaction
commands, focused in-process redaction and run-log tests, clean-install
dispatch, Rust Gitleaks command tests, Markdown and reference checks, runtime
projection generation and validation, the local pattern scan when installed,
and the required CI scanner jobs. Scanner success does not supersede the
confidentiality classes in this document.
