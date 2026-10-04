# Post-merge emergency repair

**Consumer**: `/implement` Step 8+ when `NEXT_ACTION=postmerge-repair`.

**Contract**: Repair a merged-SHA push-to-main failure on a dedicated repair branch. This is not generic `ci-fix`.

**When to load**: MANDATORY only on `NEXT_ACTION=postmerge-repair` from `ship route-exit`. Do not load for generic `ci-fix`, pre-PR repair, or any path that does not enter the post-merge emergency repair branch.

Phases are `postmerge-push-watch` → `emergency-repair` → `repair-shipped` or `stalled`.

- Write `post-merge-sentinel` only after the merged-SHA push workflow passes or emergency-repair ownership is durably recorded.
- `postmerge-push-watch` reruns failed jobs once before entering `emergency-repair`; the driver records `TRANSIENT_RETRIES` and re-enters through the normal transient route.
- On emergency-repair relaunch, if no repair branch exists and the merged SHA is green, the Rust driver finalizes the original merged PR without creating a repair PR.
- Keep `PR_NUMBER` as the original feature PR. Store the repair PR separately as `EMERGENCY_REPAIR_PR_NUMBER`.
- Create `EMERGENCY_REPAIR_BRANCH` from fresh `origin/main`. Do not commit on the original feature branch after the original PR merge, and do not commit larch logs on any branch after that merge.
- Capture redacted logs from `MAIN_REPAIR_RUN_ID`, fix the failure on the repair branch, run relevant checks, commit, push, open the repair PR, ship and merge it through the dedicated driver path, then run a commit-scoped push watch for the repair merge SHA.
- Track `ORIGINAL_BRANCH_FORBIDDEN=true`, `MAIN_REPAIR_RUN_ID`, `MAIN_REPAIR_HEAD`, `EMERGENCY_REPAIR_BRANCH`, and `EMERGENCY_REPAIR_PR_NUMBER` in validated ship state.
- Transition to `repair-shipped` only after the repair merge SHA's push workflow passes. Otherwise stall with explicit detail and preserve the repair state for resume.
