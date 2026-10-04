# design-step3b-tail.sh

## Purpose

Thin launcher-compat wrapper for the adapter-backed `/design` Step 4 rejected-findings and Gate C preview tail.

## Primary callers

- `skills/design/SKILL.md`

## Invariants

- The `.sh` file only derives and exports `CLAUDE_PLUGIN_ROOT`, then execs `scripts/larch.sh design step4-tail`.
- `scripts/larch.sh design step4-tail` resolves a supplied session env through the trusted bgjob resolver before pause handling or tmpdir use. It never sources the file.
- Delegates lifecycle decisions to `bgjob adapt` with step `design-step4-tail`, explicit tmpdir, 900-second budget, session path, and optional owner PID.
- Accepts child mode only as the terminal `--bgjob-child --merge-result-env <path>` suffix.
- Ordinary duplicate calls reattach a successful completed result when the input fingerprint matches. A matching-fingerprint nonzero result is cleared and launches fresh. When `.step3-review-result.env` is present, the owner passes its sha256 as `--input-fingerprint` to `bgjob adapt`; a fingerprint mismatch (or no stored sidecar for a prior result) is treated as stale and launches fresh. The owner does not inspect registry liveness or delete lifecycle artifacts.
- Keeps FINALIZE, rejected-finding rendering, Gate C, preview generation, and completion markers in child mode.
- Atomically publishes `STEP4_STATUS`, `SKIP_APPROVE_REQUESTED_GATEC`, rejected-body paths, preview paths, and an optional dialectic digest to the adapter merge env through the Rust bgjob writer.
- A pause race runs pause-save, publishes `STEP4_STATUS=pause-save`, and exits zero. Publication failure exits non-zero.

## Harness

Covered by the inline tests in `crates/larch-cli/src/design_step3_commands.rs` and `make test-design-structure`.
