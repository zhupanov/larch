# step-0-degraded-gate.sh

Step 0 degraded-tools gate. Reads CODEX/CURSOR presence keys from session-env and forwards them to scripts/agent degraded-tools-gate.

## Caller

`skills/implement/SKILL.md` invokes this wrapper from the named `/implement` step so the prompt-side Bash fence remains a plugin-root source guard plus one script call.

## KV grammar

The wrapper relays the underlying helper stdout unchanged unless this file names explicit keys. Explicit keys are newline-delimited `KEY=value` records and must be token-scannable by the orchestrator.

## Invariants

- Bash 3.2 portable; no associative arrays or namerefs.
- Self-rehydrates `CLAUDE_PLUGIN_ROOT` from `$IMPLEMENT_TMPDIR/plugin-root.env` where needed.
- `skills/shared/session-setup-output.md` names the session telemetry keys; this gate reads persisted values from `$IMPLEMENT_TMPDIR/session-env.sh`.

## Edit-in-sync

Update `skills/implement/SKILL.md` and the implement structure/timing harnesses when this contract or argv changes.
