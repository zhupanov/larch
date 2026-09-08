# Installation and Setup

> **New to larch?** Set up your repository first. [Preparing Your Repository](preparing-your-repo.md) covers the instruction files, guardrails, and relevant-checks contract that larch, and coding agents generally, rely on.

## Pre-requisites

### Platform

Larch's Rust executable ships only for Apple Silicon macOS
(`aarch64-apple-darwin`, macOS 11.0 or newer). The bootstrap fails closed on
Intel macOS, Linux, and Windows.

### Authentication

Set up GitHub and Google credentials before starting larch.

#### GitHub

Install the [GitHub CLI](https://cli.github.com/) and authenticate it:

```bash
gh auth login
```

Larch invokes the fixed `gh auth token --hostname github.com` credential lookup
through its typed process boundary. The authenticated Rust adapter uses the
returned credential for GitHub API requests and never falls back to `gh api`.
Larch does not read `LARCH_GH_TOKEN`, `GH_TOKEN`, or `GITHUB_TOKEN` itself. The
active session must hold every permission required by the operation. `/release`
policy checks need repository Administration read permission and need
Administration write only when they must enable a disabled setting.

Verify the setup without printing the token:

```bash
gh auth status
gh auth token --hostname github.com >/dev/null
```

The second command succeeds silently when the active `gh` session can provide
a token. Neither command prints the credential.

See the [GitHub credential and transport security contract](security/supply-chain-credentials-and-services.md#github-credential-and-transport-boundary)
for the runtime guarantees and residual limits.

#### Google Application Default Credentials

Google-backed larch features require [Application Default Credentials (ADC)](https://cloud.google.com/docs/authentication/provide-credentials-adc).
Running `gcloud auth login` does not create ADC. For local development, create
them with:

```bash
gcloud auth application-default login
```

By default, ADC are stored at
`~/.config/gcloud/application_default_credentials.json` on macOS and Linux,
and at `%APPDATA%\gcloud\application_default_credentials.json` on Windows.
Set [`GOOGLE_APPLICATION_CREDENTIALS`](https://cloud.google.com/docs/authentication/provide-credentials-adc#local-dev)
to select another credential file. An attached service account or workload
identity can also provide ADC without a local file.

On macOS or Linux, verify a local ADC file is readable and verify ADC without
printing an access token:

```bash
test -r "$HOME/.config/gcloud/application_default_credentials.json"
gcloud auth application-default print-access-token >/dev/null
```

Both commands succeed silently when the expected local file is readable and
ADC can obtain an access token. The second command is an optional operator
setup check. Larch does not run `gcloud` during service calls. GitHub-backed
Rust service calls require an authenticated `gh` session: their only runtime
`gh` invocation is the fixed `gh auth token --hostname github.com` lookup, and
the authenticated Rust adapter makes API requests directly. `gh api` is never a
service fallback. The clean-install `gh` in `scripts/larch.sh` only downloads
and verifies the release binary and is separate from runtime service access.

The Rust credential boundary follows the standard ADC order: the file named by
`GOOGLE_APPLICATION_CREDENTIALS`, the well-known local ADC file, then the
attached-service-account metadata service. It requires each service adapter to
request explicit Google OAuth scopes. The official Google authentication layer
owns access-token caching and refresh. Larch does not copy ADC files, print or
persist access tokens, or provide a separate credential store.

External-account ADC must use Google's documented token and provider endpoints.
Executable subject-token sources, custom impersonation endpoints, custom cloud
universes, and `GCE_METADATA_HOST` overrides fail closed in production.

See the [Google ADC security contract](security/supply-chain-credentials-and-services.md#google-application-default-credentials)
for the runtime guarantees and residual limits.

#### S3 and R2 credentials

Rust-owned lifecycle, standalone publication, synchronization, and preflight
commands use the official AWS SDK and its non-process credential chain for S3
and Cloudflare R2. No run-log runtime path shells out to the AWS CLI. For S3,
a profile, environment credentials, or an attached role may supply access.
Verify the exact bucket-root operation without printing object names:

```bash
aws s3 ls s3://<bucket> >/dev/null
```

R2 uses S3-compatible credentials. Set `AWS_ACCESS_KEY_ID` and
`AWS_SECRET_ACCESS_KEY` through your normal secret-management path. Also set:

```bash
export LARCH_R2_ACCOUNT_ID="<32-lowercase-hex-account-id>"
export LARCH_R2_ENDPOINT="https://${LARCH_R2_ACCOUNT_ID}.r2.cloudflarestorage.com"
```

Do not place credentials in `tools-config.toml` or
`LARCH_STORAGE_BASE_URI`. See the
[object-storage credential contract](security/supply-chain-credentials-and-services.md#object-storage-credentials-and-transport).

### Install
- **Anthropic / Claude Code**: `curl -fsSL https://claude.ai/install.sh | bash`
- **OpenAI / Codex**: `npm install -g @openai/codex`
- **Cursor / Cursor CLI** (larch uses it only as an agent, but the whole editor package needs to be installed)
- **git**: version control (used by all skills)
- **gh**: [GitHub CLI](https://cli.github.com/), authenticated with repo write access (`gh auth login`). The installed version must provide `gh release verify`, `gh attestation verify`, and immutable release metadata. Larch uses these commands for the first Rust binary install, PR creation, CI monitoring, and merge automation.
- **jq**: [JSON processor](https://jqlang.github.io/jq/).

## Auth
All vendor agents work with either web login or API tokens.
### API Token Use
Set these environment variables in the shell where you run `claude`:
- `ANTHROPIC_API_KEY`
- `OPENAI_API_KEY`
- `CURSOR_API_KEY`
### Web login
Log in through the web for each of the three vendors.
### Helpful Aliases
To easily choose between API key use and web login use for Claude Code, I recommend defining aliases that choose the model and undefine the API key for web-based login.  e.g.:
```bash
# claude with sonnet 4.6 API Token
alias c='git fetch && git rebase && [ -z "$(git stash list)" ] && ENABLE_PROMPT_CACHING_1H=1 claude --model "claude-sonnet-4-6[1m]" --settings='\''{"apiKeyHelper": "echo $ANTHROPIC_API_KEY"}'\'' || echo "Stash is not empty"'

# claude with Opus 4.8 API Token
alias opus='git fetch && git rebase && [ -z "$(git stash list)" ] && claude --model "claude-opus-4-8" --effort high --settings='\''{"apiKeyHelper": "echo $ANTHROPIC_API_KEY"}'\'' || echo "Stash is not empty"'

# claude with Fable 5 API Token
alias fable='git fetch && git rebase && [ -z "$(git stash list)" ] && claude --model "claude-fable-5" --effort high --settings='\''{"apiKeyHelper": "echo $ANTHROPIC_API_KEY"}'\'' || echo "Stash is not empty"'

# claude with sonnet 5 web login
alias cm='git fetch && git rebase && [ -z "$(git stash list)" ] && env -u ANTHROPIC_API_KEY claude --model "claude-sonnet-5" || echo "Stash is not empty"'

# claude with Opus 4.8 API web login
alias opusm='git fetch && git rebase && [ -z "$(git stash list)" ] && env -u ANTHROPIC_API_KEY claude --model "claude-opus-4-8[1m]" --effort high || echo "Stash is not empty"'

# claude with Fable 5 API Token
alias fablem='git fetch && git rebase && [ -z "$(git stash list)" ] && env -u ANTHROPIC_API_KEY claude --model "claude-fable-5" --effort high || echo "Stash is not empty"'
```

## Larch Installation
Larch is distributed as a [Claude Code plugin](https://code.claude.com/docs/en/plugin-marketplaces).
### Install
```bash
claude plugin marketplace add https://raw.githubusercontent.com/character-ai/larch/main/.claude-plugin/marketplace.json
claude plugin install larch@larch-local
```

The remote marketplace fetches only the checked runtime projection under
`plugin/`, pinned to the `stable` branch. That branch moves only when a release
is cut, and always to the tagged projection commit. That commit's first parent
is the merged release commit on `main`. An install therefore receives the
plugin content and the executable derived from the same tagged commit. Merges
to `main` between releases do not change what an install receives. Both its
fetch and the installed cache exclude Rust source,
repository linters, tests, release automation, and CI support files. Runtime
commands are provided by the release-matched `larch` executable. The projection
includes the root security policy, the
[security reference index](security/README.md), and every focused security
reference used by shipped skills.

### Git prerequisite

Keep installed Git available on `PATH`. Typed `gix` reads run in process, but
exact diff output, repository mutations, hooks, filters, signing, credentials,
worktrees, recovery, and network porcelain still use the closed installed-Git
compatibility adapter. Run `git --version` to verify the prerequisite. Larch
does not expose a caller-selected Git executable or arbitrary Git arguments.

### Rust executable bootstrap

Every Rust-backed entrypoint must call
`${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh`. On first use, the shim installs the
executable that exactly matches the plugin version.
It downloads the versioned manifest, checksum file, and host archive from the
immutable `v<plugin-version>` release. It verifies release and asset
attestations, the strict manifest, SHA-256 digests, archive members, and the
staged executable's machine-readable identity before atomically installing
`${CLAUDE_PLUGIN_ROOT}/bin/larch`.

Claude Code supplies `CLAUDE_PLUGIN_ROOT` and `CLAUDE_PLUGIN_DATA` to plugin
commands. The latter holds the bounded first-use lock. A failed bootstrap keeps
an existing executable intact and prints retry guidance. Run the same command
again after fixing a missing tool, authentication problem, or interrupted
download.

The deny, advisory, maintenance, audit, Stop-boundary, and anti-read-poll hook
wrappers set `LARCH_BOOTSTRAP_NO_INSTALL=1`. They may run an already verified override or
installed executable, but they never start a download or installation from a
hook event. When no verified executable is available, `scripts/larch.sh` exits
97 and each wrapper applies its documented local allow, deny, or fixed-advisory
fallback.
Explicit `--preflight-release` and `--latest-stable-version` actions are
unaffected.

`--preflight-release`, the upgrade path's pre-install verification, adds one
check on top of that set: the `stable` branch must be at the release's tagged
projection commit. It refuses the upgrade otherwise, before any plugin state
changes, because the plugin content an install would fetch and the executable
it would run would come from two different commits. Retry after an in-flight
release finishes. First-use bootstrap does not run this check, so an install
that is deliberately on an older release keeps bootstrapping its own matching
executable after the pin advances.

Local `--plugin-dir` development never downloads into the checkout. Build and
select the executable explicitly:

```bash
cargo build --locked --release --package larch-cli
PLUGIN_DATA_PARENT="$(cd "${TMPDIR:-/tmp}" && pwd -P)"
CLAUDE_PLUGIN_ROOT="$PWD" \
CLAUDE_PLUGIN_DATA="${PLUGIN_DATA_PARENT%/}/larch-plugin-data" \
LARCH_BINARY="$PWD/target/release/larch" \
"$PWD/scripts/larch.sh" example echo "local build"
```

The GCS run-log adapter is the narrow exception to the manual build step. When
`CLAUDE_PLUGIN_ROOT` is a local `.git` checkout and `LARCH_BINARY` is unset,
the adapter runs the same locked release build before its first GCS request,
then supplies the result to `scripts/larch.sh` through a process-scoped
`LARCH_BINARY`. This lets the mandatory `run-log lifecycle-start` command
remain first on a fresh checkout. Direct `scripts/larch.sh` use still requires
the explicit build and override above.

`pwd -P` canonicalizes the staging parent because bootstrap path validation
rejects a path whose existing ancestor is a symlink, and on macOS both
`TMPDIR` (under `/var`) and the `/tmp` fallback are symlinks into `/private`.
Canonicalizing also drops the trailing slash macOS puts on `TMPDIR`; the `%/`
trim keeps the composed path `//`-free if the parent is ever `/`.

`LARCH_BINARY` must be an absolute, regular executable. Its version and target
self-check must match the active plugin and host.

See the [bootstrap and atomic-installation security contract](security/supply-chain-credentials-and-services.md#bootstrap-and-atomic-installation)
for archive validation, rollback, and same-UID limitations.

### Configure Claude
Edit `~/.claude/settings.json` and add a `permissions`/`allow` section (if it does not have one yet) with this entry. NOTE: replace `<your-user-name>`!

```JSON
  "permissions": {
      "allow": [
        "Bash(/Users/<your-user-name>/.claude/plugins/cache/larch-local/larch/*/scripts/*)"
      ],
      "defaultMode": "bypassPermissions"
  }
```

`"defaultMode": "bypassPermissions"` skips permission prompts entirely, including for larch's skills. This is the simplest correct setup and the one larch's own dev checkout uses.

If you need stricter permissions instead (no `bypassPermissions`), drop that line and add explicit `Skill(...)` entries, one per larch skill you use. A bare `Skill(larch:*)` wildcard does **not** authorize plugin skills. See [Strict-permissions consumers](configuration-and-permissions.md#strict-permissions-consumers--skill-permission-entries) for the exact copy-paste list to add alongside the `Bash` entry above.

**Remove `apiKeyHelper` from `~/.claude/settings.json`**: larch's Claude subprocesses (voters, reviewers, fixers that skills spawn) run `claude --print`, read `~/.claude/settings.json` directly, and do **not** inherit a top-level `--settings` override. A file-level `apiKeyHelper` (for example `"apiKeyHelper": "echo $ANTHROPIC_API_KEY"`) breaks that path: in subscription/OAuth mode (no `ANTHROPIC_API_KEY` in the shell env) the helper returns empty, so `apiKeyHelper failed` leads to `401 Invalid bearer token`. A non-zero helper exit does **not** fall back to OAuth either. Keep the settings file free of `apiKeyHelper`; inject it only where you want API-key billing (the `*_api` aliases above).

### Configure Codex
- larch's Codex launch, probe, and review-fix surfaces prefer a non-whitespace `OPENAI_API_KEY`. When it is unset, empty, or whitespace-only, they fall back to `codex login` / `~/.codex/auth.json`.
- Do not keep the old top-level `env_key = "OPENAI_API_KEY"` setup advice as your Codex path. larch strips that legacy line from copied temp configs on login fallback.

### Configure Cursor
- **Do NOT edit `~/.cursor/cli-config.json` to set model or max-mode.** Cursor manages that file itself and overwrites it on each launch, so any model / `maxMode` change you make there is reverted and silently ignored by larch. For larch's own Cursor invocations, the default model remains **`composer-2.5`** (passed via `cursor agent --model composer-2.5` from `scripts/larch.sh agent model-args`), and reviewer-panel rows use the same default resolution unless a caller supplies an explicit per-slot `cursor_model` override. **Max-mode is forced on** via the `/max-mode on. Prompt:` slash-command prefix prepended by `scripts/larch.sh agent cursor-wrap-prompt`. To override the default model, set `LARCH_CURSOR_MODEL` (or `CLAUDE_PLUGIN_OPTION_CURSOR_MODEL`) in your environment rather than touching the cli-config file.

- **GUI popup suppression (issue #5797).** larch exports `NO_OPEN_BROWSER=1` into every Cursor child's environment so `cursor agent` does not open the Cursor.app "Composer" GUI window (via a `cursor://` deeplink) during headless lanes. Auth is unaffected: larch authenticates via `CURSOR_API_KEY` / keychain, never interactive login.

- **macOS keychain auth.** If `CURSOR_API_KEY` is unset and Cursor's keychain entry is missing or stale, larch fails with a specific, actionable error instead of Cursor's cryptic `Security process exited with code: 45`. See [macOS keychain interactions](macos-keychain-interactions.md) for the full mechanism and the fix.

- **Co-installed production-guard plugins (issues #8747, #8763, and `character-tech/smarts#909`).** `smarts` versions before v2.0.3 can misclassify Cursor `Shell` commands and match the short PagerDuty marker `pd` inside `--tmpdir`. Upgrade older installs. A current classifier failure can instead return `guard is unavailable` for unrelated Claude Code `Bash` or Cursor `Shell` calls. `/complete-umbrella` retries only that exact transient once, then follows its Failure rule. Claude Code has no approval API, and Cursor smart-mode approval cannot override a PreToolUse `permissionDecision: deny`. See [Complete Umbrella Recovery](complete-umbrella-recovery.md) § Harness false-denies. Repair or disable a current failing guard, report the regression upstream, or run larch workflows in a session without those guards. Do not rephrase larch drivers as `gh`, curl, or wget.

### Optionally configure run-log storage

To publish remote run-log archives, create `tools-config.toml` at the consumer
repository root:

```toml
[larch]
storage_base_uri = "<base-URI>"
```
Example base values are `s3://my-bucket-for-tool-data`,
`gs://my-bucket/prod/tools`, and `r2://my-bucket`. A base stops before the
tool name. Larch derives the client repository from local
`remote.origin.url`, then writes
`<base>/larch/<client-repo>/run-logs/<skill>/<run-id>.tar.gz`. Clone and
worktree directory names do not affect that identity.

The repository-owned file may contain tables for other tools. Larch owns and
strictly validates only `[larch]`; it ignores unrelated tool tables. There is
no global version and no `client_repo` field. A non-empty
`LARCH_STORAGE_BASE_URI` may enable storage without the file, `[larch]`, or
`storage_base_uri`. A present file is always validated first, so an override
does not hide a malformed, symlinked, unreadable, or otherwise invalid file.
Accepted schemes are `s3://`, `gs://`, and `r2://`.

When the file, `[larch]`, or `storage_base_uri` is absent and no override is
set, remote publication is disabled. Skills still run and keep local lifecycle
bookkeeping. They warn at startup and terminalization, then remove staging
without creating a remote archive, synchronized cache entry, or pending
publication. Invalid present configuration still fails. Configured but
inaccessible storage still blocks startup.

When storage is enabled, skill startup lists at most one object under the exact
`larch/<client-repo>/` prefix and ignores the listing output. It does not list
the bucket root or attempt a write. See
[Configuration and Permissions](configuration-and-permissions.md#run-log-object-storage)
and the [language-neutral storage contract](run-log-archive.md).

### Validate
Run `/status` in a `claude` session. Expect a report like this:
```
larch v52.4.17
Run-log storage: accessible
┌────────┬───────┐
│  Tool  │ State │
├────────┼───────┤
│ Codex  │ ok    │
├────────┼───────┤
│ Cursor │ ok    │
└────────┴───────┘
```

Without storage configuration, expect `Run-log storage: disabled
(<reason>)`. A configured but inaccessible provider stops `/status` during
lifecycle startup instead of reporting it as accessible.

## Upgrade
Run the `/upgrade-larch` skill in your `claude` session. It verifies the exact
immutable stable release before refreshing plugin metadata. It then resolves
the new cache root from `claude plugin list --json`, installs that root's
release-matched executable, and verifies matching plugin and binary versions.
The upgrade reads and moves the user-scope install only. Any clone that enables
larch in its `.claude/settings.json`, including the one you upgrade from, keeps
its own project-scope entry pinned to the version current when that clone was
first opened. `/upgrade-larch` ignores those entries and does not refresh them.
A failure leaves the prior cache root untouched and safe for the running
session. If the executable cannot be installed after Claude has already
switched to the new version, the driver moves the active version back to the
prior one so new sessions keep a working `bin/larch`. When it reports that it
could not roll back, run the printed command from a terminal outside Claude
Code before starting new sessions; it installs the missing executable in one
shot:

```bash
CLAUDE_PLUGIN_ROOT=<new-cache-root> CLAUDE_PLUGIN_DATA=<absolute-dir> <new-cache-root>/scripts/larch.sh --version
```

Restart `claude` after a successful install or marketplace repair.
The first upgrade from the old sparse GitHub marketplace registration replaces
that registration with the runtime-only remote source.

If `/upgrade-larch` is unavailable or a plain `claude plugin install
larch@larch-local` keeps installing an older version, the cached marketplace
registration is stale. `claude plugin install` reuses that cache, so refresh
the marketplace first, then reinstall:

```bash
claude plugin marketplace update larch-local
claude plugin uninstall larch@larch-local
claude plugin install larch@larch-local
```

`claude plugin marketplace update larch-local` is the step that pulls the
latest release. Restart `claude` afterward.

See the [upgrade and rollback security contract](security/supply-chain-credentials-and-services.md#upgrade-and-rollback-boundaries)
for verification and cache-ownership guarantees.

## Uninstalling

Remove the plugin and its cached versions:

```bash
claude plugin uninstall larch@larch-local
rm -rf ~/.claude/plugins/cache/larch-local/larch
```

The `rm` deletes only larch's cached version directories under the `larch-local`
marketplace and touches nothing else in the cache. `claude plugin uninstall`
alone leaves those versions, so a later install reuses a cached version instead
of the latest. To reinstall the latest afterward, refresh the marketplace
(`claude plugin marketplace update larch-local`), then install again.
