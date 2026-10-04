# Phantom untracked probe

**Consumer**: `/implement` orchestrator.
**Contract**: advisory untracked-file probe entrypoints, registry, and stdout KV parsing.
**When to load**: read before parsing non-clean `PHANTOM_*` telemetry that requires orchestrator action, or before changing phantom-probe call sites. Do not require a full-reference read when the already parsed macro path is a no-op, such as `PHANTOM_STATUS=clean`.

At selected `/implement` boundaries, detect non-ignored untracked files that appeared after the Step 0 tracking adoption session baseline. This is advisory only: phantoms are logged to Execution Issues, never cleaned automatically.

**Thin implementation**: shared logic lives in the Rust `git phantom-probe` command; checkpoint consumers and the Rust Step 8 dispatcher relay its advisory KV envelope. Runtime entrypoints:

- **Combined (4 sites)**: post-rebase probe is bundled into `"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" push checkpoint-probe` for Steps **1.r**, **4.r**, **7.r**, and **7a.r** (uniform `<step-prefix>-post-rebase` tokens such as `1.r-post-rebase`; see `skills/implement/references/rebase-checkpoint-routing.md`). The wrapper consumes `"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" git phantom-probe`; do not duplicate direct `git check-phantom-dirty` or warning-append calls after those checkpoints.
- **Bundled standalone tokens (2 sites)**: Step 2 post-dispatch uses `skills/implement/scripts/step-2-post-dispatch.sh`, which bundles the probe token `2-post-dispatch` with branch and optional SHA reads. The Rust Step 8 dispatcher composes the same typed probe owner in process with token `8-pre-ship` before the ship driver and writes its advisory rows to stderr, away from driver JSON stdout.

**6 sites total** per run: four combined post-rebase probes (including the uniform `1.r-post-rebase` site) plus the two bundled standalone tokens above.

**Orchestrator parsing** — token-scan the probe tail for `PHANTOM_STATUS`, optional `PHANTOM_REASON`, `PHANTOM_COUNT`, `PHANTOM_PATHS_FILE`, and optional `PHANTOM_APPEND_WARN_ERROR` (warn-append failure already logged by the wrapper — treat as advisory telemetry). Do **not** `eval`/`source` captured lines.

**Probe locations (registry)**:
- After Step 2 dispatch returns on the external-implementer `STATUS=complete` path only: `skills/implement/scripts/step-2-post-dispatch.sh` bundles the `2-post-dispatch` probe with branch and optional SHA reads. The orchestrator always consumes `PHANTOM_*` before exit-code routing; branch comparison stays in SKILL.md. Do not probe when `STATUS=claude_fallback`; Claude-fallback implementation files are uncommitted until Step 4.
- After Step 1.r / 4.r / 7.r / 7a.r `scripts/larch.sh push checkpoint-probe` returns on the success path: phantom handling is **inside** the wrapper (`1.r-post-rebase`, `4.r-post-rebase`, `7.r-post-rebase`, `7a.r-post-rebase`).
- Immediately before the active Step 8+ driver: `--step 8-pre-ship` inside `skills/implement/scripts/step-8-ship.sh`.

There is intentionally no post-Step-6 probe. When `FILES_CHANGED=true`,
review-created files are legitimately untracked until Step 7 commits them; a
post-Step-6 probe would false-positive. The post-Step-7.r probe covers the
committed review-fix state.
