# /research Evaluation Set

**Consumer**: `scripts/larch.sh eval research` — the offline harness reads this catalog, parses each entry's fields, runs `/research` once per entry, and scores the output against the entry's expectations.

**Contract**: Frozen registry of representative `/research` evaluation questions. Twenty entries balanced across five categories (`lookup`, `architecture`, `external-comparison`, `risk-assessment`, `feasibility`). Each entry declares an `id` (kebab-case, unique), a verbatim `question` string, a `category` from the enum above, an `expected_provenance_count` integer (minimum file/path/URL citations a passing answer should contain), an `expected_keywords` comma-separated list (case-insensitive substrings a good synthesis should mention), and a `notes` line for human grading guidance. Two entries are flagged adversarial in their notes: one targets a fictitious mechanism, and one targets a data-absence question. The catalog is human-edited; mechanical schema validation is performed by `scripts/larch.sh eval research --smoke-test` and the Rust coverage in `crates/larch-cli/tests/clean_install.rs`. Authors editing this file must follow the global reference rules enforced by `scripts/test-references-headers.sh` (see its sibling contract for details on the Contract-paragraph and header-triplet checks).

**When to load**: When iterating on a `/research` prompt or harness change. The harness is the only programmatic consumer; humans read this file when authoring new evaluation questions or interpreting harness output. Not loaded by `/research` itself or by any other skill at runtime.

**Source**: Anthropic's *How we built our multi-agent research system* (`anthropic.com/engineering/built-multi-agent-research-system`) and *Building Effective Agents* describe small-sample (~20-case) rubric-based LLM-as-judge evaluation as the substrate for prompt-side iteration. This catalog is the local instantiation.

---

## Entries

### eval-1: where-defined-rebase-push
- **question**: Where is `scripts/larch.sh push rebase` defined in this repository, what does its `--skip-if-pushed` flag do, and which callers consume it?
- **category**: lookup
- **expected_provenance_count**: 3
- **expected_keywords**: scripts/larch.sh push rebase, --skip-if-pushed, SKIPPED_ALREADY_PUSHED, crates/larch-cli/src/push_rebase.rs
- **notes**: Lookup; should cite the Rust owner `crates/larch-cli/src/push_rebase.rs` plus at least two consumers (the Rebase Checkpoint Macro in `/implement` and the fork-mode CI comparison caller).

### eval-2: deny-edit-write-hook-contract
- **question**: How does the `/research` skill's best-effort read-only contract partition mechanically enforced versus prompt-enforced perimeters, what hook backs the mechanical tier, and what tools fall under the prompt-enforced tier?
- **category**: lookup
- **expected_provenance_count**: 2
- **expected_keywords**: deny-edit-write.sh, activation sentinel, /tmp, PreToolUse, best-effort, Bash, external reviewers, workflow-trust-and-mutations.md
- **notes**: Lookup; should cite `scripts/deny-edit-write.sh` for the activation-gated mechanical tier (Edit/Write/NotebookEdit confined to canonical `/tmp` and the larch cache sessions root while `research-*` is fresh), name Bash + external Cursor/Codex reviewers as the prompt-enforced tier, and reference the focused workflow-security residual-risk framing.

### eval-3: larch-log-batch-slugs
- **question**: What are the 11 canonical larch-log batch slugs in `/implement`, in assembly order, and which script defines them?
- **category**: lookup
- **expected_provenance_count**: 1
- **expected_keywords**: crates/larch-core/src/run_log/batch.rs, plan-goals-test, run-statistics, token-report
- **notes**: Lookup; should list the log batch slugs verbatim and cite `crates/larch-core/src/run_log/batch.rs`, the registry the Rust `run-log write` and `append` commands read. A correct answer reproduces the table order from the canonical registry.

### eval-4: eval-baseline-q1-2026
- **question**: What was the result of the `/research` evaluation harness's 2026-Q1 baseline run, and what were the per-entry judge scores?
- **category**: lookup
- **expected_provenance_count**: 0
- **expected_keywords**: no data, schema-only stub, baseline.json, eval-research
- **notes**: ADVERSARIAL — data-absence. The harness landed in this PR; no Q1-2026 baseline run exists. `eval-baseline.json` is committed as a schema-only stub with `entries: []`. A correct answer says "no data — the harness was added in PR closing #419 and the baseline file is currently a schema-only stub awaiting a follow-up populate run." A failing answer invents numeric scores or claims a prior run.

### eval-5: plan-review-resolution
- **question**: How does `/design` Step 3 resolve accepted plan-review findings, and how does the review loop preserve progress when an external reviewer is unavailable?
- **category**: architecture
- **expected_provenance_count**: 2
- **expected_keywords**: scripts/larch.sh plan-review run, crates/larch-cli/src/plan_review_commands.rs, revise-plan-with-waterfall, plan-review voter-dispatch, fallback
- **notes**: Architecture; should explain that Step 3 runs the plan-review loop, applies accepted findings with `revise-plan-with-waterfall.sh`, records durable round state, and uses the reviewer fallback/waterfall behavior documented for the plan-review panel.

### eval-6: parent-issue-sentinel-branches
- **question**: What is the relationship between `/implement` Step 0 tracking adoption (Branch 1 vs Branch 2) and the `parent-issue.md` sentinel file, and how does the sentinel preserve idempotency across resumed runs?
- **category**: architecture
- **expected_provenance_count**: 2
- **expected_keywords**: parent-issue.md, ADOPTED, Branch 1, Branch 2, RUN_ID, scripts/larch.sh tracking-issue upsert-summary
- **notes**: Architecture; should cover sentinel-reuse (Branch 1), positional `--issue` adoption (Branch 2), when `post-tracking-issue.sh` writes the sentinel after successful metadata publication, and how `RUN_ID` / manifest init interact on resume versus fresh adopt.

### eval-7: ci-fix-rebase-step12-interaction
- **question**: How does the Rust ship driver's CI-fix rebase and conflict-resolution handoff interact with `/implement` Step 12's CI+merge loop, and what differs between Step 12 hard-bail and Step 10 best-effort behavior?
- **category**: architecture
- **expected_provenance_count**: 2
- **expected_keywords**: conflict-resolution.md, ship_pr_pre_push, run_rebase_rebump, step12, step10, hard-bail, 12d
- **notes**: Architecture; should explain why the Rust ship driver owns CI-fix rebase/force-push sequencing, when remaining conflicts hand off through `CALLER_KIND=ship_pr_pre_push`, and why Step 12 remains the strict last-chance path while Step 10 is best-effort.

### eval-8: plan-review-tenure-weighting
- **question**: How does the `/design` plan-review voting panel weight reviewer judges by tenure, and where in the codebase is the tenure-lookup table stored?
- **category**: architecture
- **expected_provenance_count**: 0
- **expected_keywords**: no tenure-weighting, YES, NO, EXONERATE, 2+ YES threshold
- **notes**: ADVERSARIAL — fictitious mechanism. The voting protocol uses YES/NO/EXONERATE voting with a 2+ YES acceptance threshold; there is no tenure-weighting and no tenure-lookup table. A correct answer rejects the premise and explains the actual mechanism. A failing answer invents tenure weights or fabricates a lookup-table location.

### eval-9: research-vs-anthropic-multi-agent
- **question**: How does `/research`'s 3-lane research-and-validation approach compare to the architecture described in Anthropic's *How we built our multi-agent research system*, and where do the two designs diverge?
- **category**: external-comparison
- **expected_provenance_count**: 3
- **expected_keywords**: anthropic.com, multi-agent, lead-orchestrator, subagent, validation
- **notes**: This is the question that filed umbrella #413. Should cite the Anthropic blog directly, name `/research`'s 3 research lanes plus its 3-reviewer validation panel, and identify divergence points (e.g., orchestrator-as-lead vs co-equal lanes).

### eval-10: implement-review-evaluator-optimizer
- **question**: How does `/implement`'s review loop align with the evaluator-optimizer pattern in Anthropic's *Building Effective Agents*, and what specific mechanics in `/implement` realize that pattern?
- **category**: external-comparison
- **expected_provenance_count**: 2
- **expected_keywords**: anthropic.com, evaluator-optimizer, /review, accepted, rejected, voting panel
- **notes**: External comparison; should cite the Anthropic post and identify `/review`'s role plus the accept/reject voting machinery as the realization of the pattern.

### eval-11: skill-judge-rubric-vs-literature
- **question**: How does `/skill-judge`'s per-dimension D1..D8 grading scheme compare to standard rubric-based LLM-as-judge approaches in the published literature on agent evaluation?
- **category**: external-comparison
- **expected_provenance_count**: 2
- **expected_keywords**: skill-judge, rubric, LLM-as-judge, dimension, threshold
- **notes**: External comparison; should cite at least one external rubric-evaluation reference (Anthropic blog, Eugene Yan, or similar) and contrast the threshold-per-dimension shape against more common percentage-only scoring.

### eval-12: prompt-eval-30-to-80-claim
- **question**: What evidence is there in the published Claude Code or Anthropic literature for the claim that prompt-side evaluations on small (~20 case) sets surface 30 to 80 percent jumps in success rate, and what caveats does the source attach?
- **category**: external-comparison
- **expected_provenance_count**: 1
- **expected_keywords**: anthropic.com, multi-agent, low-hanging fruit, eval, prompt
- **notes**: External comparison; should cite the Anthropic multi-agent post directly and reproduce the caveat that the gain is from low-hanging-fruit identification, not a sustained delta.

### eval-13: implement-concurrency-admission-sentinel
- **question**: What concurrency hazards exist when two `/implement` sessions target the same repository or tracking issue, and how do Preflight admission (`scripts/larch.sh admission gate`), the `parent-issue.md` / `RUN_ID` crash-resume sentinel, and the single-runner assumption interact?
- **category**: risk-assessment
- **expected_provenance_count**: 2
- **expected_keywords**: scripts/larch.sh admission gate, single-runner, sentinel, RESUME, ADOPTED
- **notes**: Risk; should describe dirty-tree / working-tree interleaving risks, how admission re-checks open blockers on resume while skipping some title/label gates, and the Known Limitations note about one runner per clone.

### eval-14: implement-cursor-timeout-plan-review
- **question**: What happens to a `/implement` run if `/design`'s Cursor plan-review lane times out during review, and how does external reviewer availability propagate into the resumed implementation workflow?
- **category**: risk-assessment
- **expected_provenance_count**: 2
- **expected_keywords**: scripts/larch.sh plan-review run, crates/larch-cli/src/plan_review_commands.rs, plan-review voter-dispatch, cursor_available, fallback, session-env
- **notes**: Risk; should explain the Step 3 fallback/degraded-review behavior, how reviewer availability is persisted through session-env/run params, and that `/implement` consumes the final issue plan rather than replaying `/design` reviewer lanes.

### eval-15: ci-fix-rebase-failure-modes
- **question**: What are the failure modes of `/implement`'s CI-fix rebase/conflict-resolution flow, and how does each map to Step 12 hard-bail versus Step 10 graceful-degrade behavior?
- **category**: risk-assessment
- **expected_provenance_count**: 2
- **expected_keywords**: scripts/larch.sh push rebase, conflict-resolution.md, ship_pr_pre_push, force-push, hard-bail, 12d, step10
- **notes**: Risk; should enumerate at least three failure modes (remaining rebase conflict handoff, unresolved conflict-resolution bail, force-push rejection, post-rebase verification failure) and pair each with the correct Step 12 versus Step 10 disposition.

### eval-16: deny-edit-write-bypass-blast-radius
- **question**: What is the security blast-radius if `/research`'s deny-edit-write hook is bypassed, what mechanisms in the repo backstop the hook, and what residual risk is documented?
- **category**: risk-assessment
- **expected_provenance_count**: 2
- **expected_keywords**: deny-edit-write.sh, activation sentinel, allowed-tools, workflow-trust-and-mutations.md, no mechanical fallback
- **notes**: Risk; should identify the hook as the sole active mechanical enforcement, note that `allowed-tools` declares the surface but does not confine writes, explain that stale or tokenless registrations fail open without a fresh activation sentinel, and quote the residual-risk language from the focused workflow-security reference or the SKILL.md contract paragraph.

### eval-17: research-structured-output-feasibility
- **question**: Could `/research` be extended to produce a structured machine-readable output alongside the current human-readable Research Report without breaking existing consumers, and what would the migration shape look like?
- **category**: feasibility
- **expected_provenance_count**: 2
- **expected_keywords**: Research Report, Step 3, JSON, validation-phase.md, backward compat
- **notes**: Feasibility; should reference the current Step 3 template, identify the consumer boundary (today: human + this eval harness), and propose a side-by-side emission shape.

### eval-18: implement-fully-offline
- **question**: Can `/implement` run fully offline after a `/design` plan already exists, and which workflow steps still require GitHub or network access?
- **category**: feasibility
- **expected_provenance_count**: 3
- **expected_keywords**: /implement, offline, GitHub, gh, run logs, local checks, PR
- **notes**: Feasibility; should explain that local plan materialization and verification can run partly offline, but issue fetches, PR creation, run-log publishing, CI, and merge coordination require GitHub or network access, so there is no fully offline happy path.

### eval-19: research-planner-pre-pass-feasibility
- **question**: How does `/research`'s planner pre-pass decompose a question into subquestions, and what dependencies does evaluating its marginal benefit have on this evaluation harness?
- **category**: feasibility
- **expected_provenance_count**: 2
- **expected_keywords**: planner, subquestions, eval-set, dependency, harness
- **notes**: Feasibility; should describe the planner pre-pass concept and identify the eval-harness dependency for measuring the planner's marginal benefit.

### eval-20: pairwise-blinded-eval-extension
- **question**: How feasible is extending the `/research` evaluation harness to support a blinded pairwise comparison mode for the `--baseline` workflow, where the judge sees two anonymized syntheses and ranks them, and what published evidence supports preferring relative over absolute judgments?
- **category**: feasibility
- **expected_provenance_count**: 2
- **expected_keywords**: blinded, pairwise, --baseline, scripts/larch.sh eval research, judge
- **notes**: Feasibility; should describe the harness change shape (a second judge prompt that swaps left/right, paired with a counterbalanced second call), and cite at least one external source on pairwise vs absolute evaluation stability.
