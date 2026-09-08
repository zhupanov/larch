# design-step3-entry.sh

## Purpose

Thin launcher-compat wrapper for the `/design` Step 3 entry block.

## Primary callers

- `skills/design/SKILL.md`

## Invariants

- The `.sh` file only derives and exports `CLAUDE_PLUGIN_ROOT`, then execs `scripts/larch.sh design step3-entry`.
- `scripts/larch.sh design step3-entry` owns panel-init failure, scope-anchor materialization, and the Step 3 entry preview.
- Accepts `--reentry` for Gate A / Gate C routed review re-entry, writes `$DESIGN_TMPDIR/.step3-reentry`, and clears `$DESIGN_TMPDIR/oos-aggregate-pool.md` after validating `DESIGN_TMPDIR`.
- `--reentry` does not clear `$DESIGN_TMPDIR/.step3-entry-plan-printed`; the continuation entry point owns that cleanup.
- Keeps the combined entry order: clear `.pause-save-complete`, call `plan-review step3-entry-state`, exit on `.pause-save-complete`, then materialize the scope anchor and call `plan-review step3-entry-preview`.
- Materializes and validates `$DESIGN_TMPDIR/plan-review-scope-anchor.txt` before the Step 3 review launch can be scheduled. The anchor uses the issue title plus `issue-body.txt` with any prior `larch:plan` block stripped, falling back to `feature-description.txt` or a verbal prompt only when no issue body existed, and appends an approved outline when present.
- Does not derive the root Claude PID from `$PPID` internally.

## Harness

Covered by the inline tests in `crates/larch-cli/src/design_step3_commands.rs` and `make test-design-structure`.
