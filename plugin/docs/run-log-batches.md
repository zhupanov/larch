# Run-log batch registry

Run-log batches are registered in `crates/larch-core/src/run_log/batch.rs`, the
sole durable owner read by Rust `run-log write` and `run-log append`.

Each batch declares:

- **extension**: the on-disk suffix.
- **mode**: `replace` or `append`.
- **sanitizer**: `none`, `json-object`, `json-lines`, or a specialized
  sanitizer.

The command validates the batch name before writing.
Append-mode batches must use `run-log append`.
Replace-mode batches must use `run-log write`.

The registry includes the durable implement, review, design, token, timing,
execution-issue, transcript, vendor-diagnostic, checks-digest telemetry, and
debate record carriers.

Registration defines the allowed shape when publication is enabled. Every
session-derived batch still passes through the run-log trim, temporary-path
redaction, secret scrub, and publication checks described in the canonical
[run-log security contract](security/artifacts-redaction-and-publication.md#run-logs-and-breadcrumbs).

Batch names are relative to the lifecycle-owned staging tree. They never select
or reconstruct remote storage. With storage enabled, the shared lifecycle
routes the terminal archive to
`<base>/larch/<client-repo>/run-logs/<skill>/<run-id>.tar.gz`. With storage
disabled, writers continue to use the same local batches during the run, then
successful terminalization removes staging without an archive, synchronized
cache, or pending publication. Analyzers use an explicit local `--log-root` or
the storage-origin-bound corpus root returned by the shared synchronizer. They
never treat disabled storage as an empty corpus.

## Debate record batches

Four debate carriers support the debate engine. `debate adjudicate
--vote-stalemates` produces the tally carrier and `debate synthesize` produces
the proposal carrier; the surrounding debate flow owns the participant and
round-ledger producers:

| Batch | Extension | Mode | Sanitizer |
|---|---|---|---|
| `debate-round-ledger` | `.ndjson` | append | `json-lines` |
| `debate-proposal` | `.md` | replace | `none` |
| `debate-stalemate-tally` | `.json` | replace | `json-object` |
| `debate-participants` | `.tsv` | replace | `none` |

**Session-tmpdir rejection**: for these four slugs only, `_write_batch` and
`_append_batch` reject recognized session-tmpdir pointers before
`_redact_to_temp` and before persistence. Raw text is scanned with
`contains_recognized_session_tmpdir_pointer`. For `json-lines` and
`json-object` sanitizers, decoded JSON object keys and string values are
inspected recursively so escaped-slash or Unicode-escaped session paths cannot
bypass the guard.
Malformed JSON keeps the existing post-redaction validation error. Valid debate
payloads, including operator-repository paths, still follow the existing
redaction path. Non-debate batches are unchanged.

The durable-record security statement lives in
[artifacts-redaction-and-publication.md](security/artifacts-redaction-and-publication.md#durable-debate-record-invariant).
Reconstruction roles and producer notes live in
[run-logs.md](run-logs.md#debate-durable-records).

`architectural-invariant-outcome` is a replace-mode `.json` batch with the
`json-object` sanitizer. It writes
`larch-logs/implement/<RUN_ID>/architectural-invariant-outcome.json` when an
implement run reaches Step 8 invariant-note composition. Schema version `1`
uses `outcome` values `clean`, `violation`, or `dropped`; `violation` is
blocking and feeds remediation, while `assessment_kind` is `clean` or
`violation` when a note exists. Reason `deterministic-clean` requires `clean`
with `assessment_kind=clean`. Reason `unavailable` requires the existing
`dropped` non-violation fallback with an empty assessment kind. A valid
violation remains blocking and is not downgraded by a later unavailable input.

`architectural-guideline-outcome` is a replace-mode `.json` batch with the
`json-object` sanitizer. It writes
`larch-logs/implement/<RUN_ID>/architectural-guideline-outcome.json` when an
implement run reaches Step 8 guideline-note composition.

Schema version `1` uses:

- `schema_version`: `1`.
- `phase`: `implement`.
- `step`: `8`.
- `outcome`: `pinned`, `clean`, or `dropped`.
- `reason`: stable token from `larch_core::architectural_assessment`.
- `detail`: redacted bounded diagnostic.
- `guidelines_status`: `present`, `absent`, or `invalid`.
- `head_sha`, `base_ref`, and `assessment_kind`.

Schema version `1` remains unchanged. Historical records stay valid. For
guidelines, reason `deterministic-clean` requires `outcome=clean` and
`assessment_kind=clean`; reason `unavailable` requires `outcome=dropped` and an
empty assessment kind. The same combinations apply to invariant outcomes, with
`violation` reserved for an authored `violation-note`.

New schema-version `1` writers may add optional boolean `operator_waived`.
`true` is valid only with `outcome=dropped`, `reason=unavailable`, and an empty
assessment kind. Missing and `false` remain valid for historical records and
non-waived outcomes.

Runs below `GUIDELINE_SHIP_OUTCOME_MIN_LARCH_VERSION`, or runs that did not
reach the Step 8 condition, are pre-feature-era for this batch. At or above the
cutover, Step 8-eligible runs without the artifact fail the audit scan.

`checks-digest-sizes` is an append-mode `.tsv` batch with `none` sanitizer.
It is content-free: rows contain counts and safe identifiers only.
