# /design terminal failure reporting

**MANDATORY: READ ENTIRE FILE before composing user-facing failure prose: `${CLAUDE_PLUGIN_ROOT}/skills/shared/readability-style.md`.**

**Consumer**: terminal `/design` failure and cancellation paths that stage a `failed-*` outcome or launch the Final summary block.

**Contract**: terminal `/design` failure-path contracts: Step 5c abort handling, `scripts/larch.sh design failure-report` report-gate authority, `design stage-terminal-state` mechanical staging, and `failed-*` sentinel precedence.

**When to load**: immediately before any `failed-*` `SUMMARY_OUTCOME` export or terminal-state staging, and immediately before the Final summary block on failure paths. Load it for clarify, sprawl, outline, plan-size, Split-path judge-panel, Step 3 final-summary, and Step 5c abort paths. Do not load it at green Step 5 entry.

## Step 5c abort handling

When `_publish_rc=2` or an unexpected non-zero value outside `{0,1,3,4}` appears, abort after best-effort `scripts/larch.sh design stage-terminal-state` staging as `failed-publish-tail`. This includes `_publish_rc=5`. Parse `FINAL_SUMMARY_PATH=<path>` from final `bgjob wait` `DONE` stdout or `$DESIGN_TMPDIR/bgjob/design-step5c.result.env`, follow the `/design` Read-always readiness profile to Read/cache the final summary and allowed sidecars before tmpdir loss, complete failure routing, then emit the cached body as terminal plain chat. Stop before Step 5c items 5-7, Step 5d, or Step 6.

## /design auto error reporting

`scripts/larch.sh design failure-report` owns the teardown report gate. It can file a terminal-failure report for `failed-plan-write`, `failed-publish`, `failed-postplan`, `failed-clarify`, `failed-judge-panel`, and `failed-publish-tail`, or an escalation-success report only when the final outcome is `approved` or `approved-partition`.

Sentinel precedence is terminal report, escalation-success report, then operator-action skip. Terminal failures win over escalation evidence on failed outcomes. Stale terminal state is ignored on successful outcomes. Operator-action and all `cancelled-*` outcomes do not file, but they must write `design-failure-operator-action.env`, `design-failure-operator-action-chat.md`, and a run-log audit.

`scripts/larch.sh design stage-terminal-state` is the mechanical writer for prompt-owned hard halts. It writes `design-failure-terminal-state.env` after validating tokens through `"$PLUGIN_ROOT/scripts/larch.sh" stall-recovery validate-token --profile generic --artifact-prefix design-failure --implement-tmpdir "$DESIGN_TMPDIR"` and validating the completed state through `"$PLUGIN_ROOT/scripts/larch.sh" stall-recovery validate-terminal-state ...`. Generic helper calls from /design always pin `--implement-tmpdir "$DESIGN_TMPDIR"` and pass state overrides for terminal classify and compose.

Step 3 panel degradation statuses `panel-failed`, `tally-error`, and `degraded-empty-collector` are non-terminal Gate B bypass degradation when at least one reviewer round launched. `panel-init-failed` means zero reviewers launched; it is a terminal hard stop before Gate C and Step 5. Step 2b.5 decompose-panel retry exhaustion is terminal `failed-judge-panel` and is owned by Split-path, not `design-step3-review.sh`.
