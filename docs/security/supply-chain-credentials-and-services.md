# Supply Chain, Credentials, and Services

This document is the canonical security reference for larch release provenance,
dependency controls, bootstrap and upgrade verification, credential handling,
transport policy, and typed external service boundaries. The root
[`SECURITY.md`](../../SECURITY.md) keeps the public summary.

Use the existing operational and architecture documents with this reference:

- [`docs/installation-and-setup.md`](../installation-and-setup.md) owns
  credential setup, installation, and upgrade instructions.
- [`docs/configuration-and-permissions.md`](../configuration-and-permissions.md)
  owns credential-related environment variable configuration.
- [`ARCHITECTURE.md`](../../ARCHITECTURE.md) owns dependency direction, adapter
  structure, and release constraints.
- The [GitHub service inventory](../github-service-inventory.md) and
  [Google service inventory](../google-service-inventory.md) record current
  operation owners and historical cutover evidence.

## Supply Chain

### Dependency controls

Rust dependencies are reproducible through the tracked `Cargo.lock` and pinned
`rust-toolchain.toml`. CI's required `rust-gate` runs a full-SHA-pinned
`cargo-deny` action against `deny.toml`. It rejects known advisories, unapproved
licenses, duplicate versions, wildcard requirements, and unapproved registries
or Git sources. [`ARCHITECTURE.md`](../../ARCHITECTURE.md#dependency-policy)
owns contributor instructions for dependency changes.

### CI tool bootstrap and caches

`.github/main-cache-inventory.json` is the cache-class inventory. Its canonical
key definitions live in `.github/actions/main-cache-keys/action.yaml`; every
validation restore and every trusted publication uses those same exact keys.
Actions cache version identity also binds the declared cache path. Lookup-only
publisher probes and saves therefore use the exact validation restore paths.
Temporary candidate and verification paths begin only after a genuine miss;
the publisher materializes the verified payload at its canonical path before
saving it. This prevents either a false publisher miss from requesting an
artifact that a cache-hit validation run correctly did not stage or a
path-distinct save that validation cannot restore.
The `CI` workflow handles pull requests, merge groups, and manual diagnosis.
It is read-only for production caches. A normal push to `main` runs only
`.github/workflows/main-cache-publication.yaml`, whose admission job refuses
any event or ref other than `push` or `workflow_dispatch` on
`refs/heads/main`.

The publisher has only `actions: read` and `contents: read` permissions and a
newest-wins `main-cache-publication` concurrency group. Lightweight classes
(pre-commit packages, agent tools, and gitleaks) are populated
directly on exact misses. Those jobs install or verify their pinned inputs and
save the matching cache, but deliberately do not run a lint, test, or secret
scan.

Rust CI caches Cargo registry and Git inputs separately from compiler output.
Their versioned keys bind the runner operating system and architecture,
lockfile, root and crate manifests, and pinned toolchain. The lint dependency
cache is a separate `target/debug` entry; `cargo clean --workspace` removes
workspace products before it can become a candidate. The coverage
compiler-dependency cache is a separate, versioned `target/llvm-cov-target`
class with a reviewed 1,400,000,000-byte dependency-only limit. Its exact key
also binds target triple, coverage tool, compiler-profile values, feature mode,
linker choice, Cargo configuration, and schema. It has no broad
`restore-keys` fallback. Any bound above 2 GiB needs explicit PR evidence that
transfer cost remains net-positive.

The `gitleaks` and `agent-sync` bootstrap jobs, plus only the
`test-harnesses` shard that runs `test-hook-anti-read-poll`, restore the exact
Cargo-input and lint-dependency classes read-only. They use the producer's
non-incremental, no-debug Cargo profile and then build or run the current
checkout; no restored workspace executable is trusted. These consumers have no
restore-key fallback or cache save step. A primary-key miss therefore lets Cargo
rebuild from the current checkout without using an inexact cache.

The trusted publisher's `main-cache-merge-group-source` job uses the same
exact, read-only Cargo-input and lint-dependency restores before it invokes the
typed resolver against its checked-out final `main`. Its summary records both
cache-hit values and resolver seconds; together with the canonical key in the
cache action evidence, those values support comparable exact-hit and
Cargo-graph-miss samples. The job does not save a cache or run a restored
workspace executable; an unavailable exact cache rebuilds the resolver from
that checkout.

On a successful merge-group run, an exact miss may stage a candidate artifact.
The publisher uses `larch ci-timing merge-group-source`, a typed Actions
operation, to prove that the newly landed `main` SHA has exactly one successful
`CI` merge-group run for that SHA and that its `rust-coverage` aggregate,
`rust-full shard 1` cache producer, and `rust-lint` cache producer succeeded.
The resolver accepts only a lowercase 40-character
commit SHA, queries at most 100 completed `ci.yaml` merge-group runs filtered
to that SHA, fails on missing or ambiguous evidence, and emits only that run's
numeric identifier. The publisher then downloads only named artifacts from
that run. Before a candidate reaches an Actions cache, the publisher checks its
schema, cache class, canonical key, and key-input digest (the SHA-256 of that
full canonical key), source SHA, producer job, merge-queue ref, artifact name
and deterministic payload digest, declared tool versions, maximum byte bound,
manifest member paths, regular-file shape, checksums, modes, and bounded
nanosecond modification times. The versioned manifest binds each timestamp into
the payload digest. Promotion restores modes and exact modification times
through pinned regular-file descriptors, then verifies both values. Symlinks,
unsafe timestamps, and unexpected tree entries fail closed. Cargo inputs, Rust
lint dependencies, and coverage dependencies use freshness-versioned cache keys
so a cache created before timestamp restoration cannot satisfy an exact hit.
Candidate artifact uploads explicitly retain hidden files: the manifest binds
every regular payload member, including dot-prefixed Cargo cache entries, so a
transport that silently omits one cannot publish a partial cache payload.
Cache data is never an artifact-provenance substitute and a cache hit never
skips correctness checks or artifact handoff.

Before staging the coverage target candidate, the validation workflow uploads
the coverage report and verified Linux executable, removes profile data,
reports, timing output, and workspace products, verifies that no workspace
binary or test executable remains, and uploads its directory inventory. The
publisher repeats executable digest and version checks for the pinned Cargo
tools before saving them.

The only manual compiler-output publication is an explicitly selected,
main-ref coverage-target benchmark. Its job is gated to `workflow_dispatch` on
`refs/heads/main`, uses a separate `coverage-target-deps-benchmark-*` key, and
accepts a decimal size bound no greater than 2 GiB. It cannot run from a pull
request, shares no key with the production cache, and cannot make the
production path restore or publish a target cache. During that benchmark
dispatch, the full shard and policy path remains the cache-off control. The
first zero-bound run measures and inventories dependencies without saving;
later benchmark runs use
that measured bound to compare warm candidates against the concurrent normal
coverage control. This scoped measurement exception does not change the
trusted-main production-publication rule.

CI does not delete Actions caches as part of this policy. A future collector
must first establish repository quota pressure or eviction of useful immutable
entries, limit deletion to this repository's versioned Rust-cache prefixes,
protect current keys, run only from a scheduled or manual trusted event, and
exercise selection offline before a network mutation.

`cargo-nextest` and `cargo-llvm-cov` are independent, versioned Linux tool
caches. On a miss, CI downloads the exact pinned release archive with bounded
retries and timeouts, verifies its SHA-256 before extraction, accepts only the
expected regular archive member, and installs it with an explicit mode. Before
use, including after a cache restore, CI verifies the installed binary SHA-256
and reported version. The trusted publisher repeats those checks before saving
an artifact-derived tool cache. Coverage timing artifacts explicitly record
cache restore and whether candidate staging ran or was skipped; a manual
dispatch is validation-read-only except for the separately named, main-only
target-cache benchmark described above. CI has no `cargo install` fallback for
either tool.

The dedicated `gitleaks` job builds the typed Rust history resolver, then uses
`ci gitleaks-base` through `scripts/larch.sh` to derive the history boundary.
That resolver proves the checked-out `HEAD`, resolves an ancestor merge base
against `origin/main` when available, and falls back only to `HEAD^`; it fails
closed if neither boundary can be proven. The scanner itself remains a fixed
Linux `v8.18.4` release with a fixed archive SHA-256 and extracted-binary
SHA-256 in the workflow. The cache key binds the runner OS and architecture,
scanner version, and binary digest. On every restore, the job requires real
cache directories and a regular non-symlink binary with the expected SHA-256
and reported version. Before each scan, it rechecks the binary's regular-file
shape, SHA-256, and reported version. A miss or invalid entry downloads only
over HTTPS (including
redirects), with bounded retries, timeouts, and a 16 MiB archive-size cap; it
verifies the archive digest, then
requires the exact `LICENSE`, `README.md`, and `gitleaks` member allowlist,
extracts only the regular `gitleaks` member into a private temporary directory,
verifies the binary digest, and installs it with mode `0755`. Invalid material
is never executed. The scanner inherits only its execution prerequisites
(`PATH`, `HOME`, `TMPDIR`, and `LANG`) plus noninteractive Git behavior; GitHub
and service-credential environment variables are not passed to it.

The trusted publisher may save the scanner cache only after this bootstrap
verification succeeds on `main` and only on a primary-key miss. It performs no
scan. Pull requests and other validation events may restore a cache entry but
cannot publish one, so a pull-request-provided executable cannot cross into the
trusted-main cache. The validation job retains the full-history checkout,
working-tree `--no-git` scan, and bounded `<merge-base>..HEAD` history scan as
independent required steps. Its named workflow steps and cache-hit summary
record the checkout, preparation, working-tree, and history timing phases for
cold and warm comparisons.

Each `rust-full-shards` coverage job builds the `larch` CLI under the same
instrumented target directory and Cargo test profile as its full workspace
test partition. Every cell uploads a uniquely named LCOV artifact. Shard 1 is
the only cell permitted to run plugin validation or stage cache candidates.
It also runs the bootstrap integration harness with the coverage-target
executable after verifying that executable's checksum and version. The
parallel `rust-full-policy` job installs only the pinned coverage tool, builds
an instrumented `larch` binary without the workspace test executables, runs the
single repository-policy scan, and uploads distinct LCOV and per-rule timing
artifacts. The selected `rust-partial` and `rust-skip` producers run the same
bootstrap integration contract with their candidate-built or trusted-main
executable. Every selected producer runs lifecycle start/finalize through
`scripts/larch.sh`, proves profiles cannot escape into the temporary client
repository, and runs the findings-classification contract before the stable
`rust-coverage` check can pass. In full mode, the parallel
`rust-full-lcov-tool` job installs the exact pinned Ubuntu LCOV package. It
archives only files owned by that package and the dependencies installed or
updated by the same transaction. It rejects unsafe package paths, verifies the
extracted LCOV 2.0 runtime, and uploads its archive, package inventory,
exact-version metadata, and SHA-256 manifest under the fixed coverage prefix.

The stable `rust-coverage` job first requires the complete matrix, policy job,
and runtime preparation to pass. It downloads all same-run coverage inputs with
one fixed prefix. Before extraction it requires the exact four regular,
non-symlink tool files, verifies every checksum and the pinned version metadata,
caps the archive at 64 MiB and 16,384 entries, and rejects absolute or
parent-traversing archive paths. The resolved executable must remain inside the
extraction root and report the expected LCOV 2.0 package version. The job also
requires the exact policy and numbered shard paths and rejects any report count
other than one more than the configured test-shard count. Every report must be
regular and non-symlink. The prepared LCOV runtime merges those inputs through
its parallel add-tracefile path. The unchanged 88% line threshold is then
calculated from LCOV's generated `LF` and `LH` totals in the merged report, and
malformed totals fail closed. The job uploads only that merged report under the
legacy stable artifact and member names.

On an exact Rust-policy miss, shard 1 of a successful full-mode merge-group
run stages and verifies a policy-cache candidate after the coverage target has
been pruned. No other shard can stage or upload that candidate. Shard 1 verifies
the preserved integration artifact's regular-file shape,
existing SHA-256, Rust-input digest, source SHA, and version before copying it,
then proves the staged executable against that same checksum. Its internal
provenance is the fixed `merge-group` label; pull requests, manual runs, and
other full lanes cannot stage a publishable policy candidate. After generic
candidate verification proves the final `main` SHA, the trusted publisher alone
rewrites that one provenance field to `refs/heads/main` and rechecks the bundle.

The `trusted-main-rust-policy` cache is a distinct executable cache, not a
compiler-output cache or an artifact-provenance substitute. Only the trusted
publisher may save it, and only after an exact successful merge-group source
for the newly landed `main` SHA produced the shard-1 coverage binary and the
stable aggregate proved both its plugin validation and the same-SHA policy
job. Its exact key binds the runner OS and architecture plus tracked crate Rust
sources (not generated target output),
root and crate manifests, root or crate build scripts, lockfile, toolchain, and
Cargo configuration. It has no restore-key fallback.
For a pull request, the isolated base checkout's trusted cache-key action
derives both the exact lookup key and the expected Rust-input digest. Candidate
files cannot choose either value. CI then checks every expected member is a
regular, non-symlink file; verifies the executable checksum; matches that base
digest; requires `refs/heads/main` provenance; validates the recorded source
SHA shape; and compares the executable's reported version. The selection job
supplies the executable only to the trusted pull-request-base wrapper. A miss,
corruption, unpublished base-input identity, or metadata mismatch produces a
static `full` selection without compiling or executing pull-request code. The
skip lane is the only consumer of an artifact handoff, so selection uploads the
verified cache files only when `skip` is the effective mode. The skip lane
repeats the same checks after that handoff.

Skip enforcement is enabled only after its independent pull-request observation
window records the required live evidence. Cache restoration and verification
authorize only the trusted-base selection command and the existing verified
handoff: an unavailable or invalid trusted main artifact leaves the proposed
and effective mode `full`. Once enabled, the same cache checks are required at
both the selection and handoff boundaries.

### Release provenance and attestations

The tag-triggered Rust asset workflow checks out the exact tagged projection
commit. It requires the tag, `.claude-plugin/plugin.json`, and Cargo workspace
version to agree. It builds and runs the only supported target,
`aarch64-apple-darwin`, natively. The workflow packages only `larch` and
`LICENSE` with normalized archive metadata.

Each matrix job attests its archive through GitHub artifact attestations. The
collector accepts only one archive and one metadata fragment for each required
target. It rejects missing, duplicate, empty, unexpected, mismatched, or
non-deterministic inputs. It recomputes archive sizes and SHA-256 digests,
emits the schema-v1 manifest and checksum file, attests both, verifies all
three attestations through the typed Rust GitHub attestation capability, and
revalidates the final three-file allowlist before upload.

The attestation service verifies only `character-ai/larch` artifact provenance
and immutable-release attestations. Domain callers cannot set a repository,
workflow, issuer, signer identity, trust root, API path, or absolute URL.
Artifact verification requires a valid Sigstore chain, SCT and Rekor evidence,
the GitHub Actions OIDC issuer, exact release-workflow identity, repository,
tag ref, source commit, `github-hosted` runner evidence, and one matching named
SHA-256 subject. Immutable-release verification uses GitHub's separate embedded
trust root and release identity. It verifies the signed timestamp and signature
and requires the release tag, source commit, repository, and complete unique
asset name and digest set. Missing fields fail closed.

API bodies, bundle counts, compressed and decompressed bundle bytes, redirects,
and deadlines are bounded. A response-supplied bundle URL is accepted only on
the exact HTTPS `tmaproduction.blob.core.windows.net/attestations/` path family.
Cross-host redirects, URL credentials, fragments, loops, and hop overruns fail.
Authorization stays on `api.github.com` and is not attached to the bundle-store
request. Errors retain only a fixed class and optional HTTP status. Tokens,
authorization headers, signed query strings, certificate paths, and bundle
content do not enter diagnostics. Release publication consumes this service
directly through its Rust owner. It has no `gh` fallback.

GitHub provenance ties bytes to a commit and workflow, not source or
infrastructure trust. Checksums index integrity, not trust. `/release` merges
the version candidate through the normal queue, resolves GitHub's recorded
post-merge `main` commit, builds a projection commit with that commit as its
first parent, then tags the projection and uploads only the validated
three-file set to a mutable draft. It rechecks the merge identity, projection
parent, ancestry, and versions, publishes without Latest, verifies every
immutable asset, then promotes. Failures resume the same draft or release.
Published tags and assets never change. Installation verifies separately.

### Release content pin

A release version names one synthetic projection commit, and both halves of an
install derive from it. The projection's first parent is the merged release
commit on `main`; its tree matches that parent except for the generated
`plugin/` subtree. `release stage` records the previous `stable` tip as the
projection's second parent, unless that tip is already an ancestor of the
merged commit, as it is for the first release after the cutover.
`.claude-plugin/marketplace.json` pins its `git-subdir` source to that branch,
so no merge to `main` can change what an install receives.

`release finish` fast-forwards `stable` to the tagged projection commit last,
only after immutable publication, release and asset attestation verification,
and Latest promotion succeed. It then re-reads the remote branch and fails the
release when the branch does not name the tagged projection commit. The push
carries no force and no lease. The projection's ancestry therefore keeps the
update fast-forwardable, and the pin can only advance. A published release
whose pin did not advance fails `release finish` rather than reporting success,
because no installer would see it.

`release stage --dry-run` rehearses the projection with no tag, push, or
draft. `.claude/skills/release/references/first-projection-release-runbook.md`
holds the rehearsal and the rollback plan.

Version-string equality is not content identity, so the pin is verified
separately at install time. See [`../../ARCHITECTURAL_INVARIANTS.md`](../../ARCHITECTURAL_INVARIANTS.md)
`I-Release-1`.

### Bootstrap and atomic installation

`scripts/larch.sh` is the only clean-install exec shim. It
maps the host target for binary identity checks, installs releases only on
Apple Silicon macOS (`aarch64-apple-darwin`) and fails release install and
preflight closed on every other host, and verifies the exact immutable
release, tagged projection commit, asset allowlist, build attestations, strict
manifest and checksums, sizes, digests, platform identity, and raw USTAR
layout. It rejects symlinks, special files, traversal, extra members, malformed
archives, and trailing data before extracting only `larch`. When
`CLAUDE_PLUGIN_ROOT` is unset or empty, the shim
derives it as the parent of its own resolved `scripts/` directory and exports
that validated absolute path before any further bootstrap work; an explicit
`CLAUDE_PLUGIN_ROOT` still wins and remains subject to the same absolute-path,
non-symlink directory checks.

The deny and anti-read-poll hook wrappers set
`LARCH_BOOTSTRAP_NO_INSTALL=1`. In this mode, the shim still validates and
executes a matching `LARCH_BINARY` or installed `bin/larch`, but it exits with
the distinct status 97 before lock creation, download, or installation when
neither is valid. The hook wrapper then applies its own fixed allow or deny
fallback. The fail-closed deny wrappers keep denying on status 97 but name the
one-command bootstrap repair in the deny reason, embedding the plugin root only
when it contains no JSON-significant or shell-hostile characters; every other
failure keeps the static reason. Hooks still never download or install an executable. The mode
does not intercept the explicit `--preflight-release` or
`--latest-stable-version` actions.

The staged binary must pass `--version` and compact-JSON
`larch bootstrap self-check`. A same-directory rename installs it atomically.
An existing regular binary retains a hard-link rollback through post-install
verification. A bounded `CLAUDE_PLUGIN_DATA/bootstrap.lock` serializes first
use, reclaims revalidated dead-owner locks, and makes waiters re-check before
downloading. Cleanup removes only process-owned state. Local `.git` checkouts
require an explicit matching `LARCH_BINARY` for direct shim use. The GCS
run-log adapter may lazily run the locked `larch-cli` release build in that
trusted checkout, then call the shim with the resulting path in a
process-scoped `LARCH_BINARY`. It never downloads a release into the checkout,
and installed plugin roots never take this build path.

These controls do not defend against a hostile same-UID process that can rewrite
plugin cache or data files. Runtime lints reject `cargo run` and `cargo install`
in production. They also reject `bin/larch` and
`target/{debug,release}/larch` execution. The GCS adapter confines its
`cargo build` output and does not execute the resulting binary directly. Only
verified bootstrap owners may do that. The command registry requires each live Rust-owned
selector to name a unique shared clean-install fixture. Those fixtures start
without `bin/larch`, verify version and target before dispatch, and invoke the
selector only through `scripts/larch.sh`. Issue-registry audit input is typed
JSON derived from the canonical owner and plan parsers. It is validation
evidence, not executable input.

The `service-ownership` rule rejects runtime `gcloud` execution. It also keeps
clean-install `gh` use in `scripts/larch.sh` separate from the fixed runtime
credential lookup. Neither path authorizes `gh` API calls from a runtime
adapter. Its GitHub operation matrix fails on ownership or migration-state
drift, including chief-issue placeholders, inventory gaps, and generic-token
fallback.

### Upgrade and rollback boundaries

`/upgrade-larch` never writes install stamps or recursively deletes, prunes, or
edits Claude-managed plugin version directories. Its one write outside its own
staging is the byte-identical registry rollback described below. Claude Code owns orphan
retention so active sessions keep their original roots. The installed Rust
driver invokes only Claude, validated larch executables, and the bounded
`scripts/larch.sh` bootstrap exception. Only bootstrap children inherit the
GitHub CLI auth and config allowlist. Claude and self-check
children do not.

Before any marketplace mutation, the current root's bootstrap verifies the
exact immutable stable release, complete asset allowlist, attestations,
manifest, checksums, archive, target, and staged binary identity in confined
`${CLAUDE_PLUGIN_DATA}` staging. It also verifies that the pinned `stable`
branch is at that release's tagged projection commit and reports the proof as
`LARCH_PREFLIGHT_PIN_VERIFIED=true`. The driver requires both that proof and the
preflighted version before it touches the marketplace, so a content-and-binary
mismatch is refused while the prior installation is still the active one rather
than after it has been replaced. First-use bootstrap does not gate on the pin,
because the branch moves ahead of installs that are deliberately on an older
release. The driver then uses supported Claude plugin
commands and resolves exactly one new cache root through
`claude plugin list --json`. Success requires the new root's manifest and
executable to report the expected version. A failure leaves the prior cache root
untouched and prints retry commands.

`claude plugin install|update` moves the active root for every new session
before the new root's `bin/larch` exists, and no Claude command moves it back.
The driver therefore snapshots `~/.claude/plugins/installed_plugins.json`
through a no-follow confined read immediately before that command, and again
right after it. When the command exits non-zero after rewriting the registry,
or the new root's executable fails to materialize or verify, and the registry
bytes changed, the driver restores the byte-identical snapshot through the
confined atomic writer with the original file mode, then re-reads
`claude plugin list --json` and re-verifies the prior root's executable before
it reports the rollback. Movement is decided from the registry file itself,
because that file is what new sessions read. The driver never edits registry
content. It skips the restore when the registry is unchanged or no regular
snapshot exists, and it refuses the restore when the registry no longer matches
what the install wrote, because another process then owns the newer content.
That comparison runs immediately before the rename; a write that lands inside
that window is not detected. Every outcome names the root new sessions resolve
now, prior or new, and prints the exact bootstrap command that installs a
missing executable there (#9097).

Bootstrap cleanup removes only its current staging directory and lock under
`${CLAUDE_PLUGIN_DATA}`, or its current same-filesystem binary stage. The
dev-only `/release` flow builds the released working-tree binary and routes it
through `scripts/larch.sh` with the validated `LARCH_BINARY` override. Its
internal `--plugin-root` argument keeps upgrade state bound to the separately
validated installed cache root. The `release-python-free` rule pins the final
command set and rejects restoration of the retired runtime, direct-binary, and
direct-`gh` fallback drift.

### Release-version transaction

`release set-version` accepts one semantic version and a fixed repository-owned
file inventory. It validates the current plugin, workspace, internal path
dependency, member, lockfile, and optional runtime-projection versions before
writing. Each write uses the confined atomic UTF-8 adapter and preserves file
mode. A write or postcondition failure restores every original file in reverse
order and reports rollback failures. The command exposes no caller-selected
rewrite path or content surface.

A hard process termination between final-file publications can still leave a
partial update. Replay fails closed on inconsistent old versions instead of
continuing from mixed state.

## Credentials and Transport

### Google Application Default Credentials

Google credential configuration is trusted operator input. Larch accepts it
only through the standard Application Default Credentials search order:
`GOOGLE_APPLICATION_CREDENTIALS`, the well-known local ADC file, then the
attached-service-account metadata service. Repository content, issue text,
workflow data, API responses, and agent output cannot supply credential JSON,
paths, quota projects, scopes, endpoints, or universe domains.

Before the official Google authentication builder reads a selected ADC file,
larch bounds and parses it. External-account token exchange must use
`https://sts.googleapis.com/v1/token`. Impersonation must use the documented
`iamcredentials.googleapis.com` access-token path. AWS and Azure subject-token
URLs must match their documented metadata endpoints. Executable subject-token
sources and custom universe domains fail closed. Production rejects the
test-only `GCE_METADATA_HOST` override, so an inherited emulator setting cannot
redirect attached-service-account authentication.

`google-cloud-auth` owns access-token exchange, caching, and refresh. Larch does
not shell out to `gcloud`, copy ADC files, expose authorization headers, persist
tokens, or create a credential store. Errors retain only a stable failure class,
not credential values or credential paths. Concrete Google service clients are
added only for operations recorded in the
[Google service inventory](../google-service-inventory.md), with explicit
least-privilege scopes and IAM permissions. Offline tests use local fixtures.
Live ADC tests are ignored by default, require explicit opt-in, and do not render
credential headers.

### Vendor process, descendant, and diagnostic boundary

Every larch-owned Claude, Codex, and Cursor binary launch enters through
`${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh` and a Rust command. `larch-core` builds
a typed `ProcessRequest` from the closed `VendorProgram` enum and the
`ExternalProcessRunner` port. `TokioProcessRunner` in
`crates/larch-adapters/src/process.rs` is the sole production spawn owner.
Skills, hooks, and scripts do not construct vendor processes. The retired
Python agent package is not a fallback.

The adapter clears the ambient child environment and restores only reviewed
common keys. Vendor credentials never enter that common allowlist. A launch
must add an approved typed `ChildEnvironment` override, such as
`OpenAiApiKey` or `CursorApiKey`; argv and persisted replay metadata remain
credential-free. Stdin, stdout, and stderr are bounded, and operator-facing
failure diagnostics pass through the shared redaction and truncation carriers
before rendering or publication.

Each Unix child starts in its own process group. Before cancellation or timeout
signals it, the adapter snapshots the descendant tree, groups those processes
by PGID, and captures kernel birth identities for live members. A separate
group is owned only when its leader was also captured in that tree. The adapter
sends SIGTERM from the deepest descendant group through the direct child's
group, waits the configured grace period, then refreshes the tree and
revalidates a saved member before each SIGKILL. A live group with no valid
ownership anchor fails cleanup instead of receiving a signal through a bare
PGID. The adapter then reaps the direct child. This reaches nested groups after
their parents are reparented without signaling an unverified PID. Other
platforms use Tokio's safest direct-child kill and reap path; the released
runtime is Apple Silicon macOS. This lifecycle is shared by reviewer,
implementer, drafter, probe, debate, and CI launches rather than reimplemented
per command.

### Vendor credential preflight and the reviewer-probe cache

`agent cursor-auth-preflight` runs in Rust through
`${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh`. It proves that Cursor can
authenticate before a Cursor lane starts, so Cursor never fails in-process and
returns a canned, un-reviewed response. A usable `CURSOR_API_KEY` clears the
preflight without any keychain access, and a non-Darwin host has no keychain to
consult.

On Darwin with no usable `CURSOR_API_KEY`, the preflight reads the
`cursor-user` / `cursor-access-token` keychain item with `security
find-generic-password -w`. The read is bounded, read-only, and runs under the
shared vendor startup lock with a fixed attempt budget. `security` is a closed
`HostUtilityProgram` allowlist entry, so the read uses the one approved
external-process layer rather than a second spawn path. Reading the secret,
not testing for its existence, is required: an access-controlled item can pass
an existence check and still deny the read.

The Rust preflight never mutates its own process environment. A resolved token
lives only inside `CursorCredential`, which redacts itself in `Debug`
rendering, exposes its value through one explicit accessor, and rejects a value
carrying an embedded newline or carriage return so it cannot splice a second
assignment into a child environment. The credential reaches a vendor only as a
typed `CURSOR_API_KEY` child override on an approved process request. It never
enters stdout, an operator message, a probe stamp, or a gate-detail artifact.

The reviewer-probe cache stores verdicts and Codex gate details under one
user-scoped temporary root with mode `0600`. Every entry is confined before
use, and symlinked or non-regular entries are refused rather than followed. A
positive verdict and a failing verdict carry separate lifetimes, so caching a
failure can be disabled independently. Concurrent Codex probes serialize on a
per-identity exclusive lock, so exactly one probe runs and every waiter then
observes the same published gate detail. A cached gate detail is re-derived
against the canonical gate renderer on read, so a hand-edited or corrupted
cache entry cannot inject operator-facing text into the degraded-tools
explanation.

Isolation for Cursor probes and `cursor agent models` is owned by
`CursorProbeSession`, which holds the private configuration directory and the
resolved credential together. Both are released when the session value is
dropped, so success, failure, timeout, and cancellation take one cleanup path.
The model-list child receives the same typed credential overlay as a reviewer
probe. It never inherits a Cursor credential through the broad child
environment allowlist.

### Slack issue-announce webhook transport

`slack issue-announce` runs in Rust through
`${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh`. It posts a short implement-run
announcement to the optional `LARCH_SLACK_WEBHOOK_URL` webhook. The URL is a
credential: the command never prints it, never writes it into STATUS/REASON/ERROR
rows, and redacts it from transport diagnostics before emission. Only `http` and
`https` schemes are accepted; other schemes fail closed. `--best-effort` maps
validation and transport failures to exit 0 while still emitting
`STATUS=failed`. The concrete HTTP client lives in
`crates/larch-adapters/src/http_client.rs`; core owns planning and redaction only.
The Rust Step 16/17 closeout composition forwards the webhook as a typed child
environment override only to `slack issue-announce`; timing, report, run-log,
and rejected-finding children do not receive it.

### Connectivity availability probe

`net wait-online` enters through `${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh` and
uses a credential-free fixed-endpoint probe. The adapter sends unauthenticated
`HEAD` requests only to `https://api.anthropic.com/` and
`https://api.github.com/`. It follows no redirects, sends no request body or
workflow credential, applies fixed connection and request deadlines, and treats
any received HTTP status as endpoint reachability. Transport errors collapse to
the offline state and never enter diagnostics.

Core owns the capped exponential-backoff policy, monotonic awake-time ceiling,
and seven-day hard maximum, plus probe counts and wait duration. The CLI exposes only `NET_ONLINE`,
`NET_PROBE_ATTEMPT_COUNT`, and `NET_WAIT_SECONDS`. The
`LARCH_TEST_NET_FORCE_OFFLINE=true` fault hook can only force the adapter
offline; it cannot select a URL, add headers, or weaken transport policy.

### Object storage credentials and transport

Cloud Storage uses the larch-owned `ObjectStore` port, the official Rust client,
and the hardened ADC boundary above. S3 and R2 use standard AWS credential
resolution. R2 also requires a matching account ID and an HTTPS endpoint on the
account's Cloudflare host.

A nested larch composition child (for example the Step 0 bootstrap
`run-log lifecycle-start`, or a Step 2 dispatcher spawned through
`run_verified_larch_env_in`) resolves credentials and operator overrides from
its own process environment, so the selectors the parent shell sets are
forwarded to it through the verified-larch child allowlist: the AWS profile,
config-file and shared-credentials-file overrides, static and session keys,
region selectors, the R2 account ID and endpoint, vendor API keys
(`CURSOR_API_KEY`, `OPENAI_API_KEY`, `ANTHROPIC_API_KEY`), and `LARCH_*` model
and probe overrides. They are scoped to nested larch children and never join
the generic child-inheritance list used by `gh` or vendor reviewer CLIs.

Repository-root `tools-config.toml` may select a credential-free storage base.
`LARCH_STORAGE_BASE_URI` may enable storage without a file value, but it never
hides an invalid present file. When neither source configures storage, remote
publication is disabled and larch constructs no provider adapter or command.
Larch still derives the client repository from local Git origin.

When storage is enabled, larch derives the fixed
`larch/<client-repo>/` scope. The provider credential remains the authority and
should grant list, read, and create-only write only for approved tool and
repository prefixes. Startup lists that exact prefix with a maximum of one
result; it never lists the bucket root or writes a probe. Any configured
provider failure blocks startup. The process ignores returned object names and
reduces provider failures to credential-free classes.

The provider-neutral transport accepts only validated bucket roots and object
keys. Uploads are create-only. Downloads use a private temporary file and atomic
promotion. Provider diagnostics are reduced to fixed, credential-free failure
classes. The [Google service inventory](../google-service-inventory.md) records
the Cloud Storage client, scope, permissions, operations, and current consumer
path.

Rust owns the shared run lifecycle and standalone `run-log publish` and
`run-log sync` commands, including terminal archive publication, cache
promotion, retry/resume, and synchronization. It uses the official Cloud
Storage client and the official AWS SDK with the credential-process feature
disabled for S3/R2. R2 suppresses unsupported optional AWS checksum headers;
publication downloads and hashes an existing create-only object before matching
it, while new objects are verified through returned and fetched metadata. No
run-log production route uses the AWS CLI. The complete hard-cutover boundary
lives in [Run-log storage contracts](../run-log-archive.md#rust-handoff).

The Rust-owned one-time `character-ai/larch#8081` layout migration uses the
same normalized S3 transport. Live plan, apply, and verify operations accept
only the fixed old and tool-first roots for the larch and agent-lint client
repositories. Apply requires an explicit live-migration authorization flag.
Final report publication requires a separate authorization flag. Both archive
and report uploads are create-only. Source objects, target objects, provider
credentials, and provider diagnostics are never mutated or disclosed by the
migration report.

### GitHub credential and transport boundary

The Rust GitHub service acquires exactly one credential by invoking the fixed
`gh auth token --hostname github.com` command through the core-owned typed
process operation. The process runner uses a clean environment, permits the
GitHub CLI configuration selectors, and excludes `LARCH_GH_TOKEN`, `GH_TOKEN`,
and `GITHUB_TOKEN`. Missing `gh`, an inactive login, and empty, truncated, or
non-Unicode output fail before network access with fixed guidance. The
credential is held by a non-`Debug` wrapper, registered by exact value with an
invocation-owned redactor, and omitted from child environments. Authorization
diagnostics pass through that redactor. The Octocrab build excludes its tracing
feature.

The adapter constructs one private Octocrab client inside the larch Tokio
runtime. Octocrab is pinned with default features disabled and only its rustls
AWS-LC client, timeout, and required JWT support enabled. Octocrab 0.54 requires
a JWT backend even though this adapter exposes token authentication only. Larch
selects AWS-LC because the alternative RustCrypto RSA graph carries an unpatched
advisory. `aws-lc-sys` builds its bundled C and assembly with CMake and a
platform C compiler. It adds no dynamic system-library requirement and is built
by the existing target release matrix.

The fixed credential lookup is the only normal runtime `gh` invocation for
Rust GitHub service access. GitHub API operations use the authenticated adapter
directly; `gh api` is never a service fallback, and `gcloud` is never a runtime
service fallback.

The Rust-owned PR lifecycle commands (#8790) use this boundary for PR lookup,
creation, body replacement, and check reads; authenticated-user assignment
passes through the shared `IssueMutationOwner` and a fresh read-back.
PR bodies are redacted before mutation, create conflicts reconcile by head
branch instead of retrying blindly, and body updates accept success only when
the typed mutation response carries the requested body. Branch creation and
push remain approved typed Git CLI operations; no PR command invokes `gh pr`.

Trusted GitHub Actions workflows that need the typed Rust service use the local
`github-auth-config` composite action. It accepts `github.token` only to create
a `0700` temporary `GH_CONFIG_DIR` through `gh auth login` over standard input,
then unsets token variables and verifies `gh auth status`. The following larch
process receives only that configuration-directory path, never `GH_TOKEN` or
`GITHUB_TOKEN`; this CI-only bootstrap does not call `gh api` and is not a
runtime credential fallback.

Redirects and retries are disabled. Larch sets `User-Agent` and `Accept`.
Pinned Octocrab supplies one API-version header. Both bases are pinned to
`https://api.github.com/`. Response-supplied continuations must remain HTTPS on
the same approved origin. The host policy also recognizes
`https://github.com` for typed download boundaries, but does not permit a
continuation to cross between the two origins.

Connect, read, write, and overall deadlines are fixed. Overall execution is
cooperatively cancellable. Response bodies and pagination are bounded. Only
reviewed transient failures from idempotent reads are retry inputs. Uncertain
mutations route to typed reconciliation instead of automatic retry. The core
service port exposes policy and typed transport classifications, not raw URLs,
arbitrary GraphQL documents, or the concrete client. Operation code must add
typed paths and DTOs behind this adapter. The `service-ownership` repository rule
confines the Octocrab client, GitHub request hosts, and GraphQL documents to the
adapter crate. The [GitHub service inventory](../github-service-inventory.md)
names client and operation owners. Parity fixtures independently block all
GitHub credential variables.

The read-only migration-governance aggregate is the documented
exhaustive-history exception: its dedicated typed client policy admits at most
100 pages and 10,000 raw issue-list rows, with a 256 KiB per-field limit for
historical plan bodies and a fixed three-minute aggregate deadline. Each page
read, including its bounded retry sequence, retains the ordinary 60-second
deadline. It retains the standard response-byte and nesting limits, and refuses
a larger corpus, field, or deadline overrun rather than silently dropping
historical managed leaves; ordinary GitHub operations retain the smaller general
transport bound.

Repository, issue, comment, label, and search responses are untrusted data. The
Rust operation adapter converts Octocrab models immediately into larch-owned
DTOs, rejects missing required fields and unknown states, and enforces
response-byte, page, item, string, and JSON-nesting limits before it returns
data to a caller. Pagination follows only parsed same-origin HTTPS
continuations. Issue titles, bodies, comments, labels, authors, URLs, and search
results must never become shell text, paths, format strings, or prompt
instructions.

Every raw REST row an issue list returns, pull requests and foreign-repository
rows included, counts against the page and item bounds; only matching issue rows
reach the caller, and the typed result reports both the returned rows and the
raw rows scanned so a caller never infers pagination from filtered length. Each
list caller declares exhaustive or bounded-partial intent. An exhaustive caller
still refuses with the transport-limit error when a continuation remains at the
bound, so fail-closed consumers cannot silently narrow. A bounded-partial caller
receives the admitted rows marked truncated instead, so a caller whose contract
already permits a visible partial snapshot is not converted into a false
network, authentication, or rate-limit failure at the page boundary. Reaching a
caller's own requested count is never a refusal and never a transport-limit
truncation.

The Rust-owned run audit reads pull requests through the same boundary's
bounded audit DTO: number, title, body, base ref, and RFC 3339 merge time only.
Its REST listing is constrained to closed `main` rows, then filters merged rows
after bounded pagination. The complete-history audit uses a dedicated maximum
of 50 pages and 5,000 rows; it refuses a larger history rather than silently
truncating merge-time ordering. Malformed timestamps or response fields also
refuse the read. Audit corpus synchronization keeps its root private to the
local workflow; public report output remains limited to the artifact-reference
rules in the publication reference.

`token compute-pr-line-counts` and `token compute-pr-lines` read the fixed
pull-request files REST route through this boundary, never through `gh api`.
The adapter validates a positive pull-request number and repository segments,
admits at most 100 typed rows per page, applies the general page, item, string,
body, and nesting bounds, and accepts only a bounded filename plus unsigned
addition and deletion counts. The command aggregates those values into code
and `larch-logs/` buckets. Any setup, transport, bound, or response-contract
failure degrades to the established `LINES_STATUS=unavailable` envelope without
printing untrusted response data.

Idempotent reads have bounded retry and honor a structured `retry_after` value
when GitHub supplies one. Mutations are serialized by their caller and are not
blindly retried. Issue edits and closes, comment edits and deletes, and label
changes read back the owning resource after an ambiguous transport outcome.
They return success only when the requested postcondition is present. Creates,
which lack a collision-free request identity, return a typed ambiguous-outcome
error instead of risking a duplicate issue, comment, or label.

`design file-oos-annotate` follows this boundary for its `oos-correctness`
label. It lists or creates the label through the typed service and applies the
issue label through `IssueMutationOwner` with verified read-back. The command
does not invoke `gh label`, `gh issue`, `gh api`, or accept a caller-selected
service URL. Its only credential source is the fixed GitHub credential lookup
owned by this boundary. Automatic Step 5b calls also pass the session-backed
live-mutation context; direct recovery requires explicit operator mode.

## Typed Service Boundaries

The [GitHub service inventory](../github-service-inventory.md) is the canonical
typed operation ledger. It records the adapter and command owner for each
operation and the clean-install coverage that holds the final boundary.

### Release and asset operations

The release boundary exposes typed methods for bounded listing, duplicate-safe
tag selection, policy reads and writes, draft create and update, publish,
upload, and bounded download. Draft validation binds version, PR head, tag,
exact run, mutable draft, three assets, digests, `LICENSE`, and attestations before
merge. Tags use the closed typed Git adapter. Callers use `scripts/larch.sh`
and the typed service boundaries, with no raw `gh`, raw Git, arbitrary HTTP,
or fallback.
Publication and installation stay with their owning callers.

Ambiguous create, upload, edit, publish, and Latest-promotion outcomes read back
the owning resource. A landed effect succeeds without another write. An
ambiguous draft create is not repeated when a temporary placeholder may still
be absent from the list response; a later staging run adopts it by identity.
Other mutations retry only after the owning read proves absence. Publication
preserves the prior Latest release. Promotion occurs only after immutable asset
and attestation verification, and verifies the final Latest postcondition.
Policy and draft edits always read back their state. Draft updates carry the
tag, target commit, title, and body together so a temporary GitHub
`untagged-*` association can be repaired by release id without creating a
second draft. Clear mutation responses are validated directly; ambiguous
responses still require an owning tag read. Body reconciliation accepts only
an exact match or GitHub's addition of one terminal newline.

Asset download uses an operation-specific host policy that differs from the
same-origin API continuation policy. A download may leave the API origin for a
signed content host. Each redirect hop must stay HTTPS, carry no embedded
credentials, never revisit a prior URL, and stay within the hop cap. The
credential is withheld on every cross-origin hop. The streamed body is bounded
by a per-asset byte cap, must advertise the binary octet-stream content type,
and is rejected if it ends before its declared length. Downloads are
deadline-bounded and cancellable.

### Pull-request, review, and dependency operations

Pull-request, review, and dependency operations expose typed inputs only. The
fixed review-state GraphQL query fails closed on any `errors` member, including
partial data. Create reconciles ambiguity before retry. Merge uses the
live-mutation gate and validated repository, PR, exact lowercase 40- or 64-byte
head, and closed method inputs. Merge sends at most one request, then uses
bounded exact-head read-back after uncertainty. Result classes are fixed, and
untrusted response text never egresses.

The fixed issue-closure-reference GraphQL query accepts only a validated
repository and a bounded set of issue numbers. Its connection scan, page size,
and closure-reference fields are bounded; GraphQL errors, malformed pages, and
missing cursors return a typed failure. Backlog analysis records that failure as
degraded evidence rather than treating the closure field as complete.

Release preparation uses typed, bounded reads for the Latest release, PRs, and
companion issue titles. Publication fetches through the typed Git CLI adapter,
checks ancestry through gix, and uses typed release and attestation services. It
publishes without changing Latest, verifies the immutable release, and only
then promotes it. Ambiguous promotion reads back Latest before a retry. The
final Latest state is verified. The release commands expose no raw Git, `gh`,
URL, GraphQL, or alternate implementation fallback.

Release policy verification reads only the immutable-release setting. It does
not enable merge commits or immutable releases, and it never writes repository
settings, rulesets, or bypass configuration. A disabled or unreadable required
setting fails closed with a fixed, secret-free diagnostic.

The dev-only release skill builds the current checkout before its first
Rust-backed release command and rebuilds immediately after the candidate
version write. Every working-tree release command still enters through
`scripts/larch.sh` with the checkout root and release binary supplied
explicitly. This prevents an installed or same-version stale binary from
owning either side of the candidate-version boundary.

Issue-dependency list, add, and remove use the shared live-mutation gate:
operator mode, or a regular non-symlink session file directly under a canonical
root that carries `LARCH_LIVE_MUTATION_OK=true` and the matching run ID. Writes
are idempotent and exact-read-back verified. Triage calls require expected
`updated_at`, re-read the client before writing, reject stale or protected
targets, and return a new non-empty timestamp. Before each Rust dependency
write, the triage-controlled path also rejects exact `security` or
`vulnerability` labels and security-sensitive terms in the title, body, or any
comment. Comment and dependency lists follow parsed same-origin HTTPS `Link`
continuations under shared byte, page, item, deadline, and cancellation bounds.
Malformed or incomplete comment evidence fails closed. Unavailable APIs and
transport errors are typed and redacted.

### Repository metadata reads

`larch_adapters::git::GixRepository` is the sole production implementation of
the core `RepositoryRead` port. It opens and discovers repositories with `gix`
ownership checks enabled, rejects reduced-trust ownership, and parses config in
strict mode. Each mutable-state query reopens through the same checks so later
ownership or config changes cannot reuse an earlier trusted handle. The
location method returns the immutable repository identity captured by the
trusted constructor.

`larch gh remote-repo` and `larch gh resolve-repo` parse remote names and URLs
through this typed gix port and optional `GitHubService` metadata. Malformed or
hostile remote strings never become subprocess argv. They fail closed with the
legacy stderr contract. Service setup and metadata failures are retained for
origin fallback diagnostics and are never treated as instructions. These
commands do not invoke `gh` or an untyped Git subprocess.

The adapter performs local reads only. It exposes no mutation, network,
credential, or arbitrary Git command surface. Results preserve object IDs,
paths, config values, and remote URLs as bytes. Errors use fixed classes and do
not include repository paths, config values, remote credentials, or upstream
library diagnostics.

Status and typed tree changes follow the same reopen rule. Status uses the full
configured `gix` iterator and never writes its optional index-stat refreshes.
The strict `RepositoryRead::status` operation returns `UnsupportedSemantics`
before iteration for repository and worktree clean or process filter config. It
also rejects repository attributes that select conversion or configured filter
behavior. Typed tree changes reject configured textconv and external diff
drivers. Callers that need exact diff interpretation must route those
byte-sensitive cases through the closed exact-diff Git CLI operation.
Compatibility callers that consume only status and untracked names use
`GixRepository::local_status`; it retains configured filters and does not
promise exact diff semantics. User and system filter definitions remain
operator-owned config. Effective conversion attributes are queried through the
configured attribute stack. Discovery does not follow symlinks and fails closed
when the worktree traversal exceeds its entry cap. Typed results contain paths,
modes, IDs, and flags, but no file content or upstream diagnostic text.

### Git mutation compatibility

`git stage`, `git commit`, and `git amend-add` run through
`${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh`. The script is the sole production
bootstrap and version-validation entrypoint. Callers do not select an
implementation, execute `bin/larch` directly, or invoke Cargo.

Rust composes closed `AddRequest`, `CommitRequest`, and
`InterpretTrailersRequest` operations. The installed Git executable remains the
compatibility backend. Git owns hooks, clean and process filters, signing
programs, helpers, commit-message cleanup, index updates, refs, reflogs,
diagnostics, and exit status. Arguments are typed and byte-preserving. There is
no arbitrary Git argument surface. The process adapter clears ambient Git
repository overrides, inherits only its reviewed environment allowlist, sets
`GIT_TERMINAL_PROMPT=0`, bounds captured output, and terminates and reaps the Git
process group on timeout or cancellation.

All product Git mutations and network porcelain use the closed typed `GitCli`
adapter. Each method fixes its subcommand and validates refs, paths, remotes,
refspecs, config keys, and option combinations. Treat Git-launched hooks,
filters, signing tools, transports, credential helpers, merge drivers, and
editors as hostile. Bound and redact their output. Never accept a
repository-provided path as the `git` executable.

`docs/git-operation-inventory.md` is the checked ownership boundary.
`git-ownership` has no baseline or production suppression. It rejects inventory
or `gix` drift, direct or aliased Git construction, bound executables, raw or
generic argv, a widened typed surface, restored retired-runtime entrypoints or calls, and
the retired `push rebase` state machine. Only `#[cfg(test)]`,
`larch-test-support` fixture oracles, and the lint bootstrap are bounded
non-production exceptions.

Commit messages use a private temporary file. The command removes that file on
success, Git failure, hook rejection, signing failure, filter failure, and
cancellation. The default co-author trailer is prepared through Git's
`interpret-trailers` operation. `--no-trailer` skips only that operation.
Pathspec files may be absolute because recovery files live outside the
repository. Their paths reject empty, option-like, and NUL values. Repository
paths reject absolute paths, parent traversal, options, and NUL bytes.

Index-lock recovery is narrow. Larch removes only a regular, zero-byte
`.git/index.lock` after the repository's trusted Git directory is resolved and
no holder is found by `/proc` or the typed, bounded `lsof` host-utility probe. It
verifies removal, retries the failed Git operation once, and reports the
decision. Non-empty, held, unreadable, symlink, or unverifiable locks remain
untouched. Branch-write protection is checked before staging or committing,
including the persisted original-branch prohibition used by the ship workflow.

### GitHub Actions operations

The Actions operation port builds repository, workflow, run, job, and check
paths only from validated typed inputs. Reads retry a bounded transient set
within the overall deadline and cap pages, items, body bytes, strings, and JSON
nesting. Rerun and dispatch mutations are serialized. They honor numeric
`Retry-After` pacing before read-back and report an ambiguous outcome when the
read-back cannot prove the mutation happened.

The Rust-owned `ci status` and `ci wait` commands combine a typed pull-request
REST read, the fixed merge-state GraphQL query, typed check runs, a typed
combined commit-status rollup, a typed Git fetch, and local gix commit walks. A
non-timeout pull-request-state failure retains the legacy conservative
`UNKNOWN` conflict state and queries the fixed `pull/<number>/head` selector,
so the monitor can still consume independently validated check data; a deadline
remains a status failure. The commands are read-only and have no `gh api` or
alternate implementation fallback. `ci wait --output-file` publishes its bounded `KEY=value`
result and completion marker through the shared private atomic wire writer.

The Rust-owned `merge pr` and `merge wait` commands use the same credential and
typed service boundary. Fixed GraphQL documents expose review state and merge
queue eligibility, and the queue write exposes only `enqueuePullRequest` for a
validated pull-request node and expected head object ID. Direct merges carry
the same expected-head precondition.
Every uncertain write receives bounded read-back without mutation resubmission;
an unproved outcome is reported as ambiguous. Diagnostics remain redacted and
bounded before they enter the `KEY=value` result envelope.

Workflow log archives have a 64 MiB and 60 second limit. The adapter follows at
most three redirects and rejects loops, URL credentials, fragments, plaintext,
unexpected content types, and oversize or incomplete streams. Redirect hosts
are limited to the documented `*.actions.githubusercontent.com` suffix and the
`productionresultssa<digits>.blob.core.windows.net` storage family. Octocrab
adds authorization only for `api.github.com`, so cross-origin log requests do
not carry `Authorization`. They preserve the signed query. A production-auth
loopback test checks both hops. Failures return redacted errors.

`larch gh run-logs` emits selected failed-job log bytes unchanged to preserve
the legacy stdout contract. Callers must redact that output before writing it to
a model prompt, committed artifact, or other egress surface. The typed adapter
limits archive download and decompressed output to 64 MiB, limits archive
entries to 1,024, rejects malformed archives and oversized entries, and never
treats archive paths as local filesystem paths.

`larch ci-timing harness` parses untrusted workflow archives entirely in
memory. It applies the shared 64 MiB and 1,024 entry limits, caps entry-name
length, never extracts archive paths, and emits only schema-v2 timing fields
consumed by the rebalancer. The report includes the selected run identifiers
and bounded bootstrap diagnostics alongside target rows, so the consumer can
reject an incomplete cohort rather than infer missing startup cost. One timing
operation accepts at most 20 runs and retains at most 100,000 rows, 32 MiB of
label text, and 16,384 bytes per target. Harness input is also capped at 4,096
required targets. `larch ci-timing jobs`
derives harness wall-clock durations from typed Actions job records.
`larch ci-timing rust-jobs` uses the same bounded records for Rust coverage,
treats a legacy `rust-full` as one shard, and ignores the stable aggregate when
matrix jobs are present. Both report the same selected cohort.
`larch ci-timing merge-group-source` reads bounded workflow and job records to
resolve only a trusted producer run. All four commands use the Actions adapter
and the fixed GitHub credential boundary above; they do not call `gh api`,
accept raw URLs, or expose log text in their output.

## Implementation and Verification Owners

These owners and checks keep the boundaries above discoverable without
duplicating their operation ledgers:

| Boundary | Implementation and verification pointers |
| --- | --- |
| Release, attestations, bootstrap, upgrade | `.github/workflows/rust-release-assets.yaml`, `scripts/larch.sh`, `crates/larch-cli/src/release_plugin_runtime.rs`, `crates/larch-adapters/src/github/attestation.rs`, `crates/larch-cli/tests/release_assets.rs`, and the clean-install cases in `crates/larch-cli/tests/parity.rs` |
| GitHub credentials and operations | `crates/larch-adapters/src/github/`, `crates/larch-adapters/src/github_actions.rs`, the [GitHub service inventory](../github-service-inventory.md), and the `service-ownership` rule and tests in `crates/larch-lint/` |
| Connectivity availability | `crates/larch-core/src/connectivity.rs`, `crates/larch-adapters/src/http_client.rs`, `crates/larch-cli/src/net_commands.rs`, and their focused Rust tests |
| Google ADC | `crates/larch-adapters/src/google_auth.rs`, the [Google service inventory](../google-service-inventory.md), and the `service-ownership` rule and tests in `crates/larch-lint/` |
| Vendor processes, credentials, descendants, and diagnostics | `crates/larch-core/src/process.rs`, `crates/larch-core/src/vendor/`, `crates/larch-core/src/vendor_diagnostics.rs`, `crates/larch-adapters/src/process.rs`, `crates/larch-adapters/src/vendor_diagnostics.rs`, `crates/larch-cli/src/launcher_support.rs`, and the `agent-python-free`, `codex-exec-auth`, and `subprocess-via-runner` rules and tests in `crates/larch-lint/` |
| Object storage | `crates/larch-core/src/object_store.rs`, `crates/larch-adapters/src/google_storage.rs`, `crates/larch-adapters/src/s3_storage.rs`, `crates/larch-adapters/src/run_lifecycle.rs`, the [Google service inventory](../google-service-inventory.md), and their focused Rust tests |
| Repository reads and Git compatibility | `docs/git-operation-inventory.md`, `crates/larch-adapters/src/git/`, `crates/larch-adapters/tests/git_repository.rs`, `crates/larch-lint/src/rules/git_ownership.rs`, and the command registry clean-install cases |
