# Mermaid Safe Content

## Why This Matters

Anchor comments and PR bodies embed Mermaid diagrams that GitHub renders publicly. Unsafe diagram text can make the diagram block render as raw source or fail entirely, hiding the implementation context the anchor is meant to preserve. Issue #1404 is the pinned regression case.

## Forbidden Patterns

- Literal `|` inside flowchart node text delimited by `[...]`, `(...)`, `{...}`, or `((...))`. Use quoted node text such as `["foo|bar"]`, including escaped quotes when needed, or rephrase the label.
- `<br/>`, `<br />`, or `<br>` inside `sequenceDiagram` participant or actor aliases. Use a plain alias and put multiline detail in a `Note over` line.
- `$` inside `sequenceDiagram` participant or actor aliases. Use a plain alias and mention variables in notes or message text.

## Permitted Patterns

- Flowchart edge labels such as `A -->|text| B`.
- `<br/>` inside flowchart node labels.
- Quoted flowchart node text such as `A["foo \"x\" |bar"]`.

## Enforcement Layers

- Write-time sanitizer: `/design` Step 3b, `/implement` Step 7a, and `/implement` Step 9a validate diagram candidates with `scripts/larch.sh mermaid sanitize`. Rejected diagrams are dropped; callers either omit the publish section so prior issue content is preserved or fall back to an explicit placeholder when that surface requires one.
- Tracking-issue redaction: `/design` Step 5c.5 and `/implement` Step 7a publish through `scripts/larch.sh diagrams upsert`. The Rust owner redacts secrets and temporary paths, authorizes the typed GitHub mutation, and verifies the exact comment after mutation.

## For Tool Authors

Any new Mermaid emitter must write a candidate file, run `scripts/larch.sh mermaid sanitize --from-md` when the candidate includes fence delimiters, and only promote the candidate to the public artifact on `STATUS=ok`. On `STATUS=rejected` or exit 2, drop the candidate, log a public-safe `REASON_TOKEN`, and proceed with a placeholder.

The authoritative tracking-issue surface for diagrams is the issue-scoped
`larch:diagrams` comment managed by `scripts/larch.sh diagrams upsert`.
`/design` owns the Architecture section at Step 5c.5; `/implement` owns the
Code Flow section at Step 7a. The PR body still embeds the Code Flow diagram
only, after Step 9a validation.

## Update Triggers

Update this file when sanitizer policy, diagram-emitter steps, or anchor/PR publication behavior changes.
