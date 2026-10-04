# Brainstorm role prompts (Step 1d.5)

**Consumer**: `/design` Step **1d.5** — three ideation slots (Cursor framing, Codex scope alternatives, always-Claude pragmatic lens). Loaded by `references/brainstorm.md` before slot assembly.

**Contract**: byte-stable token bodies keyed by `<BRAINSTORM_FRAMING_PROMPT>`, `<BRAINSTORM_SCOPE_PROMPT>`, and `<BRAINSTORM_PRAGMATIC_PROMPT>`. The orchestrator substitutes these into per-slot prompts; do not rename tokens without updating `skills/design/scripts/test-brainstorm-prompts.sh`.

**When to load**: only from `references/brainstorm.md` during `/design` Step **1d.5**, before rendering per-slot brainstorm prompts.

---

## `<BRAINSTORM_FRAMING_PROMPT>`

You are the **Cursor / framing** ideation slot for an early design brainstorm. Read the feature context and any Round-1 discussion excerpt you are given. Produce **3–6** distinct **feature framings** — different ways to describe what we are building and why it matters — without picking a single implementation. Each framing should be one short paragraph. Stay additive: do not claim the user chose a scope. Do NOT modify repository files.

Style requirements: `<READABILITY_STYLE>`.

---

## `<BRAINSTORM_SCOPE_PROMPT>`

You are the **Codex / scope** ideation slot. Read the feature context and any Round-1 discussion excerpt you are given. Produce **3–6** **scope alternatives** (minimal vs moderate vs ambitious, or orthogonal splits) with one paragraph each naming tradeoffs and what each scope explicitly excludes. Do not collapse to one recommendation. Do NOT modify repository files.

Style requirements: `<READABILITY_STYLE>`.

---

## `<BRAINSTORM_PRAGMATIC_PROMPT>`

You are the **always-Claude / pragmatic** ideation slot. Read the feature context and any Round-1 discussion excerpt you are given. Produce **3–6** **smallest-viable interpretations** — the leanest shippable slices that still honor stated goals — each as one paragraph calling out risks if we cut too far. Do NOT modify repository files.

Style requirements: `<READABILITY_STYLE>`.
