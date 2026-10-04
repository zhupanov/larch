# Review Acceptance Rubric (Necessity Gate)

A proposed change is **in-scope-acceptable only if the feature would be incomplete, broken, unverifiable, or regressed without it.** If a finding is real, or even valuable, but the feature
ships correctly without it, it is **not in-scope** — route it to Out-of-Scope (file a tracked
issue). To be accepted in-scope, a finding must clear at least one of these gates:

1. Completeness — the change is a required part of the specified feature; without it,
   functionality the issue explicitly asked for is missing or only partially delivered.
2. Correctness — as written, the plan or implementation would fail to deliver the feature as
   specified: a real defect on the feature's own execution path (wrong behavior, inverted logic,
   a missing case the spec requires).
3. Introduced regression or harm — the change itself introduces a security vulnerability, a
   data-loss or data-corruption path, or a breaking change to an existing caller, contract, or
   CLI/wire surface — even if the new feature works. A change also introduces harm when it adds
   an independent implementation of behavior already owned in-repo, and reuse or shared
   extraction fits the approved scope. (You do not ship a regression and file an issue about it.)
4. Necessary test — **Default a test finding to Out-of-Scope.** It clears this gate as in-scope
   only if it covers a new, currently-uncovered, risk-bearing execution path that THIS feature
   introduces, AND the test is proportionate to the behavior's risk and size. A test that could
   merely exist, restates existing coverage, broadens an unrelated harness, or is
   red-green-TDD-after-the-fact is a Nit → Out-of-Scope, never in-scope.
   **Plan-mandated deliverable carve-out:** a test, doc, generated file, cleanup task, or
   other artifact explicitly required by the supplied implementation plan is in-scope when omitted
   from the diff. This is not a license to require optional tests or docs the plan did not mandate.
   Before applying this carve-out, internally map the finding to the exact plan requirement, or
   conclude that no matching requirement exists. Do not include that mapping in voter output.
5. Unblock a pre-existing condition — a pre-existing defect that actively blocks completing,
   building, or verifying the feature (overlaps 1-2; the test is "the feature cannot be finished
   or shipped until this is fixed"). A red or flapping default-branch CI counts as actively
   blocking verification for every run: restoring or stabilizing default-branch CI clears this
   gate. The `/implement` orchestrator, not reviewers, owns executing that repair.

Default-deny. If you are unsure whether a finding clears a gate, it does not. Unsure => Out-of-Scope
or reject => never an in-scope accept.

Out-of-Scope signals, NOT acceptance signals (real, possibly worth a future issue — never an
in-scope change here): "cleaner," "more robust," "more idiomatic," "more consistent," "more
flexible / future-proof," "best practice," "while we're here," "defensive in case," refactors,
renames-for-clarity, added configurability, broadened error handling for inputs the feature
cannot produce, performance / micro-optimization claims (e.g. "redundant I/O", "avoid an extra
call/scan", "micro-cache this") when the feature already meets its stated performance
requirement, and cross-shell / cross-OS / tool-version portability speculation for shells,
platforms, or tool versions the project does not target.

Consolidating pre-existing duplication remains Out-of-Scope. Removing a second implementation
introduced by the current plan or diff is not a general refactor and may be accepted in-scope.
Exclude repeated syntax, generated output, fixtures whose duplication is the assertion, and
documented intentional forks.

Out-of-Scope is the safe harbor, not the trash. A real finding that fails the necessity gate
belongs on the Out-of-Scope list, where it can still be accepted as a tracked GitHub issue.
Deferring a good idea is the correct outcome, not a loss.

Severity interaction. A Nit can never clear the necessity gate (a Nit is by definition optional).
Neutral rescue is not inline acceptance. A neutral finding with one YES vote is routed to OOS artifacts only when the YES severity is `major`. Single-YES `minor`, `nit`, missing, or invalid severities stay dropped. Rescued neutrals keep vote-table `Result=neutral`, but classification records `scope=oos`.
After the first review round, a finding that no prior round raised is suspect: if it were
necessary, the plan or code would not have passed the earlier round — hold it to gate 2 or 3
(Correctness or Introduced-regression) only.

Anchor. Judge necessity against the spec, not against the finding text. For /design plan review,
the spec is the originating issue scope (the staged scope anchor / feature description). For
/implement and /review code review, the spec is the implementation plan (plan fidelity).

---

## Update triggers

The following surfaces embed this rubric's necessity-gate language. When the rubric changes, update all of them and run `make test-prompt-template-invariants`:

- `scripts/larch.sh render voter` — embeds rubric body verbatim for external voters
- `skills/shared/reviewer-templates.md` — Necessity gate subsection for reviewer self-filter
- `agents/code-reviewer.md` — generated from reviewer-templates.md (re-run `${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh generate code-reviewer-agent`)
- `agents/reviewer-plan-fidelity.md` — generated (re-run `${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh generate reviewer-plan-fidelity-agent`)
- `agents/reviewer-code-robustness.md` — generated (re-run `${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh generate reviewer-code-robustness-agent`)
- `agents/reviewer-security-structure-tests.md` — generated (re-run `${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh generate reviewer-security-structure-tests-agent`)
- `agents/reviewer-edge-cases.md` — hand-maintained specialist; edit directly, then run `${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh generate pre-rendered-reviewer-prompts`
- `agents/reviewer-correctness.md` — hand-maintained specialist; edit directly, then run `${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh generate pre-rendered-reviewer-prompts`
- `agents/reviewer-security.md` — hand-maintained specialist; edit directly, then run `${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh generate pre-rendered-reviewer-prompts`
- `agents/reviewer-structure.md` — hand-maintained specialist; edit directly, then run `${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh generate pre-rendered-reviewer-prompts`
- `agents/reviewer-testing.md` — hand-maintained specialist; edit directly, then run `${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh generate pre-rendered-reviewer-prompts`
- `agents/pre-rendered/reviewer-*-body.txt` — generated from every specialist above (re-run `${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh generate pre-rendered-reviewer-prompts`)
- `scripts/larch.sh render plan-review` — plan-review external prompts
- `scripts/larch.sh render specialist` — code-review external prompts (competition notice)
- `skills/design/references/plan-review-runtime.md` — structural plan-review contracts and artifact interpretation; runtime prompt bodies come from `scripts/larch.sh render plan-review` and `scripts/larch.sh render voter`
- `skills/shared/voting-protocol.md` — voter prompt template YES definition
- `skills/shared/oos-acceptance-rubric.md` — OOS legitimacy standard (separate concern: accepts genuine, concrete, non-duplicate OOS ballot items for filing)
