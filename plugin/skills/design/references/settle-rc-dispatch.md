# Settle-wrapper dispatch

**Consumer**: `/design` Gate B post-apply, Step 1e Gate A re-entry optional-trailer guard, Round 2 post-plan discussion revision, and Gate C plan revision after `scripts/larch.sh design step35-settle` returns.

**Contract**: prompt-side branch bodies for `scripts/larch.sh design step35-settle` machine actions. The Rust wrapper chooses the action through `"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" design settle-next-action`; this file does not derive actions from rc values.

**When to load**: immediately before any orchestrator branches on `scripts/larch.sh design step35-settle` output at Gate B post-apply, Gate A re-entry trailer guard, or discussion-round2 plan revision after settle returns.

---

## Dispatch key

Primary key: branch on the whole-line `SETTLE_NEXT_ACTION=...` row from `scripts/larch.sh design step35-settle` stdout.

If the `SETTLE_NEXT_ACTION` action row is absent, stop for operator repair. Do not route from the wrapper rc when the action row is missing.

If `SETTLE_NEXT_ACTION` and wrapper rc disagree, stop for repair rather than silently choosing one.

Wrapper exit codes remain diagnostics and legacy process contracts only. The orchestrator must not use them as fallback routing authority. `SETTLE_EXIT_RC` is compatibility output from the Rust action envelope.

Diagnostics:

- `POSTPLAN_RC=0` maps to exit `0`.
- `POSTPLAN_RC=10|12|13` maps to exit `10|12|13`.
- `POSTPLAN_RC=11` or pause signals map to exit `11`.
- Dedup revision needed maps to exit `1`; other dedup failures retain their exit code and emit `settle-repair`. There is no `POSTPLAN_RC=1` on the postplan path.
- Unexpected `POSTPLAN_RC` values map to exit `3`.

## Branch on SETTLE_NEXT_ACTION

| Action | Dispatch |
|---|---|
| `gate-b-continue` | Continue to loop-mode or legacy continuation handling. |
| `gate-a-return` | Return to Gate A. |
| `dedup-revise` | Revise duplicate/trailer cleanup, rewrite `plan.txt`, and retry settle. |
| `settle-repair` | Stop. Repair dedup or restore the pre-rewrite trailer snapshot, then retry. Never snapshot an already-rewritten plan. |
| `gate-b-validator-fail` | Read allowlisted validator keys from `$DESIGN_TMPDIR/.design-postplan-emit-result.env`, then execute **### Plan command validator failure (shared)** with site `design Step 3.5 / Gate B`. Fix-and-retry re-enters settle with `--round-num` when bound. |
| `gate-a-validator-fail` | Execute **### Plan command validator failure (shared)** with site `design discussion-round2`. Fix-and-retry re-enters settle. |
| `pause` | Stop at the delegated pause boundary. |
| `gate-b-hard-size` | Run the unified Split-path directly. Do not issue a local hard-size prompt. Override uses `scripts/larch.sh plan set-oversize-override --design-tmpdir "$DESIGN_TMPDIR"`, deletes `$DESIGN_TMPDIR/composed-plan.md`, then runs `scripts/larch.sh design step2b-postplan --write-completion-only` before continuing. |
| `gate-a-hard-size` | **MANDATORY: READ ENTIRE FILE**: Read `${CLAUDE_PLUGIN_ROOT}/skills/design/references/step2b5-rc-handling.md` immediately before dispatch. Use the retained Step 2b.5 direct Split-path behavior. |
| `gate-b-split` | Run Split-path only. Non-exiting Split returns use `scripts/larch.sh design step2b-postplan --write-completion-only` before continuing. |
| `gate-a-split` | Run Split-path per `decompose-panel.md`. |
| `gate-c-return` | Re-enter `resume@4b` only. Re-run Gate C present-note, spawn a fresh `larch:arch-assessor`, and re-judge the revised plan. This is the sole action that re-assesses. |
| `gate-c-validator-fail` | Execute **### Plan command validator failure (shared)** with site `design Gate C`. Fix-and-retry re-enters settle; do not re-assess until a subsequent clean `gate-c-return`. |
| `gate-c-hard-size` | Run the unified Split-path directly. Do not issue a local hard-size prompt. Do not re-assess until a subsequent clean `gate-c-return`. |
| `gate-c-split` | Run Split-path only. Do not re-assess until a subsequent clean `gate-c-return`. |

Gate C uses these actions. It cannot restore `plan-pre-apply-round-N.txt`; each tier snapshots trailers before editing. Never route from wrapper rc.

**Compatibility:** `gate-a` and `discussion-round2` both map to postplan site `discussion-round2`.
