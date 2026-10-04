---

# larch-run-lifecycle: shared-v1 skill=cleanup
name: cleanup
description: "Use when cleaning up stale larch session temp directories by age and reaping dangling /design session-env symlinks."
allowed-tools: Bash
---

**MANDATORY: Follow the complete shared lifecycle contract in `${CLAUDE_PLUGIN_ROOT}/skills/shared/run-lifecycle.md` with declared skill `cleanup`.**

# cleanup

**MANDATORY: READ ENTIRE FILE before composing user-facing prose: `${CLAUDE_PLUGIN_ROOT}/skills/shared/readability-style.md`.**

Remove stale larch session temp directories from `~/.cache/larch/sessions/` (the canonical location), from `/tmp` (legacy fallback), and from the process temp root `$TMPDIR` resolves to (on macOS this is a per-user path distinct from `/tmp`, and it is where bare `tempfile.mkdtemp()`/`mkstemp()` calls actually land; see issue #5923). Retention is age-based (`LARCH_CLEANUP_RETENTION_DAYS`, default 7): directories are removed only when no file within the bounded `maxdepth 5` nested-activity scan is newer than the cutoff, so a directory with fresh deep activity is retained. Matching loose top-level files under either temp root are removed by top-level age and pattern match. Symlinked top-level session or pattern entries are skipped. Also reaps dangling `current-design-env-*.sh` symlinks in the sessions parent. Always runnable — multiple concurrent Claude sessions do not block cleanup.

## NEVER

- Never abort cleanup just because `SESSION_COUNT` is greater than `1`; the count is informational only.
- Never invent removal counts if the cleanup script exits non-zero or omits any required stdout key.

## Flags

- `--run-id <ID>`: Shared flag contract: `${CLAUDE_PLUGIN_ROOT}/skills/shared/run-id-flag.md`.

<!-- step:1 — Run cleanup -->

```bash
"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" cleanup run
```

Parse `SESSION_COUNT`, `CACHE_REMOVED`, `TMP_REMOVED`, and `SYMLINKS_REMOVED` from stdout and relay them to the user.

<!-- step:2 — Verify -->

Verify the script exited successfully (exit code 0). Confirm stdout emitted all four keys (`SESSION_COUNT`, `CACHE_REMOVED`, `TMP_REMOVED`, `SYMLINKS_REMOVED`). If it exited non-zero, stop and surface the error; do not invent removal counts.
