# skills/design/scripts/check-plan-size.md

Mechanical plan-size detector for `/design` **Step 2b.5** (issue #2670). Runtime owner: `scripts/larch.sh plan check-size` (Rust: `crates/larch-cli/src/plan_quality_commands.rs`). Threshold semantics are normatively documented in [`skills/design/references/flags.md`](../references/flags.md).

## argv

- `--design-tmpdir DIR` (required): design session root; default plan path is `$DIR/plan.txt`.
- `--plan-file PATH` (optional): override plan path (must still satisfy the trailer contract).

## Input contract

Validates `$DESIGN_TMPDIR` via `validate_design_tmpdir` after the required-arg check, before reading `$DESIGN_TMPDIR/plan.txt`; failure maps to argv exit 3 (rc 2 remains reserved for `PLAN_SIZE_STATUS=missing-*`).

- Plan file MUST exist (otherwise exit **2**, `PLAN_SIZE_STATUS=missing-plan` on the contract stream — see **Exit codes**).
- The **final non-empty line** MUST match `scripts/larch.sh plan-review emit` grammar: the literal prefix `diff_lines:` followed by **exactly one ASCII space** and then ASCII digits only to end-of-line — same rule as `scripts/larch.sh plan-review emit` (`case "$last_line" in diff_lines:\ *)` + digit validation). Tabs, multiple spaces after the colon, or other whitespace variants are rejected so the helper never accepts a trailer `plan-review emit` would refuse.
- **Plan body line count (`PLAN_LINES`)** is the number of physical lines **before** that final non-empty trailer line (blank lines count; the trailer line itself is excluded), **minus** any recognized optional metadata trailer lines in the final contiguous metadata block immediately above `diff_lines:` (see below). Legacy plans without optional trailers keep the same `PLAN_LINES` as before.

### Optional metadata trailers (final block only)

Designers MAY append these lines in the **final contiguous metadata block** immediately **above** the required final `diff_lines: <N>` line (same strict grammar as `diff_lines:` — literal token, exactly one ASCII space, value to end-of-line):

| Trailer | Accepted full-line regex |
|---------|--------------------------|
| `diff_added: <N>` | `^diff_added: [0-9]+$` |
| `diff_deleted: <N>` | `^diff_deleted: [0-9]+$` |
| `mechanical_churn: true\|false` | `^mechanical_churn: (true\|false)$`; numeric legacy values are normalized to `true` for resilience |
| `oversize_override: operator` | `^oversize_override: operator$`; explicit override |

Parsing rules (implemented in `plan_quality.parse_optional_metadata`; CLI surface: `scripts/larch.sh plan optional-trailers`):

- Scan upward from the line above `diff_lines:`; the block contains only strict trailer lines matching the regexes above.
- Stop at the first line above `diff_lines:` that is **not** one of those regexes (including blank lines).
- Malformed trailer-looking lines are treated as absent and stop the block.
- `diff_added: 08` / `09` and `diff_deleted: 08` / `09` match the strict line regex but are rejected as absent metadata (same rule as optional-trailer snapshot/validate); threshold logic then falls back to legacy `diff_lines` when `diff_added` is absent.
- Duplicate keys inside the block: **last match in file order** wins (closest to `diff_lines:`).
- `mechanical_churn: false` is explicit no-downgrade; absent mechanical values normalize to `false`.
- Only lowercase `true` and `false` should be emitted for `mechanical_churn` values. Numeric legacy values such as `mechanical_churn: 35` normalize to `true` so a drafter estimate does not stall the run. Other present values, for example `mechanical_churn: TRUE`, exit **2** with `PLAN_SIZE_STATUS=invalid-mechanical-churn` before size-gate calculations.
- Threshold comparisons use decimal coercion on emitted `DIFF_ADDED` / `DIFF_LINES` values (e.g. `diff_added: 002001` trips at 2001).
- Malformed `oversize_override:` values stop the optional block.

## Output contract (`emit_kv` on FD 3)

Machine-readable lines use the CLI contract stream, matching `scripts/larch.sh plan-review emit` and the Rust plan-review command tests.

Emitted keys (exit **0** only):

| Key | Meaning |
|-----|---------|
| `PLAN_LINES` | Body lines excluding the final `diff_lines:` trailer and recognized optional metadata trailers above it |
| `DIFF_LINES` | Integer from the required final `diff_lines:` trailer |
| `DIFF_ADDED` | Integer from `diff_added:` when present in the final metadata block; empty string when absent |
| `DIFF_DELETED` | Integer from `diff_deleted:` when present; empty when absent (informational only — never a trigger) |
| `MECHANICAL_CHURN` | `true` or `false` from the final metadata block |
| `FIRM_HEADINGS` | Count of firm `### NEW:`, `### UPDATED:`, and `### REWRITTEN:` headings |
| `SURFACES_TOUCHED` | Distinct plan surfaces derived from firm heading paths |
| `OVERSIZE_OVERRIDE` | `operator` when the override trailer is present; empty otherwise |
| `SOFT_ADVISORY` | `true` when `mechanical_churn: true` downgraded a diff-side hard trigger or an override suppressed a hard trigger; `false` otherwise |
| `SIZE_TRIGGER_FIRED` | `true` or `false` |
| `TRIGGER_REASONS` | Comma-separated tokens in fixed priority order: `plan-body-lines`, diff reason, `firm-headings`, `surfaces`. Empty when no hard crossing. |
| `PLAN_SIZE_STATUS` | `ok` on successful parsing and evaluation |
| `DRIFT_TRIGGER_FIRED` | `true` when the current plan-body line count or diff count exceeds the write-once baseline by more than `LARCH_DESIGN_DRIFT_MULTIPLE`; otherwise `false` |
| `DRIFT_MULTIPLE` | Positive integer multiple used for drift comparison; invalid env values fall back to `2` |
| `DRIFT_PLAN_RATIO`, `DRIFT_DIFF_RATIO` | Current-to-baseline ratios, with `inf` when a zero baseline grows above zero |
| `BASELINE_PLAN_LINES`, `BASELINE_DIFF_LINES` | Baseline counts used for drift comparison |

**Threshold semantics** (strict `>` — equality does not trip):

- Plan body: `PLAN_LINES > 800`.
- Diff (new-style): `diff_added > 2000` when the `diff_added:` trailer is present in the final metadata block.
- Diff (legacy fallback): `diff_lines > 1500` when `diff_added` is absent.
- Firm heading count: `FIRM_HEADINGS > 25`, excluding `### MAY_UPDATE:`.
- Surfaces: `SURFACES_TOUCHED > 4`. Under `crates/<crate>/`, group by `src/<top-module>`, direct `src/<file-stem>`, or `tests`; group other paths by first segment.
- Deletions never trip; `diff_deleted` is informational only.
- `mechanical_churn: true` suppresses the diff hard trigger and sets `SOFT_ADVISORY=true` when a diff trigger would have fired; plan-body hard triggers are unaffected.
- `oversize_override: operator` forces `SIZE_TRIGGER_FIRED=false` while keeping `TRIGGER_REASONS` visible.

## Drift baseline

`$DESIGN_TMPDIR/drift-baseline.env` is write-once. When absent, the first successful parse writes `BASELINE_PLAN_LINES=<PLAN_LINES>` and `BASELINE_DIFF_LINES=<DIFF_LINES>` and emits `DRIFT_TRIGGER_FIRED=false` for that call. Later calls compare both axes and fire drift when **either** `PLAN_LINES` or `DIFF_LINES` is strictly greater than its baseline multiplied by `LARCH_DESIGN_DRIFT_MULTIPLE`.

If the baseline file is a symlink, missing either key, or contains a non-integer value for either key, the helper emits a `WARN`, records an unreadable-baseline marker under `$DESIGN_TMPDIR/.drift-baseline-unreadable`, and **fails closed** with `DRIFT_TRIGGER_FIRED=true` for that call. It attempts recovery from `$DESIGN_TMPDIR/plan.txt-original` when present; on successful recovery it rewrites `drift-baseline.env` from the recovered anchor and compares drift normally. When recovery fails, `BASELINE_PLAN_LINES` and `BASELINE_DIFF_LINES` are empty and both drift ratios are `inf`. It does not partially trust one key while replacing the other with the current plan size, because that would silently disable one drift axis. A later call while the unreadable marker is set does not re-seed the anchor from the current plan size; it repeats fail-closed handling until the operator repairs or removes the marker (for example by fixing `drift-baseline.env`). Zero baselines are valid: `0 → 0` has ratio `1`; `0 → positive` has ratio `inf` and exceeds any positive multiple. Drift is an advisory signal logged by merged drivers (`design-postplan-emit.sh`); it does not block with an operator prompt.

## Exit codes

| rc | Meaning |
|----|---------|
| 0 | Valid plan; KV lines emitted as above, including `PLAN_SIZE_STATUS=ok` |
| 2 | Missing plan file → `PLAN_SIZE_STATUS=missing-plan`; or missing/malformed trailer → `PLAN_SIZE_STATUS=missing-diff-lines`; or invalid `mechanical_churn:` → `PLAN_SIZE_STATUS=invalid-mechanical-churn` |
| 3 | Invocation / argv error (e.g. missing `--design-tmpdir`, unknown flag) — stderr only; **no** `PLAN_SIZE_STATUS` on the contract stream |

## Callers

- **Merged**: `design-postplan-emit.sh --with-plan-size` (initial Step 2b, Gate B, discussion-round2 / Gate A after-discussion).
- **Retained**: `SKILL.md` Step 2b.5 procedure (Override-after-defects).

**Site-aware retained hard prompts**: initial/discussion Step 2b.5, Gate B, and retained Step 2b.5 all use Split/Override/Cancel. Override writes `oversize_override: operator` to `plan.txt` and deletes stale `composed-plan.md`.

Merged mode treats check-size rc 2/3 nonfatally in the driver. The Rust plan-review owner preserves that malformed-check warning behavior, routes hard-size postplan rc 12 back to Gate B, and gates `partition_requested` handoff on plan-size rc=0.

## Edit in sync

Update config, `crates/larch-cli/src/plan_quality_commands.rs`, publish/compose consumers, trailer scanners, tests, `docs/issue-anchored-plan.md`, this file, `flags.md`, and `SKILL.md` Step 2b / 2b.5 when changing thresholds or optional-trailer contracts.
