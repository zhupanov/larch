# `hook-deny-run-in-background.sh` contract

`scripts/hook-deny-run-in-background.sh` is the fail-closed PreToolUse shim for the Rust-owned `hook deny-run-in-background` command. It forwards stdin through `scripts/larch.sh` with `LARCH_BOOTSTRAP_NO_INSTALL=1`. A missing launcher, unavailable verified binary, or nonzero Rust command emits a static deny envelope and exits zero; hooks never download or install an executable. The launcher's exit 97 (no executable for this plugin version) still denies, but the reason names the `scripts/larch.sh --version` repair, as described in `block-submodule-edit.md`.

The Rust owner denies a Bash `run_in_background` launch when a shared-codec registry row has a canonical `CLONE_PATH` that overlaps the event's canonical cwd. Missing registry state allows. Malformed hook JSON, an unresolvable cwd for a definite background Bash launch, or an unreadable regular registry row denies.

The only active-registry carve-out is a combinator-free `${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh bgjob wait` command using that literal launcher prefix. Whitespace and documented multiline continuations are accepted; shell combinators and same-named decoy commands are not. Migrated regression cases live in `crates/larch-cli/src/hook_commands.rs` and `crates/larch-adapters/src/bgjob_registry.rs`.
