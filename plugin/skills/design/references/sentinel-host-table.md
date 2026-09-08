# Completion sentinel host table

**Consumer**: `/design` maintainers auditing pause/resume sentinel host mappings.

**Contract**: moved host-table body for Phase 7 absorbed completion sentinels, boundary-local writes, direct-review restore, Q&A-only terminal prefix, and kept preludes.

**When to load**: maintainer or pause/resume sentinel-audit use only, such as editing sentinel host mappings or debugging pause/resume sentinels. Normal `/design` orchestration does not load this table.

---

## Folded contract and tradeoff

Phase 7 folds absorbed prior-step sentinel writes into adjacent real-work Bash fences.

- Bgjob result envs are the completion source of truth for migrated long-running waits. Terminal sentinels remain compatibility transition markers and are never sufficient by themselves.
- Step 4 completion truth is `$DESIGN_TMPDIR/bgjob/design-step4-tail.result.env`; `.completed/step-4` remains a boundary-local compatibility sentinel.
- Every absorbed prior-step write happens after `source-env` and before the `scripts/larch.sh design pause-save` pause-check in the host fence.
- Boundary-local writes that remain at step success boundaries still follow the step-body-success rule, including `step-1d.5`, `step-4`, `step-5b`, postplan `step-2b`/`step-2b.5`, Gate-B-bypass dual writes, `step-5b.5`, and in-fence `step-5c`.
- `step-6` remains the deliberate exception. It is written after pause-check and before `session cleanup-tmpdir` in the Step 6 cleanup fence.
- Folding removes near-empty Bash turns, but coarsens timing-ledger granularity and widens pause latency. A pause requested during folded pure-LLM discussion is honored only at the next real Bash boundary.
- Folded sentinels are written first at that boundary so resume skips discussion already completed before the boundary. A pause requested mid-discussion can still replay in-flight LLM work that had not reached its host fence.
- Coverage: `crates/larch-cli/src/design_pause_commands.rs` (`make test-design-pause-resume`).

## Sentinel host table

| Sentinel | Host fence(s) | Ordering |
|----------|---------------|----------|
| `step-1c`, `step-1d` | Step 1d.5 prelude when `brainstorm_requested` is true; Step 1d.7 brainstorm-off elision host when run-params `brainstorm_requested` is not exactly true; Step 2b drafter entry (folded Step 2a idempotent repair) | before pause-check |
| `step-1d.5` | Step 1d.5 boundary-local success; Step 1d.7 brainstorm-off elision host when run-params `brainstorm_requested` is not exactly true; Step 2b drafter entry when `brainstorm_requested` false | boundary-local or before pause-check |
| `step-1d.7`, `step-1e` | Step 2b drafter entry; Step 3 writes `step-1e` only when `scripts/larch.sh plan-review step3-state --direct-review-entry` runs with `.step3-reentry` present | before pause-check |
| `step-2a` | Step 2b drafter entry folded sentinel prep. Still the resume sentinel; first repair/write happens in the Step 2b drafter wrapper. | before pause-check |
| `step-3` | Step 3.5 prelude; `scripts/larch.sh plan-review step3-state --gate-b-bypass` on bypass paths; cleared by `scripts/larch.sh plan-review step3-state --auto-continuation-entry` before automatic follow-up rounds | before pause-check / before Step 3b / before auto-continuation Step 3 re-entry |
| `step-3.5` | Step 3b finalize entry | before pause-check |
| `step-4` | Step 4 success boundary | boundary-local |
| `step-4b` | Step 5b prepare prelude | before pause-check |
| `step-5b` | Step 5b success boundary | boundary-local |
| `step-5b.5` | post-approval diagram entry/sanitize fences | boundary-local, between `step-5b` and `step-5c` |
| `step-5c` | `scripts/larch.sh design step5c` fence when `PLAN_WRITE_OK=true` | in-fence gated |
| `step-5d` | Step 6 prelude | before pause-check |
| `step-6` | Step 6 cleanup fence | **after** pause-check |
| Step 1e re-entry clears | Gate B(c)/Gate C(b) re-entry fence | `rm` stale `step-1e`…`step-4b` before pause-check |
| Step 3 direct-review restore | Step 3 entry via `scripts/larch.sh plan-review step3-state --direct-review-entry` | clear stale downstream state, restore `step-2a`/`step-2b`/`step-2b.5`, and consume `.step3-reentry` before pause-check |
| Q&A-only terminal prefix | Step 0b ad-hoc Q&A-only branch | contiguous through `step-1d.5` before Final summary |
| Kept preludes | Step 1d.5 (brainstorm externals); Step 0c folded discussion block; Step 1d.7 (`SKIP_APPROVE_REQUESTED` read fence) | pause-check retained |
