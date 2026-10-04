# Gate B Explicit Approval Reference

**Consumer**: `/design` Step 3.5 Gate B explicit mode.

**Contract**: owns the `--per-round-approval` Apply all / Go through each / Switch to discussion mode prompt and one-by-one iteration prompt.

**When to load**: only after the zero-findings short-circuit passes, after Presentation, when `approve_requested=true`, accepted findings are non-empty, and the run is not on a post-apply-only resume path that skips re-prompting. Do not load at Step 3.5 entry, on default auto-apply, on zero-findings short-circuit, or on post-apply-only settle resume.

## Prompt

**Explicit mode only (`approve_requested=true`).** Under default auto-apply (`approve_requested=false`) this entire prompt is skipped. Gate B runs `### Apply-all body` directly after the `ℹ 3.5: Gate B — auto-applying N accepted finding(s)` breadcrumb. When `--per-round-approval` is set, fire `AskUserQuestion` with exactly three options:

- **Apply all**: Execute `### Apply-all body` verbatim. The dedup-sweep and shared post-apply pipeline run there.
- **Go through each**: Iterate only the Rust-emitted `FINDING_IDS` list. For each id, fire `AskUserQuestion` with three options: apply / skip / switch to discussion mode. If any per-finding prompt picks switch to discussion mode, stop the iteration immediately, discard any unapplied per-finding intent, and exit to Gate A. Otherwise, after the iteration completes, run the single post-iteration apply/update path documented below.
- **Switch to discussion mode**: Skip plan revision entirely. Exit to Gate A. `plan.txt` remains as it was before Step 3.

Run `scripts/larch.sh plan-review gate-b-counts --design-tmpdir "$DESIGN_TMPDIR"` before asking. Bind all counts from stdout KVs. Do not inspect or classify finding blocks in the orchestrator.

Question text depends on `GATE_B_SEVERITY_MODE`:

- **`structured`**: `"Plan review returned N findings (H high / M medium / L low). How would you like to handle them?"`
- **`fallback`**: `"Plan review returned N findings (C critical / H high / M medium / L low). How would you like to handle them?"`

Header: `"Plan findings"`. Substitute the bound counts before asking.

## One-by-one iteration prompt

For **Go through each**, use Rust-emitted fields only:

1. Run `scripts/larch.sh plan-review gate-b-counts --design-tmpdir "$DESIGN_TMPDIR"`. Parse `FINDING_IDS` and `ACCEPTED_COUNT`.
2. Split `FINDING_IDS` on `,`. Skip empty tokens. Iterate the numeric ids in that order only. Never iterate `1..ACCEPTED_COUNT`.
3. For each id, run `scripts/larch.sh plan-review gate-b-finding-line --design-tmpdir "$DESIGN_TMPDIR" --finding-id <id>`.
4. Parse `ONE_BY_ONE_PROMPT_LINE` and `ONE_BY_ONE_HEADER` from stdout KVs. You may also parse `ONE_BY_ONE_ORDINAL` and `ONE_BY_ONE_TOTAL` for diagnostics.
5. Fire `AskUserQuestion` with question text exactly `ONE_BY_ONE_PROMPT_LINE` and header exactly `ONE_BY_ONE_HEADER`. The header is `Finding <ordinal>/<total>`, where ordinal is the list position, not the raw finding id.

The orchestrator must not manually classify findings, invent severity labels, or re-read `### FINDING_N:` blocks for severity, reviewer, or concern text. It may only pass through Rust-emitted display fields and the Rust-emitted id list.

Options:

- **Apply**: record in the applied set.
- **Skip**: record in the skipped set; the finding moves from accepted to rejected.
- **Switch to discussion mode**: abort iteration; exit to Gate A; do NOT revise `plan.txt`.

After iteration completes (all findings answered without an early abort), the orchestrator revises `plan.txt` per the applied set only, writes the per-finding outcomes back to `$DESIGN_TMPDIR/accepted-plan-findings.md` (apply set retained) and `$DESIGN_TMPDIR/rejected-findings.md` (skip set appended with `Reason not implemented: rejected by user during one-by-one review`), then Execute `### Shared post-apply pipeline` verbatim.
