# Session Setup Output Reference

Canonical session setup stem:

```bash
"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" session setup
```

Shared reviewer-session flag tail for `/research` and `/review`:

```text
--skip-preflight --skip-branch-check --skip-repo-check --check-reviewers
```

Consumers cite this stem and tail once, then list local deltas such as `--prefix <name>`, optional `--caller-env`, probe-skip flags, or the `/design` `design step0-session` wrapper.

## Output keys

Always emitted core keys:

- `SESSION_TMPDIR`
- `SESSION_ID`
- `LARCH_RENDER_CACHE_DIR`
- `REPO_ROOT` (operator repo root resolved once at setup: caller-env value, then `CLAUDE_PROJECT_DIR`/`REPO_ROOT` env, then the setup cwd)

Emitted when `--check-reviewers` is used:

- `CODEX_BINARY_FOUND`
- `CURSOR_BINARY_FOUND`
- `CODEX_PRESENT`
- `CURSOR_PRESENT`

Emitted when `--deny-edit-write <token>` is passed (token must be a recognized `scripts/deny-edit-write.sh` allowlist token; setup fails closed without leaving a sentinel when activation cannot be proven):

- `DENY_EDIT_WRITE_SENTINEL` (absolute path of the setup-owned scoped Write-hook activation sentinel)

Optional caller-derived stdout keys (when present in caller-env and forwarded):

- `LARCH_TOKEN_SESSION_ID`
- `LARCH_CLAUDE_SOURCE_FILE`

Session-env-only telemetry (not emitted on `session setup` stdout; rehydrate from `session-env.sh` or `$SESSION_ENV_PATH`):

- `LARCH_TIMING_LEDGER`

Optional repo keys when repo probing is not skipped:

- `REPO`
- `REPO_UNAVAILABLE`

## Semantics

Presence keys (`CODEX_PRESENT`, `CURSOR_PRESENT`) are only for the immediate degraded-tools gate.
Binary-found keys (`CODEX_BINARY_FOUND`, `CURSOR_BINARY_FOUND`) are for later launch guards.

Telemetry keys such as `LARCH_TIMING_LEDGER` are consumed from `session-env.sh` on some paths and are not always present on `session setup` stdout.

## Update triggers

Update this file when `session setup` changes its shared invocation stem, reviewer-session flag tail, emitted key set, or presence-vs-binary-found semantics.
