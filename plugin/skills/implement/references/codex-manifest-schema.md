# Codex Implementer Manifest Schema

**Consumer**: `/implement` Step 2 — `scripts/larch.sh implement step2-dispatch` dispatcher (validation), `skills/implement/prompts/codex-implementer.md` (production), and downstream Steps 4 / 8a / 9a / 9a.1 (consumption).

**Contract**: Single normative source for the JSON manifest Codex writes at `$IMPLEMENT_TMPDIR/manifest.json` after each implementation attempt. The dispatcher validates the manifest with `jq -e` per the rules below, then — on `status=complete` — uses `manifest.commit_message` to commit Codex's working-tree edits (`git add -A && git commit -F …`). Codex itself does NOT commit (it runs under `workspace-write` sandbox semantics that forbid `.git/` writes). Downstream SKILL.md steps consume only the validated, sanitized manifest — they never read Codex's transcript or run `git diff` to figure out what changed.

**When to load**: when editing `scripts/larch.sh implement step2-dispatch` (manifest validation), `skills/implement/prompts/codex-implementer.md` / `skills/implement/prompts/cursor-implementer.md` (production), or the Rust ship driver Steps 8a / 9a / 9a.1 (consumption). The `/implement` orchestrator handles only the manifest path (`MANIFEST_PATH` / `--manifest-path`); it never parses manifest JSON in-prompt.

---

## Edit-in-sync note

`agents/_implementer-base.md` carries an inline copy of the manifest shape under `## Manifest JSON template`, plus prompt-side `jq -e` checks under `## Self-validate before atomic rename`. Any schema change here MUST be mirrored there before regenerating `skills/implement/prompts/codex-implementer.md` and `skills/implement/prompts/cursor-implementer.md`; the duplicate exists so long-context implementer runs see the exact required JSON shape at manifest-write time. Keep the prompt-side `schema_version` predicate in the coercing form `(.schema_version | tostring) == "1"` to match dispatcher stringification, keep the `qa-pending.json.tmp` self-validation requirement in sync with the dispatcher `qa-pending-missing` gate, and require `architectural_acknowledgment` in the prompt-side manifest predicate whenever `step2-architectural-knowledge.env` records `ARCHITECTURAL_KNOWLEDGE_REQUIRED=true`.

## Schema

```json
{
  "schema_version": "1",
  "status": "complete|needs_qa|bailed",
  "files_touched": [
    {"path": "<repo-relative path>", "lines_added": <int>, "lines_removed": <int>}
  ],
  "tests_added_or_modified": ["<repo-relative path>", ...],
  "summary_bullets": ["<bullet 1>", "<bullet 2>", "<bullet 3>"],
  "architectural_acknowledgment": "<one-line acknowledgment when architectural knowledge was supplied>",
  "commit_message": "<subject line>\n\n<optional body paragraphs>",
  "difficulty": {"predicted_tier": "TRIVIAL|MODERATE|HARD", "confidence": "low|medium|high", "rationale": "bounded rationale"},
  "todos_left": ["<actionable todo>", ...],
  "oos_observations": [
    {"title": "<short title>", "description": "<full description>", "phase": "implement"}
  ],
  "bail_reason": "<token>",
  "needs_qa": {
    "questions": [{"id": "<stable id>", "text": "<full question text>"}, ...]
  }
}
```

## Required keys per status

| Field | `complete` | `needs_qa` | `bailed` |
|-------|------------|------------|----------|
| `schema_version` (string `"1"`) | required | required | required |
| `status` (enum) | required | required | required |
| `files_touched` (array of `{path, lines_added, lines_removed}`) | required, non-empty | optional | optional |
| `tests_added_or_modified` (array of strings) | required (may be empty) | optional | optional |
| `summary_bullets` (array of strings, length 1–5) | required | optional | optional |
| `architectural_acknowledgment` (string) | required when architectural knowledge was supplied | required when architectural knowledge was supplied | optional |
| `commit_message` (string) | required, non-empty | optional | optional |
| `difficulty` (object with `predicted_tier`, `confidence`, `rationale`) | required | optional | optional |
| `todos_left` (array of strings) | required (may be empty) | optional | optional |
| `oos_observations` (array of `{title, description, phase}`) | required (may be empty) | optional | optional |
| `bail_reason` (string) | absent or empty | absent or empty | required, non-empty |
| `needs_qa.questions` (non-empty array) | absent | required, non-empty | absent |

`oos_observations[]` contains only post-triage filed-OOS candidates. It excludes inline-folded rules 1-2 items from the OOS triage policy in `${CLAUDE_PLUGIN_ROOT}/skills/implement/references/execution-issues-tracking.md`, which are folded into the implementer's own commit with `Inline-triage rule N:` annotations in `commit_message`. It also excludes security findings, which route through `${CLAUDE_PLUGIN_ROOT}/docs/security/workflow-trust-and-mutations.md` instead of the public OOS issue path. The array may be empty when every applicable item was folded inline or security-routed.

Optional fields MAY be present in the non-`complete` statuses but are not required and are not consumed by downstream SKILL.md steps.

`todos_left[]` is for actionable deferred implementation work only. It excludes validation-only notes about unrun full suites when `/implement` intentionally runs focused relevant checks and CI owns broad validation on the green path.

`architectural_acknowledgment` is required only when Step 2 launch supplied valid architectural knowledge from `ARCHITECTURAL_INVARIANTS.md` and/or `ARCHITECTURAL_GUIDELINES.md`. The launcher records that decision in `$IMPLEMENT_TMPDIR/step2-architectural-knowledge.env` as `ARCHITECTURAL_KNOWLEDGE_REQUIRED=true|false`; the dispatcher treats a well-formed snapshot as authoritative and falls back to the shared reader predicate only when the snapshot is absent or malformed. Missing or empty acknowledgment on `status=complete` or `status=needs_qa` is a non-recoverable `STATUS=bailed REASON=architectural-acknowledgment-missing` result, with no `RECOVERY_FROM=` fallback and no dispatcher commit. `status=bailed` is exempt because the coder may have stopped before reading all context. This field proves visible acknowledgment only; reviewers enforce semantic compliance.

## Validation rules (dispatcher applies via `jq -e`)

1. `schema_version == "1"`. Future schema bumps will add new accepted values.
2. `status` is one of the three enum literals above. No other value is accepted.
3. Per-status required keys per the table; the dispatcher rejects (`STATUS=bailed reason=manifest-schema-invalid`) any manifest that fails this check.
4. `difficulty.predicted_tier` is one of `TRIVIAL`, `MODERATE`, or `HARD`; `confidence` is one of `low`, `medium`, or `high`; `rationale` must be non-empty after control-character stripping and is capped by the dispatcher. Codex and Cursor self-rate every `complete` invocation against the shared rubric in `scripts/larch.sh difficulty render-rubric`.
5. **Path normalization** (applied to every `path` in `files_touched` and every entry in `tests_added_or_modified`): the path MUST be repo-relative. Reject if it contains `..` or starts with `/`. NUL bytes are rejected implicitly: bash variables cannot hold a NUL, so the dispatcher's `read -r` over the jq output terminates the field at any NUL in upstream JSON, and the iterator never sees a path-with-NUL. Also reject any path equal to OR under a submodule root (per `git submodule status --recursive`). Symlink-aware containment (rejecting paths that resolve outside the repo via a symlink chain) is **not** mechanically enforced today — external implementers are trusted not to commit symlink-escape paths under the same trust model documented in `${CLAUDE_PLUGIN_ROOT}/docs/security/workflow-trust-and-mutations.md`.
6. **Sanitization** (applied AFTER schema validation, BEFORE the canonical manifest is written to `$IMPLEMENT_TMPDIR/manifest.json`): `summary_bullets[*]`, `architectural_acknowledgment`, `commit_message`, `oos_observations[*].title`, `oos_observations[*].description`, and `todos_left[*]` pass through the Rust redactor exposed as `scripts/larch.sh redact secrets`, which redacts the secrets family (API keys, tokens, OAuth, JWT, passwords, certificates) → `<REDACTED-TOKEN>`. `architectural_acknowledgment` also has CR/LF collapsed to spaces and is capped to 500 characters. Internal hostnames/URLs and PII redaction are NOT mechanically applied by the dispatcher; external implementers are instructed to pre-redact those patterns before manifest emission, and downstream consumers (`/issue` outbound shell scrubber, `scripts/larch.sh tracking-issue`) provide a second-line backstop for the secrets family only. Operators handling internal-URL- or PII-rich content should review the manifest before allowing PR / issue / release notes publication. `bail_reason` is NOT piped through `redact secrets`; it is sanitized only for KV-grammar safety (whitespace and control characters collapsed; capped at ~200 chars) so the bail token cannot break the orchestrator's KV stdout parser. `needs_qa.questions[*].text` is NOT mechanically sanitized; the orchestrator surfaces questions verbatim via `AskUserQuestion`, and external implementers are instructed to phrase questions without sensitive content.

## Atomic write rule

Codex MUST write `manifest.json` and `qa-pending.json` atomically: write to `<path>.tmp`, then `mv <path>.tmp <path>`. The dispatcher reads `manifest.json` only — never `manifest.json.tmp`. A crashed Codex that left only `manifest.json.tmp` looks identical to "no manifest written" and trips the `STATUS=bailed reason=manifest-missing` path.

## Bail-reason tokens

When `status=bailed`, `bail_reason` MUST be one of these stable tokens (downstream tooling pattern-matches on them):

- `resume-incompatible` — Codex inspected branch state on resume and could not reconcile prior partial work with the new operator answers. The branch is left as-is for operator inspection.
- `qa-loop-exceeded` — dispatcher's resume cap (5) tripped on the 6th invocation. Set by the dispatcher, not by Codex itself.
- `manifest-schema-invalid` — manifest failed JSON or schema validation, OR the resume counter file was corrupt (non-numeric content). Set by the dispatcher.
- `protected-path-modified` — Codex's working tree touched a submodule, or `manifest.files_touched` listed a forbidden path. Set by the dispatcher.
- `submodule-dirty` — `git submodule status --recursive` reported any non-clean entry. Set by the dispatcher.
- `branch-changed` — current branch differs from spawn-time branch. Set by the dispatcher.
- `dirty-state-after-timeout` — Codex timed out and the dispatcher refused to retry because the working tree / index was dirty. Set by the dispatcher.
- `wrapper-validation-failure` — the Step 2 launcher wrapper exited 2 before producing a valid implementer result (missing or invalid wrapper flags, path validation failure, or equivalent wrapper-side validation). The dispatcher does not retry this class because the invocation contract, not the external model runtime, failed. Set by the dispatcher.
- `qa-pending-missing` — Codex emitted `status=needs_qa` but `qa-pending.json` is missing, empty, or its `questions` array is missing/empty. Set by the dispatcher.
- `architectural-acknowledgment-missing` — architectural knowledge was supplied at launch time, but a `complete` or `needs_qa` manifest omitted a non-empty `architectural_acknowledgment`. This is a hard bail, not recoverable `manifest-schema-invalid` fallback. Set by the dispatcher.
- `redactor-not-executable`: legacy compatibility token retained for a redaction-boundary availability refusal. The dispatcher now uses the in-process Rust redactor behind `scripts/larch.sh redact secrets` and still fails closed rather than emit unsanitized text. Set by the dispatcher.
- `codex-runtime-failure` — launcher returned non-zero exit code or no manifest written, and the bounded retry also failed. **Carve-out (issue #3383)**: a non-zero `LAUNCHER_EXIT` does NOT bail when the on-disk `manifest.json` parses as `schema_version "1"` / `status "complete"`; the dispatcher salvages that complete manifest (continuing to schema validation + the dispatcher commit) and annotates the run with `WARN_CODEX_NONZERO_EXIT=true` instead. This covers Codex finishing the work and writing the manifest, then a self-verification step exiting non-zero. A non-zero exit with no manifest, or with a non-`complete` manifest, still bails here.
- `cursor-runtime-failure` — Cursor launcher returned non-zero exit code or no manifest written, and the bounded retry also failed. The Codex complete-manifest salvage carve-out above is intentionally not applied to Cursor (classified launcher-parity asymmetry; see `G-Ext-1`, `G-Wire-1`, and `skills/implement/references/step2-dispatch.md`).
- `cursor-bailed-no-reason` — Cursor-authored `status=bailed` manifest did not provide a usable `bail_reason`, so the dispatcher substituted the Cursor-specific fallback token.
- `cursor-modified-history` — Cursor moved `HEAD` before the dispatcher could commit on Cursor's behalf. Set by the dispatcher, not by Cursor itself.
- `manifest-missing` — manifest file is absent or empty after Codex returned. Set by the dispatcher (defense-in-depth on top of `codex-runtime-failure`'s `MANIFEST_WRITTEN=false` path).
- `main-branch-prohibited` — `implement step2-dispatch` refuses to launch an external implementer when the spawn-time branch is `main` or `master`, `FORKED_TARGET` is not `true` (read from `$IMPLEMENT_TMPDIR/session-env.sh` when that file exists; otherwise treated as `false`), and the run is issue-anchored: either `$IMPLEMENT_TMPDIR/parent-issue.md` contains a non-empty `ISSUE_NUMBER=` value **or** `$IMPLEMENT_TMPDIR/session-env.sh` exists (file presence alone suffices; `ISSUE_NUMBER` need not be set in session-env). Set by the dispatcher. Offline harnesses with neither parent-issue nor session-env are unaffected.
- `detached-head-prohibited` — `implement step2-dispatch` refuses to launch an external implementer on an issue-anchored, non-fork run when the spawn-time symbolic branch name is empty (detached HEAD / not on a branch) or the legacy literal `HEAD` token is recorded in `step2-spawn-branch.txt`. Same `FORKED_TARGET` / issue-anchor predicates as `main-branch-prohibited`. Set by the dispatcher.
- `interactive-subprocess-unsupported` — Codex's plan requires a persistent interactive subprocess session pattern that the Codex CLI cannot reliably support (`exec_command` kept alive for `write_stdin` / `read_stdout`). Set by Codex itself per `agents/_implementer-base.md` Hard guard #9 (issue #2991). Operators inspect the plan and rephrase the affected step to use heredoc / pipe / input-file input before retrying.
- Free-form Codex-authored token — Codex MAY emit any string in `manifest.bail_reason`; the dispatcher preserves it verbatim in the canonical `manifest.json`. The orchestrator's `REASON=` stdout line is sanitized for KV-grammar safety only (whitespace and ASCII control characters collapsed to single spaces; capped at ~200 characters) so an adversarial or malformed bail token cannot break the orchestrator's stdout parser. Use this for genuine fatal errors Codex itself diagnoses (e.g., `unable-to-resolve-import-cycle`, `external-api-down`).

## Example: `complete` manifest

```json
{
  "schema_version": "1",
  "status": "complete",
  "files_touched": [
    {"path": "skills/foo/SKILL.md", "lines_added": 14, "lines_removed": 3},
    {"path": "scripts/foo-helper.sh", "lines_added": 42, "lines_removed": 0}
  ],
  "tests_added_or_modified": ["scripts/test-foo-helper.sh"],
  "summary_bullets": [
    "Add foo-helper.sh with deterministic stdout contract",
    "Wire helper into skills/foo/SKILL.md Step 3",
    "Cover helper with offline harness"
  ],
  "architectural_acknowledgment": "honoring G-Skill-1 for this change",
  "commit_message": "Add foo-helper.sh and wire it into /foo Step 3\n\nReplaces the inline awk block previously inlined in SKILL.md.",
  "difficulty": {"predicted_tier": "MODERATE", "confidence": "medium", "rationale": "Adds a helper and skill wiring with harness coverage."},
  "todos_left": [],
  "oos_observations": [],
  "bail_reason": "",
  "needs_qa": {"questions": []}
}
```

## Example: `needs_qa` manifest

```json
{
  "schema_version": "1",
  "status": "needs_qa",
  "files_touched": [],
  "tests_added_or_modified": [],
  "summary_bullets": [],
  "architectural_acknowledgment": "honoring no parsed invariant entries for this question",
  "commit_message": "",
  "todos_left": [],
  "oos_observations": [],
  "bail_reason": "",
  "needs_qa": {
    "questions": [
      {"id": "q1", "text": "Should the helper use jq -e or jq --exit-status (older jq versions)?"}
    ]
  }
}
```

The `qa-pending.json` companion file (also atomic-written) carries the same `questions` array in a flat shape:

```json
{"questions": [{"id": "q1", "text": "..."}]}
```

`qa-pending.json` is what the orchestrator reads to drive `AskUserQuestion`; the manifest's `needs_qa.questions` is informational redundancy for tooling that prefers a single file.

## Edit-in-sync

Any change to this schema MUST be paired with edits in:

- `scripts/larch.sh implement step2-dispatch` — dispatcher validation (`jq -e` filters).
- `skills/implement/prompts/codex-implementer.md` — Codex prompt's manifest-writing instructions.
- `skills/implement/prompts/cursor-implementer.md` — Cursor prompt's manifest-writing instructions.
- `skills/implement/SKILL.md` — Step 4 (commit verification), Step 9a (PR `## Summary`), Step 9a.1 (OOS pipeline) consumption blocks. Phase 1 (#3364) retired `/implement` Step 8a release notes; manifest `summary_bullets` feed PR summary / OOS only until `/release` owns release notes updates.
- The inline tests in `crates/larch-cli/src/implement_step2_commands.rs`: dispatcher and manifest fixtures.
