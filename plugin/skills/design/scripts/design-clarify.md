# `design clarify`

Rust owner for the two-phase `/design` Step 0b clarify helper.

## Purpose

Validates public argv, loads the optional trusted session env, and owns
route-state fallback, pause-save termination, fetch, publish, result env
writes, and failure routing.

## Phases

- `--phase fetch --issue N`: verifies `clarify state` is `awaiting-response`,
  fetches the matching request comment in process, writes
  `$DESIGN_TMPDIR/clarify-request.md`, and emits
  `CLARIFY_FETCH_STATUS=ok` plus durable file paths.
- `--phase publish --issue N`: reads `$DESIGN_TMPDIR/clarify-plan.md` and
  `$DESIGN_TMPDIR/clarify-response.md`, redacts and writes the plan block,
  runs design-log publish fail-closed, posts the clarify response, removes the
  clarification label, and renames to `[DESIGNING]` only when `SESSION_ID` is
  non-empty and `PUBLISH_OK=true`.

## Invariants

- Invoked through `scripts/larch.sh` with explicit `--session-env-path` and
  `--claude-pid` values.
- Accepts the current issue explicitly through `--issue`.
- The Rust owner validates `--phase`, `--issue`, and `--claude-pid` before
  any phase effect.
- The Rust owner falls back to `.design-step0-route-state.env` for `REPO` when
  the session env lacks it. Missing route state is benign. Present unreadable
  route state emits `route-state-read-failed`; fetch stages `failed-clarify`,
  publish does not.
- Fetch failures emit `SUMMARY_OUTCOME=failed-clarify` with the fetch status.
  `state-read-failed` and `fetch-read-failed` are legacy subprocess-only tokens
  and are not emitted by `scripts/larch.sh design clarify`.
- Never prints request or response bodies on stdout. Bodies move through files.
- Does not emit `--state designed`; clarify-only updates are not terminal
  design completion.

## Harness

`make test-design-clarify` runs the Rust owner's in-crate tests in
`crates/larch-cli/src/clarify_orchestrator/tests.rs`. Structural pins run in
`make test-design-structure`.
