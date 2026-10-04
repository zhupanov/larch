---

# larch-run-lifecycle: shared-v1 skill=upgrade-larch
name: upgrade-larch
description: "Use when upgrading larch to the latest stable plugin and matching verified executable."
allowed-tools: Bash
---

**MANDATORY: Follow the complete shared lifecycle contract in `${CLAUDE_PLUGIN_ROOT}/skills/shared/run-lifecycle.md` with declared skill `upgrade-larch`.**

**MANDATORY: READ ENTIRE FILE before composing user-facing prose: `${CLAUDE_PLUGIN_ROOT}/skills/shared/readability-style.md`.**

Upgrade the larch plugin to the latest stable version. This skill is for the runtime-only remote marketplace install documented in `docs/installation-and-setup.md`. Contributors using a local checkout (`claude --plugin-dir .`) should `git pull` instead.

## Flags

- `--run-id <ID>`: Details live in `${CLAUDE_PLUGIN_ROOT}/skills/shared/run-id-flag.md`.

## Steps

1. Run the upgrade script:

```bash
"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" upgrade-larch run
```

2. Verify the script exited successfully with no recovery banner. If it printed `Binary verification passed. No upgrade needed.`, report that the current plugin and executable match, so no restart is required. If it printed `LARCH_MARKETPLACE_RECONCILED=true`, report the runtime-only marketplace migration. Otherwise, confirm the `Installed larch plugin version (user scope):` line matches the preflighted version. The `claude plugin list` block under it may also show project-scope rows that other clones pinned to older versions; that is expected and not a mismatch. Tell the user to restart Claude Code after an install or marketplace migration.

If the driver stopped because the marketplace-pinned `stable` branch is not at the release's tagged commit, report that no plugin state changed and that a release is likely still in flight. Tell the user to retry once it finishes.

If the driver printed `Rolled the active larch plugin root back to`, report that new sessions stay on that prior version and tell the user to retry. If it printed a repair command instead, relay that command verbatim and tell the user to run it from a terminal outside Claude Code before starting new sessions.

See the Rust `upgrade-larch` command for the driver contract and failure recovery. `/release` Step 7 runs both `upgrade-larch release-step7-root` and `upgrade-larch run` from the release working tree.

Edit-in-sync: marketplace-source changes also touch `.claude-plugin/marketplace.json`, the Rust `upgrade-larch` command, `.claude/skills/release/SKILL.md`, `docs/installation-and-setup.md`, `docs/skills.md`, and `docs/security/supply-chain-credentials-and-services.md`.
