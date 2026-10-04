# design-step5b-prepare.sh

## Purpose

Thin launcher-compat wrapper for the `/design` Step 5b prepare block.

## Primary callers

- `skills/design/SKILL.md`

## Invariants

- The `.sh` file only derives and exports `CLAUDE_PLUGIN_ROOT`, then execs `scripts/larch.sh design step5b-prepare`.
- `scripts/larch.sh design step5b-prepare` owns the Step 5 prelude and OOS prepare behavior.
- The Rust entrypoint hydrates the wrapper environment before reading session keys.
- The `DESIGN_TMPDIR` guard rejects only an empty value, matching the retired Bash prelude.
- The prepare entrypoint creates `$DESIGN_TMPDIR/.completed` before writing `.completed/step-4b`.
- The prepare entrypoint returns immediately through pause-save when `.pause-requested` exists.
- Before it writes `.completed/step-4b` or starts OOS filing, the prepare entrypoint refuses Step 5 when a present, non-empty architectural source lacks its required regular Gate C assessment artifact. It reports invariants before guidelines and uses the persisted `REPO_ROOT` when available; wrapper rehydration retains that key.
- It marks `design Step 5 — finalize` timing after the pause check.
- It captures OOS prepare stdout to `oos-filing-prepare.env` and stderr to `oos-filing-prepare.stderr.log`.
- It emits `NEXT_ACTION=skip-pipeline|file-issues|label-only` on stdout for deterministic Step 5b routing. Every skip status (`skip-sentinel`, `skip-already-filed-sentinel`, `skip-no-items`, `skip-all-security`) emits `NEXT_ACTION=skip-pipeline`. `ready` emits `NEXT_ACTION=file-issues`. `label-only-retry` emits `NEXT_ACTION=label-only`.
- It writes the wrapper routing rows back to `oos-filing-prepare.env` so prompt-side fallback reads use the same machine keys as stdout.
- It emits `OOS_SKIP_BREADCRUMB=` for known skip statuses. Prompt-side Step 5b reprints this breadcrumb when non-empty.
- `STEP5B_NEEDS_ANNOTATE=true` remains the annotate routing key. It is always emitted for `ready`.
- For `label-only-retry`, prepare emits `FILE_DESIGN_OOS_STATUS=label-only-retry`, `NEXT_ACTION=label-only`, and `STEP5B_NEEDS_ANNOTATE=true`. It does not write `.completed/step-5b`.
- Durable pending, combined, sentinel, and filing-order sidecars under `~/.cache/larch/design-oos-filed/` trigger label-only retry after tmpdir cleanup. The guard runs before skip-sentinel, skip-already-filed-sentinel, and skip-no-items.
- For `skip-already-filed-sentinel`, `STEP5B_NEEDS_ANNOTATE=true` is emitted only when `oos-issue.stdout.txt` exists and is non-empty.
- The prepare entrypoint writes `.completed/step-5b` for terminal skip paths and for `skip-already-filed-sentinel` when annotate is not needed. When `STEP5B_NEEDS_ANNOTATE=true`, prepare defers completion to annotate.
- It relays `WARN=` rows for skip-already recovery diagnostics.
- Prepare failure emits `NEXT_ACTION=skip-pipeline`, keeps the existing warning path, and writes `.completed/step-5b`.
- When `NEXT_ACTION` is absent from degraded output, prompt-side Step 5b falls back to `FILE_DESIGN_OOS_STATUS=` per `skills/design/references/oos-step5b-dispatch.md`.

## Harness

Covered by the inline tests in `crates/larch-cli/src/design_oos_commands.rs` and `make test-design-structure`.
