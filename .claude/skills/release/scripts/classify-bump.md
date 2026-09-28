# larch release classify-bump

`scripts/larch.sh release classify-bump` is the deterministic classifier for the dev-only `/release` skill. It inspects the public plugin surface (`skills/**`, `agents/**`), writes `$IMPLEMENT_TMPDIR/bump-version-reasoning.md` when the session tmpdir is writable (otherwise a uniquely created `bump-version-reasoning.*` file under `${TMPDIR:-/tmp}`), and emits `CURRENT_VERSION`, `NEW_VERSION`, `BUMP_TYPE`, and `REASONING_FILE`.

## Optional `--base <ref>` and `--head <ref>`

When `--base` is set (consumer: dev-only `/release` via `scripts/larch.sh release prepare`):

- Resolves `<ref>` through the typed repository reader and uses that commit as `BASE` directly, skipping merge-base resolution.
- Skips the idempotency short-circuit (`BUMP_TYPE=NONE` for trailing `Bump version to X.Y.Z` commits), so aggregate classification over `BASE..HEAD` is not suppressed by historical version commits on `main`. `/release` consumers never see `NONE` from this script.

When `--head <ref>` is set (consumer: dev-only `/release` via `scripts/larch.sh release prepare`):

- Resolves `<ref>` through the typed repository reader and uses that commit as the diff head instead of `HEAD`, so aggregate classification is anchored to `origin/main` (or another explicit ref) when the caller is not checked out at the release tip.
- Reads `CURRENT_VERSION` from the typed commit-tree blob and fails closed when the worktree `plugin.json` version disagrees.

When `--base` / `--head` are omitted, the classifier uses the merge-base against `main` / `origin/main` and applies the local idempotency shortcut.

## Idempotency

The classifier treats the branch as already versioned when the idempotency head is a `Bump version to X.Y.Z` commit. Before checking that subject, it walks past up to three transparent `Update CHANGELOG for ...` commits that touch only `CHANGELOG.md` or historical `chore(larch-logs): ...` commits that touch only `larch-logs/**`. Subject matches alone are not trusted.

## Edit-in-sync

`release prepare --no-fetch` reads the existing `origin/main` ref. The release
skill uses it for dry runs so preparation does not fetch or update Git refs.
Normal preparation still fetches before selecting the release window.

Keep this file aligned with `larch release classify-bump`, `crates/larch-cli/tests/release_prepare.rs`, and `larch release prepare`.
