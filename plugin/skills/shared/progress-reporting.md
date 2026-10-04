# Progress Reporting Contract

Shared formatting rules for step progress output across all larch skills. Each **orchestrator skill** (debate, design, implement, review, research) maintains its own **Step Name Registry** (mapping step numbers to short names) in `skills/<name>/scripts/step-name-registry.tsv` (tab-separated, `step` and `name` columns, header row first, UTF-8 LF). These skills load the file via a MANDATORY directive at session start and reference it for breadcrumb name resolution.

## Breadcrumb Format

By default, progress lines use prose payloads:

```
{icon} {step_number}: {breadcrumb_path}[ — {payload}]
```

- **`{icon}`**: One of the icons below, indicating the line type.
- **`{step_number}`**: The full numeric step designation including any parent prefix (e.g., `1.2b.5` when `/design` step `2b.5` is called from `/implement` step `1`).
- **`{breadcrumb_path}`**: Human-readable path from root to current step, segments joined by ` | `. Standalone runs use the leaf step name only. Nested runs prepend the parent text segment from `--step-prefix` before the leaf segment; see `skills/shared/step-prefix-encoding.md` for encoding and parsing.
- **`{payload}`**: Optional description, outcome, or reason — appended after ` — `.

`🔶` **step start lines** include an additional `/{skill_path}` token between the icon and step number — see `## Step Start Formatting` below.

Exception: `/implement` step-boundary skip lines (`⏩`, `⏭️`) use the compact skip format below. `/implement` start lines (`🔶`), warnings (`⚠`), intermediate lines (`⏳`), rebase lines (`🔃`), reviewer status tables (`📊`), and child-skill breadcrumbs keep their normal formats.

## Icon Taxonomy

| Icon | Line type | When to use |
|------|-----------|-------------|
| `🔶` | Step start | Entering a new step |
| `⏩` | Sub-step skip | Optimization or workflow-conditional skip (quick mode, no changes, etc.) |
| `⚠` | Warning | Non-fatal issue within a step |
| `🔃` | Rebase | Rebase-related operation |
| `⏳` | Intermediate | Progress update within a long-running step |
| `⚡` | Quick mode | Special quick-mode announcements |

**Semantic distinction**: `⏩` and `⏭️` are intentionally separate. `⏩` indicates a lightweight skip within the normal flow; `⏭️` indicates a precondition failure that causes an entire major step to be bypassed.

## Step Start Formatting

Step start lines (`🔶`) get special visual treatment to make them easy to spot:

1. **Separator line**: Print a line of 80 `━` characters immediately before every step start line.
2. **Bold text**: Render the entire step start line in bold using `**...**` markdown.
3. **Blockquote**: Wrap the bold line in a markdown blockquote (`>`) for color differentiation.
4. **Skill path**: Insert `/{skill_path}` between the icon and step number. For standalone runs use the local skill name (e.g., `/design`). For nested runs append the child skill to the parent skill path carried by `--step-prefix` (e.g., `/implement:/design`). Full nested encoding lives in `skills/shared/step-prefix-encoding.md`.

```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
> **🔶 /implement 2: implementation**
```

Only `🔶` step start lines get the separator, blockquote, bold, and `/{skill_path}` treatment. Skip (`⏩`/`⏭️`), warning (`⚠`), and other lines do NOT get separators, blockquotes, bold, or the skill-path prefix.

## Elapsed Time

Every line that marks the **end** of a skipped, failed, or timed-out step/work item must include elapsed time. This applies to: `⏩`, `⏭️`, and step-ending `⚠` lines.

**Step-ending `⚠`** means any `⚠` that contains a step-number prefix (e.g., `⚠ 7a: ...`, `⚠ 14: ...`). Unnumbered bail lines (e.g., `⚠ Rebase onto main failed. Bailing to cleanup.`) do not require elapsed time.

### Step progress lines

For prose-format lines, append the elapsed time in parentheses at the end of the line, using short form. The timer starts when the step logically began (its `🔶` start line, or entry into the step if no `🔶` line exists).

```
⏩ 6: checks (2) — skipped, no review changes (1s)
⚠ 7a: diagrams — generation failed, proceeding without diagram (12s)
```

### Compact Skip Format

`/implement` step-boundary skip lines use compact key/value payloads:

```
<icon> <step_number>: <step_short_name> [key=value ...]
```

- The icon, step number, and short name prefix are unchanged.
- The separator ` — ` is omitted.
- Required fields: `status=<token>` and `elapsed=<time>`.
- `status=skip` maps to `⏩`.
- `status=bypass` maps to `⏭️`.
- `status=ready` is reserved for transition lines where a step is ready for the next action but not yet complete.
- Optional semantic fields include `reason=`, `outcome=`, `bump=`, `from=`, `to=`, `pr=`, `issue=`, `sha=`, `round=`, and `action=`.
- Values must not contain spaces, quotes, or `=`. Use only alphanumerics, hyphens, dots, slashes, and digits.
- Use lowercase hyphenated tokens for reasons and outcomes, except version bump class values (`PATCH`, `MINOR`, `MAJOR`) where uppercase is conventional.

Examples:

```
⏩ 8: version bump status=skip reason=bump-type-none elapsed=1s
⏭️ 12: CI+merge loop status=bypass reason=merge-not-set elapsed=0s
```

### Compact status tables (`📊` lines)

**`/design` Step 3 exception**: Step 3 and Step 3 resume fences do not format reviewer tables in the orchestrator. The Rust owner writes the pre-rendered single line to `$DESIGN_TMPDIR/reviewer-status-table.txt`; the orchestrator Read tool emits that file verbatim after the completion gate. If the file is absent or a symlink, print the SKILL.md missing-table warning. Do not apply the manual formatting rules below to Step 3.

For other skills and phases, include elapsed time immediately after each `✅` and `❌`. The timer for each entry starts when that agent/reviewer was launched.

`/debate` reports the fixed persistent panel with slot names and preserves unavailable slots explicitly:

```text
📊 Panel: | Cursor: ✅ 2m31s | Codex: ⊘ | Claude: ✅ 1m4s |
```

Voting-Protocol skills (`/design`, `/review`) use the 3-reviewer composition:

```
📊 Reviewers: | Code: ✅ 2m31s | Codex: ⏳ | Cursor: ✅ 4m12s |
```

`/research` uses a fixed 4-lane research phase plus 3-reviewer validation panel.

For Phase 1 (research), the table is labelled `Agents` and uses slot names reflecting the diversified angle assignments (each Codex slot runs `RESEARCH_PROMPT_ARCH` / `RESEARCH_PROMPT_EDGE` / `RESEARCH_PROMPT_EXT` / `RESEARCH_PROMPT_SEC` respectively):

```
📊 Agents: | Codex-Arch: ✅ 2m31s | Codex-Edge: ⏳ | Codex-Ext: ✅ 3m5s | Codex-Sec: ⏳ |
```

For Phase 2 (validation), the attribution is `Code` / `Codex` / `Cursor`:

```
📊 Reviewers: | Code: ✅ 2m31s | Codex: ⏳ | Cursor: ✅ 4m12s |
```

When an external is unavailable in the validation panel, a single Claude fallback lane appears in its slot (attributed as `Code`). When Codex is unavailable for a research lane, the Claude Agent-tool fallback keeps the slot name (the entry stays labelled `Codex-Arch` / `Codex-Sec` etc. on the status table — the fallback is a plain research subagent filling the same research slot).

`⏳` (in-progress) and `⊘` (skipped/unavailable) do not include timing.

### Time format

Use the shortest representation:
- Under 1 minute: `45s`
- 1–59 minutes: `2m31s`
- 1+ hours: `1h3m` (seconds are always omitted in the hours tier)

Omit zero components: use `2m` not `2m0s`, use `1h` not `1h0m`.

## Nested parent-to-child prefixing

Standalone progress rules remain in this file. Nested parent-to-child `--step-prefix` encoding lives in `skills/shared/step-prefix-encoding.md`.

Standalone runs use the leaf step name only. Nested runs prepend the parent text segment from `--step-prefix` before the leaf segment.

## Section headers and structured output

Do NOT prefix section headers (e.g., `## Implementation Plan`), structured output headers, artifact labels, or compact reviewer status tables with breadcrumb formatting. Breadcrumbs apply only to progress status lines.

Markdown step headers such as `## Step N — Description` and `### Step Na — Description` MUST be written as HTML comments instead:

```
<!-- step:N — Description -->
```

`🔶` start breadcrumbs are the only step markers rendered in chat output.
