# Contributing to Larch

## Install

If you are hacking on larch itself and want Claude Code to load the plugin directly from your working checkout (so `${CLAUDE_PLUGIN_ROOT}` resolves to the repo you are editing), launch Claude Code with `--plugin-dir`:

```bash
git clone https://github.com/zhupanov/larch.git
cd larch
gh repo set-default zhupanov/larch
claude --plugin-dir .
```

Alternatively, add the working checkout as a local marketplace:

```bash
cd larch
claude plugin marketplace add .
claude plugin install larch@larch-local
```

This reads the descriptor from your checkout, but the descriptor's plugin source
is a remote `git-subdir` entry pinned to the `stable` branch, so the install
still fetches `plugin/` from GitHub at the latest tagged projection commit, not
from your working tree. Use `--plugin-dir` when you need Claude Code to load the
code you are editing.

## Plugin cache vs. working-tree version

When larch is installed via the plugin system, Claude Code caches the installed version under `~/.claude/plugins/cache/larch-local/larch/<version>/`. Skills and scripts run from this **cached copy**, not from your live working tree. This means:

- A bug fix committed to your working tree does not take effect until you reinstall or refresh the plugin from that checkout. `/larch:upgrade-larch` updates the latest stable GitHub install; a local checkout install (`claude --plugin-dir .` or `claude plugin marketplace add .`) needs a local reinstall/refresh instead. Until then, every `/implement` run uses the older cached version, including ship-driver fixes for Step 8+.
- Multiple concurrent clones (e.g., `larch1/`, `larch2/`) share the same plugin cache. Upgrading from one clone upgrades for all.

**Automatic detection**: when the installed version is behind your working-tree version, larch emits a warning at session setup time:

```
**⚠ larch: installed plugin version (X.Y.Z) is behind the working tree (A.B.C).
Reinstall or refresh the plugin from this checkout before the next run to pick up the latest fixes.
Continuing with the cached version.**
```

This warning fires once per `session-setup.sh` invocation from a larch dev clone when preflight is enabled. Typical entrypoints include `/implement`; `/review` skips preflight and does not emit the warning in its default flow. After reinstalling or refreshing the plugin cache, restart Claude Code to pick up the new version.

## Mermaid diagram rendering

`/design` and `/implement` produce Mermaid architecture and code-flow
diagrams as fenced Markdown blocks. These render automatically in the
Claude Code chat panel and on GitHub. No extra tooling is needed to
view them.
