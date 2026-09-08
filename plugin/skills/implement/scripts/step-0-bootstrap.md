# step-0-bootstrap.sh

Step 0 initial/resume bootstrap wrapper around `scripts/larch.sh bootstrap invoke`. In fork mode, captures `scripts/larch.sh admission fork-env` metadata into `CALLER_ENV_PATH`, `UPSTREAM_REPO`, and sibling fork variables before invoking bootstrap. Emits the parsed routing envelope and the progress prompt on initial bootstrap; resume uses `--preserve-coder` parsing.

The wrapper delegates the continue-tail (degraded-tools gate + checkpoint `1.r`) to `bootstrap invoke`. It forwards an explicit `--non-interactive true|false` computed from the canonical `/implement` non-interactive predicate (not `LARCH_SKILL_NON_INTERACTIVE` alone). It also forwards an optional `--difficulty TRIVIAL|MODERATE|HARD` override into bootstrap run flags. Degraded explanation blocks emitted on stderr remain operator-visible while stdout stays parseable.

## Caller

`skills/implement/SKILL.md` invokes this wrapper from the named `/implement` step so the prompt-side Bash fence remains a plugin-root source guard plus one script call.

## KV grammar

The wrapper relays the underlying helper stdout unchanged unless this file names explicit keys. Explicit keys are newline-delimited `KEY=value` records and must be token-scannable by the orchestrator.

Continue-tail routing keys relayed on stdout include: `DEGRADED`, `BOTH_DOWN`, `CODEX_STATE`, `CURSOR_STATE`, `DEGRADED_PROMPT_REQUIRED`, `ROUTE`, `CHECKPOINT_NEXT`, `REBASE_RC`, `REBASE_OUTCOME`, `CONFLICT_FILES`, `REBASE_ERROR`, `SKIPPED_ALREADY_PUSHED`, `SKIPPED_ALREADY_FRESH`. Advisory `PHANTOM_*` keys may trail on stdout only; they are excluded from `$IMPLEMENT_TMPDIR/bootstrap-routing.env`.

## Invariants

- Bash 3.2 portable; no associative arrays or namerefs.
- Self-rehydrates `CLAUDE_PLUGIN_ROOT` from `$IMPLEMENT_TMPDIR/plugin-root.env` where needed.
- Session telemetry keys are defined in `skills/shared/session-setup-output.md`; Step 0 bootstrap consumers read them from `$IMPLEMENT_TMPDIR/session-env.sh`.
- Resume mode restores prior `coder` / `coder_fallback` from a regular, non-symlinked `bootstrap-routing.env` before the absorbed tail runs inside `bootstrap invoke`.

## Edit-in-sync

Update `skills/implement/SKILL.md` and the implement structure/timing harnesses when this contract or argv changes.
