# /review self-review fallback

**Consumer**: `/review` Step 3 zero-survivor fallback and `/review --diff --subagent` heavy-worker zero-survivor fallback.
**Contract**: Authoritative main-agent self-review procedure when all launched external reviewers fail at runtime with `THRESHOLD_REASON=no successful launched reviewer output`.
**When to load**: at zero-survivor `panel-failed` handoff before self-review execution.
**Binding convention**: single normative source for main-agent review fallback. The orchestrator applies accepted findings to the fix pipeline contract below.

## Procedure

1. Print `**⚠ /review Step 3: all external reviewers failed at runtime; main agent is self-reviewing before continuing.**`.
2. Read gathered context directly:
   - Diff mode: branch diff and description artifacts under `$REVIEW_TMPDIR`.
   - Description mode: `DESCRIPTION_TEXT` plus gathered scope files.
3. Review the gathered context yourself. Write raw findings to `$REVIEW_TMPDIR/findings.md` using the `### FINDING_N:` format.
4. Write OOS observations to `$REVIEW_TMPDIR/oos.md` or `$REVIEW_TMPDIR/oos-accepted-review.md` only when they fit existing `/review` OOS rules.
5. **Accepted-findings handoff**: copy in-scope self-review findings into `$REVIEW_TMPDIR/accepted-findings.md` under the normal `### FINDING_N:` contract. Auto-accept in-scope items. Write rejected items to `$REVIEW_TMPDIR/rejected-findings.md` when needed. Bind `ACCEPTED_FINDINGS_FILE` to `$REVIEW_TMPDIR/accepted-findings.md`.
6. **Summary/tally refresh**: after adjudication, replace the stale panel-failed zero-finding summary. Ensure `REVIEW_MODE` is already bound to the active mode (`diff` or `description`; the heavy-worker path is always `diff`). Write `$REVIEW_TMPDIR/self-review-tally.env` with counts derived from self-review artifacts: `ACCEPTED_COUNT`, `REJECTED_COUNT`, `EXONERATED_COUNT`, and `NEUTRAL_COUNT`. Then invoke:

```bash
"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" review emit-tally \
  --tally-file "$REVIEW_TMPDIR/self-review-tally.env" \
  --accepted-findings-file "$ACCEPTED_FINDINGS_FILE" \
  --oos-file "$REVIEW_TMPDIR/oos.md" \
  --review-tmpdir "$REVIEW_TMPDIR" \
  --session-env-path "${SESSION_ENV_PATH:-}" \
  --round "${ROUND_NUM:-1}" \
  --mode "$REVIEW_MODE" \
  --scout-status na \
  --dynamic-slots 0 \
  --static-slot-count 0
```

This overwrites `review-round-summary.md` and `review-summary.json` from the self-review counts. Run it before Step 4, before `review log-phase`, and before heavy-worker return so parent and nested `/implement` consumers never read the pre-self-review panel-failed summary.

7. If no in-scope issues remain after adjudication, still run step 6 so Step 4 and `review log-phase` see zero counts from self-review, not the stale panel-failed emit.
8. If in-scope accepted findings exist:
   - Diff mode: set `REVIEW_CORE_STATUS=fix-required` and continue through the existing `/review-and-fix` and relevant-checks flow unchanged, passing `--findings-file "$ACCEPTED_FINDINGS_FILE"`.
   - Description mode: proceed to final summary artifacts without `/review-and-fix`.

Do not add audit-run analytics in this fallback.
