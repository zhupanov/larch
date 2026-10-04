---

# larch-run-lifecycle: shared-v1 skill=research
name: research
description: "Use when conducting read-only research; planner pre-pass + 4 Codex-first lanes (arch/edge/ext/security) with Claude fallback; 3-reviewer panel (Claude+Codex+Cursor); --no-issue skips auto-issue."
argument-hint: "[--no-issue] <research question or topic>"
allowed-tools: Bash, Read, Grep, Glob, Agent, Task, WebFetch, WebSearch, Skill, Write, Edit, NotebookEdit
hooks:
  PreToolUse:
    - matcher: "Edit|Write|NotebookEdit"
      hooks:
        - type: command
          command: "${CLAUDE_PLUGIN_ROOT}/scripts/deny-edit-write.sh research"
          timeout: 5
---

**MANDATORY: Follow the complete shared lifecycle contract in `${CLAUDE_PLUGIN_ROOT}/skills/shared/run-lifecycle.md` with declared skill `research`.**

# Research Skill

**MANDATORY: READ ENTIRE FILE before composing user-facing prose: `${CLAUDE_PLUGIN_ROOT}/skills/shared/readability-style.md`.**

Collaborative best-effort read-only-repo research task with a fixed-shape lane topology. The research phase runs a planner pre-pass that decomposes `RESEARCH_QUESTION` into 2–4 focused subquestions, then four Codex-first lanes (architecture / edge cases / external comparisons / security) covering those subquestions, each with a per-lane Claude `Agent` fallback when Codex is unavailable or fails. The validation phase runs three reviewers in parallel: 1 Claude Code Reviewer subagent + 1 Codex + 1 Cursor. Produces a structured research report; tracked repo files are not modified by Claude's `Edit | Write | NotebookEdit` tool surface while the fresh `research-*` activation sentinel exists (mechanically enforced by the skill-scoped PreToolUse hook permitting only canonical `/tmp` and the larch session cache), while Bash and the external Cursor/Codex reviewers run with full filesystem access and are prompt-enforced only: see the Read-only-repo contract below. May invoke `/issue` via the Skill tool to file research-result issues.

**Anti-halt continuation reminder.** After every child `Skill` tool call (e.g., `/issue`) returns, IMMEDIATELY continue with this skill's NEXT numbered step: do NOT end the turn on the child's cleanup output, and do NOT write a summary, handoff, status recap, or "returning to parent" message: those are halts in disguise. The rule is strictly subordinate to any explicit non-sequential control-flow directive in THIS file (e.g., `skip to Step N`, `bail to cleanup`, `jump back`, `loop back`, `fall through`, `break out`). A normal sequential `proceed to Step N+1` instruction is the default continuation this rule reinforces, NOT an exception. → shared/subskill-invocation.md#anti-halt

**Flags**: Parse flags from the start of `$ARGUMENTS` before treating the remainder as the research question. Flags may appear in any order; stop at the first non-flag token. After stripping all flags, save the remainder as `RESEARCH_QUESTION`.

- `--no-issue` (boolean): Set a mental flag `RESEARCH_NO_ISSUE=true`. When set, Step 3.5 (auto-issue) is skipped: no GitHub issue is created for the research results. Default: `RESEARCH_NO_ISSUE=false`. When `RESEARCH_NO_ISSUE=false` (default), Step 3.5 creates a GitHub issue containing the research question, full report, and token spend metadata via `/issue` single mode.
- `--run-id <ID>`: Use `${CLAUDE_PLUGIN_ROOT}/skills/shared/run-id-flag.md` for the shared contract.

**Fail-closed unknown-flag guard**: After flag parsing finishes, inspect the next token of `$ARGUMENTS`. If it begins with `--`, abort:

```
**⚠ /research: unsupported flag <flag>. Aborting.**
```

This guard catches mistyped or previously-supported flags (the prior `/research` accepted scale/plan/interactive/adjudicate/token-budget/keep-sidecar/verbosity flags; all are now removed) before any Step 0 work.

## Empty-question preflight

After flag parsing completes, validate that `RESEARCH_QUESTION` is non-empty AND not whitespace-only **before any subsequent step** (in particular before any heredoc that interpolates `RESEARCH_QUESTION` into a prompt). On empty / whitespace-only `RESEARCH_QUESTION`, print `**⚠ /research: research question is required. Aborting.**` and exit. This abort runs before Step 0 setup so no tmpdir is created on the empty-question path.

## Token telemetry (observability)

Step 4 always renders a `## Token Spend` section (immediately before `cleanup-tmpdir.sh`) summarizing per-phase Claude subagent token totals. The renderer (`scripts/larch.sh token lane-write/lane-report report`) globs per-lane sidecar files written by the orchestrator after each `Agent`-tool return. Sidecar schema: `PHASE=research|validation`, `LANE=<stable slot name>`, `TOOL=claude`, `TOTAL_TOKENS=<integer or "unknown">`. Rust token lane commands own the helper contract in `crates/larch-cli/src/token_commands.rs`. Telemetry is observability-only: there is no budget enforcement.

### Cost column (optional)

When the env var `LARCH_TOKEN_RATE_PER_M` is set to a positive number (USD per million tokens), the Step 4 token report includes a `$` cost column. When unset (default), the cost column is omitted entirely. See `${CLAUDE_PLUGIN_ROOT}/docs/configuration-and-permissions.md` for the env-var entry.

The research question is described by `RESEARCH_QUESTION` (not raw `$ARGUMENTS`). Use `RESEARCH_QUESTION` wherever human-readable topic text is needed (e.g., agent prompts, report headers, temp file content).

**Read-only-repo contract (best-effort)**: This skill does not create branches, make commits, or modify tracked repo files via Claude's `Edit | Write | NotebookEdit` tool surface. The contract is enforced as two distinct tiers: only the first is mechanical. See `${CLAUDE_PLUGIN_ROOT}/docs/security/workflow-trust-and-mutations.md` for the full residual-risk framing.

- **Mechanically enforced while active (Claude `Edit | Write | NotebookEdit` surface)**: the skill-scoped PreToolUse hook `${CLAUDE_PLUGIN_ROOT}/scripts/deny-edit-write.sh research` matches `Edit|Write|NotebookEdit` and permits the call only when a fresh `research-*` activation sentinel exists and the target `tool_input.file_path` (or `tool_input.notebook_path`) resolves to an absolute path under canonical `/tmp` or the larch cache sessions root (`~/.cache/larch/sessions`, where nested session-setup tmpdirs live); any other active path outcome denies. A leaked hook registration without a fresh `research-*` sentinel allows with empty stdout. The hook is the **sole** mechanical enforcer of that scratch-only policy. The hook's matcher does **not** include `Bash` or `Skill`.

- **Prompt-enforced only (everything else)**:
  - **External reviewers (Cursor, Codex)** run through larch's Rust process boundary against the invoking checkout's working tree (`cursor agent ... --workspace <working tree>`, `codex exec --sandbox workspace-write -C <working tree>`) and have full user-level filesystem access. Their non-modification is requested in the reviewer prompt only: no mechanical guard prevents repo writes if a reviewer ignores the instruction.
  - **Claude's own `Bash` calls** (heredoc writes, `>>` redirects, subprocesses, `git`) are prompt-only constrained. Existing Bash heredoc writes to `$RESEARCH_TMPDIR` under `/tmp` continue to work unchanged: that placement is by convention, not by the hook.
  - **`Skill` is unscoped** in `allowed-tools` so `/research` may invoke `/issue` via the Skill tool for research-results-to-issues flows; child skills run under their own `allowed-tools` and hooks, not under this one.
  - **`Agent`-tool fallbacks** (Claude subagents launched in degraded mode when Cursor or Codex is unavailable: see `references/research-phase.md` and `references/validation-phase.md`) run as separate subprocesses with their own `allowed-tools`. The `/research` skill-scoped PreToolUse hook does NOT propagate to spawned Agent subagents.

- **Residual risk**: even the mechanical hook depends on the running Claude Code version honoring `permissionDecision: "deny"`. `/tmp` is shared scratch, not session-scoped: another skill in the same session may read this skill's tmpdir, so `/tmp` placement is *not a confidentiality boundary*.

**Known limitation**: concurrent repo changes during a long research run may cause agents to see slightly different snapshots.

## Sub-skill invocation

Invoke `/issue` via the Skill tool when the research brief calls for filing the findings as GitHub issues. Follow Pattern B conventions in `${CLAUDE_PLUGIN_ROOT}/skills/shared/subskill-invocation.md`: parse `/issue`'s stdout machine lines (`ISSUES_CREATED`, `ISSUES_FAILED`, `ISSUES_DEDUPLICATED`) and continue with the parent's next step after the child returns.

> **Continue after child returns.** When the child Skill returns, execute the NEXT step of this skill: do NOT end the turn, and do NOT write a summary, handoff, or "returning to parent" message. → shared/subskill-invocation.md#anti-halt

## Filing findings as issues

Defense in depth: stdout parsing of `ISSUES_*` is the primary post-`/issue` mechanical check; the sentinel-file gate below is a supplemental layer.

1. **Defensive sentinel clear**: before invoking `/issue`, remove any stale sentinel from a prior run that may have reused the same tmpdir:

   ```bash
   rm -f "$RESEARCH_TMPDIR/issue-completed.sentinel"
   ```

2. **Invoke `/issue`** via the Skill tool with `--sentinel-file` pointing to the path above:

   ```
   --sentinel-file $RESEARCH_TMPDIR/issue-completed.sentinel <other-/issue-args>
   ```

3. **Parse `/issue` stdout** for the canonical machine lines:
   - `ISSUES_CREATED=<N>`, `ISSUES_DEDUPLICATED=<N>`, `ISSUES_FAILED=<N>`.
   - Per-item `ISSUE_<i>_NUMBER`/`ISSUE_<i>_URL` for created issues.

4. **Mechanical sentinel verification** (defense in depth):

   ```bash
   "${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" verify skill-called --sentinel-file "$RESEARCH_TMPDIR/issue-completed.sentinel"
   ```

   Parse `VERIFIED=true|false` and `REASON=<token>` from stdout. On `VERIFIED=true`, continue. On `VERIFIED=false`, print the fail-closed warning citing `REASON` and abort:

   ```
   **⚠ /research: /issue did not complete cleanly (VERIFIED=false REASON=<token>): aborting.**
   ```

   Remove `"$RESEARCH_DENY_ACTIVE_SENTINEL"` before stopping.

5. **Fail-closed-on-any-failure intent**: when `/issue` reports `ISSUES_FAILED>=1`, the sentinel is suppressed by design and `/research` aborts at step 4. Remove `"$RESEARCH_DENY_ACTIVE_SENTINEL"` before stopping. Research-result-filing semantics require all items to succeed; partial failure is operator-investigation territory.

## Progress Reporting

**Every step MUST print clearly visible breadcrumb status lines** so the user can instantly see where execution is. Follow shared/progress-reporting.md rules.

- Print a **start line** when entering a step: e.g., `> **🔶 /research 1: research**`

**MANDATORY at session start**: Read `${CLAUDE_PLUGIN_ROOT}/skills/research/scripts/step-name-registry.tsv` to get the Step Name Registry (step number → short name mapping for progress breadcrumbs).

**MANDATORY at session start: READ ENTIRE FILE**: `${CLAUDE_PLUGIN_ROOT}/skills/shared/orchestrator-never.md`. Contains cross-skill NEVER rules that apply to this skill in addition to the skill-specific Anti-patterns below.

## Anti-patterns

1. **NEVER improvise ScheduleWakeup outside skill-script direction.** See `${CLAUDE_PLUGIN_ROOT}/skills/shared/orchestrator-never.md` (loaded via MANDATORY above). **CI-backed**: yes: `scripts/test-anti-improvised-wakeup.sh` pins the literal in the shared file.

<!-- step:0: Session Setup -->

Print: `> **🔶 /research 0: setup**`

### 0a: Session Setup and Reviewer Check

Use `${CLAUDE_PLUGIN_ROOT}/skills/shared/session-setup-output.md` for the shared session setup stem, reviewer tail, and output-key semantics. Local delta: `--prefix claude-research`.

```bash
"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" session setup --prefix claude-research --skip-preflight --skip-branch-check --skip-repo-check --check-reviewers
```

If the script exits non-zero, print the error and abort.

Parse the output for `SESSION_TMPDIR`, `CODEX_BINARY_FOUND`, `CURSOR_BINARY_FOUND`, `CODEX_PRESENT`, and `CURSOR_PRESENT`. Set `RESEARCH_TMPDIR` = `SESSION_TMPDIR`.

Set lane launch guards from `CODEX_BINARY_FOUND` / `CURSOR_BINARY_FOUND` or fresh executable checks, and remember each lane's pre-launch attribution status (`ok` / `fallback_binary_missing`):

- If `CODEX_BINARY_FOUND=false`: do not launch Codex; pre-launch status = `fallback_binary_missing`. Print: `**⚠ Codex not available (binary not found). Proceeding with Claude fallback for Codex lanes.**`
- Else: attempt Codex and let launcher-owned retry/fallback handle runtime failures. Pre-launch status = `ok`.
- Same logic for Cursor.

**Degraded-tools gate (#3207).** After parsing Step 0 probe and binary KVs, run the **Degraded-tools gate (Step 0)** procedure in `${CLAUDE_PLUGIN_ROOT}/skills/shared/external-reviewers.md`: invoke `"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" agent degraded-tools-gate` with explicit `--codex-binary-found` / `--codex-present` / `--cursor-binary-found` / `--cursor-present` from the session-setup parse in this Step 0 block (do not omit flags and rely on shell exports) and `--skill research`. Use the canonical interactive predicate from that shared procedure. Apply the shared contract: one-down without a prior Continue sentinel requires an operator decision, including non-interactive prompt-required routing; both-down hard-fails in every mode. The gate is not a lane-routing input; research lanes use binary-found or launcher fallback semantics.

### 0a.5: Activate read-only Write/Edit hook

Create the activation sentinel only after Step 0 setup and degraded-tools gate resolution fully succeed. This must run immediately before the first `Write` / `Edit` / `NotebookEdit` need in Step 0b.

```bash
if [[ -z "${XDG_CACHE_HOME:-}" && -z "${HOME:-}" ]]; then
  echo "**⚠ /research: failed to activate read-only Write/Edit hook. Aborting.**"
  "${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" session cleanup-tmpdir --dir "$RESEARCH_TMPDIR"
  exit 1
fi
RESEARCH_DENY_ACTIVE_DIR="${XDG_CACHE_HOME:-${HOME}/.cache}/larch/deny-edit-write-active"
RESEARCH_DENY_ACTIVE_SENTINEL="$RESEARCH_DENY_ACTIVE_DIR/research-$PPID"
if ! mkdir -p "$RESEARCH_DENY_ACTIVE_DIR" || ! : > "$RESEARCH_DENY_ACTIVE_SENTINEL"; then
  echo "**⚠ /research: failed to activate read-only Write/Edit hook. Aborting.**"
  "${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" session cleanup-tmpdir --dir "$RESEARCH_TMPDIR"
  exit 1
fi
printf 'RESEARCH_DENY_ACTIVE_SENTINEL=%s\n' "$RESEARCH_DENY_ACTIVE_SENTINEL"
```

### 0b: Initialize lane-status record

Write `$RESEARCH_TMPDIR/lane-status.txt` with the per-angle pre-launch attribution (4 research angles, all Codex-first) plus the 3 validation reviewer slots (Code / Cursor / Codex). The 14-key schema is consumed by `${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh render lane-status` at Step 3 to render the final-report header.

The four research lanes use the **Codex pre-launch status** (each lane is Codex-first; the per-lane fallback is Claude `Agent`). The `VALIDATION_CODE_*` slot uses pre-launch status `ok` (Claude code-reviewer subagent has no fallback path); `VALIDATION_CURSOR_*` and `VALIDATION_CODEX_*` use the Cursor/Codex pre-launch statuses respectively.

Write `$RESEARCH_TMPDIR/lane-status.txt` using a quoted-delimiter heredoc (`<<'EOF'`) so that shell metacharacters in sanitized reason values are preserved verbatim. 14 keys (one `KEY=VALUE` per line):

- `RESEARCH_ARCH_STATUS`, `RESEARCH_EDGE_STATUS`, `RESEARCH_EXT_STATUS`, `RESEARCH_SEC_STATUS`: codex pre-launch status for each lane
- `RESEARCH_ARCH_REASON`, `RESEARCH_EDGE_REASON`, `RESEARCH_EXT_REASON`, `RESEARCH_SEC_REASON`: sanitized reason or empty
- `VALIDATION_CODE_STATUS=ok`, `VALIDATION_CODE_REASON=` (always constant)
- `VALIDATION_CURSOR_STATUS`, `VALIDATION_CURSOR_REASON`, `VALIDATION_CODEX_STATUS`, `VALIDATION_CODEX_REASON`: per pre-launch presence result

### 0c: Record Research Context

Record the current branch and commit for inclusion in the final report:

```bash
"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" git branch-info
```

Parse the output for `HEAD_SHA` and `CURRENT_BRANCH`. If `CURRENT_BRANCH` is empty (detached HEAD), use `"(detached HEAD)"` in the report.

<!-- step:1: Collaborative Research Perspectives -->

Print: `> **🔶 /research 1: research**`

**MANDATORY: READ ENTIRE FILE** before executing Step 1: `${CLAUDE_PLUGIN_ROOT}/skills/research/references/research-phase.md`. It carries the planner pre-pass (Step 1.1, always on), the TTY-only interactive review checkpoint (Step 1.1.c, no hard-fail when stdin is not a TTY: passthrough proceeds with the planner output), the fixed 4-lane mapping (architecture / edge cases / external comparisons / security), the four named angle-prompt literals (`RESEARCH_PROMPT_ARCH`, `RESEARCH_PROMPT_EDGE`, `RESEARCH_PROMPT_EXT`, `RESEARCH_PROMPT_SEC`), the single collection block at Step 1.4, and the single synthesis branch at Step 1.5 using `scripts/larch.sh research banner`. Each external lane is Codex-first with a per-lane Claude `Agent` fallback. Cursor is NOT used for research lanes (still in validation panel). **Do NOT load `${CLAUDE_PLUGIN_ROOT}/skills/research/references/validation-phase.md` at Step 1**: that reference is Step 2's body. **Do NOT load `${CLAUDE_PLUGIN_ROOT}/skills/research/references/citation-validation-phase.md` at Step 1**: that reference is Step 2.5's body. **Do NOT load `${CLAUDE_PLUGIN_ROOT}/skills/research/references/critique-loop-phase.md` at Step 1**: that reference is Step 2.6's body.

Execute Step 1 per the reference file above. SKILL.md is the sole owner of Step 1 entry breadcrumbs; the reference file emits none. Step 1.1 invokes `"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" research run-planner` (contract in `crates/larch-cli/src/research_commands.rs`); the offline regression tests live in `crates/larch-cli/src/research_commands/tests.rs`. The Step 1.5 synthesis-subagent contract is structurally pinned by `${CLAUDE_PLUGIN_ROOT}/skills/research/scripts/test-synthesis-subagent.sh` (contract in sibling `${CLAUDE_PLUGIN_ROOT}/skills/research/scripts/test-synthesis-subagent.md`), also wired into `make lint`.

<!-- step:2: Findings Validation -->

Print: `> **🔶 /research 2: validation**`

**MANDATORY: READ ENTIRE FILE** before executing Step 2: `${CLAUDE_PLUGIN_ROOT}/skills/research/references/validation-phase.md`. It carries the 3-reviewer panel (1 Claude Code Reviewer subagent + 1 Codex + 1 Cursor), the always-on Claude Code Reviewer subagent lane with the research-validation variable bindings (`{REVIEW_TARGET}` / `{CONTEXT_BLOCK}` / `{OUTPUT_INSTRUCTION}`) and research-specific acceptance criteria, the Cursor and Codex validation-reviewer launch bash blocks with their per-slot Claude Code Reviewer subagent fallbacks, the process-Claude-findings-immediately rule, Step 2.4 `COLLECT_ARGS` + zero-externals branch + runtime-timeout replacement, the Codex/Cursor negotiation delegation to `${CLAUDE_PLUGIN_ROOT}/skills/shared/external-reviewers.md`, and the Finalize Validation procedure (where synthesis revision is routed to a separate Claude Agent subagent when accepted findings exist, with the same per-profile structural validator and atomic rewrite of `$RESEARCH_TMPDIR/research-report.txt`). **Do NOT load `${CLAUDE_PLUGIN_ROOT}/skills/research/references/research-phase.md` at Step 2**: that reference is Step 1's body. **Do NOT load `${CLAUDE_PLUGIN_ROOT}/skills/research/references/citation-validation-phase.md` at Step 2**: that reference is Step 2.5's body. **Do NOT load `${CLAUDE_PLUGIN_ROOT}/skills/research/references/critique-loop-phase.md` at Step 2**: that reference is Step 2.6's body.

Execute Step 2 per the reference file above. SKILL.md is the sole owner of Step 2 entry breadcrumbs; the reference file emits none.

<!-- step:2.5: Citation Validation -->

Print: `> **🔶 /research 2.5: citation-validation**`

**Skip preconditions** (emitted FIRST, before any reference load): if `$RESEARCH_TMPDIR/research-report.txt` does not exist OR is zero bytes, print `⏩ 2.5: citation-validation: skipped (no synthesis to validate) (<elapsed>)` and proceed to Step 3 without loading `citation-validation-phase.md`.

**MANDATORY: READ ENTIRE FILE** before executing Step 2.5: `${CLAUDE_PLUGIN_ROOT}/skills/research/references/citation-validation-phase.md`. It carries the input gate, the validator invocation (`"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" research validate-citations`), the sidecar schema (3-state ledger PASS/FAIL/UNKNOWN with reason classifier), the SSRF and network-safety recap, DOI validation, file:line spot-check semantics, the fail-soft contract, the Step 3 splice contract, and the idempotency rerun rule. **Do NOT load `${CLAUDE_PLUGIN_ROOT}/skills/research/references/research-phase.md` at Step 2.5**: that reference is Step 1's body. **Do NOT load `${CLAUDE_PLUGIN_ROOT}/skills/research/references/validation-phase.md` at Step 2.5**: that reference is Step 2's body. **Do NOT load `${CLAUDE_PLUGIN_ROOT}/skills/research/references/critique-loop-phase.md` at Step 2.5**: that reference is Step 2.6's body.

Execute Step 2.5 per the reference file above. Invoke the validator:

```bash
"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" research validate-citations \
  --report "$RESEARCH_TMPDIR/research-report.txt" \
  --output "$RESEARCH_TMPDIR/citation-validation.md" \
  --tmpdir "$RESEARCH_TMPDIR"
```

The script always exits 0 (fail-soft). Parse the last stdout line `SUMMARY=PASS=<n> FAIL=<n> UNKNOWN=<n> TOTAL=<n>` to drive advisory warnings.

Then conditionally print advisory warnings (not errors: fail-soft):

- When `<fail> > 0`: `**⚠ 2.5: citation-validation: <fail> claim(s) FAILED. See ## Citation Validation in the report.**`
- When `<unknown> > 0`: `**ℹ 2.5: citation-validation: <unknown> claim(s) UNKNOWN. See ## Citation Validation in the report.**`

The sidecar at `$RESEARCH_TMPDIR/citation-validation.md` is consumed by Step 3 (splice contract: appended as a `## Citation Validation` section to `research-report-final.md` before the user-visible `cat`). See `crates/larch-cli/src/research_commands/citations.rs` for the full validator contract; the offline regression tests live in `crates/larch-cli/src/research_commands/tests.rs`. Budget-exhaustion coverage lives in `crates/larch-cli/src/research_commands/tests.rs` with injected fetcher seams, so the suite stays offline and deterministic.

<!-- step:2.6: Critique Loop -->

Print: `> **🔶 /research 2.6: critique loop**`

**Skip preconditions**: if `$RESEARCH_TMPDIR/research-report.txt` is missing or zero-bytes, print `⏩ 2.6: critique loop: skipped (no synthesis to critique) (<elapsed>)` and proceed to Step 3. If `$RESEARCH_TMPDIR/citation-validation.md` is missing (Step 2.5 skipped on its own input gate), print `⏩ 2.6: critique loop: skipped (no citation sidecar) (<elapsed>)` and proceed to Step 3.

**MANDATORY: READ ENTIRE FILE** before executing Step 2.6: `${CLAUDE_PLUGIN_ROOT}/skills/research/references/critique-loop-phase.md`. It carries the loop control (cap `RESEARCH_CRITIQUE_MAX=2`, per-iteration critique-pass + categorical-Important gate + refine-pass + citation re-validation), the critique prompt template, the in-scope `**Important**`-finding parser-scope rule, the parser fail-safe, the refine-subagent contract, the canonical slot-name list (`Critique-1`, `Critique-2`, `Revision-Critique-1`, `Revision-Critique-2`), and the citation-revalidation invariant. **Do NOT load `${CLAUDE_PLUGIN_ROOT}/skills/research/references/research-phase.md` at Step 2.6**: that reference is Step 1's body. **Do NOT load `${CLAUDE_PLUGIN_ROOT}/skills/research/references/validation-phase.md` at Step 2.6**: that reference is Step 2's body. **Do NOT load `${CLAUDE_PLUGIN_ROOT}/skills/research/references/citation-validation-phase.md` at Step 2.6**: that reference is Step 2.5's body.

Execute Step 2.6 per the reference file above. SKILL.md is the sole owner of Step 2.6 entry breadcrumbs; the reference owns intermediate operator prints: notably the per-iteration citation-revalidation breadcrumb. If the byte-equal idle-cycle guard fires with zero Important findings, print `⏩ 2.6: critique loop: refine produced no change at iter <N>; exiting loop (<elapsed>)`.

<!-- step:3: Final Research Report -->

Print: `> **🔶 /research 3: report**`

Render the per-lane attribution headers from `$RESEARCH_TMPDIR/lane-status.txt`:

```bash
"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" render lane-status --input "$RESEARCH_TMPDIR/lane-status.txt"
```

Parse the seven output lines via prefix-strip (NOT `cut -d=`, since rendered values can contain `=` characters):
- `RESEARCH_ARCH_HEADER="${line#RESEARCH_ARCH_HEADER=}"`
- `RESEARCH_EDGE_HEADER="${line#RESEARCH_EDGE_HEADER=}"`
- `RESEARCH_EXT_HEADER="${line#RESEARCH_EXT_HEADER=}"`
- `RESEARCH_SEC_HEADER="${line#RESEARCH_SEC_HEADER=}"`
- `VALIDATION_CODE_HEADER="${line#VALIDATION_CODE_HEADER=}"`
- `VALIDATION_CURSOR_HEADER="${line#VALIDATION_CURSOR_HEADER=}"`
- `VALIDATION_CODEX_HEADER="${line#VALIDATION_CODEX_HEADER=}"`

If the helper exits non-zero, substitute the placeholder line `_Lane attribution unavailable._` for every header row and log: `**⚠ 3: report: render-lane-status failed; lane attribution unavailable.**`.

Print the final research report under a `## Research Report` header. Required sections (in order):
- `**Research question**:`: `<RESEARCH_QUESTION>`
- `**Codebase context**:`: branch and commit
- `**Research phase** (4 lanes):`: one bullet each for `<RESEARCH_ARCH_HEADER>`, `<RESEARCH_EDGE_HEADER>`, `<RESEARCH_EXT_HEADER>`, `<RESEARCH_SEC_HEADER>`
- `**Validation phase** (3 reviewers):`: one bullet each for `<VALIDATION_CODE_HEADER>`, `<VALIDATION_CURSOR_HEADER>`, `<VALIDATION_CODEX_HEADER>`
- `### Findings Summary`: synthesized validated findings by topic
- `### Risk Assessment`: Low/Medium/High + rationale, or N/A
- `### Difficulty Estimate`: S/M/L/XL + rationale, or N/A
- `### Feasibility Verdict`: feasibility + rationale, or N/A
- `### Key Files and Areas`: relevant files/modules/areas
- `### Open Questions`: unresolved questions or areas needing further investigation

When any external research lane ran as a Claude-fallback (`N_FALLBACK >= 1` per the §1.5 banner preamble in `${CLAUDE_PLUGIN_ROOT}/skills/research/references/research-phase.md`), Step 1.5 prepends a reduced-diversity banner under `## Research Synthesis`. The banner is computed by the canonical executable helper `"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" research banner` (contract in `crates/larch-cli/src/research_commands.rs`); the orchestrator forks the helper with the lane-status.txt path and prepends the helper's stdout to the synthesis subagent's body before writing `research-report.txt`. The banner contract is guarded by `crates/larch-cli/src/research_commands/tests.rs`. Example degraded-path preview (one Codex angle fell back):

```markdown
## Research Synthesis

**⚠ Reduced lane diversity: 1 of 4 external research lanes ran as Claude-fallback. The model-family heterogeneity claim does not hold for this run.**

[Then the usual agree / diverge / significance / architectural patterns / risks content follows...]
```

If risk assessment, difficulty estimate, or feasibility verdict are not applicable to the nature of the research question (e.g., a pure "how does X work?" question), mark them as **N/A** with a brief explanation.

<!-- step:3 final-report write + sidecar generation -->

Single authoritative emission: write the full templated report block (header lines + every section above, with substituted values) to `$RESEARCH_TMPDIR/research-report-final.md` first, **append the Step 2.5 citation-validation sidecar** when present, then `cat` the file for user-visible output.

The orchestrator writes the rendered report block to `$RESEARCH_TMPDIR/research-report-final.md` via Bash. Wrap the write in an explicit `if … ; then … ; fi` guard so a `cat >` failure (disk full, permission, etc.) does NOT abort the orchestrator block under `set -euo pipefail`: the cleanup at Step 4 must still run. On write failure, set a mental flag `SKIP_SIDECAR=true` and emit a warning. Also write the research question to `$RESEARCH_TMPDIR/research-question.txt` so the helper can embed it in the audit-context line.

**Citation-validation splice** (Step 2.5 → Step 3 contract): after `research-report-final.md` is written successfully AND `SKIP_SIDECAR != true`, append the citation-validation sidecar to it when present. The sidecar already opens with the `## Citation Validation` header so no extra header is added; a single blank line separates the report block from the spliced section. On a missing/empty sidecar (Step 2.5 was skipped per its input gate), the splice is a no-op:

```bash
if [[ "$SKIP_SIDECAR" != "true" ]] && [[ -s "$RESEARCH_TMPDIR/citation-validation.md" ]]; then
  printf '\n' >> "$RESEARCH_TMPDIR/research-report-final.md"
  cat "$RESEARCH_TMPDIR/citation-validation.md" >> "$RESEARCH_TMPDIR/research-report-final.md"
fi
```

The append happens BEFORE the `scripts/larch.sh research render-findings-batch` invocation below AND BEFORE the user-visible `cat`.

**Mental flag init**: initialize `SKIP_SIDECAR=false` at the top of Step 3 before the guarded write block above.

After the write succeeds, invoke the helper to generate the sidecar:

```bash
if [[ "$SKIP_SIDECAR" != "true" ]]; then
  if ! "${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" research render-findings-batch \
    --report "$RESEARCH_TMPDIR/research-report-final.md" \
    --output "$RESEARCH_TMPDIR/research-findings-batch.md" \
    --research-question-file "$RESEARCH_TMPDIR/research-question.txt" \
    --branch "$CURRENT_BRANCH" --commit "$HEAD_SHA"; then
    echo "**⚠ 3: report: render-findings-batch helper exited non-zero (likely empty findings: see warning above). Continuing.**"
  fi
fi
```

Helper exit 3 (empty findings) is non-fatal: the helper writes an empty sidecar and prints a warning to stderr. Exits 1 / 2 indicate operator/orchestrator bugs and are also logged but non-fatal here. See `crates/larch-cli/src/research_commands.rs` for the full contract; the offline regression tests live in `crates/larch-cli/src/research_commands/tests.rs`.

Finally print the report for user visibility, gated on a successful write:

```bash
if [[ "$SKIP_SIDECAR" != "true" ]] && [[ -s "$RESEARCH_TMPDIR/research-report-final.md" ]]; then
  cat "$RESEARCH_TMPDIR/research-report-final.md"
fi
```

<!-- step:3.5: Auto-file Research Issue -->

Print: `> **🔶 /research 3.5: auto-issue**`

Automatically create a GitHub issue containing the research question, full report, and token spend metadata. This is a convenience/archival feature: the research report (already visible in the terminal) is the core deliverable; the issue is a persistent copy.

**Skip conditions** (any true → print skip breadcrumb, proceed to Step 4):

- `RESEARCH_NO_ISSUE=true` (user passed `--no-issue`): print `⏩ 3.5: auto-issue: skipped (--no-issue) (<elapsed>)`.
- `$RESEARCH_TMPDIR/research-report-final.md` is missing or empty: print `⏩ 3.5: auto-issue: skipped (no report file) (<elapsed>)`.

### Compose issue body

Capture token spend by running `scripts/larch.sh token lane-write/lane-report report` with the same arguments as Step 4:

```bash
# lint-consecutive-bash: ok token snapshot feeds deferred issue-body composition refactor
TOKEN_SPEND=$("${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" token lane-report \
  --dir "$RESEARCH_TMPDIR" 2>/dev/null || echo "_(token telemetry unavailable)_")
```

Write to `$RESEARCH_TMPDIR/research-issue-body.md` via Bash:

```bash
{
  echo "**Research question**: $RESEARCH_QUESTION"
  echo "**Branch**: $CURRENT_BRANCH"
  echo "**Commit**: $HEAD_SHA"
  echo ""
  cat "$RESEARCH_TMPDIR/research-report-final.md"
  echo ""
  echo "$TOKEN_SPEND"
} > "$RESEARCH_TMPDIR/research-issue-body.md"
```

On write failure, print `**⚠ 3.5: auto-issue: failed to compose issue body. Continuing.**` and proceed to Step 4.

### Compose title

Derive from `RESEARCH_QUESTION`: `[Research Report] <RESEARCH_QUESTION>`, truncated so the full title fits within `/issue`'s 80-character title limit. No timestamp, no SHA. Dedup is skipped via `--no-dedup` because each research run produces genuinely different content.

### Invoke /issue

> **Continue after child returns.** When the child Skill returns, execute the NEXT step of this skill: do NOT end the turn, and do NOT write a summary, handoff, or "returning to parent" message. → shared/subskill-invocation.md#anti-halt

1. Clear stale sentinel:

   ```bash
   rm -f "$RESEARCH_TMPDIR/research-issue.sentinel"
   ```

2. Invoke `/issue` via the Skill tool in single mode:

   ```
   --sentinel-file $RESEARCH_TMPDIR/research-issue.sentinel --body-file $RESEARCH_TMPDIR/research-issue-body.md --no-dedup --label research [Research Report] <truncated question>
   ```

   No `--go`: this is archival/tracking output, not an approved work queue item.

3. Parse `/issue` stdout for `ISSUES_CREATED`, `ISSUES_DEDUPLICATED`, and per-issue `ISSUE_<i>_NUMBER` / `ISSUE_<i>_URL`.

4. Sentinel verification (defense in depth):

   ```bash
   "${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" verify skill-called --sentinel-file "$RESEARCH_TMPDIR/research-issue.sentinel"
   ```

   Parse `VERIFIED` and `REASON`.

5. **On success** (`VERIFIED=true` AND `ISSUES_CREATED >= 1`): proceed to Step 4.

6. **On dedup** (`VERIFIED=true` AND `ISSUES_CREATED == 0` AND `ISSUES_DEDUPLICATED >= 1`): proceed to Step 4.

7. **On failure** (`VERIFIED=false`, or `/issue` error, or `ISSUES_FAILED >= 1`): remove `"$RESEARCH_DENY_ACTIVE_SENTINEL"`, print `**⚠ 3.5: auto-issue: /issue failed (REASON=<token>). Research results were not archived to GitHub. Continuing.**`, and proceed to Step 4.

<!-- step:4: Cleanup and Final Warnings -->

Print: `> **🔶 /research 4: cleanup**`

### Token Spend report

Render the `## Token Spend` section before `cleanup-tmpdir.sh` so sidecars under `$RESEARCH_TMPDIR` are still readable. The script owns the full section (header + body): SKILL.md just executes it and prints the stdout:

```bash
"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" token lane-report --dir "$RESEARCH_TMPDIR"
```

The script is a no-op-safe call: when no sidecars exist, it prints a `_(no measurements available)_` placeholder; if `$RESEARCH_TMPDIR` was already removed, it prints `_(token telemetry unavailable)_`. Either path exits 0. See `crates/larch-cli/src/token_commands.rs` for the full contract.

### Cleanup tmpdir

Remove the session temp directory and all files within it (unconditional):

```bash
rm -f "$RESEARCH_DENY_ACTIVE_SENTINEL"
"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" session cleanup-tmpdir --dir "$RESEARCH_TMPDIR"
```

**Repeat any external reviewer warnings** from earlier steps (Step 0a binary checks, Step 1 research-phase failures/timeouts, or Step 2 validation failures) so they are visible at the end of the workflow. For example:
- `**⚠ Codex not available: <reason>**`
- `**⚠ Cursor review failed: <reason>**`
- `**⚠ Codex research timed out / produced empty output**`
