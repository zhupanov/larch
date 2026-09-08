# step-2-post-dispatch.sh

Step 2 post-dispatch wrapper. Runs the phantom untracked probe, emits the checked-out branch, best-effort emits the current short commit SHA, persists ship-seed context, and emits the post-dispatch route token in one foreground call.

## Caller

`skills/implement/SKILL.md` invokes this wrapper only on the `/implement` Step 2.2 `STATUS=complete` external-implementer path via:

```bash
"$HOME/.cache/larch/sessions/implement-run-$PPID.sh" skills/implement/scripts/step-2-post-dispatch.sh --expected-branch "$BRANCH_NAME"
```

`--expected-branch` is required by the Rust parser. `claude_fallback` must not call it.

## Exit codes

- `0`: the probe ran and routing was emitted as `POST_DISPATCH_NEXT=continue|bail`.
- `2`: argparse usage failure, including omitted `--expected-branch`.

Detached HEAD, branch read failure, an empty expected branch, or a branch mismatch emit `POST_DISPATCH_NEXT=bail` and `BAIL_REASON=main-branch-post-dispatch` on stdout instead of relying on a wrapper process failure.

`git rev-parse --short HEAD` is non-fatal. If it fails after a successful branch read, the wrapper exits `0` and omits `COMMIT_SHA=`.

## KV grammar

Stdout is newline-delimited `KEY=value` records:

- `PHANTOM_*` from `phantom_probe_with_warn "2-post-dispatch"`.
- `BRANCH=<name>` after a successful symbolic branch read.
- `COMMIT_SHA=<short-sha>` when the best-effort SHA read succeeds.
- `POST_DISPATCH_NEXT=continue|bail` as the routing token.
- `BAIL_REASON=main-branch-post-dispatch` when `POST_DISPATCH_NEXT=bail`.

Do not source or `eval` wrapper stdout.

## Ship seed persistence

After a successful symbolic branch read, the wrapper merge-appends missing keys to `$IMPLEMENT_TMPDIR/ship-seed-input.env` before emitting `POST_DISPATCH_NEXT`, even when the branch mismatches `--expected-branch`:

- `MANIFEST_PATH`: `$IMPLEMENT_TMPDIR/codex-step2-out/manifest.json` when readable, else `$IMPLEMENT_TMPDIR/manifest.json` when readable, else empty.
- `TOOL_LABEL`: maps `$IMPLEMENT_TMPDIR/bootstrap-routing.env` `coder` from `codex` to `Codex`, `cursor` to `Cursor`, and all other values to `claude`.

Existing keys are preserved. Step 0 owns run flags in the same file.

## Bootstrap

The wrapper requires `IMPLEMENT_TMPDIR`, resolves `${CLAUDE_PLUGIN_ROOT}`, and delegates directly to `scripts/larch.sh implement step-2-post-dispatch`.

## Orchestrator contract

`SKILL.md` token-scans `PHANTOM_*`, optional `BRANCH=`, optional `COMMIT_SHA=`, and exactly one `POST_DISPATCH_NEXT=` from wrapper stdout. It routes by `POST_DISPATCH_NEXT`, not by wrapper exit plus prompt-side branch byte comparison.
