# Sub-skill Invocation Conventions

Canonical style guide for larch skills that delegate to other skills via the `Skill` tool. Cited by `AGENTS.md`. When you author a new skill that invokes another skill, follow the patterns below. When you change a convention here, update the cited source-example skills in the same PR (or file a follow-up issue) so the examples stay in sync with the rules.

## Two invocation patterns

Every larch skill that invokes another skill uses exactly one of two first-class shapes. Pick the one that matches your intent.

### Pattern A — Pure delegator (bulleted)

Used when the parent skill mostly forwards to a child with preset flags or light argument assembly. Appears in `skills/im/SKILL.md § Behavior`. Canonical form:

```
Invoke the Skill tool:
- Try skill: "implement" first (bare name). If no skill matches, try skill: "larch:implement" (fully-qualified plugin name).
- args: --merge $ARGUMENTS
```

Keep the block together. The bare-name-first rule is important — see `## Bare-name-then-fully-qualified fallback` below.

### Pattern B — Stateful orchestrator (inline)

Used when the parent runs setup, exports `SESSION_ENV_PATH` for the child session merge, invokes the child, and then parses structured output to continue. Appears in `skills/implement/SKILL.md` (nested `/review` and `/issue` calls only after Phase 1 #3364 — `/implement` no longer nests `/release`; `/design` runs separately before `/implement` on the issue-anchored happy path). Canonical form:

```
Invoke `/implement` via the Skill tool:

- `/implement [--merge] [--no-admin-fallback] [--no-logs-commit] [--coder=<value>] [--forked] [--draft] [--run-id <ID>] <issue-N>`
```

Export `SESSION_ENV_PATH="$PARENT_TMPDIR/session-env.sh"` in the environment before the Skill tool call when the parent owns a caller session-env file — `/implement` Step 0 merges from `SESSION_ENV_PATH` via `session-setup.sh --caller-env` when set; do **not** pass removed `--session-env` argv.

The step heading + explicit Skill-tool line + scannable args shape makes the invocation impossible to miss. Do **not** collapse Pattern B into a single paragraph — see `## Avoid conditional phrasing for sub-skill invocations` below.

The pinned `agent-lint` rule S058 mechanically enforces line-local co-location: every direct-invocation line that says ``Invoke `/<name>`'' (with optional `the` and a bounded `**bold-span**`) must also contain `via the Skill tool` on the same line.

## allowed-tools narrowing heuristic

Set `allowed-tools` to the minimum needed by the parent skill itself — never mirror the child skill's broader tool set. Three tiers cover every larch skill:

| Tier | `allowed-tools` | Example (with stable anchor) |
|---|---|---|
| Pure delegator | `Skill` | `skills/im/SKILL.md` frontmatter (allowed-tools line) — forwards only |
| Delegator that validates first | `Bash, Skill` | `skills/block-issue/SKILL.md` frontmatter — runs Bash helpers before delegating; see `skills/research/SKILL.md § Sub-skill invocation` for another Bash-plus-Skill call-site shape in a stateful parent |
| Hybrid orchestrator | `Skill` plus whatever the parent needs | `skills/implement/SKILL.md`, `skills/review/SKILL.md`, `skills/alias/SKILL.md`, `skills/research/SKILL.md`, `skills/file-bug/SKILL.md`, `skills/triage/SKILL.md`, `skills/complete-umbrella/SKILL.md`, `skills/debate/SKILL.md`: parent runs setup, file I/O, git ops, and post-delegation verification. |

`allowed-tools: Skill` alone is **neither necessary nor sufficient** to classify a skill as a pure delegator — some delegators need `Bash` for input validation. Conversely, a skill with `Skill` in its allowed list is not automatically a delegator; hybrid orchestrators include `Skill` as one tool among many.

When in doubt, start narrow and widen only for tools the parent actually uses. If your skill adds `Skill` to `allowed-tools`, also confirm the frontmatter includes every other tool your parent invokes (Bash, Read, Edit, Glob, Grep, etc.). Omitting a needed tool produces silent runtime denials — not error messages — so the narrowing heuristic must be paired with a concrete accounting of parent tool usage.

## Post-invocation verification

**Scope**: this rule applies to **orchestrators that continue execution based on a child skill's side effects** — e.g., a parent that reads the child's output to decide the next step. Pure forwarders (`/im`, `/block-issue`) are exempt — once they delegate, they do nothing further, so there is nothing to verify.

For every mandatory sub-skill call inside an orchestrator's step, pair the call with a **mechanical check that the parent cannot satisfy without the child's side effects**. The check must read the filesystem, parse stdout, or compare counters — never rely on the child's prose acknowledgement. If the child silently skipped or internally bailed, the check must notice.

Canonical examples:

- **Step 8+ active-driver contract parse** — the orchestrator uses JSON stdout + exit code from the wrapper-internal Rust `ship pr`, not `ship-pr-state.sh` continuation parsing:

  ```bash
  # Parse STATUS, PHASE, OOS_PENDING, STALL_TRACKING, STALL_STEP, RESUME_PHASE,
  # CALLER_KIND, and CONFLICT_FILES (when present) from the scoped state files.
  # Parse JSON stdout first and read CONFLICT_FILES from
  # $IMPLEMENT_TMPDIR/ship-pr-state.sh only for the scoped Exit 4 ship_pr_pre_push handoff.
  # On Exit 4 with RESUME_PHASE=ship-pr-rrr-phase14 and CALLER_KIND=ship_pr_pre_push,
  # run conflict-resolution.md before re-invoking the active Step 8+ driver.
  ```

  See `skills/implement/SKILL.md § Step 8+ — Ship PR State Machine` for the exit-code matrix. Phase 1 (#3364) removed `/implement` `/release` gates on the ship path; use `/release` or manual `.claude/skills/release` when versioning is required outside implement.

- **Parsed stdout machine value after `/issue`** — the orchestrator reads `ISSUES_CREATED=<N>` / `ISSUES_FAILED=<N>` / per-issue `ISSUE_N_NUMBER`/`ISSUE_N_URL` lines from `/issue`'s stdout. `/design` Step 5b captures those lines for `design file-oos-annotate`; without them, the parent cannot verify the batch or annotate filed URLs. See `skills/design/references/finalize-step5.md`. `/implement` Step 9a.1 is not an `/issue` caller.

- **Sentinel file** — on the issue-anchored path, `/implement` Step 0 (`skills/implement/SKILL.md § Step 0 — Session Setup`, via `skills/implement/scripts/step-0-bootstrap.sh --mode initial`; envelope parse per Step 0) copies the parsed plan into `$IMPLEMENT_TMPDIR/plan.txt` — no separate design manifest file is read.

- **Sentinel file (defense in depth), `/research`, `/file-bug`, `/complete-umbrella`, and `/debate` → `/issue`:** when one of these parents invokes `/issue` to file GitHub issues, `/issue` writes a small KV sentinel at the caller-supplied `--sentinel-file <path>` path. The parent runs the canonical `${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh verify skill-called --sentinel-file "<sentinel-path>"` post-return and aborts on `VERIFIED=false`. Defense-in-depth precedence: **stdout parsing of `ISSUES_*` (the immediately-prior bullet) is the primary post-`/issue` mechanical check** for any caller; this sentinel-file gate is defense-in-depth on top of stdout parsing, not a replacement. The sentinel proves *execution* (gate: `ISSUES_FAILED=0 AND !dry_run`), not creation count. See `skills/research/SKILL.md § Filing findings as issues`, `skills/complete-umbrella/SKILL.md § Step 5: File and attach one audit gap`, `skills/debate/SKILL.md § Step 1 - Resolve source`, and `skills/issue/SKILL.md § Sentinel file (post-success)` for the anchored contracts.

- **Verified relation and timestamp — `/triage` → `/block-issue`** — `/triage` requires `SUCCESS=true`, `RELATION_VERIFIED=true`, the exact requested blocked-by edge, and a fresh `UPDATED_AT` before advancing its compare-and-swap timestamp. Its `/issue` calls use both canonical `ISSUES_*` and per-item stdout parsing plus `verify skill-called` on a caller sentinel. Either missing postcondition aborts remaining dependency and follow-up processing.

If you cannot name a concrete mechanical check, the call is not actually mandatory — reclassify it as Pattern A (pure delegation) or restructure so the child's side effect is observable.

See `## Anti-halt continuation reminder` below — the two sections govern the same call-site boundary from complementary directions (verification asks "did the child run?"; anti-halt asks "did the parent continue?").

<a id="anti-halt"></a>
## Anti-halt continuation reminder

**Scope**: this rule applies to the same orchestrator set as `## Post-invocation verification` above: stateful orchestrators (`/implement`, `/review`, `/alias`, `/research`, `/file-bug`, `/triage`, `/umbrella`, `/complete-umbrella`, `/debate`) that run additional steps after a child `Skill` tool call returns. Pure forwarders (`/im`, `/block-issue`) are exempt; once they delegate, they do nothing further. The two sections are complementary: `## Post-invocation verification` asks **"did the child run?"**; this section asks **"did the parent continue?"** Both failure modes are distinct and real (see GitHub issue #177 for the originating report).

**The rule**: after every child `Skill` tool call (`/design`, `/review`, `/release`, `/issue`, `/implement`) returns AND after every `Bash` tool call that completes a numbered step or sub-step, including `scripts/larch.sh checks run-relevant`, the main agent MUST immediately continue with the parent skill's NEXT step. The child's cleanup / summary output and helper stdout are NOT end-of-turn. Visible outputs (plans, diagrams, voting tallies, skip breadcrumbs, PR URLs, helper KEY=VALUE envelopes) are intermediate artifacts, NOT stopping points. Likewise, a summary, handoff, status recap, or "returning to parent" turn-ending message is a halt in disguise, not a valid continuation. In long sessions where the child produces many tokens (e.g., `/design` with 3 reviewers + voting easily produces 15k+ tokens), the main agent's attention can drift to the child's local "mission accomplished" framing and lose the parent orchestration frame. A short, standardized banner at the top of every orchestrator plus short per-call-site micro-reminders reinforce the rule where attention is most at risk.

**Carve-out (critical)**: the rule is strictly subordinate to any explicit non-sequential control-flow directive in the parent skill — including `skip to Step N`, `bail to cleanup`, `jump back to Step Na`, `loop back to Step 3a`, `fall through to 12c`, `break out of the loop`, or any other explicit redirect. A normal numerically-sequential `proceed to Step N+1` directive is the default continuation path that anti-halt reinforces — NOT an exception.

**Loop-internal carve-out**: when an orchestrator's step explicitly loops (a hypothetical Skill-tool call inside a loop body), the "next step" of the parent IS the loop-continuation directive, not the first textually-following section header. Use the loop-aware micro-reminder variant at loop-internal child-Skill call sites.

**Generic relevant-checks clause**: every direct `scripts/larch.sh checks run-relevant` helper call anywhere in an orchestrator SKILL.md is covered by this rule. The parent must resume after the check returns — whether that means advancing to the next numbered step, re-running validation after a fix, or committing the fixed files.

**/design note**: `/design` may cite `#anti-halt` for the generic numbered-step continuation core while retaining its local deltas in `skills/design/SKILL.md`. This does not add `/design` to the `test-anti-halt-banners.sh` scope list below.

### Canonical banner (top of each orchestrator SKILL.md, after the title body, before `## Progress Reporting`)

````markdown
**Anti-halt continuation reminder.** After every child `Skill` tool call (e.g., `/design`, `/review`, `/release`, `/issue`, `/implement`) returns AND after every numbered-step `Bash` helper call, including `scripts/larch.sh checks run-relevant`, IMMEDIATELY continue with this skill's NEXT numbered step — do NOT end the turn on the child's cleanup output or helper stdout, and do NOT write a summary, handoff, status recap, or "returning to parent" message — those are halts in disguise. The rule is strictly subordinate to any explicit non-sequential control-flow directive in THIS file (e.g., `skip to Step N`, `bail to cleanup`, `jump back`, `loop back`, `fall through`, `break out`). A normal sequential `proceed to Step N+1` instruction is the default continuation this rule reinforces, NOT an exception. Every relevant-checks helper call anywhere in this file is covered by this rule. → shared/subskill-invocation.md#anti-halt
````

The substring `**Anti-halt continuation reminder.**` is a contract token asserted by `${CLAUDE_PLUGIN_ROOT}/scripts/test-anti-halt-banners.sh`.

### Canonical micro-reminder (per Skill-tool call site — branch-specific placement)

Place the micro-reminder **inside the specific branch that actually invokes the child** — not at the top of a step whose body may skip the invocation on some branches (e.g., `/implement` Step 0 tails that skip a nested child on a branch; `/design` Step 5b OOS branches that skip `/issue` when the combined file is empty). The reminder belongs next to the real Skill-tool call, inside the branch that emits it.

Standard variant:

````markdown
> **Continue after child returns.** When the child Skill returns, execute the NEXT step of this skill — do NOT end the turn, and do NOT write a summary, handoff, or "returning to parent" message. → shared/subskill-invocation.md#anti-halt
````

Loop-aware variant (for loop-internal Skill-tool call sites in orchestrators with explicit loop bodies):

````markdown
> **Continue after child returns (loop-internal).** When the child Skill returns, continue the loop per the parent's explicit loop-back directive — do NOT exit the loop unless the exit condition fires, and do NOT write a summary, handoff, or "returning to parent" message. → shared/subskill-invocation.md#anti-halt
````

The substring `Continue after child returns` is a contract token asserted by `${CLAUDE_PLUGIN_ROOT}/scripts/test-anti-halt-banners.sh` (matches both the standard and loop-internal variants).

### Scope list

The banner MUST appear in these orchestrator SKILL.md files:

- `skills/implement/SKILL.md`
- `skills/review/SKILL.md`
- `skills/alias/SKILL.md`
- `skills/research/SKILL.md`
- `skills/file-bug/SKILL.md`
- `skills/triage/SKILL.md`
- `skills/umbrella/SKILL.md`
- `skills/complete-umbrella/SKILL.md`

The banner MUST NOT appear in pure-delegator SKILL.md files:

- `skills/im/SKILL.md`
- `skills/block-issue/SKILL.md`

Both presence and absence are enforced by `${CLAUDE_PLUGIN_ROOT}/scripts/test-anti-halt-banners.sh`, wired into `make lint` via the `test-anti-halt` target.

<a id="step-boundary"></a>
## Step-boundary anti-halt

**Scope**: this rule covers numbered-step boundaries where the parent skill has just completed a step or sub-step and the next action is another numbered step, not a child `Skill` return. It is especially important after skip breadcrumbs, status footers, and terminal-sounding helper output where there is no immediately adjacent Skill-tool micro-reminder to re-anchor continuation.

**Canonical form**: use a two-line blockquote at the specific boundary:

````markdown
> **Continue to Step N IMMEDIATELY.** <One-sentence reason this boundary is not terminal and what remains.>
````

This is deliberately separate from the `Continue after child returns` micro-reminder above. The micro-reminder fires at Skill-tool call sites; the step-boundary reminder fires between numbered steps, including Bash-only or prose-only tails. Use the step-boundary form sparingly at halt-prone boundaries and include a pointer to this section on the first such reminder in each SKILL.md so future edits can find the single source of truth.

## Session-env handoff

2. The parent passes `--session-env "$PARENT_TMPDIR/session-env.sh"` to the child.
3. The child reads the file via `"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" session setup ... --caller-env "$SESSION_ENV_PATH"`.

Canonical producers and consumers in the live tree:

- `skills/implement/SKILL.md § Step 0 — Session Setup` allocates `$IMPLEMENT_TMPDIR/session-env.sh` and merges caller keys when `SESSION_ENV_PATH` is set. On the issue-anchored path `/implement` does **not** Skill-invoke `/design` — Preflight reads `larch:plan` from the GitHub issue and Step 0 materializes plan files. Child surfaces that still nest under `/implement` (e.g. review/relevant-checks Bash contracts keyed off `$IMPLEMENT_TMPDIR/session-env.sh`) continue to read the same session-env file per that skill's Bash blocks. It also writes `PREV_IMPLEMENT_TMPDIR=$IMPLEMENT_TMPDIR` so a future `/implement` session can copy the previous session's `larch-logs` subtree into its fresh tmpdir, and `LARCH_CLAUDE_PLUGIN_ROOT` so later Bash blocks can recover `${CLAUDE_PLUGIN_ROOT}` without sourcing the file.
- The same `/implement` handoff may also carry `LARCH_DYNAMIC_ARCHETYPES_MAX=<0..1>` when a parent skill forwarded that cap via caller session-env (for example `LARCH_DYNAMIC_ARCHETYPES_MAX` set in the environment merged through `session-setup.sh --caller-env`); nested review launchers should preserve that validated key through `session-setup.sh --caller-env` / `--write-session-env` so Step 5 can replay the chosen cap.
- `skills/design/SKILL.md § Step 0: Session Setup` and `skills/review/SKILL.md § Step 0 — Session Setup` both accept `--session-env` as an `--caller-env` forward; their Bash blocks also read `LARCH_CLAUDE_PLUGIN_ROOT` directly from that file when `${CLAUDE_PLUGIN_ROOT}` needs rehydration before helper invocation.

<a id="artifact-only-return"></a>
## Artifact-only return contract (nested mode)

When `SESSION_ENV_PATH` is non-empty, a child skill is running in nested mode under a parent orchestrator such as `/implement`. In this mode, child skills emit ONLY:

- a terminal machine footer made of a structured heading plus `KEY=VALUE` lines; and
- artifact file paths needed by the parent to read human-facing content.

All human-readable content must be file-backed. Step breadcrumbs, round summaries, voting tallies, reviewer scoreboards, implementation plans, architecture diagrams, rejected-finding prose, explanatory prose, and status narration are forbidden in parent-visible output when nested. If the parent needs any of that content, the child writes it to an artifact and the parent reads the artifact on demand.

Canonical examples:

- `/design` writes plan, tally, OOS, rejected-finding, accepted-finding, and optional architecture-diagram artifacts, then exports the design manifest for `/implement`.
- `/review --diff` writes `$REVIEW_TMPDIR/review-round-summary.md` before Step 4 returns. When nested, Step 4 copies it to `$(dirname "$SESSION_ENV_PATH")/review-round-summary.md`, suppresses inline prose, and emits only the `### review-result` KV footer plus that artifact path; `/implement` reads the stable parent-tmpdir summary file for its `code-review-tally` log batch.

Standalone invocations (`SESSION_ENV_PATH` empty) preserve their normal visible prose and artifact replay behavior.

### Security — never `source` a session-env file

**Do NOT `source` `session-env.sh`.** Parse it line-by-line with `KEY=VALUE` matching. The file crosses a trust boundary (written by one skill, consumed by another), so `source` would execute arbitrary shell if any line contained `$(...)`, backticks, or command substitution. The canonical safe-parse pattern lives in `"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" session setup` (the `--caller-env` reader).

When your skill consumes a session-env file, always route through `session-setup.sh --caller-env` rather than ad-hoc `while read` loops so the safe-parse invariant is centralized.

### Health sidecar

## Subagent execution topology

`--session-env` forwarding and `/design`'s internal heavy-worker dispatch address orthogonal concerns.

- **`/design` execution topology.** `/design` runs sentinel prep → direct plan drafting → plan review → finalize inline in the current context. There is no Agent-tool subagent dispatch.

`SESSION_ENV_PATH` / `--caller-env` shape what crosses the parent/child call boundary. When `/design` is nested under another orchestrator with a non-empty session env, verbosity suppression in `/design` still follows that nested contract.

### Normative pattern for issue-anchored `/implement`

`/design` authors the `larch:plan` GitHub issue block; `/implement <issue-N>` runs Preflight + plan-adequacy audit, then Step 0's foreground `skills/implement/scripts/step-0-bootstrap.sh --mode initial` (envelope parse per `skills/implement/SKILL.md` Step 0) copies the parsed plan into `$IMPLEMENT_TMPDIR/plan.txt` and resolves `coder=` — the anti-halt banner pins treating the `AUDIT=pass` envelope as **non-terminal** (`do NOT end the turn on the audit-pass envelope`). `/implement` does not dispatch `/design` on this happy path.

## Avoid conditional phrasing for sub-skill invocations

The worst shape, and the one that gets skipped most often, is a single-line conditional paragraph that buries the Skill-tool invocation:

> Call `/implement --merge $ISSUE_NUMBER` after exporting `SESSION_ENV_PATH="$TMPDIR/session-env.sh"`.

Prose conditionals bury the invocation and reliably slip past the executing model — especially mid-run. Rewrite as an explicit numbered sub-step whose center is the `Skill` tool call (or as Pattern B's heading + variant bullets shape), so the Skill-tool call is the visual center of the step.

<a id="bare-name-fallback"></a>
## Bare-name-then-fully-qualified fallback

Skill resolution from a consumer repo differs from resolution inside the larch plugin repo itself. In a consumer repo with the plugin installed, `"implement"` resolves correctly — but in a repo where the plugin is installed under a different namespace, the bare name may miss. Always use the two-step fallback:

- **First**: try the bare name — `"implement"`, `"design"`, `"review"`.
- **Second** (only if no skill matched): try the fully-qualified name — `"larch:implement"`, `"larch:design"`, `"larch:review"`.

Never start with the fully-qualified name — it couples the caller to the plugin namespace and breaks in repos that install the plugin under a different name. The alias generator at `"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" alias generate` emits this fallback automatically for every alias; follow the same shape when authoring an invocation by hand.

## Agent-type qualified-name-first fallback

Agent resolution differs from skill resolution. Plugin-defined agents (e.g., `agents/code-reviewer.md`) are namespaced at runtime as `<plugin-name>:<agent-name>` — the bare name does **not** resolve. This is the opposite of the skill-name pattern, where bare names resolve first.

- **First**: try the fully-qualified name — `"larch:code-reviewer"`.
- **Second** (only if not found): try the bare name — `"code-reviewer"`.

All `subagent_type` references in larch skills use the qualified name `larch:code-reviewer`. If a consumer installs the plugin under a different namespace, the bare-name fallback activates.

---

## Cross-references

- `AGENTS.md § Canonical sources` — lists this file as a canonical source (update triggers live at the bottom of this file).
- `skills/shared/progress-reporting.md` — adjacent contract for step-progress formatting.
- `skills/shared/reviewer-templates.md` — canonical source for the Code Reviewer archetype (parallel shared-doc pattern).

## Update triggers

This file is the canonical source for sub-skill invocation conventions (Pattern A bulleted vs Pattern B inline, `allowed-tools` narrowing heuristic, post-invocation verification for orchestrators, anti-halt continuation reminder for orchestrators (closes #177), `session-env` handoff and safe-parse rule, artifact-only return contract for nested child skills, subagent execution topology and the dual-flag (`--session-env` + `--subagent`) handoff for nested orchestrators (closes #1039), anti-conditional-phrasing for Skill-tool calls, bare-name-then-fully-qualified fallback, agent-type qualified-name-first fallback). Runtime surface (ships to consumers under `skills/`). No generated artifact — update directly. Update trigger: when a cited source-example skill (`/im`, `/alias`, `/implement`, `/review`) changes its invocation pattern, artifact-only nested return behavior, or its anti-halt banner/micro-reminder, update the corresponding example in the guide in the same PR. Additional trigger: when `/design` (`skills/design/SKILL.md` or `skills/design/references/heavy-worker.md`) changes `--subagent`, `--quick`, `--session-env`, manifest export, or nested verbosity behavior, update the `## Subagent execution topology` section in the same PR. `scripts/test-anti-halt-banners.sh` is the regression harness for the anti-halt banner and micro-reminder — it asserts banner presence in orchestrator SKILL.md files, absence in pure-delegator SKILL.md files (`/im`, `/block-issue`), and micro-reminder presence in each orchestrator. `/alias` is classified as an orchestrator because its Step 4 runs a sentinel-file verification after `/implement` returns. `/research` is classified as an orchestrator because it may invoke `/issue` via the Skill tool and continue to its report/cleanup steps. `/file-bug` is classified as an orchestrator because it invokes `/issue` via the Skill tool and continues to parse stdout, verify a sentinel, clean up, and report the issue URL. Wired into `make lint` via the `test-anti-halt` target.
