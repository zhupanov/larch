# Analyzer state

Run archives are immutable analysis inputs. Stateful analyzers synchronize the
remote run corpus once, then read ordinary files from
`${XDG_CACHE_HOME:-$HOME/.cache}/larch/run-logs/v2/<client-repo>/<storage-origin-id>/`
for the rest of the invocation. They retain the corpus root returned by the
shared synchronizer and never reconstruct it from the checkout directory.

Mutable cursors, ledgers, retry bundles, generated analyses, and generated
measurements do not belong under `run-logs/`. They live at:

```text
${XDG_STATE_HOME:-$HOME/.local/state}/larch/analysis-state/v2/
  <client-repo>/<storage-origin-id>/<owner>/
```

The client repository is derived from Git origin. The storage-origin ID is the
lowercase hexadecimal SHA-256 of the canonical
`<base>/larch/<client-repo>` URI. Provider, bucket, base-prefix, tool, or
repository changes therefore select different cache and analyzer-state roots.
Old basename-keyed state is not silently imported.

Repository and owner names remain validated path components. Files use
private directories and `0600` modes. Writers use a per-file advisory lock,
atomic replacement, and an expected SHA-256 identity when a workflow carries
state across multiple steps. A stale concurrent writer fails without replacing
the newer file. Corrupt, symlinked, non-regular, or unreadable state fails
closed.

Old basename-keyed and repository-local state remains untouched for explicit
operator cleanup. No v2 owner imports it, including after a provider, bucket,
prefix, tool, or repository identity change. Run-log cloud sync never lists,
downloads, uploads, or interprets this mutable state tree.

`/rejected-analysis` owns `rejected-analysis/ledger.tsv` and
`rejected-analysis/verdicts.tsv`. `/difficulty-calibration` reads the verdict
sidecar from that owner after one run-log sync. It does not copy mutable state
into the cache or a run archive. An explicit `--log-root DIR` is an offline
fixture boundary: readers use it directly without reading repository storage
configuration or making network requests. A fixture may keep a local legacy
sidecar for compatibility.
