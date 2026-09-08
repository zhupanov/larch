# External Agents

## Availability Checks

At the start of each skill, a binary check plus runtime probe (`scripts/larch.sh agent check-reviewers` via `session-setup.sh --check-reviewers`) determines which external tools are usable:

- If **Codex** is unavailable (binary not on `PATH`, or its runtime auth/quota probe fails), a warning is printed
- If **Cursor** is unavailable (same two failure modes), a warning is printed

Probe retries keep separate budgets. `LARCH_PROBE_RETRIES` covers transient
`rc == 1` failures. `LARCH_EXTERNAL_AUTH_RETRIES` covers auth-classified
failures. `LARCH_PROBE_TIMEOUT_RETRIES` covers timeout exits only, defaults to
`0`, and therefore leaves the default health-gate timeout latency unchanged.
Cursor keychain preflight and preread on Darwin run under the shared external
startup lock unless `CURSOR_API_KEY` is already usable.

**Degraded-tools gate (issue #3207).** Beyond the warning, every skill that uses external tools (`/design`, `/implement`, `/review`, `/research`) runs the **Degraded-tools gate** in Step 0 (`scripts/larch.sh agent degraded-tools-gate`; procedure in `skills/shared/external-reviewers.md`). Healthy probes proceed silently. When one tool is unavailable, the gate presents an explanation (what is down, why, binary-missing vs runtime-probe-failed, and the degradation to expect) and requires explicit **Continue** or **Abort** before proceeding; in non-interactive, CI, eval, autonomous-loop, and `/review --subagent` contexts, it emits a prompt-required envelope instead of auto-proceeding. If the Continue sentinel (`.degraded-tools-gate-prompted`) already exists from a prior operator choice, one-down runs proceed degraded in every mode, including non-interactive resume. When both tools are unavailable, the gate hard-fails in every mode, ignores stale sentinels, and does not ask Continue or Abort.

**Codex auth scope.** Covered Codex paths prefer a live non-whitespace `OPENAI_API_KEY` through per-invocation `-c` provider overrides; unset, empty, or whitespace-only falls back to `codex login` / `~/.codex/auth.json`. The merged inventory is: `scripts/larch.sh agent launch-review --tool codex`, `scripts/larch.sh agent launch-codex-ci`, `scripts/larch.sh agent launch-codex-implement`, the Codex health probe in `scripts/larch.sh agent check-reviewers`, `scripts/larch.sh review-and-fix apply-findings`, `scripts/larch.sh agent launch-codex-exec`, `/research` Codex research lanes, `/research` validation lane, shared Codex voter/judge fences, `scripts/larch.sh checks lint-fix`, and `scripts/larch.sh agent run-negotiation-round`.

## Trust boundary (filesystem access)

Every external-reviewer command enters through `scripts/larch.sh` and is composed by the Rust CLI. Vendor argv is built from typed requests in `larch-core`; `TokioProcessRunner` in `larch-adapters` is the only production spawn owner. It clears ambient environment state, applies explicit credential overrides, bounds captured output, and terminates the child process tree across nested groups on cancellation or timeout. The removed Python agent package is not a fallback, and runtime skills, hooks, and scripts do not launch vendor binaries directly.

That process boundary is not an operating-system sandbox. Each review command selects the vendor's reviewed read-only or plan-mode argv and retains dirty-tree backstops, but a delegated same-user process can still have workspace authority supplied by its platform. The canonical limits are documented in [Workflow Trust, Mutation, and Private Findings](security/workflow-trust-and-mutations.md#permissions-tools-and-delegated-processes).

## Persistent vendor sessions

`/debate` uses the Rust persistent-session transport for its Cursor and Codex participants.

- **Cursor**: `cursor agent create-chat` (option-free) returns a chat ID. Resume uses `cursor agent -p --resume <chatId> --mode plan --trust --output-format json --workspace <workdir> …`.
- **Codex initial**: reuses the existing read-only `codex exec --sandbox read-only -C <workdir> … --json` builder.
- **Codex resume**: `codex exec resume <UUID> … -c sandbox_mode="read-only" --json --output-last-message …`. Resume omits `--sandbox`, `-C`, and `--add-dir`; the caller must launch with `cwd=<workdir>`.
- **UUID capture**: the Rust session parser accepts exactly one structured Codex `thread.started` / `thread_id` event. Capture failure is fail-closed and terminal with `session-capture-failed`, no auth retry, and no second subprocess.
- **`--last` is prohibited** by construction. Resume builders take an explicit validated handle only.

## Launching External Reviewers

External reviewers are launched via `scripts/larch.sh agent run-external-agent`, which provides:

- **Timeout enforcement** — Kills the process after a configurable timeout
- **Sentinel file creation** — Writes a `.done` file containing the exit code when the process completes
- **Output capture** — two patterns, opt-in per invocation:
  - **stdout capture under `--capture-stdout`** — when the reviewer writes its results to stdout, pass `--capture-stdout` and the Rust launcher redirects the tool's stdout/stderr to `--output`. Cursor pattern; canonical examples at `skills/review/SKILL.md:146-148, 177-179`.
  - **tool-managed output path** — when the reviewer takes its own output-path argument (e.g., Codex's `--output-last-message`), omit `--capture-stdout`; the launcher does not capture stdout and the reviewer writes results directly to the file. The `--output` flag still names the expected destination so downstream readers know where to look. Codex pattern; canonical examples at `skills/review/SKILL.md:160-163, 186-190`.
- **Elapsed time tracking** — Reports how long the review took

During review and voting phases, reviewers are launched with `run_in_background: true` so they run concurrently with other work. (Negotiation rounds in `/research` run synchronously.)

## Launch Order

External reviewers are always launched in a specific order to maximize parallelism — **slowest first**:

1. **Cursor** (slowest) — launched first
2. **Codex** — launched second
4. **Claude subagents** (fastest) — launched last

All launches happen in a single message to ensure true parallel execution.

## Sentinel File Monitoring

The Rust launcher writes a `.done` sentinel file when the process completes. This is the only reliable way to detect completion:

- **Do not read output files until the sentinel exists** — Cursor buffers all stdout until exit, so its output file is empty until the process finishes
- **Poll for sentinels** using `${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh agent wait-reviewers`, which checks every 5 seconds and prints compact progress dots
- Sentinel files contain the exit code (e.g., `0` for success)

## Output Validation

Validation happens in two layers. The first layer (default collector behavior) always runs; the second layer (substantive-content check) is **opt-in** via collector flags.

### Default collector behavior (always on)

After the sentinel file exists, `scripts/larch.sh agent collect-results` performs:

1. Read the output file.
2. Check that it is non-empty.
3. If empty despite exit code 0, **retry once** with a fresh invocation (output file gets a `-retry` suffix).
4. If the initial row is `FAILED`, `TIMED_OUT`, or `SENTINEL_TIMEOUT` with a transient-network diagnostic, retry once through the same `.meta` replay path.
5. If still empty after retry, if retry also fails, or if the initial failure is not retry-eligible, emit `STATUS=EMPTY_OUTPUT` / `STATUS=FAILED` / `STATUS=TIMED_OUT` / `STATUS=SENTINEL_TIMEOUT` and the caller falls back per its skill-specific contract (typically Runtime Timeout Fallback — see `skills/shared/external-reviewers.md`).

Treat `STATUS=OK` with empty `FAILURE_REASON` as the success signal; do not use `EXIT_CODE` alone. `EXIT_CODE=0` can still appear on retry-failure rows when the retry sentinel was `0` but the retry output stayed empty (`STATUS=EMPTY_OUTPUT`). See `crates/larch-cli/src/collector_commands.rs` for the full retry-row exit-code semantics.

### Opt-in substantive-content check

When the collector is invoked with `--substantive-validation`, it additionally validates each `STATUS=OK` output through the Rust `eval validate-research-output` owner in-process. Validator failure is rewritten to `STATUS=NOT_SUBSTANTIVE`, and the caller treats it identically to a timeout (Claude-subagent fallback). This catches outputs that pass sentinel + non-empty + retry but contain only banner text (e.g., `Authentication required`) or other non-substantive content.

The optional `--validation-mode` modifier forwards `--validation-mode` to the validator, which (a) lowers the body-word floor from 200 to 30, (b) accepts the canonical JSON no-findings sentinel `{"no_issues_found": true}` and legacy `NO_ISSUES_FOUND` token as substantive without further checks, (c) accepts the shipped reviewer template's short no-findings prose (`### In-Scope Findings` plus `No in-scope issues found.`) as substantive, (d) maps `CURSOR_EMPTY_RESPONSE` to its own status, and (e) keeps the citation requirement unchanged. External reviewers should still prefer the JSON sentinel for no-findings output. Arbitrary thin narration, such as process updates or mixed prose around the no-findings sentence, still fails validation.

**Currently opted in by:**

| Caller | Flags |
|--------|-------|
| `/research` research phase (Standard / Deep) | `--substantive-validation` (no `--validation-mode`; 200-word floor + citation requirement; outputs are 2-3-paragraph research prose) |
| `/research` validation phase (Step 2.4) | `--substantive-validation --validation-mode` (30-word floor + no-findings sentinel short-circuit + `CURSOR_EMPTY_RESPONSE` mapping + citation requirement; outputs are short numbered findings) |
| `/review` Step 3a code review | `--substantive-validation --validation-mode` |
| `/design` Step 3 plan review | `--substantive-validation --validation-mode` |

Authoritative flag documentation lives in the usage line and option grammar of `crates/larch-cli/src/collector_commands.rs`; update both this section and that module in lockstep when adding a new caller.

## Timeout Handling

- The process group is terminated by the shared Rust process runner
- The sentinel file records a non-zero exit code
- A warning is printed and the skill proceeds without that reviewer

## Roles Across the Workflow

External reviewers participate in multiple phases:

The **fallback taxonomy** (issue #3207 audit): **full waterfall** = the assigned external tool → the *other* external tool → Claude, per slot (via `scripts/larch.sh agent dispatch-waterfall` for reviewer panels, or selection/runtime tiers for coders); **replacement-first** = tool unavailable → Claude directly (no cross-tool tier); **skip** = tool unavailable → slot dropped, no substitution.

| Phase | Role | Skills | Fallback behavior |
|---|---|---|---|
| Plan review | Review implementation plans | `/design` | Registry role `design.plan_review_panel`. Static archetypes are `arch`, `innovation`, `pragmatic`, and `requirements`; Cursor reviewer rows resolve to `composer-2.5` by default when Cursor is available; Codex rows emit when Codex is available. All Codex and dynamic rows use the `review` role. No generic Codex row emits. Reviewer panels dispatch with `--no-fallback`, so missing vendors drop rows instead of backfilling. |
| Code review | Review code changes | `/review`, `/implement` Step 5 | Registry role `review.panel`. Cursor reviewer rows resolve to `composer-2.5` by default when Cursor is available; Codex static specialists emit when Codex is available. The three static slots are `correctness`, `edge-cases`, and `testing`; Codex static and dynamic rows use the tier-aware `review` role. No generic Codex row emits. Reviewer panels dispatch with `--no-fallback`, so missing vendors drop rows instead of backfilling. |
| [Voting](voting-process.md) | Vote on findings | `/design`, `/review` | Registry roles `design.plan_voters` and `review.voters` share the same three-slot shape: validity, plan-fidelity, and pragmatism are **Codex→Cursor→Claude** in one waterfall manifest when either external is present, with `--claude-read-tools-add-dir` for plan-voter Claude fallbacks. Both external tools down shrinks to one dedicated Claude voter-1 floor. `VOTER_N_TOOL` uses binding-resolved semantic labels such as `codex-validity` or `cursor-validity`. |
| Plan revision | Apply accepted plan findings | `/design` | Registry role `design.plan_revision`: **Codex→Cursor→Claude**. Codex fixers use the fix role, default `gpt-5.6-terra`; Cursor stays `composer-2.5`; Claude uses `claude-sonnet-4-6[1m]`. |
| Implementer (Step 2) | Write the implementation | `/implement` | Registry role `implement.step2_coder` makes a first-eligible single pick: **TRIVIAL** and **MODERATE** use **Cursor (`cursor-grok-4.6-high`)→Codex (`gpt-5.6-terra`)→Claude**; **HARD** uses **Codex (`gpt-5.6-terra`)→Cursor→Claude**. `--coder codex` and `--coder cursor` put the selected external tool first, retain the other external tool as the next fallback, and leave Claude last. `--coder claude` selects Claude directly. Codex/Cursor implementers receive valid `ARCHITECTURAL_INVARIANTS.md` before valid `ARCHITECTURAL_GUIDELINES.md` through the launch prompt as untrusted, plan-scoped evidence and must return `architectural_acknowledgment` in the Step 2 manifest when knowledge was supplied. |
| review-and-fix coders | Apply accepted review fixes | `/implement`, `/review` | Registry role `review.fix_coder`: **Codex→Cursor→Claude**, then main-agent-required. Codex fixers use the fix role, default `gpt-5.6-terra`; Cursor resolves to `composer-2.5` by default; Claude uses `scripts/larch.sh agent launch-claude-review-fix` with `claude-sonnet-4-6[1m]`. Successful automated tiers with no edits fall through to the next tier or main-agent-required after registry exhaustion. |
| lint-fix coders | Repair local lint/check failures | `/implement`, `/review` | Registry role `implement.lint_fix_coder`: **Claude/Opus 4.8→Codex→Cursor**, then main-agent-required. |
| Architectural assessment | Assess Step 8 architectural evidence | `/implement` | No vendor waterfall. A read-only in-session `larch:arch-assessor` Claude Code Agent-tool subagent (`agents/arch-assessor.md`, Read/Grep/Glob only) authors every requested kind from materialized evidence paths supplied by `"$CLAUDE_PLUGIN_ROOT/scripts/larch.sh" architectural-assessment materialize`; the main agent persists each note fail-closed via `architectural-assessment submit`. The main agent never reads the evidence diff or the architectural knowledge files on this path. Docs-only diffs persist `deterministic-clean` with zero subagent spawns. The fix ladder spawns a `larch:claude-implementer` coder subagent for adverse verdicts. |
| CI / checks recovery | Fix failing CI/checks | `/implement` (active Step 8+ driver) | Registry role `implement.ci_recovery_fixer`: **Codex→Cursor→Claude**. Rebase conflict fixing is a distinct role, `implement.rebase_conflict_fixer`: **Claude→Codex→Cursor**. |
| Brainstorm framing/scope | Generate optional ideation | `/design` | Registry roles `design.brainstorm_framing` and `design.brainstorm_scope` are consumed by Step 1d.5 through `external-defaults role`. Framing defaults Cursor→Codex→Claude; scope defaults Codex→Cursor→Claude. Pragmatic brainstorming is parent-session Claude and is not registry-backed. |
| Decompose panel | Propose issue partitions | `/design` | Registry role `design.decompose_panel`: Cursor and Codex are allowed parallel tools, but only present vendors emit rows per archetype. The both-absent Claude generic path remains explicit dispatch logic. |
| Decompose aggregator | Merge partition proposals | `/design` | Registry role `design.decompose_aggregator`: Codex-primary single slot. |
| Debate panel | Argue a proposal in persistent vendor sessions | `/debate` | Registry role `debate.panel`. Fixed Cursor, Codex, and Claude slots with no silent substitution. The Claude slot is an in-session Agent-tool subagent; Cursor and Codex use the typed subprocess transport. |
| Debate synthesizer | Synthesize a converged debate into a proposal | `/debate` | Registry role `debate.synthesizer`: Codex-primary with fixed Codex, then Cursor, then Claude fallback order. |
| Findings aggregators | Merge review findings | `/design`, `/review`, `/implement` Step 5 | Registry roles `review.findings_aggregator` and `design.plan_findings_aggregator`: Codex-primary single slots through dispatch-waterfall, with Codex using the `review` model role before Cursor or Claude fallback. |
| Negotiation | Multi-round dispute resolution | `/research` | Replacement-first |
| Research lanes | Read-only investigation | `/research` | Replacement-first (Codex→Claude; Cursor deliberately excluded for diversity banner) |
| Dynamic-archetype scout | Propose ephemeral reviewer archetypes | Standalone `/review --diff`, `/design` Step 2b | Registry roles `review.dynamic_archetype_scout` and `design.plan_archetype_scout`: **Cursor→Claude**. Codex is not in the scout waterfall. |

`scripts/larch.sh agent launch-claude-review` remains read-only for reviewer and voter lanes. Review-fix uses `scripts/larch.sh agent launch-claude-review-fix` so Claude can edit the working tree without weakening review/voter launchers.
