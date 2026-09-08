# post-tracking-issue.sh

Delegates to the Rust-owned `tracking post-issue` command, which publishes the
Step 0 `larch:metadata` tracking-issue summary.

Usage:

```bash
post-tracking-issue.sh --implement-tmpdir PATH [--issue-number N] [--run-id ID] [--adopted true|false] [--force-requested true|false]
```

All session state is read from files under `IMPLEMENT_TMPDIR` rather than
CLI arguments to reduce non-determinism and context bloat:

- `parent-issue.md` → `ISSUE_NUMBER`, `RUN_ID` (when `--issue-number` absent)
- `session-env.sh` → `REPO`, `AGENT`, `CODER`
- `session-id` → `RUN_ID` fallback when `--run-id` and `parent-issue.md` are absent
- `session-env.sh` → `LARCH_TOKEN_SESSION_ID` fallback when `--run-id`, `parent-issue.md`, and `session-id` are absent

`RUN_ID` precedence is: `--run-id` > `parent-issue.md` `RUN_ID` > `session-id` > `session-env.sh` `LARCH_TOKEN_SESSION_ID`.
`--run-id`, when present, must match `^[A-Za-z0-9._-]+$`.

When `--issue-number N` is provided, `N` is used directly for `ISSUE_NUMBER`
and `parent-issue.md` is written after a successful metadata post (with
`ISSUE_NUMBER`, `RUN_ID`, and `ADOPTED=$adopted`). This co-locates the
sentinel write with confirmed success, so the file is never present without
a confirmed `larch:metadata` comment.

When `--issue-number` is absent, the script reads `ISSUE_NUMBER` from the
existing `parent-issue.md` (Branch 1 resume path — sentinel already present).

`--adopted` defaults to `true` and is only meaningful when `--issue-number`
is provided.

`--force-requested` defaults to `false`. When `true`, the composed
`larch:metadata` body includes `Force: true`; when false, the line is
omitted.

Output:

- `POSTED=true|false`
- `COMMENT_URL=<url-or-empty>`
- `ERROR=<message>` on failure

The Rust command writes `summary-metadata.md` under `IMPLEMENT_TMPDIR` with a
private atomic write and calls the in-process `tracking-issue upsert-summary`
owner with the `<!-- larch:metadata v1 runid=<R> -->` marker. Its stdout is
`POSTED=true|false`, `COMMENT_URL=<url-or-empty>`, and an optional
`ERROR=<message>`.
