# Preparing Your Repository for Agent-Assisted Development

Larch works best in a repository that is set up for agents. The good news: the
same setup that makes a repo work well with larch makes it work well with **any**
coding agent (Claude Code, Codex, Cursor, Gemini). This guide is that setup,
distilled from how the larch repository configures itself.

There are two ways to use it:

- **Lift the files.** Point your agent at the [larch repo](https://github.com/zhupanov/larch),
  copy the files in the [Starter kit](#starter-kit), and adapt the ones marked *Adapt*.
- **Let your agent do it.** Paste the recipe in [Let your agent do it](#let-your-agent-do-it)
  and let it scaffold the equivalents for your repo.

The sections move from advisory context (files the agent reads) to mechanical
enforcement (checks it cannot skip), then to environment and navigability. Adopt
them in that order, or pull single items from the [Starter kit](#starter-kit).

## How to use this guide

Each section names a **Larch source** (the file to open in the larch repo) and a
**Disposition** (how to reuse it):

- **Copy.** Take it near-verbatim. Only the filename or a framing line changes.
- **Adapt.** Keep the structure; rewrite the content for your stack.
- **Pattern.** Do not copy larch's version. Reproduce the idea in your own tooling.

## Let your agent do it

Give your agent this prompt. It reads larch's setup and scaffolds the equivalents
for your repo:

```text
Point yourself at the larch repository (https://github.com/zhupanov/larch).
Read: CLAUDE.md, AGENTS.md, KARPATHY_CLAUDE.md, BASH_AUTHORING.md,
ARCHITECTURAL_INVARIANTS.md, ARCHITECTURAL_GUIDELINES.md, hooks/hooks.json,
docs/linting.md, and .pre-commit-config.yaml.

Then scaffold the equivalents for THIS repository (stack: <fill in>):
- Copy KARPATHY_CLAUDE.md nearly verbatim.
- Write a thin CLAUDE.md that @-imports AGENTS.md (plus our behavioral/stack files).
- Draft an AGENTS.md describing THIS repo's layout, conventions, and canonical sources.
- Adapt the linters and hooks to our stack.
- Stub a `checks run-relevant` equivalent that runs our changed-file checks.

Ask me before writing anything you are unsure about.
```

---

## Part I: Instruction files (the context your agent always loads)

The always-on brief. Keep it small; everything here is paid for on every turn.

### 1. `CLAUDE.md` + `AGENTS.md`: a thin root that imports a shared file

*Larch source: `CLAUDE.md`, `GEMINI.md`, `AGENTS.md`. Disposition: **Copy** the pattern; write your own `AGENTS.md`.*

Keep one shared instruction file, `AGENTS.md`. Make every tool's native entry file
a thin shim that imports it.

`AGENTS.md` is the emerging cross-vendor convention. Claude Code, Codex, Cursor,
and others look for it. Write your instructions there once. You then avoid
duplicating them across `CLAUDE.md`, `GEMINI.md`, and per-tool configs, and you
get a single source of truth that cannot drift.

In larch, each shim is one line per import. `CLAUDE.md`:

```text
@AGENTS.md
@KARPATHY_CLAUDE.md
@BASH_AUTHORING.md
```

`GEMINI.md`:

```text
@./AGENTS.md
```

Claude Code inlines each `@path` into the always-loaded context. So `CLAUDE.md`
pulls in the shared brief plus the behavioral guardrails (§2) and stack-authoring
notes (§3). `GEMINI.md` pulls in just the brief. Keep each shim to imports only.
Anything you write directly in `CLAUDE.md` can later contradict `AGENTS.md`.

**What to put in `AGENTS.md`.** Treat it as the briefing for a capable contributor
who does not know the repo yet:

- **What the repo is.** One or two lines: what it produces, who consumes it.
- **Repository layout.** The runtime surface versus everything supplementary, so
  edits land in the right place.
- **Editing rules.** What is off-limits, and what to re-run after a change (lints,
  tests).
- **Conventions.** Language and tooling defaults, style, commit-message norms.
- **How to answer questions about this repo.** An escalation policy (see §11).
- **Canonical sources.** A "for X, read Y" map (see §11).
- **Honesty and reporting rules.** Do not fabricate. Do not overstate completion.
  Surface failures.

Copy the shims as-is and swap the filenames for yours. Write `AGENTS.md` from
scratch. Most of this guide is about what goes in it.

### 2. Behavioral guardrails (the `KARPATHY_CLAUDE.md` pattern)

*Larch source: `KARPATHY_CLAUDE.md`. Disposition: **Copy** near-verbatim.*

Some agent mistakes are model mistakes, not repo mistakes. The model assumes
instead of asking. It over-engineers a simple task. It refactors code that was not
in scope. It declares done without verifying. A short behavioral file heads these
off. It targets the model's habits, not your codebase, so it ports to any repo
almost unchanged.

larch's `KARPATHY_CLAUDE.md` has four rules:

- **Think before coding.** State assumptions, surface tradeoffs, and ask when
  unclear instead of guessing.
- **Simplicity first.** Write the minimum code that solves the problem. Nothing
  speculative.
- **Surgical changes.** Touch only what the task requires. Do not "improve"
  adjacent code.
- **Goal-driven execution.** Turn the task into a verifiable check, then loop until
  it passes.

Copy the file and wire it into your `CLAUDE.md` import list (§1). Keep only the one
framing line that hands off to your repo-specific instructions. The filename is
arbitrary. Larch's is an homage; `BEHAVIORAL_GUIDELINES.md` reads just as well.
Just match the import in `CLAUDE.md`.

### 3. Stack-specific authoring notes (the `BASH_AUTHORING.md` pattern)

*Larch source: `BASH_AUTHORING.md`. Disposition: **Pattern**; write your own.*

§2 covers mistakes the model makes anywhere. This file covers mistakes specific to
your stack: your language, shell, framework, build tool, or test runner. An agent
repeats a subtle environment gotcha until something tells it not to. This file is
that something. It turns a lost debugging session into a paragraph the agent reads
before it writes.

larch's `BASH_AUTHORING.md` is all Bash: quoting hygiene, macOS Bash 3.2
portability, and exit-code traps in shell probes. None of it helps a Python or
TypeScript repo. That is the point. Copy the habit, not the content.

Build the file from real incidents. Each time you or your agent burns time on a
stack-specific trap, add a short entry: the trap, the symptom, the fix. A Python
repo might note import-time side effects or fixture teardown order. A TypeScript
repo might note `strictNullChecks` corners or ESM-versus-CommonJS resolution.

Wire the file into your `CLAUDE.md` import list (§1). Keep it lean, because it
loads on every turn (§5). If it grows past a screen, move narrow file-specific guidance into controlled docs, hooks, lints, or tests (§6) so the root brief stays small.

### 4. Architectural knowledge (`ARCHITECTURAL_INVARIANTS.md`, `ARCHITECTURAL_GUIDELINES.md`)

*Larch source: `ARCHITECTURAL_INVARIANTS.md`, `ARCHITECTURAL_GUIDELINES.md`. Disposition: **Adapt**.*

Use two files when you need both hard constraints and judgment-tier preferences.
`ARCHITECTURAL_INVARIANTS.md` is the hard-constraint sibling: each `I-*` entry is
an invariant for the current change and should later gain a mechanical backstop
when possible. `ARCHITECTURAL_GUIDELINES.md` holds nuanced `G-*` design
preferences that are too subtle to mechanize yet.

A blank invariant file is valid. It tells larch the tier exists, even before you
have entries to add. Do not treat prose as the final enforcement layer: add lint,
hook, or test coverage later for every invariant that can be checked
deterministically.

Guidelines are softer. They are aspirational principles with judgment and
exceptions. Reviewers, human or agent, surface meaningful deviations. Guidelines
do not hard-block a change the way invariants do.

larch codes each rule and gives it two parts:

- **Why it exists.** One line of rationale, so the agent can weigh it in context.
- **When to deviate.** The explicit exceptions.

The "when to deviate" line matters most. It stops an aspirational rule from
becoming dogma the agent applies blindly. larch's `G-Rs-2`, for example, says to
fail loudly and never swallow errors, then names the one narrow case where a quiet
degraded path is fine.

Keep this the softest layer. It cannot override your `AGENTS.md` or your skills.
When a guideline stops needing judgment, graduate it to a lint, hook, or test (§6, §8).
larch states this as its own rule, `G-Enf-1`: prefer mechanical enforcement, and
let entries here earn their place only until a linter can take over. That
migration, from prose preference to mechanical check, is the through-line of this
whole guide.

Adapt the structure, not the rules. The coded formats port to any repo. The
specific rules are larch's, about Rust, shell, and skill authoring. Write your
own for your architecture.

### 5. Keep the always-loaded context lean

*Larch source: agent-lint D004. Disposition: **Pattern**.*

Everything imported by `CLAUDE.md` loads on every turn. That context is a standing
tax: it costs tokens on each request, and it dilutes the agent's attention across
instructions that may not apply. Treat the root brief as scarce space.

Keep only always-relevant instructions in the imported files. Push everything else
to surfaces that load on demand:

- **Skills** load when invoked.
- **Guidelines, hooks, lints, and tests** carry conditional guidance and enforcement.
- **Docs** load when the agent chooses to read them.

The test is simple. If an instruction only matters when editing certain files, it belongs in a narrower controlled source, not the root brief.

larch enforces this with a lint that caps the size of its root imports, so the
brief cannot creep upward over time. You do not need the lint to start. The habit
is what matters, and a size cap makes it stick.

---

## Part II: From judgment to machinery (enforcement)

The core move: when a judgment call recurs, promote it to a guideline, hook, lint, or
test. Then the agent cannot get it wrong, or it gets reminded exactly when it
matters.

### 6. Controlled sources for conditional guidance

*Larch source: `ARCHITECTURAL_GUIDELINES.md`, hook sibling docs, `docs/linting.md`, and regression harnesses. Disposition: **Adapt**.*

Keep the always-on brief lean by routing conditional guidance to controlled sources:

- Put judgment-tier design advice in `ARCHITECTURAL_GUIDELINES.md`.
- Put never-skip constraints in hooks or tests.
- Put mechanical gotchas in linters and the linter docs.
- Put hook behavior beside the hook script and keep the harness in sync.

This keeps advice available at deliberate review points and lets mechanical checks block mistakes. Advisory reminders alone are not enforcement; when you need the agent stopped, use a hook, lint, or test.

### 7. Hooks (guardrails the agent cannot skip)

*Larch source: `hooks/hooks.json`, `scripts/block-submodule-edit.sh`, `scripts/sessionstart-health.sh`. Disposition: **Adapt**.*

A hook runs your own command at a lifecycle point. Unlike advisory prose, a `PreToolUse` hook can deny a tool call outright. This is the layer the
model cannot talk its way past.

Claude Code fires hooks at several points. larch uses four:

- **`PreToolUse`** runs before a tool call and can block it.
- **`PostToolUse`** runs after a tool call.
- **`SessionStart`** runs at session start or resume and can inject context.
- **`Stop`** runs when the agent tries to end a turn and can fail closed.

Two of larch's hooks copy well into any repo:

- **Block edits to off-limits files.** `block-submodule-edit.sh` denies `Edit` and
  `Write` on files inside a git submodule. Point the same idea at vendored,
  generated, or build-output directories.
- **Warn on missing tools at session start.** `sessionstart-health.sh` checks that
  required binaries are on `PATH` and warns early, before the first failed command.

You wire hooks in `hooks.json` (for a plugin) or in `.claude/settings.json` (per
project). Adapt larch's wiring; the specific scripts are larch's own.

### 8. Linters (lint what changed; encode your recurring gotchas)

*Larch source: `.pre-commit-config.yaml`, `docs/linting.md`, `scripts/lint-*.sh`. Disposition: **Adapt**.*

Linters are the mechanical backstop. A linter catches a whole class of mistake
every time, so you never review it by hand again. Two habits from larch carry over
to any stack.

**One source of truth, changed files locally, full sweep in CI.** larch defines its
linters once (in `pre-commit`), runs them on changed files locally for fast
feedback, and runs the full set in CI as the enforced backstop. The agent sees
problems in seconds; CI guarantees nothing slips through.

**Write a lint for the mistake that recurs.** When the same review comment shows up
twice, larch turns it into a lint. This is `G-Enf-1` (§4) in practice: a judgment
call becomes a mechanical check that never tires. larch has lints that ban unsafe
shell probes and non-portable shell constructs, both former sources of repeated
bugs.

Adapt, do not copy. Use whatever runner fits your stack: `pre-commit`, a Makefile,
or package scripts. The habits port; the specific lints are larch's. These linters
also feed the single self-validation command in §9.

### 9. The relevant-checks contract (one command to self-validate)

*Larch source: `scripts/larch.sh checks run-relevant`, `docs/skills.md` ("Relevant checks script"), README ("Non-skill entrypoints"). Disposition: **Implement** your own.*

This is the most important item in the guide, and the one most agents miss. It
answers a single question: how does the agent know its change is good?

larch does not hardcode how to validate your repo. It calls one command that you
provide. That command runs the right checks for the changed files and returns a
fail-closed verdict. larch reads the verdict. On failure it loops, fixes, and calls
again. Any agent can use the same loop.

**The contract.** larch ships a reference implementation for its own repo, and a
consuming repo provides its own at the same entrypoint. You can see the behavior
larch expects by running the reference:

```text
$ scripts/larch.sh checks run-relevant --site my-change --tmpdir <session dir>
RELEVANT_CHECKS_OK=true SITE=my-change COVERAGE=full PHASE=unknown
```

- **It scopes to the change.** It finds branch, staged, unstaged, and untracked
  files, then runs the checks that apply to them. It does not lint the whole repo
  on every call.
- **It runs your real checks.** In larch that means changed-file `pre-commit`, a
  pin-validation check, and agent-config linting when available. In your repo it
  means your linters (§8) and fast tests.
- **It fails closed.** On success it prints the green `RELEVANT_CHECKS_OK=true`
  envelope. On any structural error it prints `STATUS=fail FAILURE_REASON=...` and
  exits non-zero. It never passes silently.
- **`--site`** is a telemetry label. **`--tmpdir`** is a session scratch directory
  for verbose logs, validated to block path injection.

**What to build.** An executable that computes the changed files, runs your linters
and fast tests on them, prints a clear pass-or-fail envelope, and exits non-zero on
any failure. Whether you match larch's `checks run-relevant` invocation or wrap your
own runner, keep the two properties that make it useful: changed-file scope, and
fail-closed output. That is the single command your agent runs before it calls a
change done.

---

## Part III: Environment and navigability

Reduce friction, and help the agent find its way around.

### 10. Permissions and settings (`.claude/settings.json`)

*Larch source: `.claude/settings.json`, `docs/configuration-and-permissions.md`. Disposition: **Adapt**.*

`settings.json` sets how much friction the agent hits. The main lever is the
permission allowlist. Without one, the agent pauses to ask before every common
command. With one, it runs the safe commands and keeps moving.

larch allowlists the commands it uses constantly: `git`, `gh`, `jq`, `grep`, and
the like, plus its own scripts and skills. It also sets `env` (model, effort level)
and `additionalDirectories` (extra writable roots, such as `/tmp`).

You do not have to write the list by hand. The `/fewer-permission-prompts` helper
scans your transcripts and proposes an allowlist from the calls you already
approve. Start there, then trim.

Allowlist only what you trust. Add the safe, frequent commands. Leave destructive
ones to prompt every time.

### 11. A canonical-sources map and a "how to answer questions" policy

*Larch source: `AGENTS.md` ("Canonical sources", "Answering questions about this repo"). Disposition: **Adapt**.*

Two habits help the agent orient fast instead of wandering.

**A canonical-sources map.** A curated "for X, read Y" list in `AGENTS.md`. Instead
of letting the agent grep around and guess, you tell it where the truth lives.
larch lists, per topic, the file that owns it: setup, linting, the workflow
lifecycle, security. The agent reads the right file first.

**A how-to-answer policy.** An escalation ladder for questions about the repo.
Direct file reads first. Then one or two targeted greps. Escalate to a subagent
only when the answer spans many files. larch spells this out so the agent does not
default to a broad, noisy search.

Together they turn "explore the repo" into "read these three files." That is faster
and cheaper on every task. Write your own map and policy; the shape ports directly.

### 12. Anti-drift prose conventions

*Larch source: `ARCHITECTURAL_GUIDELINES.md` (`G-Md-1` / `G-Md-2`) and docs linters. Disposition: **Adapt**.*

Prose in docs and instruction files goes stale in silence. A hardcoded count, a
line-number reference, or a machine-local path rots the moment the code moves. The
agent then reads a confident, wrong instruction and acts on it.

larch's guideline and lint-backed conventions ban the drift-prone shapes:

- **No hardcoded counts in prose.** Point at a single source of truth instead.
- **No line-number references** like `file.py:74`. Refer to a symbol by name; the
  name survives edits.
- **No machine-local absolute paths.** Use repo-relative paths.
- **On a rename, grep the docs and instructions** for the old name before you
  finish.

This matters most for the very files this guide is about. Your instruction set
guides every future change, so it must not rot. Copy larch's guideline shape and back it with docs linting where practical.

### 13. Multi-tool support

*Larch source: `CLAUDE.md`, `GEMINI.md`, `AGENTS.md`, `docs/external-reviewers.md`. Disposition: **Pattern**.*

This is the payoff of the §1 split. Because your instructions live in `AGENTS.md`,
adding another tool is one shim, not a rewrite. larch supports Claude Code and
Gemini through `CLAUDE.md` and `GEMINI.md`, both pointing at `AGENTS.md`. Any tool
that reads its own entry file gets the same brief for one line of wiring.

The same shared brief lets you bring in more than one agent per task. larch runs
Codex and Cursor as extra reviewers and coders alongside Claude, all starting from
the same `AGENTS.md`. That orchestration is larch-specific, but the enabling idea
is not: one source of instructions, many tools reading it.

You already did the work in §1. This section is the reason it pays off.

### 14. Keep security documentation in sync

*Larch source: `SECURITY.md`, `docs/security/README.md`, AGENTS.md editing rule. Disposition: **Pattern**.*

Keep a `SECURITY.md`, and give detailed security contracts clear canonical
owners. Make updating the appropriate security document a required step when
security-relevant behavior changes. larch's `AGENTS.md` points contributors to
the ownership map in `docs/security/README.md`. That turns a good intention into
a step the agent follows without making the root policy own every detail.

Agents need this stated. An agent will change auth, secret handling, or input
validation to satisfy a task and never notice the security docs fell behind. A
written editing rule catches the gap. Add the sync requirement to your
`AGENTS.md` editing rules or enforce it with a hook, lint, or test (§6).

---

## Starter kit

The whole guide in one table. Open each file in the larch repo, then reuse it per
its disposition:

| From larch | What it gives you | Disposition |
|---|---|---|
| `KARPATHY_CLAUDE.md` | Model-mistake guardrails | **Copy** near-verbatim |
| `CLAUDE.md` / `GEMINI.md` | Thin shims importing `AGENTS.md` | **Copy** the pattern |
| `AGENTS.md` | Repo layout, conventions, canonical sources | **Adapt** |
| `BASH_AUTHORING.md` | Stack-specific authoring pitfalls | **Pattern** (write your own) |
| `ARCHITECTURAL_INVARIANTS.md` | Hard architectural constraints, valid even when blank | **Adapt** |
| `ARCHITECTURAL_GUIDELINES.md` | Aspirational, coded design rules | **Adapt** |
| `hooks/hooks.json` + hook scripts | Deterministic guardrails | **Adapt** |
| `.pre-commit-config.yaml` | Changed-file linting | **Adapt** to your stack |
| `checks run-relevant` entrypoint | One command the agent runs to validate a change | **Implement** your own |
| `.claude/settings.json` | Permission allowlists, env, model | **Adapt** |

## Suggested adoption order

You do not need all of it at once. Adopt in three passes.

1. **Minimum viable.** `CLAUDE.md` and `AGENTS.md` (§1), `KARPATHY_CLAUDE.md`
   (§2), and a `checks run-relevant` entrypoint (§9). This alone makes an agent
   markedly more effective.
2. **Enforcement.** Changed-file linting (§8), a hook or two (§7), and
   controlled source docs or structural tests (§6).
3. **Polish.** Architectural guidelines (§4), the canonical-sources map (§11), a
   permissions allowlist (§10), and multi-tool shims (§13).
