# Umbrella Partition Handoff

**Consumer**: `/implement` after Step 0 when the target issue must be replaced by two or more implementation issues.

**Contract**: `/umbrella` exclusively owns leaf filing, dependency wiring, and in-place conversion of the managed original.

**When to load**: load only when `/implement` chooses a 2+ issue replacement. The one scope-disposition follow-up and accepted OOS disposition issues are not replacements and stay on their existing paths.

## Prepare and approve

Require `CONTEXT_FILE`, `$ISSUE_NUMBER`, and a managed `[IMPLEMENTING]` source. Create `$IMPLEMENT_TMPDIR/umbrella-partition`, then write the exact generic `/issue` batch to `partition-input.txt` and its 1-based `<blocker>\t<blocked>` dependency TSV to `partition-deps.tsv`. Ask one operator question approving those exact files. After approval, remove any stale `umbrella-complete.sentinel`.

Invoke `/umbrella` via the Skill tool. Try bare `umbrella` first, retry as `larch:umbrella` only on `Unknown skill`, and continue this procedure when the child returns. Pass `--lifecycle-parent-context "$CONTEXT_FILE"` first, then `--skip-approve`, `--prepared-root "$IMPLEMENT_TMPDIR/umbrella-partition"`, `--prepared-input-file "$IMPLEMENT_TMPDIR/umbrella-partition/partition-input.txt"`, `--prepared-deps-file "$IMPLEMENT_TMPDIR/umbrella-partition/partition-deps.tsv"`, `--completion-sentinel "$IMPLEMENT_TMPDIR/umbrella-partition/umbrella-complete.sentinel"`, and `$ISSUE_NUMBER`. The child consumes the exact approved files without another question.

`/umbrella` is the only filing and original-issue mutation owner. Keep `/issue` direct filing, original closure, deduplication, dependency wiring, and conversion logic out of `/implement`.

## Verify completion

After the child returns, run:

```bash
"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" umbrella verify-completion \
  --sentinel-file "$IMPLEMENT_TMPDIR/umbrella-partition/umbrella-complete.sentinel" \
  --sentinel-root "$IMPLEMENT_TMPDIR/umbrella-partition" \
  --prepared-input "$IMPLEMENT_TMPDIR/umbrella-partition/partition-input.txt" \
  --prepared-deps "$IMPLEMENT_TMPDIR/umbrella-partition/partition-deps.tsv" \
  --repo "$REPO" \
  --issue "$ISSUE_NUMBER"
```

Require exit zero, `UMBRELLA_COMPLETION_VERIFIED=true`, and exact `UMBRELLA_NUMBER=$ISSUE_NUMBER`. Missing context, invalid or stale artifacts, child failure, or failed verification leaves the original open and preserves session state. A verified split converts the original in place and ends the current implementation attempt before code, PR, or tracking-issue completion.
