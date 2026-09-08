# `design step5c`

## Purpose

Adapter-backed `/design` Step 5c Rust command.

## Primary callers

- `skills/design/SKILL.md`

## Invariants

- Resolves a supplied session env through the trusted bgjob resolver before parent routing or tmpdir use. It never sources the file.
- Delegates lifecycle decisions to `bgjob adapt` with step `design-step5c`, explicit tmpdir, 21600-second budget, session path, and optional owner PID.
- Passes the resolved larch entrypoint as the first worker token after `--`, followed by `design step5c`. It never passes a bare larch verb as the worker program.
- Ordinary calls reattach a successful completed result when the input fingerprint matches. A matching-fingerprint nonzero result is cleared and launches fresh. When `.step3-review-result.env` is present, the Rust owner passes its SHA-256 as `--input-fingerprint` to `bgjob adapt`; a fingerprint mismatch (or no stored sidecar for a prior result) is treated as stale and launches fresh. The adapter-private `--fresh-attempt` control maps to `--replace-completed-result` only for documented repair and refusal retries.
- Never forwards `--fresh-attempt` to the child `design step5c` invocation. It preserves all other public argv cells.
- Accepts child mode only as the terminal `--bgjob-child --merge-result-env <path>` suffix.
- The child requires the authoritative `.design-step5c-status.env` to contain publish, validation, final-summary, and cleanup rows, then atomically copies those rows to the adapter merge env.
- Missing Step 5b, pause, publish refusal, validation failure, and success all write the authoritative status envelope before return.
- `$DESIGN_TMPDIR/bgjob/design-step5c.result.env` remains the prompt-side completion source after `bgjob wait` reports `DONE`.

## Harness

`make test-design-step5c` runs the inline Rust tests in
`crates/larch-cli/src/design_finalize_commands/tests.rs`. Structural pins run
in `make test-design-structure`.
