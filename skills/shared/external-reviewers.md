# External Reviewer Procedures (Codex + Cursor)

Shared mechanical procedures for running Codex and Cursor as external reviewers. Each skill provides its own reviewer invocation commands (prompts, output paths, tmpdir variables) — this file covers the common scaffolding.

## Binary Check and Health Probe (Step 0b)

Run the shared `check-reviewers.sh` script with `--probe` to check for binaries **and** verify each tool is actually responding:

```bash
${CLAUDE_PLUGIN_ROOT}/scripts/check-reviewers.sh --probe
```

Parse the output for `CODEX_AVAILABLE`, `CURSOR_AVAILABLE`, `CODEX_HEALTHY`, `CURSOR_HEALTHY`.

**Session-env override**: If session-env (from `--caller-env` or `--session-env`) provides `CODEX_HEALTHY=false` or `CURSOR_HEALTHY=false`, set the corresponding `*_available` mental flag to `false` immediately **without re-probing that tool**. Pass `--skip-codex-probe` and/or `--skip-cursor-probe` to `check-reviewers.sh` for tools already known unhealthy. Still run the full check (binary + probe) for tools whose health is unknown (not present in session-env). Print: `**⚠ <Codex|Cursor> marked unhealthy by caller — using Claude replacement for this session.**`

Set mental flags `codex_available` and `cursor_available` based on the combined result:
- If `CODEX_AVAILABLE=false`: `codex_available=false`. Print: `**⚠ Codex not available (binary not found). Proceeding without Codex reviewer.**`
- Else if `CODEX_HEALTHY=false`: `codex_available=false`. Print: `**⚠ Codex installed but not responding (health check failed). Using Claude replacement.**`
- Else: `codex_available=true`
- Same logic for Cursor.

**Note**: `*_AVAILABLE` is a pure install-state signal (binary exists on PATH). `*_HEALTHY` indicates whether the tool actually responded to a trivial prompt within the 60-second probe timeout. Callers must combine both to determine runtime usability.

## Runtime Timeout Fallback

When processing reviewer results (after `wait-for-reviewers.sh` returns), check each reviewer's sentinel file exit code and output validity. If any of the following are true for a reviewer, set the corresponding `*_available` mental flag to `false` for **all subsequent steps in this session**:

- Sentinel exit code is `124` (timeout — the common case when `run-external-reviewer.sh` enforces its timeout)
- Sentinel exit code is non-zero (any other failure)
- Output is empty/invalid after the retry-once procedure (per "Validating External Reviewer Output" below)
- `wait-for-reviewers.sh` reports `TIMEOUT` for the reviewer (sentinel never appeared — wrapper killed externally)

Print: `**⚠ <Reviewer> timed out — using Claude replacement for remainder of session.**`

This is a mental flag flip within the current skill invocation. For cross-skill propagation within `/implement`, child skills write a structured health status file — see the `/implement` SKILL.md for details.

**Note**: Once a reviewer is marked unhealthy during a session, it stays unhealthy for the remainder of that session. This is intentional — it prevents oscillation and wasted time on flaky tools during extended outages.

## Monitoring External Reviewers

After launching Codex and/or Cursor as background tasks, **poll for `.done` sentinel files** to detect completion. The wrapper script (`run-external-reviewer.sh`) writes `<output-file>.done` (containing the exit code) when it finishes. Do NOT poll the output files directly — Cursor buffers all stdout until exit, so its output file is empty until the process finishes.

1. Continue working on other tasks (e.g., processing Claude subagent results) while external reviewers run in the background.
2. After all other tasks are done, invoke the wait script with the sentinel file paths for all launched reviewers:
   ```bash
   ${CLAUDE_PLUGIN_ROOT}/scripts/wait-for-reviewers.sh --timeout 1860 "<cursor-output-file>.done" "<codex-output-file>.done"
   ```
   Only include paths for reviewers that were actually launched. For this `wait-for-reviewers.sh` Bash tool call, use `timeout: 1860000` (1860 seconds = 1 860 000 ms) and **do NOT** set `run_in_background: true` — this call must block until all sentinels are found. The script polls every 5 seconds, prints compact dot-based progress to stderr, and outputs machine-parseable `DONE <name>: exit=<code>` or `TIMEOUT <name>` lines to stdout. It always exits 0. **Important**: Invoke the wait script exactly ONCE with ALL sentinel file paths in a SINGLE Bash tool call. Do NOT make multiple Bash calls or write ad-hoc polling loops. If you launched the reviewer with a custom timeout via `run-external-reviewer.sh`, pass the matching value plus 60 seconds grace with `--timeout <seconds>` (seconds) and set the Bash tool `timeout` to the same value in milliseconds.
3. **Do NOT read output files until the corresponding `.done` sentinel file exists.** Reading early will see an empty file (especially for Cursor, whose stdout is fully buffered). Parse the script's stdout `DONE`/`TIMEOUT` lines to determine which reviewers completed.
4. For any reviewer that shows `TIMEOUT` in the script output, print `**⚠ <Reviewer> sentinel file not found after timeout. The wrapper may have been killed externally. Proceeding without <Reviewer> findings.**` and continue without that reviewer.

## Validating External Reviewer Output

**Only after** the corresponding `.done` sentinel file exists, read the output file. The exit code is in the sentinel file (e.g., `cat "<output-file>.done"`). The wrapper script (`run-external-reviewer.sh`) also prints diagnostic lines to its own stdout (captured in the background task output):
- `✓ <tool> review: completed (exit code 0, Xs elapsed, output N bytes)` — success
- `❌ <tool> review: FAILED (exit code N, Xs elapsed, output N bytes)` — failure with details
- `⚠ <tool> review: completed but OUTPUT IS EMPTY` — process succeeded but wrote nothing

**Validation steps:**
1. Read the output file with the Read tool.
2. Check that it is non-empty and looks like a review (contains numbered findings or "NO_ISSUES_FOUND").
3. If the output is empty despite exit code 0, **retry the reviewer once** with a fresh invocation (same prompt, new output file path with `-retry` suffix). Some tools have transient startup failures that resolve on retry.
4. If the output is empty after retry, or if the reviewer exited non-zero, print a detailed warning including the exit code and elapsed time from the wrapper's diagnostic output: `**⚠ <Reviewer> review failed (exit code X, Ys elapsed, 0 bytes output). Proceeding without <Reviewer> findings.**`

## Negotiation Protocol

> **Note**: `/design` and `/review` now use the **Voting Protocol** in `voting-protocol.md` instead of this Negotiation Protocol. This section is retained for skills that still use negotiation: `/loop-review` and `/research`.

> **Variable substitution**: Replace `<skill-tmpdir>` in all paths below with the session tmpdir variable passed by the caller (e.g., `$DESIGN_TMPDIR` or `$REVIEW_TMPDIR`).

> **Parameters**: `max_rounds` (default: 3) — the maximum number of negotiation rounds. Callers may override this (e.g., `/loop-review` uses `max_rounds=1` to keep runtime manageable across multiple slices).

Negotiate with each external reviewer (Codex, Cursor) for up to **`max_rounds` rounds** of back-and-forth:

1. Evaluate each finding. **Accept** it unless it is factually incorrect (references wrong file/line, misunderstands the code) or contradicts a project convention documented in CLAUDE.md.
2. For findings you disagree with, write a response to a negotiation prompt file explaining your reasoning. Use the Write tool if available; if the skill does not allow Write (e.g., `/research`), write the prompt file via the `run-negotiation-round.sh` script's `--prompt-file` argument (the caller must create the file through whatever means the skill permits). The prompt should include the original finding, your counter-argument, and ask the reviewer to either maintain its position with additional justification or withdraw the finding.
   - **Codex**: Write to `<skill-tmpdir>/codex-negotiation-prompt.txt`, then:
     ```bash
     ${CLAUDE_PLUGIN_ROOT}/scripts/run-negotiation-round.sh --tool codex --prompt-file "<skill-tmpdir>/codex-negotiation-prompt.txt" --output "<skill-tmpdir>/codex-negotiation-output.txt" --workspace "$PWD"
     ```
   - **Cursor**: Write to `<skill-tmpdir>/cursor-negotiation-prompt.txt`, then:
     ```bash
     ${CLAUDE_PLUGIN_ROOT}/scripts/run-negotiation-round.sh --tool cursor --prompt-file "<skill-tmpdir>/cursor-negotiation-prompt.txt" --output "<skill-tmpdir>/cursor-negotiation-output.txt" --workspace "$PWD"
     ```
   Use `timeout: 300000` on both Bash tool calls.
3. Repeat up to 3 rounds total. After round 3 (or earlier if all disagreements are resolved), **Claude makes the final call** on any remaining disputes.
