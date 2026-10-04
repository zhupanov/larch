# refresh-execution-issues.sh

Thin compatibility wrapper for the Rust-owned `execution-issues refresh`
command. It resolves the plugin root, enters through `scripts/larch.sh`, and
forwards arguments, output, and the exit status unchanged.

Usage:

```bash
refresh-execution-issues.sh --implement-tmpdir PATH
```

The Rust command reads session state from files under `IMPLEMENT_TMPDIR`:

- `parent-issue.md` → `ISSUE_NUMBER`, `RUN_ID`
- `session-env.sh` → `REPO`, `AGENT`, `CODER`

Its machine output is:

- `REFRESHED=true|false`
- `REASON=issue-not-set` when no tracking issue is set (skips cleanly)
- `ERROR=<message>` on failure
