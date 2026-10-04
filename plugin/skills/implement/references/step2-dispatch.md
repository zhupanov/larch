# Step 2 dispatch Rust contract

**Consumer**: `/implement` Step 2 orchestrator (`skills/implement/SKILL.md` §2.1 dispatch).

**Contract**: Authoritative reference for `scripts/larch.sh implement step2-dispatch` / `implement run-dispatch` stdout grammar, envelope invariants, manifest schema, recovery paths, and commit-on-behalf semantics. Absorbs the normative contracts from the retired shell dispatcher docs.

**When to load**: MANDATORY before parsing `step2-dispatch` stdout. Read before extending stdout grammar, changing envelope invariants, or modifying manifest schema. `skills/implement/SKILL.md` §2.1 is the orchestrator-side consumer.

This is the surviving `/implement` Step 2 dispatch contract after the atomic Rust cutover. It absorbs the normative contract formerly split across the Step 2 dispatcher, its run-dispatch adapter, recovery-path helper, Step 4 commit wrapper, and external implement launcher docs.

## Step 2 dispatcher

**Orchestrator wait contract**: `scripts/larch.sh implement run-dispatch` starts or reattaches the durable `implement-step2-dispatch` bgjob. The orchestrator waits only through `scripts/larch.sh bgjob wait`; on `BGJOB_STATUS=WAIT`, it repeats the identical wait fence with no intervening tool or prose, and it reads the result env only after `DONE`. It MUST NOT call `ScheduleWakeup`. See `skills/implement/SKILL.md` NEVER #8.

**Invariants**:
- Implementer-coder set: `{claude} ∪ external_tools`. `claude` is the implementer-only fallback path, never an external tool. The `TOOL=` envelope-line contract on external implementer paths continues to mean external implementer only.
- Cursor binary gate: `--cursor-binary-found true|false|""` is accepted. Empty and missing values fall back to `CURSOR_BINARY_FOUND` from session env or a fresh executable check. `--cursor-present` remains compatibility-only and must not block a selected Cursor coder when the binary exists.
- `--coder` is required. `/implement` Step 0 resolves the omitted operator flag in `crates/larch-cli/src/implement_bootstrap_continuation.rs::phase_coder`, and the Step 2 adapter forwards that explicit value to `scripts/larch.sh implement run-dispatch`. A direct dispatcher call without `--coder` exits 2 before git resolution.
- Stdout is KV-only — `STATUS`, `TOOL`, `MANIFEST`, `QA_PENDING`, `REASON`, `TRANSCRIPT`, `SIDECAR_LOG`, `SCOUT_CODER_MANIFEST`, `SCOUT_CODER_STATUS`, `ORCHESTRATOR_EDIT_AUTHORITY`, plus optional advisory KVs `WARN_CODEX_NONZERO_EXIT=true`, `WARN_PLAN_FILES_UNTOUCHED=true`, and `WARN_PLAN_FILES_UNTOUCHED_COUNT=<N>` on documented `STATUS=complete` paths. The launcher's progress chatter is captured to the sidecar log; the implementer transcript is captured to disk; neither leaks to stdout. SKILL.md Step 2's parser is a fixed grammar; the `WARN_*` markers are trailing advisory KVs (like the `PHANTOM_*` probe tail) that Step 2 does not branch on.
- Spawn-time baseline files are written ONCE on the first invocation under `$TMPDIR_ARG`: `step2-baseline.txt` (HEAD SHA), `step2-spawn-branch.txt` (branch name). All resume invocations reuse them. The baseline SHA anchors the launcher-retry "clean state" guard (post-failure HEAD must equal baseline). There is no fail-closed manifest path equality gate after the dispatcher took over committing. The dispatcher now writes plan-coverage artifacts from the Step 0 materialized plan; below-middle coverage gaps remain warning-only, while missing or malformed coverage on a complete path fails closed and high-band or blocking actionable `todos_left` gaps require a recorded scope disposition. Full-suite validation-only reminders are ignored for this gate. The existing warn-only undeclared-manifest diagnostic still runs when git touched-path probes succeed.
- **Plan-file coverage diagnostic**: Warn-only gate comparing explicit firm `### NEW:` / `### UPDATED:` / `### REWRITTEN:` headings against working-tree touched paths; emits `WARN_PLAN_FILES_UNTOUCHED=true` and `WARN_PLAN_FILES_UNTOUCHED_COUNT=<N>` when untouched; skips `### MAY_UPDATE:`; still commits on any probe outcome. See the inline Rust Step 2 route tests and `scripts/larch.sh plan scope-paths` for full gate semantics.
- Immediately before launching the external implementer, `step2-dispatch` writes malformed-manifest recovery baselines under `$TMPDIR_ARG`: `step2-prelaunch-porcelain.nul` (`git status --porcelain=v1 -z --untracked-files=all`), `step2-prelaunch-content-digests.txt` (SHA-256 snapshots for pre-dirty paths), and `step2-prelaunch-index.env` (`PRELAUNCH_INDEX_NONEMPTY=true|false`). If `$TMPDIR_ARG` is under the repo root, paths under that tmpdir are filtered from the recovery delta so harness scratch artifacts do not count as implementation work.
- Per-tool files under `$TMPDIR_ARG` use `${TOOL_TAG}-...` names: `${TOOL_TAG}-resume-count.txt`, `${TOOL_TAG}-impl-transcript.txt`, `${TOOL_TAG}-impl.log`, `${TOOL_TAG}-commit-message.txt`, and `${TOOL_TAG}-commit-stderr.txt`. `TOOL_TAG=codex` preserves the historical Codex filenames byte-for-byte. For **`CODER=codex` only**, `${TOOL_TAG}-impl-transcript.txt`, `manifest.json`, and `qa-pending.json` live under `$TMPDIR_ARG/codex-step2-out/`; Cursor keeps those files directly under `$TMPDIR_ARG/`.
- `$TMPDIR_ARG/scout-coder-manifest.json` is the normalized coder-produced scout sidecar for Codex, Cursor, and main-agent Claude. The marker filename remains `$TMPDIR_ARG/step2-external-scout-eligible.txt` for compatibility, but the concept is coder-produced rather than external-only. `step2-spawn-coder.txt` keeps its cross-coder tmpdir reuse guard semantics and is not the Step 5 scout marker.
- `scripts/larch.sh implement normalize-coder-scout --tmpdir "$IMPLEMENT_TMPDIR" --input <path> [--producer external|main-agent]` owns normalization. External implementers pass the tool-specific `launch_scout_manifest` path: Codex uses `$TMPDIR_ARG/codex-step2-out/scout-coder-manifest.json`; Cursor uses the tmpdir-local launch path. Main-agent Claude writes `$TMPDIR_ARG/scout-coder-manifest.raw.json` and passes that path. The helper never infers the producer input path except to handle missing or unreadable input as `missing-or-invalid`.
- Implement normalization always calls `scout filter-manifest --mode review`. The default plan-review mode is not valid here because it reserves plan-review-only slugs such as `arch`.
- `SCOUT_CODER_STATUS=ok` requires either intentional empty raw input (`{"archetypes":[]}`) or a non-empty filtered output from a non-empty raw input. Raw non-empty input that filters to zero is `missing-or-invalid`, not success. Missing input, malformed JSON, filter rejection, missing output, and malformed output are also `missing-or-invalid`.
- The normalizer writes the canonical normalized manifest and `step2-scout-coder-status.env` every time. It writes `step2-external-scout-eligible.txt` only when `SCOUT_CODER_STATUS=ok`. Missing or invalid output is non-fatal, prints a loud stderr warning, writes canonical `{"archetypes":[]}`, and degrades Step 5 to static-only review. The persisted `Warnings` entry is emitted later by `review dispatch-panel` after diff classification, not by the normalizer.
- Step 5 status meanings: `pre-scouted` means a valid non-empty coder manifest was forwarded; `pre-scouted-empty` means a valid intentional empty coder manifest was forwarded and static reviewers are sufficient; `producer-missing` and `producer-invalid` mean no eligible coder manifest reached Step 5; `skipped-docs-only`, `skipped-test-only`, and `skipped-generated-only` are by-design diff-classification skips and are not producer failures.
- `$TMPDIR_ARG` is canonicalized with `cd "$TMPDIR_ARG" && pwd -P` immediately after validation and before any derived path is constructed. Manifest, QA, transcript, sidecar, baseline, raw-manifest, and resume-count paths therefore use canonical bytes, which keeps Codex `--add-dir` grants aligned with sandbox path resolution even when the caller supplied a symlinked or `..`-containing tmpdir spelling. On the codex path only, Codex-written outputs (manifest, qa-pending, transcript) are placed in `$TMPDIR_ARG/codex-step2-out/` and the Codex `--add-dir` grant is narrowed to that subdir via `dirname("$MANIFEST_PATH")`; `scripts/larch.sh agent launch-codex-implement` rejects symlink parents and refuses a grant rooted at `$IMPLEMENT_TMPDIR`.
- Immediately after canonicalizing `$TMPDIR_ARG`, the dispatcher exports `IMPLEMENT_TMPDIR="$TMPDIR_ARG"`. If `$TMPDIR_ARG/session-id` exists and is non-empty, its trimmed contents overwrite any inherited `LARCH_TOKEN_SESSION_ID`; the canonical tmpdir file wins over stale environment state on external-implementer paths. If `$TMPDIR_ARG/claude-source.env` exists, `LARCH_CLAUDE_SOURCE_FILE` is exported to that snapshot path. The leaf launchers repeat the same overwrite defensively before spawning external tools.
- `scripts/larch.sh implement run-dispatch` owns once-only Step 2 token and timing marks after it acquires `dispatch.lock`. `step2-dispatch` must not emit Step 2 timing rows. External launchers must not emit `token mark "Step 2 — implementation"`; they keep the token-budget preflight (`scripts/larch.sh token check-budget`) before external launch. Token marks themselves go through `scripts/larch.sh token mark`. `run-dispatch` skips the once-only mark on `--answers` redispatch and after `$IMPLEMENT_TMPDIR/.step2-telemetry-marked` exists. The adapter replaces a completed result only on that explicit `--answers` redispatch; otherwise it reattaches or returns the prior completed result.
- Resume counter is incremented ONLY when `--answers PATH` is supplied. Cap is 5; the 6th `--answers` invocation emits `STATUS=bailed REASON=qa-loop-exceeded` without spawning the implementer.
- On `status=complete`, independently computed required plan-coverage gaps or blocking `todos_left` trigger up to three in-process completion re-dispatches before the dispatcher commits. Each retry preserves the compatible working-tree edits, receives the untouched paths and blocking todos as delimited untrusted feedback, and records its count plus plan-coverage fingerprint in `step2-completion-retry-state.env`. The retry re-computes and verifies that fingerprint before consuming the feedback. Once the cap is exhausted, the existing scope-disposition gate remains required; a retry never silently permits partial shipping.
- Launcher wrapper exit is captured separately from the implementer-reported `LAUNCHER_EXIT=` KV. Wrapper exit `2` is a validation failure and emits `STATUS=bailed REASON=wrapper-validation-failure` immediately without retrying. Wrapper exit `0` keeps the existing KV parsing behavior. Other non-zero wrapper exits enter the existing one-shot retry path only when no manifest was written and post-failure state is fully clean (`git status --porcelain` empty, no `.git/index.lock`, HEAD == `BASELINE_SHA`). Launcher stdout/stderr is captured to a temp file under the canonical `$TMPDIR_ARG`, capped to 65 KiB for bail-path parsing, and removed by an EXIT trap.
- **Non-zero `LAUNCHER_EXIT` salvage (issue #3383)**: Codex-only; when `manifest.json` is non-empty and parses as `schema_version "1"` / `status "complete"`, the dispatcher salvages it, continues to Step 5/7b, and appends `WARN_CODEX_NONZERO_EXIT=true`. Not mirrored to Cursor. The inline tests in `crates/larch-cli/src/implement_step2_commands_route.rs` cover the salvage decision.
- The dispatcher does NOT `git reset`, NOT `git checkout`, NOT discard working-tree state. On `status=complete` it stages and commits implementer edits with `git add -A && git commit -F <commit-message-file>`. The manifest supplies `commit_message` without a diff or subject cross-check. The dispatcher runs the Rust redactor exposed as `scripts/larch.sh redact secrets` immediately before `git commit -F`, so the secrets-family scrubber that protects the canonical on-disk manifest also protects git history. After a successful commit, it invokes the verified `scripts/larch.sh run-log checkpoint` runtime best-effort. This gives active `/implement` log writes a local recovery checkpoint behind the implementer commit. On any other status, the dispatcher leaves the working tree untouched. On `commit-failed`, `git add -A` has already run and the index stays staged. Operators inspect `git status` and `$IMPLEMENT_TMPDIR/${TOOL_TAG}-commit-stderr.txt` before deciding whether to `git reset` or amend. Implementer hard guard #1 (no destructive git ops) is mirrored here as "the dispatcher never destroys operator work either."
- Path validation rejects `..`, leading `/`, and any path under a submodule (per `git submodule status --recursive`). NUL bytes are rejected implicitly — bash variables cannot hold a NUL, so the `read -r` consuming the jq output terminates the field at any NUL upstream; an explicit `*$'\0'*` glob would expand to `**` (since `$'\0'` is empty in bash strings) and match every non-empty path, so the check must not be expressed that way.
- `manifest-schema-invalid` can recover to `STATUS=claude_fallback` only when the raw manifest parses as a JSON object, prior status is `complete` or empty with the legacy `{status, summary, checks}` fingerprint and no `schema_version`, the prelaunch index was empty, the post-launch NUL-safe delta is non-empty after tmpdir filtering and pre-dirty content-snapshot comparison, and post-implementer safety gates pass (branch unchanged, submodules clean including dirty paths under initialized submodules, Cursor HEAD unchanged). Recovery writes `step2-recovery-paths.nul`, quarantines `manifest-raw.json` to `manifest-raw.invalid.json`, and writes `recovery-metadata.json`.
- Exit code is 0 on every documented outcome (including `STATUS=bailed`). Exit 2 is reserved for caller-error (missing flag, bad path, bad enum value) before any Codex spawn.

**Stdout contract**:
```
STATUS=<complete|needs_qa|bailed|claude_fallback>
MANIFEST=<path>          # set ONLY when STATUS=complete or needs_qa, or when STATUS=bailed
                         # came from an implementer-authored manifest (status=bailed in the manifest
                         # itself, e.g. resume-incompatible). Dispatcher mechanical bails
                         # (commit-failed, manifest-schema-invalid, manifest-missing,
                         # branch-changed, protected-path-modified, submodule-dirty,
                         # qa-pending-missing, qa-loop-exceeded, redactor-not-executable,
                         # dirty-state-after-timeout, wrapper-validation-failure,
                         # codex-runtime-failure, cursor-runtime-failure,
                         # coder-mismatch-tmpdir-reuse) DO NOT emit
                         # MANIFEST= — and on commit-failed the manifest files are deleted
                         # from $IMPLEMENT_TMPDIR before bail to avoid leaving un-sanitized
                         # text on disk.
QA_PENDING=<path>        # set ONLY when STATUS=needs_qa
REASON=<token>           # set ONLY when STATUS=bailed
TRANSCRIPT=<path>        # set when launcher actually ran
SIDECAR_LOG=<path>       # set when launcher actually ran
WARN_CODEX_NONZERO_EXIT=true
                         # OPTIONAL, advisory. Codex STATUS=complete path only,
                         # when a complete manifest was salvaged after a non-zero
                         # implementer exit (issue #3383). Trailing advisory KV;
                         # Step 2 does not branch on it.
WARN_PLAN_FILES_UNTOUCHED=true
WARN_PLAN_FILES_UNTOUCHED_COUNT=<N>
                         # OPTIONAL, advisory. STATUS=complete path only, when
                         # explicit plan-file headings name paths that are absent
                         # from the pre-commit working-tree touched-path set.
                         # The gate is warn-only and Step 2 does not branch on it.
ORCHESTRATOR_EDIT_AUTHORITY=<allowed|forbidden>
                         # ALWAYS emitted (every exit-0 outcome). `allowed` iff STATUS=claude_fallback;
                         # `forbidden` on every external-implementer outcome (complete/needs_qa/bailed).
                         # Mechanical gate for SKILL.md Step 2.4 Claude-fallback authority
                         # (orchestrator spawns larch:claude-implementer; main agent does not Edit/Write).
RECOVERY_FROM=manifest-schema-invalid
RECOVERY_PRIOR_TOOL=<codex|cursor>
RECOVERY_PATHS_FILE=<path-to-step2-recovery-paths.nul>
                         # Optional all-or-none triplet emitted only with
                         # STATUS=claude_fallback on malformed-manifest recovery.
                         # The paths file is NUL-delimited and is the authoritative
                         # path list for recovery commit scoping.
```

**Flags**:

| Flag | Required | Purpose |
|------|----------|---------|
| `--tmpdir PATH` | yes | `$IMPLEMENT_TMPDIR` (where baseline / counter / manifest / transcript / sidecar log live) |
| `--plan-file PATH` | yes | The plan to implement (passed through to Codex) |
| `--feature-file PATH` | yes | The original feature description (passed through to Codex) |
| `--coder VALUE` | yes | `claude`, `codex`, or `cursor`. Resolved by `/implement` Step 0 and forwarded by `scripts/larch.sh implement run-dispatch`. |
| `--difficulty VALUE` | no | `TRIVIAL`, `MODERATE`, or `HARD`. `run-dispatch` uses the persisted operator override first, then `DESIGN_DIFFICULTY` from `difficulty-prior.env`, and forwards it to Codex Step 2 launches when present. |
| `--codex-available VALUE` | optional (deprecated) | `true` (maps to `--coder codex`) or `false` (maps to `--coder claude`). Emits a stderr deprecation warning. Mutually exclusive with `--coder`. |
| `--cursor-present VALUE` | optional, deprecated | Compatibility-only probe-health flag. Ignored for routing. |
| `--cursor-binary-found VALUE` | optional | `true`, `false`, or empty. A false value reaches the dispatcher fallback branch and returns `STATUS=claude_fallback`; empty falls back to session env or a fresh executable check. |
| `--codex-binary-found VALUE` | optional | `true`, `false`, or empty. A false value reaches the dispatcher fallback branch and returns `STATUS=claude_fallback`; empty falls back to session env or a fresh executable check. |
| `--answers PATH` | optional | Operator answers to a prior `needs_qa` cycle; presence increments the resume counter |

External implementer launches use a fixed 7200-second wall-clock timeout. The parent form of `scripts/larch.sh implement run-dispatch` starts or reattaches the `implement-step2-dispatch` bgjob, and the adapter re-executes the same Rust verb in child mode. The child atomically publishes the full dispatcher envelope to the adapter merge env; after bgjob completion, the daemon publishes that envelope with `BGJOB_RC` and `STEP` at `$IMPLEMENT_TMPDIR/bgjob/implement-step2-dispatch.result.env`. `crates/larch-cli/src/implement_dispatch_commands.rs` defines the shared 7200-second budget used by the adapter and external launcher.

**Outcomes** (`STATUS` values):
- `complete`: all post-Codex mechanical checks passed; the dispatcher committed Codex's working-tree edits using `manifest.commit_message` (redacted immediately before `git commit -F` by the Rust owner exposed as `scripts/larch.sh redact secrets`); on **`CODER=codex`**, the sanitized manifest is emitted at `$TMPDIR/codex-step2-out/manifest.json` (i.e. `$MANIFEST_PATH` after the codex subdir retarget); on **`CODER=cursor`**, it remains at `$TMPDIR/manifest.json` under the tmpdir root.
- `needs_qa` — Codex wrote `qa-pending.json` with operator questions; SKILL.md Step 2 collects answers and re-invokes the dispatcher with `--answers`.
- `bailed` — Codex itself emitted `status=bailed`, OR the dispatcher overrode `complete` because mechanical validation failed. `REASON` token list is in `skills/implement/references/codex-manifest-schema.md` (Bail-reason tokens section). When the dispatcher overrides Codex, the dispatcher's reason wins.
- `bailed` with `REASON=prior-attempt-unfinalized` — an older interrupted external attempt left a content delta relative to its preserved prelaunch snapshot. The dispatcher writes `step2-recovery-paths.nul` and does not capture a new baseline or launch another implementer.
- `claude_fallback` with `RECOVERY_FROM=manifest-schema-invalid` — external implementer produced a malformed manifest but left a recoverable working-tree delta. This is commit-only recovery: the orchestrator must not re-implement or rewrite those files, and must commit only the NUL-delimited recovery path list after plan-scope alignment.
- Claude fallback paths remove stale coder scout artifacts so Step 5 cannot consume stale state: normalized manifest, eligibility marker, `step2-scout-coder-status.env`, `scout-coder-manifest.raw.json`, and tool-specific outdir copies such as `codex-step2-out/scout-coder-manifest.json`.
- **`main-branch-prohibited`** — dispatcher-authored bail before the external launcher runs: spawn-time branch is `main` or `master`; `FORKED_TARGET` is not `true` (read from `$TMPDIR_ARG/session-env.sh` when that file exists; otherwise treated as `false`); and the run is issue-anchored — non-empty `ISSUE_NUMBER=` in `$TMPDIR_ARG/parent-issue.md` **or** `$TMPDIR_ARG/session-env.sh` exists (presence alone suffices; `ISSUE_NUMBER` may be absent in session-env). Harness runs with neither parent-issue nor session-env are not affected (external implementer may still run on `main`/`master` in those narrow harnesses — ship-time `bump-branch-guard` remains the non-negotiable backstop for mis-anchored production tmpdirs).
- **`detached-head-prohibited`** — same pre-launcher gate as `main-branch-prohibited`, but when the spawn-time symbolic branch is missing (detached HEAD / not on a branch) or legacy `step2-spawn-branch.txt` contains the literal `HEAD` from older `rev-parse --abbrev-ref` captures. Uses the same issue-anchored + non-fork predicates; fork mode skips via `FORKED_TARGET=true`.

**Bail-reason tokens emitted by the dispatcher** (set internally; full list in `codex-manifest-schema.md`):

**Call sites**:
- `skills/implement/SKILL.md` Step 2 — the only authorized caller.

**Edit-in-sync**:
- `skills/implement/references/codex-manifest-schema.md` — manifest schema and bail-reason tokens.
- `agents/_implementer-base.md` — inline `## Manifest JSON template` and self-validation prompt copied into both generated implementer prompts.
- `skills/implement/prompts/codex-implementer.md` — the system prompt this dispatcher invokes.
- `skills/implement/prompts/cursor-implementer.md` — the system prompt this dispatcher invokes.
- `skills/implement/SKILL.md` Step 2 — the caller; any change to the KV envelope must be mirrored in Step 2's parser.
- The inline tests in `crates/larch-cli/src/implement_step2_commands.rs`: any new outcome or reason token must be exercised.
- `scripts/larch.sh plan scope-paths` — shared `## Files to modify/create` scope grammar used by recovery plan-scope alignment.
- `scripts/larch.sh dirty-tree scope-check` — fail-closed recovery scope verifier for malformed-manifest preservation.
- `scripts/larch.sh implement recovery-paths` — shared recovery-delta recompute helper used by the dispatcher and Step 2.4 recovery path; `--capture-postlaunch` refreshes the postlaunch porcelain before diffing for orchestrator-owned fallback pathspecs.
- `scripts/external-tool-registry.md` — update its "Sourced by" list when this script's source-list status changes.

**Makefile wiring**: `make test-run-step2-dispatch` and `make test-step2-dispatch`.

## Run-dispatch adapter

`scripts/larch.sh implement run-dispatch` owns both sides of the durable adapter.
The parent launches or reattaches the bgjob, and the child serializes the call to
`scripts/larch.sh implement step2-dispatch`. Both forms reduce the worker input to
the implement tmpdir and selected coder while deriving the rest of the context
from session artifacts.

Caller: `skills/implement/SKILL.md` Step 2.1 and Q/A redispatch in Step 2.3.

Arguments:

- `--implement-tmpdir PATH` is required directly or through `IMPLEMENT_TMPDIR`.
- `--coder CODER` is required.
- `--answers PATH` is optional and is only for Step 2.3 Q/A redispatch.
- `--bgjob-child --merge-result-env PATH` are adapter-only paired flags. They publish the exact successful dispatcher envelope to the daemon merge env; callers do not parse child stdout.

Derived sources:

- `$IMPLEMENT_TMPDIR/plan.txt`: always forwarded as `--plan-file` (conventional
  path; the launcher does not read `PLAN_FILE` from `session-env.sh`).
- No workflow flag is passed; the launcher reads only the conventional plan, feature file, coder, cursor-presence, and optional answers inputs.
- `$IMPLEMENT_TMPDIR/session-env.sh`
  - `CURSOR_BINARY_FOUND`: forwarded as `--cursor-binary-found`; `CURSOR_PRESENT` is compatibility-only when present in old examples.
  - `LARCH_CLAUDE_PLUGIN_ROOT`: resolves the downstream script path when
    `CLAUDE_PLUGIN_ROOT` is not already set.
- `$IMPLEMENT_TMPDIR/feature-description.txt`: forwarded as `--feature-file`.
- `$IMPLEMENT_TMPDIR`: forwarded as `--tmpdir`.

Behavior:

- The wrapper validates `session-env.sh`, `plan.txt`, `feature-description.txt`, and plugin root before acquiring `dispatch.lock`.
- Missing Cursor or Codex binaries no longer hard-fail in the wrapper. The wrapper forwards the `--cursor-binary-found` / `--codex-binary-found` values so `step2-dispatch` can choose the documented `claude_fallback` branch.
- After the child returns, if stdout contains line-anchored `STATUS=claude_fallback` and `ORCHESTRATOR_EDIT_AUTHORITY=allowed`, the wrapper resolves `git rev-parse --show-toplevel` and captures `step2-prelaunch-porcelain.nul`, `step2-prelaunch-content-digests.txt`, and `step2-prelaunch-index.env` when absent. It fails closed without relaying a success envelope if git root resolution or capture fails.

Exception:

- `--answers PATH` cannot be derived safely from tmpdir state because each Q/A
  redispatch writes a new `$IMPLEMENT_TMPDIR/codex-answers-$RESUME_N.json`.
  Picking "latest" would be order-sensitive and could replay stale answers, so
  the Q/A loop passes the exact answers file for redispatch only.

Harness: the inline tests in `crates/larch-cli/src/implement_step2_commands.rs`.

## Step 4 commit wrapper

Thin Step 4 wrapper around `scripts/larch.sh git commit`. The Rust owner emits `scripts/larch.sh token mark` and `scripts/larch.sh timing` marks for "Step 4 — commit implementation" before running the commit. It inherits `LARCH_TIMING_LEDGER` and `LARCH_TOKEN_SESSION_ID` from the caller environment and forces `LARCH_TIMING_SKILL=implement` for the timing mark.

After Step 0 bootstrap, invoke through the session launcher so plugin-root rehydration matches `skills/implement/SKILL.md` Step 4.

Usage:

```bash
"$HOME/.cache/larch/sessions/implement-run-$PPID.sh" scripts/larch.sh implement commit --message "Implement feature" [files...]
"$HOME/.cache/larch/sessions/implement-run-$PPID.sh" scripts/larch.sh implement commit --message "Recover implementation" --pathspec-from-file paths.nul --pathspec-file-nul
```

When `--pathspec-from-file` is present, positional file args are ignored and the wrapper passes `--only --pathspec-from-file <PATH>` through to `scripts/larch.sh git commit`. Add `--pathspec-file-nul` for NUL-delimited path lists. This mode is used by malformed-manifest recovery so pre-existing staged content is not swept into the synthesized implementation commit.

Output:

- `COMMITTED=true|false`
- `SHA=<head-sha-or-empty>`
- `ERROR=<message>` on failure

On unknown option or other usage error, the wrapper exits 2 with `COMMITTED=false` and emits a stderr hint: `HINT: --stage-all belongs to review-and-fix commit-fixes (Step 5 review fixes); implementation commits name specific files or use --pathspec-from-file.`

When telemetry env keys are absent, the wrapper self-rehydrates `LARCH_TOKEN_SESSION_ID`, `LARCH_CLAUDE_SOURCE_FILE`, and `LARCH_TIMING_LEDGER` from `$IMPLEMENT_TMPDIR/session-env.sh` before marking Step 4.
