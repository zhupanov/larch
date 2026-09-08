# design-step35.sh

## Purpose

Thin launcher-compat wrapper for the `/design` Gate B block.

## Primary callers

- `skills/design/SKILL.md`

## Invariants

- The `.sh` file only derives and exports `CLAUDE_PLUGIN_ROOT`, then execs `scripts/larch.sh design gate-b`.
- `scripts/larch.sh design gate-b` owns the Gate B skip/abort messages, `.completed/step-3` marker, pause-save preemption, timing mark, and `APPROVE_REQUESTED=` row.
- Accepts `--session-env-path` from the prompt-side Bash call.
- Accepts `--claude-pid` when the wrapped logic must refresh session state.
- Does not derive the root Claude PID from `$PPID` internally.

## Harness

Covered by the inline tests in `crates/larch-cli/src/design_step3_commands.rs` and `make test-design-structure`.
