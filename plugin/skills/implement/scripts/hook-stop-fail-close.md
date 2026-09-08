# skills/implement/scripts/hook-stop-fail-close.sh — contract

`hook-stop-fail-close.sh` is the thin Stop-hook shim for the Rust-owned `larch hook stop-fail-close` verb. It guards the unresolved post-`/review` boundary inside an active `/implement` run.

The shim enters the verified `scripts/larch.sh` bootstrap with `LARCH_BOOTSTRAP_NO_INSTALL=1`. If that runtime is unavailable or the verb fails, it exits 0 with no stdout or stderr.

The Rust owner reads the Stop payload once. `stop_hook_active: true` allows the stop to prevent continuation loops. A non-empty payload `session_id` binds active-run resolution; an empty, missing, or null value cannot inherit a stale session identity. Active-run lookup uses the in-process session resolver rather than spawning another CLI process.

When `review-round-summary.md` exists but neither `.review-boundary-passed` nor `.run-cleaned-up` exists, the hook emits the existing compact top-level `{"decision":"block","reason":"..."}` envelope. The recovery remains: complete Cross-Skill Presence Propagation, track rejected code-review findings, write the Step 6 breadcrumb, then create `.review-boundary-passed`. All other states allow Stop with no output.

The retired post-`/design` and post-`/release` gates remain unenforced. Missing or malformed payloads, resolver errors, stale candidates, and cleaned runs fail open. Rust tests in `crates/larch-cli/src/hook_commands.rs` cover the exact envelope and boundary states.

Edit in sync with `hooks/hooks.json`, `skills/implement/SKILL.md` Steps 6 and 8, `crates/larch-cli/src/session_lifecycle_commands.rs`, and `scripts/test-implement-anti-halt.sh`.
