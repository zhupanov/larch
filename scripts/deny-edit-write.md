# `deny-edit-write.sh` contract

`scripts/deny-edit-write.sh` is the fail-closed PreToolUse shim for the Rust-owned `hook deny-edit-write <token>` command. It forwards stdin through `scripts/larch.sh` with `LARCH_BOOTSTRAP_NO_INSTALL=1`. A missing launcher, unavailable verified binary, or nonzero Rust command emits the fixed deny envelope and exits zero; hooks never download or install an executable. The launcher's exit 97 (no executable for this plugin version) still denies, but the reason names the `scripts/larch.sh --version` repair, as described in `block-submodule-edit.md`.

## Activation gate

The Rust owner checks `${XDG_CACHE_HOME:-${HOME:-}/.cache}/larch/deny-edit-write-active` before reading stdin. A fresh regular sentinel named `<token>-*` activates one of these recognized tokens: `research`, `audit-umbrella`, `file-bug`, `complete-umbrella`, `debate`, `triage`, or `umbrella`. The TTL is 360 minutes. The suffix is diagnostic only; activation does not correlate parent PIDs.

A missing or unreadable activation directory, stale sentinel, missing token, or unrecognized token is inactive. An inactive Rust command exits zero with empty stdout. The shim fallback denies because delegation failure cannot prove that the token is inactive.

## Active scratch enforcement

An active call is allowed only when `tool_input.file_path` or `tool_input.notebook_path` resolves to an absolute path under canonical `/tmp` or the existing larch cache `sessions/` root. Missing or malformed input, a relative path, traversal outside an allowed root, a symlink cycle, or a resolution failure denies.

Allow emits no stdout. Deny emits one byte-stable `hookSpecificOutput` JSON envelope with a fixed reason and exits zero. The Rust owner does not depend on `jq`. Migrated regression cases in `crates/larch-cli/src/hook_commands.rs` cover activation, token isolation, freshness, NotebookEdit fallback, canonical containment, traversal, symlinks, and exact serialization.
