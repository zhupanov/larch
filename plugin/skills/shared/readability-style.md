# Readability Style

**Consumer**: all larch skills' user-facing prose, including chat narration, plans, summaries, gate prose, findings presentation, and issue and PR bodies.

**Contract**: this file is the single source of truth for larch readability. Inline skill orchestration reads it before composing user-facing prose. External prompt files embed `<READABILITY_STYLE>` only where existing prompt assembly already expands that token.

**When to load**: load only when a skill is about to compose user-facing prose or render a prompt that asks another agent to compose such prose. Do not load during setup, pure validation, or byte-stable artifact handling.

---

## Style Axes

Write in three styles at once:

- **Strunk & White**: use active voice. Omit needless words. Prefer concrete nouns and verbs.
- **Dyslexia-friendly**: use short sentences. Prefer simple words. Break dense ideas into headings and bullets.
- **Brevity**: shorter is better. Minimize total artifact length while preserving meaning. When unsure how short to go, go shorter.
- **No em dashes**: never use em dashes; use periods, commas, colons, or semicolons instead.
- **Hedge vs. direct**: hedge uncertain claims ("may contain", "can fail") instead of absolutes; keep directives imperative.

## Precision Contract

Keep these byte-stable unless the task explicitly edits them:

- fenced code blocks
- backticked tokens
- file paths
- identifiers
- flag names
- `KEY=value` stdout grammars
- plan grammar: `### NEW:`, `### UPDATED:`, `### REWRITTEN:`, `### MAY_UPDATE:`, and `diff_lines: <N>`
- vote-table structure
- manifest JSON structure

Do not rewrite those items for style. Prose inside templates still follows this file when the template emits user-facing text, but structural tokens and grammar stay exact.

## Plan-Drafting Reminders

When `/design` Step 2b drafts a plan that adds, removes, or converts Bash fences in `skills/implement/SKILL.md`:

- List `### UPDATED: scripts/test-implement-fence-shape.sh` in **Files to modify/create**.
- Note that `EXPECTED_OLD` / `EXPECTED_NEW` may need updates.

This is a discoverability reminder only. Do not add plan grammar.

## Precedence

When rules conflict, use this order:

`code references > meaning > brevity > dyslexia-friendly chunking > Strunk & White micro-rewrites`

Apply it directly:

- Preserve code references first.
- Preserve exact meaning next.
- Cut words before adding layout.
- Add headings or bullets only when they improve scanning.
- Polish grammar last.

## Substitution Token

External-agent prompt files that already embed this literal token continue to use it:

`<READABILITY_STYLE>`

Before launch, replace every literal `<READABILITY_STYLE>` token with the full contents of this file. This issue adds no new token wiring outside the existing `/design` brainstorm and plan-review prompt surfaces. All other skills use direct readability directives.

## Examples

Before: "It is recommended that the implementer should consider adding a validation step."

After: "Add a validation step."

Before: "The plan should make sure that users are not able to continue when the file cannot be read."

After: "Block the flow when the file cannot be read."

Before: "In order to avoid potential future confusion, documentation can be updated."

After: "Update the docs to avoid confusion."

Before: "The script performs an operation that checks if the path exists."

After: "The script checks that the path exists."

Before: "This could possibly be out of scope depending on how the team wants to proceed."

After: "This may be out of scope."
