---
name: research
description: "Use when read-only research is needed. 3 research agents then 3 validation reviewers produce findings summary, risk assessment, difficulty estimates, and feasibility verdict."
argument-hint: "[--debug] <research question or topic>"
allowed-tools: Bash, Read, Grep, Glob, Agent, Task, WebFetch, WebSearch
---

# Research Skill

Collaborative read-only research task using 3 research agents (Claude inline + Cursor + Codex, uniformly briefed) and 3 validation reviewer lanes (Codex deep + Codex broad + Cursor generic). Claude Code Reviewer subagent fallbacks preserve the 3-lane invariant in each phase when Cursor or Codex is unavailable. Produces a structured research report without modifying the repository.

**Flags**: Parse flags from the start of `$ARGUMENTS` before treating the remainder as the research question. Flags may appear in any order; stop at the first non-flag token. After stripping all flags, save the remainder as `RESEARCH_QUESTION`. **All boolean flags default to `false`. Only set a flag to `true` when its `--flag` token is explicitly present in the arguments. Flags are independent — the presence of one flag must not influence the default value of any other flag.**

- `--debug`: Set a mental flag `debug_mode=true`. Controls output verbosity — see Verbosity Control below. Default: `debug_mode=false`.

The research question is described by `RESEARCH_QUESTION` (not raw `$ARGUMENTS`). Use `RESEARCH_QUESTION` wherever human-readable topic text is needed (e.g., agent prompts, report headers, temp file content).

**Read-only contract**: This skill does NOT create branches, modify files, or make commits. All scratch artifacts are written to `/tmp` via Bash. The `allowed-tools` frontmatter omits `Edit`, `Write`, and `Skill` — the orchestrating agent cannot use those tools. External reviewers (Codex, Cursor) are instructed not to modify files, but this is a behavioral constraint (prompt-enforced), not mechanically enforced. Known limitation: concurrent repo changes during a long research run may cause agents to see slightly different snapshots.

## Progress Reporting

**Every step MUST print clearly visible breadcrumb status lines** so the user can instantly see where execution is. Follow the formatting rules in `${CLAUDE_PLUGIN_ROOT}/skills/shared/progress-reporting.md`.

- Print a **start line** when entering a step: e.g., `> **🔶 1: research**`
- Print a **completion line** when done: e.g., `✅ 1: research — synthesis complete, 3 agents (3m12s)`

Step Name Registry:
| Step | Short Name |
|------|------------|
| 0 | setup |
| 1 | research |
| 2 | validation |
| 3 | report |
| 4 | cleanup |

### Verbosity Control

**When `debug_mode=false` (default):**

- Use empty string for the `description` parameter on all Bash tool calls.
- Use terse 3-5 word descriptions for Agent tool calls.
- Do not produce explanatory prose between tool call outputs — only print: step breadcrumb lines (start `🔶`, completion `✅`, skip `⏩`), all warning/error lines (`**⚠ ...`), structured summaries (findings, risk assessments, research report sections), and the compact agent status table (see below).

**Compact agent status table**: After launching research agents (Step 1) or validation reviewers (Step 2), maintain a mental tracker of each agent's status. Print a compact table after EACH status change:

```
📊 Agents: | Claude: ✅ 2m31s | Cursor: ⏳ | Codex: ✅ 3m5s |
```

Icons: ✅ done (with elapsed time since launch), ⏳ pending/in-progress, ❌ failed/timeout (with elapsed time since launch), ⊘ skipped (unavailable). This replaces individual per-agent completion messages in non-debug mode. See `${CLAUDE_PLUGIN_ROOT}/skills/shared/progress-reporting.md` for elapsed time and step start formatting rules.

**Suppressed output (only when `debug_mode=false`):** explanatory prose, script paths, rationale for decisions between tool calls, per-agent individual completion messages.

**When `debug_mode=true`:** use descriptive text for `description` on all Bash and Agent tool calls; print full explanatory text and BOTH status table and per-agent details.

**Limitation**: Verbosity suppression is prompt-enforced and best-effort.

## Step 0 — Session Setup

### 0a — Session Setup and Reviewer Check

Run the shared session setup script:

```bash
${CLAUDE_PLUGIN_ROOT}/scripts/session-setup.sh --prefix claude-research --skip-preflight --skip-branch-check --skip-slack-check --skip-repo-check --check-reviewers
```

If the script exits non-zero, print the error and abort.

Parse the output for `SESSION_TMPDIR`, `CODEX_AVAILABLE`, `CURSOR_AVAILABLE`, `CODEX_HEALTHY`, `CURSOR_HEALTHY`. Set `RESEARCH_TMPDIR` = `SESSION_TMPDIR`. Substitute the actual path in every command below.

Set mental flags `codex_available` and `cursor_available` based on the output:
- If `CODEX_AVAILABLE=false`: `codex_available=false`. Print: `**⚠ Codex not available (binary not found). Proceeding without Codex reviewer.**`
- Else if `CODEX_HEALTHY=false`: `codex_available=false`. Print: `**⚠ Codex installed but not responding (health check failed). Using Claude replacement.**`
- Else: `codex_available=true`
- Same logic for Cursor.

### 0c — Record Research Context

Record the current branch and commit for inclusion in the final report:

```bash
${CLAUDE_PLUGIN_ROOT}/scripts/git-branch-info.sh
```

Parse the output for `HEAD_SHA` and `CURRENT_BRANCH`. If `CURRENT_BRANCH` is empty (detached HEAD), use `"(detached HEAD)"` in the report.

Print: `✅ 0: setup — researching on branch <CURRENT_BRANCH> at <HEAD_SHA> (<elapsed>)`

## Step 1 — Collaborative Research Perspectives

**IMPORTANT: The collaborative research phase MUST ALWAYS run with 3 agents (using Claude subagent fallbacks when an external tool is unavailable). Never skip or abbreviate this phase regardless of how simple the research question appears. Multiple independent perspectives surface insights that a single agent would miss.**

A diverge-then-converge phase where 3 agents independently explore the codebase under a single uniform brief before synthesizing findings. Diversity comes from model-family heterogeneity (Claude + Cursor's backing model + Codex's backing model), not from differentiated per-lane personalities.

The 3 research agents:

1. **Claude (inline)** — the orchestrating agent's own research, run with the shared `RESEARCH_PROMPT` below.
2. **Cursor** (if available) — or a **Claude subagent** fallback via the Agent tool, running the same `RESEARCH_PROMPT`.
3. **Codex** (if available) — or a **Claude subagent** fallback via the Agent tool, running the same `RESEARCH_PROMPT`.

Print `> **🔶 1: research**` and proceed to 1.2.

### 1.2 — Launch Research Perspectives in Parallel

**Critical sequencing**: You MUST launch all external research Bash tool calls (with `run_in_background: true`) AND any Claude subagent fallbacks BEFORE producing your own inline research. External reviewers take significantly longer than Claude — launching them first maximizes parallelism.

**Spawn order**: Cursor first (slowest), then Codex, then any Claude subagent fallbacks, then your own inline research (fastest). Issue all Bash and Agent tool calls in a single message.

**Shared prompt** (used verbatim by all 3 lanes — Cursor, Codex, inline Claude, and any Claude fallbacks):

`RESEARCH_PROMPT` = `"You are researching a codebase to answer this question: <RESEARCH_QUESTION>. Consider alternative perspectives to the obvious interpretation. Actively scrutinize for edge cases, gaps, missing pieces, and assumption failures. Explore the codebase to ground your findings. Write 2-3 paragraphs covering: (1) key findings and observations, including any that challenge the obvious reading, (2) relevant files/modules/areas and architectural patterns, (3) risks, constraints, feasibility concerns, edge cases, and gaps. Do NOT modify files."`

**Cursor research** (if `cursor_available`):

```bash
${CLAUDE_PLUGIN_ROOT}/scripts/run-external-reviewer.sh --tool cursor --output "$RESEARCH_TMPDIR/cursor-research-output.txt" --timeout 1800 --capture-stdout -- \
  cursor agent -p --force --trust $("${CLAUDE_PLUGIN_ROOT}/scripts/reviewer-model-args.sh" --tool cursor) --workspace "$PWD" \
    "<RESEARCH_PROMPT>"
```

Use `run_in_background: true` and `timeout: 1860000` on the Bash tool call.

**Cursor fallback** (if `cursor_available` is false): Launch a Claude subagent via the Agent tool carrying `RESEARCH_PROMPT` verbatim. **Do NOT use `subagent_type: code-reviewer`** — the code-reviewer archetype mandates a dual-list findings output that conflicts with the 2-3 prose paragraph shape this phase requires.

**Codex research** (if `codex_available`):

```bash
${CLAUDE_PLUGIN_ROOT}/scripts/run-external-reviewer.sh --tool codex --output "$RESEARCH_TMPDIR/codex-research-output.txt" --timeout 1800 -- \
  codex exec --full-auto -C "$PWD" $("${CLAUDE_PLUGIN_ROOT}/scripts/reviewer-model-args.sh" --tool codex) \
    --output-last-message "$RESEARCH_TMPDIR/codex-research-output.txt" \
    "<RESEARCH_PROMPT>"
```

Use `run_in_background: true` and `timeout: 1860000` on the Bash tool call.

**Codex fallback** (if `codex_available` is false): Launch a Claude subagent via the Agent tool carrying `RESEARCH_PROMPT` verbatim. Same rule as the Cursor fallback above — **do NOT use `subagent_type: code-reviewer`**.

**Claude research (inline)**: Only after all external and fallback launches are issued, produce your own 2-3 paragraph research inline using `RESEARCH_PROMPT` as your brief. Print it under a `### Claude Research (inline)` header. Write this **before** reading any external or subagent outputs to preserve independence.

### 1.3 — Wait and Validate Research Outputs

Collect and validate external research outputs using the shared collection script. Only include output paths for external reviewers that were actually launched (not Claude fallbacks — those return via Agent tool).

**Zero-externals branch**: If BOTH Cursor and Codex are unavailable (both fell back to Claude subagents), **skip `collect-reviewer-results.sh` entirely** — the script exits non-zero when called with an empty path list. In that case, proceed directly to Step 1.4 with the 3 Claude outputs (inline + 2 fallback subagents).

Otherwise:

```bash
${CLAUDE_PLUGIN_ROOT}/scripts/collect-reviewer-results.sh --timeout 1860 "$RESEARCH_TMPDIR/cursor-research-output.txt" "$RESEARCH_TMPDIR/codex-research-output.txt"
```

Use `timeout: 1860000` on the Bash tool call. **Do NOT** set `run_in_background: true` — this call must block. Only include output paths for external reviewers that were actually launched.

Parse the structured output for each reviewer's `STATUS` and `REVIEWER_FILE`. For any reviewer with `STATUS` not `OK`, follow the **Runtime Timeout Fallback** procedure in `${CLAUDE_PLUGIN_ROOT}/skills/shared/external-reviewers.md`. For research outputs, additionally check that valid output contains at least one paragraph of substantive prose (the script validates non-empty; content validation is the caller's responsibility).

### 1.4 — Synthesis

Read all 3 research outputs (Claude inline + Cursor or its fallback + Codex or its fallback). Produce a synthesis that:

1. Identifies where the perspectives **agree** on key findings
2. Identifies where they **diverge** and makes a reasoned assessment on each contested point
3. Notes which insights from each perspective are most significant
4. Highlights **architectural patterns** observed in the codebase (each lane's prompt requires coverage of this dimension)
5. Highlights **risks, constraints, and feasibility** concerns (each lane's prompt requires coverage of this dimension)

Print the synthesis under a `## Research Synthesis` header. Write the synthesis to `$RESEARCH_TMPDIR/research-report.txt` via Bash so it can be used by Step 2. The file should contain:
- The original research question
- The branch and commit being researched
- The synthesized findings

Print: `✅ 1: research — synthesis complete, 3 agents (<elapsed>)`

## Step 2 — Findings Validation

Print: `> **🔶 2: validation**`

**IMPORTANT: Findings validation MUST ALWAYS run with 3 lanes: Codex deep + Codex broad + Cursor generic. When an external tool is unavailable, Claude Code Reviewer subagent fallbacks preserve the 3-lane invariant — 2 Claude subagents (deep + broad) replace Codex, 1 Claude subagent (generic) replaces Cursor. Never skip or abbreviate this step regardless of how straightforward the findings appear. Reviewers validate against the actual codebase state, catching inaccuracies or omissions that the research phase may have missed.**

Launch **all 3 lanes in parallel** (in a single message). **Spawn order matters for parallelism** — launch the slowest first: Cursor (slowest), then both Codex instances, then any Claude subagent fallbacks (fastest). Each reviewer receives the research report and the original question. Each must **only report findings** — never edit files.

### External Reviewer Setup (if `codex_available` or `cursor_available`)

The research report is already written to `$RESEARCH_TMPDIR/research-report.txt` from Step 1.4, so both Codex and Cursor can read it.

### Cursor Reviewer (generic) (if `cursor_available`)

Run Cursor **first** in the parallel message (it takes the longest):

```bash
${CLAUDE_PLUGIN_ROOT}/scripts/run-external-reviewer.sh --tool cursor --output "$RESEARCH_TMPDIR/cursor-validation-output.txt" --timeout 1800 --capture-stdout -- \
  cursor agent -p --force --trust $("${CLAUDE_PLUGIN_ROOT}/scripts/reviewer-model-args.sh" --tool cursor) --workspace "$PWD" \
    "Review the research findings in $RESEARCH_TMPDIR/research-report.txt for accuracy and completeness. Read the report, then explore the codebase to verify claims. Combine 4 perspectives: (1) General: Are findings accurate? Is anything important missing? Are conclusions well-supported by evidence? (2) Correctness: Are specific code references correct? Are there factual errors about the codebase? (3) Risk/Completeness: Are risks properly identified? Are there blind spots or omissions? (4) Architecture: Are architectural observations accurate? Are there structural patterns that were missed? Return numbered findings with perspective, concern, and suggested correction. If the research is accurate and complete, output exactly NO_ISSUES_FOUND. Do NOT modify files."
```

Use `run_in_background: true` and `timeout: 1860000` on the Bash tool call.

**Cursor fallback** (if `cursor_available` is false): Launch **1 Claude Code Reviewer subagent** via the Agent tool (`subagent_type: code-reviewer`) using the unified Code Reviewer archetype from `${CLAUDE_PLUGIN_ROOT}/skills/shared/reviewer-templates.md` with the research-validation variable bindings below. Attribute as `Code Reviewer (generic)`.

### Codex Reviewer (broad perspective) (if `codex_available`)

Run Codex (broad perspective) **second** in the parallel message (output file name `codex-general-validation-output.txt` is kept for backward compatibility with existing call sites and collect scripts):

```bash
${CLAUDE_PLUGIN_ROOT}/scripts/run-external-reviewer.sh --tool codex --output "$RESEARCH_TMPDIR/codex-general-validation-output.txt" --timeout 1800 -- \
  codex exec --full-auto -C "$PWD" $("${CLAUDE_PLUGIN_ROOT}/scripts/reviewer-model-args.sh" --tool codex) \
    --output-last-message "$RESEARCH_TMPDIR/codex-general-validation-output.txt" \
    "Review the research findings in $RESEARCH_TMPDIR/research-report.txt for accuracy and completeness. Read the report, then explore the codebase to verify claims. Focus on 2 perspectives: (1) General: Are findings accurate? Is anything important missing? Are conclusions well-supported by evidence? (2) Risk/Completeness: Are risks properly identified? Are there blind spots or omissions? Return numbered findings with perspective, concern, and suggested correction. If the research is accurate and complete, output exactly NO_ISSUES_FOUND. Do NOT modify files."
```

Use `run_in_background: true` and `timeout: 1860000` on the Bash tool call.

### Codex Reviewer (deep perspective) (if `codex_available`)

Run Codex (deep perspective) **third** in the parallel message (output file name `codex-deep-validation-output.txt` is kept for backward compatibility):

```bash
${CLAUDE_PLUGIN_ROOT}/scripts/run-external-reviewer.sh --tool codex --output "$RESEARCH_TMPDIR/codex-deep-validation-output.txt" --timeout 1800 -- \
  codex exec --full-auto -C "$PWD" $("${CLAUDE_PLUGIN_ROOT}/scripts/reviewer-model-args.sh" --tool codex) \
    --output-last-message "$RESEARCH_TMPDIR/codex-deep-validation-output.txt" \
    "Review the research findings in $RESEARCH_TMPDIR/research-report.txt for accuracy and completeness. Read the report, then explore the codebase to verify claims. Focus on 2 perspectives: (1) Correctness: Are specific code references correct? Are there factual errors about the codebase? (2) Architecture: Are architectural observations accurate? Are there structural patterns that were missed? Return numbered findings with perspective, concern, and suggested correction. If the research is accurate and complete, output exactly NO_ISSUES_FOUND. Do NOT modify files."
```

Use `run_in_background: true` and `timeout: 1860000` on the Bash tool call.

**Codex fallback** (if `codex_available` is false): Launch **2 Claude Code Reviewer subagents** via the Agent tool (`subagent_type: code-reviewer`), one for each perspective. Attribute as `Code Reviewer (broad perspective)` and `Code Reviewer (deep perspective)`. Add a per-lane instruction in each prompt: for the broad lane, `"Emphasize code quality + risk/integration concerns in your findings."`; for the deep lane, `"Emphasize correctness + architecture concerns in your findings."` Use the shared Code Reviewer archetype variable bindings below.

### Claude Code Reviewer Subagent Variable Bindings (fallback lanes only)

Use the unified Code Reviewer archetype from `${CLAUDE_PLUGIN_ROOT}/skills/shared/reviewer-templates.md`, filling in the variables for **research validation**:

- **`{REVIEW_TARGET}`** = `"research findings"`
- **`{CONTEXT_BLOCK}`**:
  ```
  ## Research question
  {RESEARCH_QUESTION}

  ## Research findings to validate
  {SYNTHESIZED_FINDINGS}
  ```
- **`{OUTPUT_INSTRUCTION}`** = `"What the concern is (inaccuracy, omission, or unsupported claim)"` + `"Suggested correction or addition"`

**Research-specific acceptance criteria**: Accept a finding unless it is factually incorrect (misreads the codebase, references wrong file/line) or is already addressed in the synthesis. For research validation, "factually incorrect" means the finding misidentifies code, misattributes behavior, or contradicts something verifiable by reading source files.

### After all reviewers return

**If any Claude fallback lanes were launched** (Cursor unavailable, Codex unavailable, or both): Process their findings immediately — do not wait for external reviewers before starting. Collect and deduplicate Claude fallback findings first. In the happy path (both Cursor and Codex available) no Claude fallback lanes are launched and this substep is a no-op.

### 2.4 — Collect and Validate External Reviewers

**Zero-externals branch**: If BOTH Cursor and Codex are unavailable (all 3 lanes are Claude fallbacks), **skip `collect-reviewer-results.sh` entirely** (the script exits non-zero with an empty path list) and **skip all external negotiation** below. Merge the 3 Claude fallback findings and proceed to Finalize Validation.

Otherwise, after processing any Claude fallback findings, collect and validate external reviewer outputs using the shared collection script. Only include output paths for reviewers that were actually launched:

```bash
${CLAUDE_PLUGIN_ROOT}/scripts/collect-reviewer-results.sh --timeout 1860 "$RESEARCH_TMPDIR/cursor-validation-output.txt" "$RESEARCH_TMPDIR/codex-general-validation-output.txt" "$RESEARCH_TMPDIR/codex-deep-validation-output.txt"
```

Use `timeout: 1860000` on the Bash tool call. **Do NOT** set `run_in_background: true` — this call must block.

1. Parse the structured output for each reviewer's `STATUS` and `REVIEWER_FILE`. For any reviewer with `STATUS` not `OK`, follow the **Runtime Timeout Fallback** procedure in `${CLAUDE_PLUGIN_ROOT}/skills/shared/external-reviewers.md`. Read valid output files.
2. Merge external reviewer findings into any already-processed Claude fallback findings.

### Codex and Cursor Negotiation (in parallel)

If any external reviewers produced findings, negotiate with each independently. With up to 2 Codex instances (broad + deep perspectives), negotiate with each separately using distinct prompt/output file paths (e.g., `codex-general-negotiation-prompt.txt` / `codex-general-negotiation-output.txt` and `codex-deep-negotiation-prompt.txt` / `codex-deep-negotiation-output.txt`). Run all negotiations **in parallel** when multiple external reviewers produced findings. Use the **Negotiation Protocol** in `${CLAUDE_PLUGIN_ROOT}/skills/shared/external-reviewers.md`, using `$RESEARCH_TMPDIR` as the tmpdir.

**Note on negotiation prompt files**: `/research` does not have `Write` in its `allowed-tools` frontmatter. Create each negotiation prompt file via a Bash heredoc (e.g., `cat > "$RESEARCH_TMPDIR/codex-general-negotiation-prompt.txt" <<'EOF' ... EOF`) or pass the prompt to `run-negotiation-round.sh --prompt-file`. Use distinct prompt file paths per Codex instance so the parallel negotiations do not collide.

Merge accepted/rejected outcomes after all complete.

### Finalize Validation

If any findings were accepted (from Claude subagents, Codex, or Cursor):
1. Print them under a `## Validation Findings` header.
2. Revise the research synthesis to incorporate corrections and additions.
3. Print the revised synthesis under a `## Revised Research Findings` header.

If all reviewers report no issues, print: `✅ 2: validation — all findings validated, no corrections needed (<elapsed>)`

## Step 3 — Final Research Report

Print: `> **🔶 3: report**`

Print the final research report under a `## Research Report` header with the following structure:

```markdown
## Research Report

**Research question**: <RESEARCH_QUESTION>
**Codebase context**: Branch `<CURRENT_BRANCH>`, commit `<HEAD_SHA>`
**Research phase**: <N> agents (Cursor: ✅/❌, Codex: ✅/❌)
**Validation phase**: <N> reviewers (Cursor: ✅/❌, Codex: ✅/❌)

### Findings Summary
<synthesized and validated findings, organized by topic>

### Risk Assessment
<Low/Medium/High with rationale, or N/A if not applicable to this research question>

### Difficulty Estimate
<S/M/L/XL with rationale, or N/A if not applicable>

### Feasibility Verdict
<assessment of feasibility with rationale, or N/A if not applicable>

### Key Files and Areas
<list of the most relevant files/modules/areas identified during research>

### Open Questions
<any unresolved questions or areas that need further investigation>
```

If risk assessment, difficulty estimate, or feasibility verdict are not applicable to the nature of the research question (e.g., a pure "how does X work?" question), mark them as **N/A** with a brief explanation.

Print: `✅ 3: report — complete (<elapsed>)`

## Step 4 — Cleanup and Final Warnings

Remove the session temp directory and all files within it:

```bash
${CLAUDE_PLUGIN_ROOT}/scripts/cleanup-tmpdir.sh --dir "$RESEARCH_TMPDIR"
```

**Repeat any external reviewer warnings** from earlier steps (Step 0b binary checks, Step 1 research-phase failures/timeouts, or Step 2 validation failures) so they are visible at the end of the workflow. For example:
- `**⚠ Codex not available: <reason>**`
- `**⚠ Cursor review failed: <reason>**`
- `**⚠ Cursor research timed out / produced empty output**`
- `**⚠ Codex research timed out / produced empty output**`

Print: `✅ 4: cleanup — research complete! (<elapsed>)`
