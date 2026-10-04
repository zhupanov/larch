# Run-log corpus re-audit: 2026-08-09

This is the final operational record for `character-ai/larch#8295`. It records
only aggregate evidence. It excludes `CORPUS_ROOT`, archive names, archive
contents, provider diagnostics, credentials, and per-run results.

## Rust corpus verification

The Rust `run-log sync` command ran through `scripts/larch.sh` with larch
`56.2.2`. The cold pass listed and materialized the complete configured
inventory without an archive canonicality failure:

| Pass | Listed | Present | Downloaded | Repaired | Result |
| --- | ---: | ---: | ---: | ---: | --- |
| Cold | 2,735 | 78 | 2,657 | 0 | `SYNC_OK=true` |
| Warm | 2,735 | 2,735 | 0 | 0 | `SYNC_OK=true` |

The warm pass reported this opaque inventory identity:

```text
INVENTORY_SHA256=c8a6970791fea1dd5b8648ee9babefdfe4d20ff34c01c1671997e19373fc37fd
```

`INVENTORY_SHA256` binds the normalized sorted archive-key and listed-size
inventory. It is not an archive digest and does not print inventory entries.

## Reporting proof

The synchronized corpus completed both Rust analyzer paths without issue
posting or plots:

- `report-tokens analyze --skill implement --no-issue --no-plot`
- `report-tokens analyze --skill design --no-issue --no-plot`

The versioned offline matrix in `crates/larch-test-support/src/run_log.rs`
covers historical manifest and lifecycle forms, a legacy panel-prompt shape,
and current token, timing, progress, transcript, and terminal-report inputs.
Focused Rust contracts passed for archive materialization, timing, progress,
final reports and run summaries, transcript rendering, diagrams, charts, token
reports, token cost, and streaming corpus scans.
`python/tests/report/test_run_log_corpus.py` also passed against the Rust
machine envelope.

## Ownership boundary

`run-log sync` now emits the opaque inventory identity for future audit records.
The retained Python corpus reader validates it but does not write a run-log
artifact. The reporting ownership and writer boundary remain Rust-owned and
are enforced by the reporting, registry, and runtime-entrypoint lint rules.
