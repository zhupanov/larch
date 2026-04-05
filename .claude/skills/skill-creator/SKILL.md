---
name: skill-creator
description: Create new skills, modify and improve existing skills, and measure skill performance. Use when users want to create a skill from scratch, edit or optimize an existing skill, run evals to test a skill, benchmark skill performance, or optimize a skill's description for better triggering. Trigger this skill when the user says things like "build a skill", "create a skill for X", "turn this into a skill", "make a slash command", "capture this workflow as a skill", "improve my skill", "optimize skill triggering", or "test my skill".
---

# Skill Creator

Create, test, and iteratively improve Claude skills.

## Quick start: Where is the user in the process?

Determine which phase applies and jump in:

1. **Starting from scratch** ("I want a skill for X"): Go to [Creating a skill](#creating-a-skill).
2. **Has a draft** ("Here's my skill, help me test it"): Go to [Running evals](#running-and-evaluating-test-cases).
3. **Has eval results** ("I reviewed the outputs, here's my feedback"): Go to [Improving the skill](#improving-the-skill).
4. **Skill works, wants better triggering** ("It doesn't trigger when I expect"): Go to [Description optimization](#description-optimization).
5. **Just wants to iterate casually** ("Don't need formal evals, just vibe with me"): Skip the eval framework and iterate directly with the user.
6. **Finished** ("Package it up"): Run `python -m scripts.package_skill <path/to/skill-folder>` and direct the user to the resulting `.skill` file. If `present_files` tool is available, use it to surface the artifact; otherwise, tell the user where the file is so they can share or install it manually.

**Environment check**: If running in Claude.ai or Cowork (not Claude Code), read `references/platforms.md` first — it contains important workflow overrides (e.g., no subagents on Claude.ai, headless viewer in Cowork).

Add steps to your TodoList if available. Include "Run `eval-viewer/generate_review.py` so human can review test cases" to ensure it happens.

## Communicating with the user

Users range from non-technical to expert. Pay attention to context cues:
- Terms like "evaluation" and "benchmark" are fine for most users.
- For "JSON" and "assertion", look for cues that the user knows what these mean before using them without explanation.
- When in doubt, briefly explain terms.

---

## Creating a skill

### Capture intent

Start by understanding what the user wants. The current conversation might already contain a workflow to capture (e.g., "turn this into a skill"). If so, extract answers from the conversation history first — the tools used, the sequence of steps, corrections the user made, input/output formats observed. The user may need to fill gaps, and should confirm before proceeding.

1. What should this skill enable Claude to do?
2. When should this skill trigger? (what user phrases/contexts)
3. What is the expected output format?
4. Should we set up test cases? Skills with objectively verifiable outputs (file transforms, data extraction, code generation) benefit from test cases. Skills with subjective outputs (writing style, art) often do not. Suggest the appropriate default, but let the user decide.

### Interview and research

Proactively ask about edge cases, input/output formats, example files, success criteria, and dependencies. Wait to write test prompts until this is ironed out.

Check available MCPs — if useful for research (searching docs, finding similar skills), research in parallel via subagents if available, otherwise inline.

### Write the SKILL.md

Based on the interview, fill in these components:

- **name**: Skill identifier (kebab-case, must match folder name)
- **description**: What it does AND when to use it. This is the primary triggering mechanism. Include specific trigger phrases. Claude tends to undertrigger skills, so make descriptions a bit assertive — include contexts where the skill should activate even if the user does not explicitly ask for it.
- **compatibility**: Required tools, dependencies (optional, rarely needed)
- **Body**: The full instructions

### Skill writing guide

#### Anatomy of a skill

```
skill-name/
├── SKILL.md (required)
│   ├── YAML frontmatter (name, description required)
│   └── Markdown instructions
└── Bundled Resources (optional)
    ├── scripts/    - Executable code for deterministic/repetitive tasks
    ├── references/ - Docs loaded into context as needed
    └── assets/     - Files used in output (templates, icons, fonts)
```

#### Progressive disclosure

Skills use a three-level loading system:
1. **Metadata** (name + description) — Always in context (~100 words)
2. **SKILL.md body** — In context whenever skill triggers (<500 lines ideal)
3. **Bundled resources** — As needed (unlimited, scripts can execute without loading)

Key patterns:
- Keep SKILL.md under 500 lines. If approaching that limit, add hierarchy with clear pointers to reference files.
- Reference files clearly from SKILL.md with guidance on when to read them.
- For large reference files (>300 lines), include a table of contents.

**Domain organization**: When a skill supports multiple domains/frameworks, organize by variant:
```
cloud-deploy/
├── SKILL.md (workflow + selection)
└── references/
    ├── aws.md
    ├── gcp.md
    └── azure.md
```
Claude reads only the relevant reference file.

#### Safety

Skills must not contain malware, exploit code, or content that could compromise system security. A skill's contents should not surprise the user in their intent if described.

#### Writing patterns

Prefer imperative form in instructions.

**Defining output formats:**
```markdown
## Report structure
Use this template so reports are consistently scannable:
# [Title]
## Executive summary
## Key findings
## Recommendations
```

**Examples pattern:**
```markdown
## Commit message format
**Example 1:**
Input: Added user authentication with JWT tokens
Output: feat(auth): implement JWT-based authentication
```

#### Writing style

Explain to the model why things are important rather than relying on heavy-handed directives. LLMs have good theory of mind and can go beyond rote instructions when given reasoning. If you find yourself writing ALWAYS or NEVER in all caps, reframe and explain the reasoning instead.

Make skills general, not overly narrow to specific examples. Write a draft, then look at it with fresh eyes and improve it.

### Test cases

After writing the skill draft, come up with 2-3 realistic test prompts — the kind of thing a real user would actually say. Share them with the user and ask if they want to add more.

Save test cases to `evals/evals.json`. Write prompts first; add assertions later while runs are in progress.

```json
{
  "skill_name": "example-skill",
  "evals": [
    {
      "id": 1,
      "prompt": "User's task prompt",
      "expected_output": "Description of expected result",
      "files": []
    }
  ]
}
```

See `references/schemas.md` for the full schema (including the `expectations` field).

---

## Running and evaluating test cases

The eval workflow has 5 steps: spawn runs, draft assertions, capture timing, grade + launch viewer, read feedback. Critical invariants to follow:

- **Workspace layout**: `<skill-name>-workspace/iteration-<N>/eval-<ID>/{config}/run-1/outputs/` (where `{config}` is `with_skill`, `without_skill`, `old_skill`, etc.)
- **grading.json field names**: The expectations array must use `text`, `passed`, and `evidence` (not `name`/`met`/`details`) — the viewer depends on these exact names.
- **Always use `generate_review.py`** to create the viewer. Do not write custom HTML. Generate the viewer BEFORE evaluating outputs yourself — get results in front of the human as soon as possible.

For the detailed step-by-step workflow (spawning runs, drafting assertions, grading, aggregating benchmarks, and launching the viewer), read `references/eval-workflow.md`.

---

## Improving the skill

This is the heart of the iteration loop. After running test cases and collecting user feedback, improve the skill based on what you learned.

### How to think about improvements

1. **Generalize from the feedback.** The goal is skills that work across many different prompts, not just the test examples. Rather than adding narrow, overfitting changes, try different approaches — branching out with different metaphors or patterns is relatively cheap and may produce better results.

2. **Keep the prompt lean.** Remove instructions that are not pulling their weight. Read the transcripts, not just the final outputs — if the skill is causing the model to waste time on unproductive steps, remove those instructions.

3. **Explain the why.** Transmit understanding of the task and the user's intent into the instructions. When the reasoning behind a directive is clear, the model can generalize better than it can from rigid rules.

4. **Look for repeated work across test cases.** If all test runs independently wrote similar helper scripts or took the same multi-step approach to something, that is a strong signal the skill should bundle that script. Write it once, put it in `scripts/`, and tell the skill to use it.

### The iteration loop

After improving the skill:

1. Apply your improvements
2. Rerun all test cases into a new `iteration-<N+1>/` directory, including baseline runs (for new skills, the baseline is always `without_skill`; for improving existing skills, decide whether to baseline against the original version or the previous iteration)
3. Launch the reviewer with `--previous-workspace` pointing at the previous iteration
4. Wait for the user to review and tell you they are done
5. Read the new feedback, improve again, repeat

Keep going until the user is happy, the feedback is all empty, or you are not making meaningful progress.

### Advanced: Blind comparison

For rigorous A/B comparison between two skill versions, read `agents/comparator.md` and `agents/analyzer.md`. This is optional, requires subagents, and most users will not need it.

---

## Description optimization

After creating or improving a skill, offer to optimize the description for better triggering accuracy. The description field is the primary mechanism that determines whether Claude invokes a skill.

For the full optimization workflow (generating trigger eval queries, reviewing with the user, running the optimization loop, and applying results), read `references/description-optimization.md`.

---

## Common issues

**grading.json field name mismatch**: The viewer expects `text`, `passed`, and `evidence` in the expectations array. Using `name`/`met`/`details` or other variants causes the viewer to render incorrectly. Always verify field names match.

**timing.json data loss**: Subagent completion notifications include `total_tokens` and `duration_ms`. This data is only available at notification time and is not persisted elsewhere. Capture it immediately to `timing.json` — if you miss it, the data is gone.

**Eval queries too simple to trigger skills**: Claude only consults skills for tasks it cannot easily handle on its own. Simple queries like "read this PDF" will not trigger skills regardless of description quality. Make eval queries substantive enough that Claude would benefit from consulting a skill.

**No runs found by eval viewer**: `generate_review.py` looks for directories containing an `outputs/` subdirectory. If the workspace layout does not match the expected structure, the script exits with an error. Verify that each `run-1/` (or `run-N/`) directory inside config directories contains an `outputs/` subdirectory.

---

## Reference files

The `agents/` directory contains instructions for specialized subagents. Read them when you need to spawn the relevant subagent:

- `agents/grader.md` — How to evaluate assertions against outputs
- `agents/comparator.md` — How to do blind A/B comparison between two outputs
- `agents/analyzer.md` — How to analyze why one version beat another

The `references/` directory has additional documentation:

- `references/schemas.md` — JSON structures for evals.json, grading.json, benchmark.json, etc.
- `references/eval-workflow.md` — Detailed step-by-step eval running, grading, viewer launching, and feedback reading
- `references/description-optimization.md` — Trigger eval queries, optimization loop, and triggering mechanics
- `references/platforms.md` — Claude.ai and Cowork environment adaptations
