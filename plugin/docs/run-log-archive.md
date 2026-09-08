# Run-log storage contracts

This document defines the run-log storage boundary. Rust owns the lifecycle,
archive, provider, synchronization, publication, and reader paths.

The shared provider fixture is `tests/fixtures/run-log-object-store-contract-v1.json`.
Rust storage tests load it directly. A later storage change must preserve this
contract or version it explicitly.

## Configuration resolution

Discover the Git top level from the invocation's startup directory and read
exactly `tools-config.toml` there. The repository-owned file is shared by
independently configured tools: each tool owns one table named for the tool and
ignores unrelated top-level tool tables. There are no shared fields, no global
version, and no `client_repo` field.

Remote publication uses a strict table:

```toml
[larch]
storage_base_uri = "s3://zhupanov"
```

Resolve storage once at lifecycle start:

1. Reject non-empty `LARCH_LOGS_URI`.
2. If `tools-config.toml` exists, reject a symlink, non-regular path,
   unreadable file, malformed TOML, non-table `[larch]`, unknown `[larch]`
   keys, or an empty, padded, invalid, or type-invalid
   `[larch].storage_base_uri`.
3. A non-empty `LARCH_STORAGE_BASE_URI` overrides a valid file value and may
   enable storage when the file, table, or field is absent. It never hides an
   invalid present file.
4. Without the override, a missing file, missing `[larch]`, or omitted field
   disables storage with reason `config-file-missing`,
   `larch-table-missing`, or `storage-base-uri-omitted`. A valid file field
   enables storage with reason `repository-config`.

Never probe `config.toml`, `.larch/config.toml`, or `tool-config.toml`.
Disabled mode carries no fake bucket, URI, provider, or
`ToolRepositoryStorage`. Provider-dependent commands fail with guidance to
configure `[larch].storage_base_uri` or `LARCH_STORAGE_BASE_URI`.

`StorageBase` is a validated provider, bucket, and optional base prefix. Accept
only `gs://`, `s3://`, and `r2://`, including bucket roots. Reject credentials,
ports, queries, fragments, trailing slashes, whitespace, control characters,
empty segments, `.`, and `..`. Preserve accepted bucket and prefix text.

`ToolRepositoryStorage` adds the fixed tool `larch` and a derived client
repository. Larch reads the repository's local `remote.origin.url`, accepts
standard HTTPS, SSH URL, and SCP-like Git syntax, strips exactly one terminal
`.git`, converts ASCII uppercase to lowercase, and otherwise requires a strict
slug. It never uses a checkout or worktree directory name, a provider API,
bucket text, config field, or environment override for repository identity.
Missing, ambiguous, credential-bearing, port-bearing, or invalid origins fail
without echoing embedded credentials.

Disabled staging uses a SHA-256 namespace derived from the validated absolute
repository root with the versioned
`larch-run-log-local-namespace-v1` domain separator. Only the digest appears
in path components. Lifecycle manifests and contexts pin publication mode,
resolution reason, client repository, and either this local namespace ID or
the enabled canonical storage fields and storage-origin ID.

## Remote and local layout

The remote schema is
`<storage-base>/<tool-name>/<client-repo>/<data-type>/<data>`. The larch tool
repository is `<base>/larch/<client-repo>`. The implemented data type is
`run-logs`, so archives use
`<base>/larch/<client-repo>/run-logs/<skill>/<run-id>.tar.gz`.

The checked-in base produces
`s3://zhupanov/larch/larch/run-logs/design/<run-id>.tar.gz`. Other examples
include
`gs://character-tool-logs/larch/sre/run-logs/investigate/<run-id>.tar.gz`,
`s3://company-data/prod/tools/larch/service-a/run-logs/review/<run-id>.tar.gz`,
and `r2://tool-data/larch/service-a/run-logs/review/<run-id>.tar.gz`.

Only skill directories exist directly below `run-logs/`. Mutable analyzer
state, ledgers, reports, measurements, indexes, and migration artifacts never
appear there. Future data types must define their own object, collision,
mutability, and retention contracts.

Define `storage_origin_id` as lowercase hexadecimal SHA-256 of the canonical
tool repository URI's UTF-8 bytes. Local paths are:

```text
${XDG_CACHE_HOME:-$HOME/.cache}/larch/run-logs/v2/
  <client-repo>/<storage-origin-id>/<skill>/<run-id>/

${XDG_STATE_HOME:-$HOME/.local/state}/larch/run-log-pending/v2/
  <client-repo>/<storage-origin-id>/<skill>/<run-id>/

${XDG_STATE_HOME:-$HOME/.local/state}/larch/run-log-locks/v2/
  <client-repo>/<storage-origin-id>/<skill>/<run-id>.lock
```

The cache is private local state, not a publication target. Changing provider,
bucket, base prefix, tool, or client repository selects a cold namespace. Old
basename-keyed cache and state are neither renamed nor silently imported.

## Provider operations

Every provider implements the same operations:

| Operation | Contract |
|---|---|
| `preflight_prefix` | List the exact `<tool>/<client-repo>/` prefix with a maximum of one result. Decide success from provider status only; ignore object names and provider diagnostics. Do not list the bucket root or write a probe object. |
| `list` | List the requested prefix through every page. Return relative keys, byte sizes, and optional opaque ETag and version values. Reject malformed pages, repeated page tokens, and keys outside the configured root. |
| `upload_create` | Create one object only if absent. Never replace an existing object. Return normalized metadata. |
| `metadata` | Return normalized metadata for one exact key. |
| `download` | Write to a private sibling temporary file and atomically promote it. Never merge with a destination. |

When storage is disabled, startup skips provider construction and provider
commands, emits `STORAGE_PREFLIGHT=skipped-disabled`, and admits the workflow.
When storage is enabled, startup fails closed on every configuration,
credential, transport, network, bucket, or prefix-access error. For the checked-in
base and repository, startup preflight lists only
`larch/larch/` in bucket `zhupanov`.

The Rust-owned lifecycle, standalone publication, synchronization, and
preflight paths use the official AWS SDK for S3 and R2. They use the SDK's
non-process credential chain; `credential_process` is disabled so profile
configuration cannot introduce a child process. R2
also requires `LARCH_R2_ACCOUNT_ID` and `LARCH_R2_ENDPOINT`. The endpoint must
be `https://<account-id>.r2.cloudflarestorage.com`, and the account ID must
match the host. GCS uses the narrow Rust transport through
`${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh` and standard Google Application Default
Credentials. Local development checkouts may supply a validated `LARCH_BINARY`
override to the wrapper. Installed plugins use the release-matched executable
and do not build. Credentials should grant list, read, and write only to
approved tool and client-repository prefixes.

## Machine-readable errors

Provider diagnostics are untrusted and may contain credentials. Adapters reduce
them to this closed set before orchestration consumes them:

| Kind | Meaning | GCS transport exit |
|---|---|---:|
| `transport` | Request launch, timeout, network, or unclassified provider failure | 1 |
| `invalid-response` | Invalid request shape or malformed provider response | 2 |
| `authentication` | Credentials are missing, invalid, expired, or denied | 3 |
| `already-exists` | A create-only destination already exists | 4 |
| `not-found` | The requested bucket or object does not exist | 5 |
| `local-io` | A local source, destination, or atomic file operation failed | 6 |

Configuration failures occur before transport selection. In memory, an error
carries only `kind`, `provider`, and `operation`. The GCS command uses the fixed
exit mapping above and labels exit 2 as `invalid-request-or-response`. Do not
parse provider stderr or expose it as the machine contract.

## Archive, publication, and synchronization

`${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh run-log archive` packages one completed, sanitized
staging tree as `<run-id>.tar.gz`. The source tree is not changed.

The archive is a POSIX PAX tar stream inside gzip. Every member has a
normalized slash-separated NFC path. Members are ordered by that normalized
path, use timestamp `0`, owner and group `0`, empty owner/group names, and
normalized modes: `0644` for non-executable files and `0755` for directories
and executable files. Gzip metadata uses timestamp `0` and no filename.

Only regular files and directories are accepted. Symlinks, devices, FIFOs,
sockets, reserved paths, and Unicode-normalization collisions fail closed.

Each archive has a root `archive-manifest.json` member. It is UTF-8 canonical
JSON with schema version `1`, the skill and run ID, and one entry per source
tree member. File entries include their byte size and SHA-256 digest; directory
entries record size `0` and no digest. The manifest does not describe itself,
avoiding a recursive digest. The command emits SHA-256 digests for both the
complete archive and its manifest so later publication can use the archive
digest for idempotence.

`${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh run-log materialize` validates an archive before it
writes run files. It rejects unsafe paths, collisions, links, special files,
malformed contents, and integrity mismatches. Defaults limit archives to
10,000 members, 256 MiB per member, 1 GiB expanded, and a 1,000:1 ratio.

Materialization writes into a private temporary sibling, verifies the complete
tree, renames it into place, and verifies it again. Failures remove the staged
tree. It never merges with or replaces a destination. Cache entries contain ordinary files and the manifest.

`${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh run-log publish --repo-root <root> --skill <skill>
--run-id <run-id> --staging-root <tree>` persists the archive before attempting
the create-only upload to `run-logs/<skill>/<run-id>.tar.gz`. Failed attempts
remain under the storage-origin-specific `run-log-pending/v2` path
with content-pinned retry metadata. Repeating the command may omit
`--staging-root` when that pending state exists. When pending state already
exists, the publisher retries and populates the cache from that archive. It
does not use a later mutable staging tree as the retry source.

An existing remote key succeeds only when its downloaded bytes match the
pending archive; different content fails closed. A new upload is verified by
remote metadata. The normal success path copies the sanitized staging tree
directly into the storage-origin-specific `run-logs/v2` cache,
without downloading or decompressing the archive. Retry without staging safely
materializes the durable archive instead. A per-run lock covers upload,
collision verification, cache promotion, and atomic retirement of pending
state. Any failure returns nonzero, retains pending state, and prevents clean
workflow success.

Both `--log-root` and direct `--staging-root` publication redact the source
tree before archive construction. Redaction is verified idempotently; a
surviving secret or unsafe source tree fails before any archive byte reaches a
provider.

When storage is disabled, standalone `run-log publish` and `run-log sync`
perform no archive, provider, pending-state, or cache operation and exit zero
with `RUN_LOG_STORAGE=disabled`, `RUN_LOG_STORAGE_REASON=<token>`, and their
normal `PUBLISH_OK=true` or `SYNC_OK=true` terminal field. Publish also emits
`RUN_LOG_PUBLICATION=skipped-disabled`; sync emits zero archive counters and an
empty `CORPUS_ROOT` and `INVENTORY_SHA256`. Analyzer consumers reject that
skipped sync as an empty corpus and return actionable storage guidance.
Analysis commands with an explicit `--log-root` continue to read that local
corpus without storage or network access.

`${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh run-log sync --repo-root <root>` lists the complete
`run-logs/` remote prefix once, including every provider pagination page. It
downloads and safely materializes only runs without a valid local directory.
Valid cached runs remain untouched. Invalid entries are quarantined under the
per-run publication lock, replaced atomically after validation, and restored if
repair fails. Interrupted download, materialization, promotion, and quarantine
entries are removed before the next attempt.

Enabled sync emits `INVENTORY_SHA256`: SHA-256 over the schema tag, inventory
count, and each sorted normalized relative archive key plus its listed byte
size. It is an opaque inventory identity for audit records, not an archive
digest. It never prints an archive name, local path, archive content, or
provider diagnostic. The current aggregate audit record is
[Run-log corpus re-audit: 2026-08-09](run-log-corpus-audit-2026-08-09.md).

Normal synchronization accepts only manifest-bearing archives. It does not read
a legacy migration descriptor, inventory, or basename-keyed cache. The
Rust-owned layout migration command retains the descriptor parser as an
explicit operator API for the coordinated historical migration only; it does
not discover repository configuration or participate in normal sync.

Cut over while runs are frozen: land and release this runtime, drain v1 pending
publications, complete and verify the object migration, commit the new config,
prewarm a clean v2 cache, then resume. Roll back by restoring the prior plugin
release and old remote prefixes. The old local cache and state remain untouched.

The command returns the unpacked storage-origin-specific repository corpus. The shared
`run_log_corpus.synchronized_run_log_root` API performs the same one-time sync
and returns that root. An analyzer must retain the returned path and use normal
local file reads for all later files and waves in the same invocation.
Cross-session `/design` pause and resume require this verified published cache.
Pause rejects disabled storage before writing a GitHub pause marker.

## Ownership

Rust owns `run-log archive`, `run-log materialize`, `run-log publish`,
`run-log sync`, `run-log storage-preflight`, and the shared lifecycle verbs,
including terminal archive publication and cache promotion. Configuration
resolution lives in `larch-core`; GCS uses `GoogleCloudStorage`, while S3 and
R2 use the official AWS SDK through `S3Storage`. All providers preserve the
same credential-free error classes. Do not add a compatibility shim, bridge,
implementation selector, fallback, or dual-write period.
