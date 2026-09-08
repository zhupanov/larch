## Binary Presence Check (Step 0)

`session setup --check-reviewers` performs two separate checks:

- `CODEX_BINARY_FOUND` / `CURSOR_BINARY_FOUND`: whether the CLI binary is on `PATH`.
- `CODEX_PRESENT` / `CURSOR_PRESENT`: immediate Step 0 probe health for the degraded-tools gate only.

The Codex probe resolves the luna review role. It classifies either `Model metadata for <model> not found` or `requires a newer version of Codex` as a CLI model gate. The degraded-tools gate and `/status` then show ``codex CLI too old for <model>; run `npm install -g @openai/codex@latest```. The diagnostic is paired with the probe's review-model and auth-mode cache identity. A short-lived handoff preserves it for the immediate degraded-tools call when normal probe caching is disabled.

Durable session env files must keep only the binary-found keys. Do not persist `CODEX_PRESENT`, `CURSOR_PRESENT`, `CODEX_AVAILABLE`, or `CURSOR_AVAILABLE`. Treat legacy `--codex-present`, `--cursor-present`, `--codex-available`, and `--cursor-available` flags as compatibility input only.

Later vendor routing must use `CODEX_BINARY_FOUND` / `CURSOR_BINARY_FOUND` or a fresh `command -v` / `shutil.which()` check. Step 0 probe health is not a launch-routing input.

`/debate` is the narrow exception. Its Step 0 probe is immediately followed by persistent Cursor and Codex session bootstrap, so `skills/debate/SKILL.md` passes exact `CODEX_PRESENT` / `CURSOR_PRESENT` values into `debate init-run`. One unavailable external slot proceeds with a named per-slot warning; two unavailable external slots hard-fail before the source title changes. This exception does not write presence values to durable session env and does not change routing for another skill.

The Step 0 probe does not probe the sol implementation role or terra vote and fix roles. Those models remain launch-time checks, and their failures use the existing local fallback paths.

## Degraded-tools gate (Step 0)

Issue #3207: Step 0 health probes exist to warn or stop the operator before work starts. They do not decide later reviewer, voter, fixer, scout, or implementer routing.

Immediately after `session setup --check-reviewers`, pass explicit probe KVs to the gate:

```bash
"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" agent degraded-tools-gate \
  --codex-binary-found "$CODEX_BINARY_FOUND" --codex-present "$CODEX_PRESENT" \
  --cursor-binary-found "$CURSOR_BINARY_FOUND" --cursor-present "$CURSOR_PRESENT" \
  --skill <design|implement|review|research>
```

Parse `DEGRADED`, `CODEX_STATE`, `CURSOR_STATE`, `BOTH_DOWN`, `DEGRADED_HARD_FAIL`, and `PRESENCE_INPUT_EMPTY`. Preserve the explanation block between `DEGRADED_EXPLANATION_BEGIN` and `DEGRADED_EXPLANATION_END`.

Apply this contract:

- **Healthy**: `DEGRADED=false`. Proceed silently.
- **One vendor down, no sentinel**: emit or surface the explanation and require explicit operator **Continue** or **Abort**. In non-interactive, CI, eval, autonomous-loop, and `/review --subagent` contexts, emit a prompt-required envelope instead of proceeding.
- **One vendor down, sentinel exists**: proceed degraded. The sentinel must mean a prior operator chose Continue.
- **Both vendors down**: hard-fail in every mode. Ignore any stale sentinel. Emit `DEGRADED_HARD_FAIL=true` when producing an envelope. Do not ask Continue / Abort.
- **`PRESENCE_INPUT_EMPTY=true`**: record a warning. Treat empty presence as fail-safe down for the gate.

Only the explicit Continue path may create `.degraded-tools-gate-prompted`. The detection path must not create it.

`/debate` does not invoke this generic Continue / Abort gate. Its three-seat quorum already defines the degraded policy: Claude plus one healthy external is sufficient, while two unavailable externals are not. Default-mode interactivity is reserved for stalemate adjudication, not vendor admission.

## Runtime Waterfall Fallback

Runtime zero-survivor collapse at the review stage falls back to Claude-subagent self-review for `/implement` Step 5 (`larch:claude-self-reviewer`) and main-agent self-review for `/design` and `/review` when the normalized reason is `no successful launched reviewer output`. Step 0 both-down remains a hard fail before any review-stage fallback can run. `/implement` conflict-resolution Phase 3 is an exception to this runtime reviewer waterfall: non-trivial `ship_pr_pre_push` resolutions use `larch:ci-fixer` (`MODE=conflict`) self-review only and do not launch external reviewers.

When processing reviewer results, failed external slots should fall through the waterfall dispatcher rather than flipping session-wide availability:

- Phase 1 launches the slot's assigned external tool when its binary is present or the manifest intentionally attempts the slot.
- Phase 2 retries the slot with the other binary-present external tool.
- Phase 3 launches a Claude reviewer subprocess via `scripts/larch.sh agent launch-claude-review`.

Use this warning template when a slot reaches Phase 3:

- `**⚠ <Reviewer> failed — <FAILURE_REASON>. Using Claude replacement for this slot.**`

Where `<FAILURE_REASON>` is the `FAILURE_REASON` value from `scripts/larch.sh agent collect-results` output (or from the `.diag` file if collecting results manually). Always include the reason so the user can diagnose the root cause (e.g., timeout duration, exit code, last error output).

Do not write runtime failure status back to session env. `CODEX_PRESENT` and `CURSOR_PRESENT` are immediate Step 0 gate outputs only; per-slot launch failures must stay local to the slot result.

## Collecting External Reviewer Results

After all other tasks are done, collect and validate external reviewer outputs using the shared collection script:

```bash
"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" agent collect-results --timeout <seconds> <output-file> [<output-file> ...]
```

Only include output file paths for reviewers that were actually launched. For the Bash tool call, use `timeout: <seconds>000` (milliseconds) and use a foreground collector invocation. The script internally calls `${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh agent wait-reviewers` to poll for `.done` sentinel files, validates each output, and retries once on empty output (using `.meta` files written by `${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh agent run-external-agent`). Wait records are correlated by 1-based argv index, so callers should pass output files in the same order they want result blocks interpreted.

**Output**: The script emits structured `KEY=value` blocks on stdout (one block per reviewer, separated by blank lines):
```
REVIEWER_FILE=<output-path>
STATUS=<OK|TIMED_OUT|FAILED|EMPTY_OUTPUT|SENTINEL_TIMEOUT|NOT_SUBSTANTIVE>
EXIT_CODE=<N>
FAILURE_REASON=<explanation>
```

Parse each reviewer's `STATUS`, `REVIEWER_FILE`, and `FAILURE_REASON`:
- `STATUS=OK`: Read the output file — it is non-empty and validated. `FAILURE_REASON` is empty.
- Any other status: The reviewer failed. `FAILURE_REASON` explains why (e.g., "Timed out after 1800s (limit: 1800s). Process was killed after exceeding the timeout." or "Failed with exit code 1 after 5s. Last output: error message here"). Follow the **Runtime Timeout Fallback** procedure above, including `FAILURE_REASON` in the message.
- Treat `STATUS=OK` with empty `FAILURE_REASON` as the success signal; do NOT use `EXIT_CODE` alone. Retry-row semantics are owned by `crates/larch-cli/src/collector_commands.rs`.

**Important**: Do NOT read output files before calling `scripts/larch.sh agent collect-results`. Cursor buffers all stdout until exit — its output file is empty until the process finishes. The collection script handles all sentinel polling and validation internally.

**Substantive-content validation is opt-in.** The default collector behavior described above is sentinel + non-empty + retry. Substantive-content classification (`STATUS=NOT_SUBSTANTIVE`) only runs when callers pass `--substantive-validation` (and optionally `--validation-mode` for short reviewer-style outputs). See the option grammar of `crates/larch-cli/src/collector_commands.rs` for the authoritative flag documentation and `docs/external-reviewers.md` Output Validation for the per-skill opt-in matrix.

## Negotiation Protocol

> **Note**: `/design` and `/review` now use the **Voting Protocol** in `voting-protocol.md` instead of this Negotiation Protocol. This section is retained for skills that still use negotiation: `/research`.

> **Variable substitution**: Replace `<skill-tmpdir>` in all paths below with the session tmpdir variable passed by the caller (e.g., `$DESIGN_TMPDIR` or `$REVIEW_TMPDIR`).

> **Parameters**: `max_rounds` (default: 3) — the maximum number of negotiation rounds.

Negotiate with each external reviewer (Codex, Cursor) for up to **`max_rounds` rounds** of back-and-forth:

1. Evaluate each finding. **Accept** it unless it is factually incorrect (references wrong file/line, misunderstands the code) or contradicts a project convention documented in CLAUDE.md.
2. For findings you disagree with, write a response to a negotiation prompt file explaining your reasoning. Use the Write tool if available; if the skill does not allow Write (e.g., `/research`), write the prompt file via the `agent run-negotiation-round` CLI verb's `--prompt-file` argument (the caller must create the file through whatever means the skill permits). The prompt should include the original finding, your counter-argument, and ask the reviewer to either maintain its position with additional justification or withdraw the finding.
   - **Codex**: Write to `<skill-tmpdir>/codex-negotiation-prompt.txt`, then:
     ```bash
     "${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" agent run-negotiation-round --tool codex --prompt-file "<skill-tmpdir>/codex-negotiation-prompt.txt" --output "<skill-tmpdir>/codex-negotiation-output.txt" --workspace "$PWD"
     ```
   - **Cursor**: Write to `<skill-tmpdir>/cursor-negotiation-prompt.txt`, then:
     ```bash
     "${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" agent run-negotiation-round --tool cursor --prompt-file "<skill-tmpdir>/cursor-negotiation-prompt.txt" --output "<skill-tmpdir>/cursor-negotiation-output.txt" --workspace "$PWD"
     ```
   Use `timeout: 300000` on both Bash tool calls. `agent run-negotiation-round` distinguishes failure modes by exit code: `0` success, `1` argv/usage or `agent model-args` propagation, `2` Codex auth setup failure or reviewer command (`cursor agent` / `codex exec`) failed, `3` Cursor `cursor_auth_preflight` failed before the reviewer ran. Wrappers that need to disambiguate auth-vs-tool failures should branch on these codes; see ``agent run-negotiation-round` implementation in `crates/larch-cli/src/drafter_commands.rs`` for the full contract and the `RESPONSE_FILE=` stdout key.
3. Repeat up to 3 rounds total. After round 3 (or earlier if all disagreements are resolved), **Claude makes the final call** on any remaining disputes.
## External Reviewer Procedures
