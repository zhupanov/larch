# Citation Validation Phase Reference

**Consumer**: `/research` Step 2.5 — loaded via the `MANDATORY: READ ENTIRE FILE` directive at Step 2.5 entry in SKILL.md.

**Contract**: citation-credibility check. Runs unconditionally on every `/research` invocation that produced a `research-report.txt`, executing between Step 2 (validation) and Step 2.6 (critique loop). The phase reads the validated synthesis at `$RESEARCH_TMPDIR/research-report.txt`, extracts cited provenance (file:line, URL, DOI), validates each unique URL and DOI in parallel under the SSRF guards implemented in `crates/larch-cli/src/research_commands.rs` (HTTPS only, no proxy environment variables, no normal URL redirects, RFC1918/IPv6 link-local/RFC6598 hostname pre-rejection including carrier-grade NAT `100.64.0.0/10`, DNS resolved-IP private-range check, connection pinning to the checked public IP, and global budget cancellation of in-flight fetch workers), classifies domain credibility heuristically (advisory only — never flips PASS to FAIL), validates DOIs syntactically and via `HEAD https://doi.org/<doi>` under the same SSRF rules, spot-checks file:line existence + line-range against `git rev-parse --show-toplevel` with `realpath` canonical-path containment check, and writes a 3-state per-claim ledger (`PASS` / `FAIL` / `UNKNOWN` with reason classifier on `UNKNOWN`) to `$RESEARCH_TMPDIR/citation-validation.md` (sidecar). Step 3 splices the sidecar as a `## Citation Validation` section into `research-report-final.md` after the standard report block. **Fail-soft**: per-claim failures surface as warnings only; the validator command always exits 0 on validation paths (exit 2 only for argument/flag errors); Step 3 is never blocked.

**When to load**: once Step 2.5 is about to execute. Do NOT load during Step 0, Step 1, Step 2, Step 2.5, Step 3, or Step 4. SKILL.md emits the Step 2.5 entry breadcrumb; this file does NOT emit it — it owns body content only.

---

<!-- step:2.5 — Citation Validation -->

**IMPORTANT: Citation validation runs unconditionally when a synthesis exists. The phase is fail-soft: every per-claim failure is recorded in the sidecar; the validator exits 0 on validation paths (exit 2 only for argument/flag errors); Step 3 never blocks on this phase. Domain credibility is advisory only — it never flips a `PASS` to `FAIL`.**

## 2.5.1 — Skip preconditions (input gate)

**Empty-synthesis gate.** If `$RESEARCH_TMPDIR/research-report.txt` does not exist OR is empty (zero bytes), skip Step 2.5 entirely and proceed to Step 3. Print:

```
⏩ 2.5: citation-validation — skipped (no synthesis to validate) (<elapsed>)
```

The empty-synthesis path is reachable when Step 1 inline-fallback synthesis failed and produced no body.

## 2.5.2 — Invoke the validator

Invoke the Rust validator (it owns argv, SSRF, DNS, TLS, regex, and sidecar contracts):

```bash
"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" research validate-citations \
  --report "$RESEARCH_TMPDIR/research-report.txt" \
  --output "$RESEARCH_TMPDIR/citation-validation.md" \
  --tmpdir "$RESEARCH_TMPDIR"
```

The Rust command writes the sidecar to the path passed via `--output` and exits 0 on validation paths. Usage and bad flags exit 2; degraded argument cases still write a minimally-formed sidecar and `SUMMARY=PASS=0 FAIL=0 UNKNOWN=0 TOTAL=0` so Step 3's splice consumer has a sidecar to read.

See `crates/larch-cli/src/research_commands.rs` and `crates/larch-cli/src/research_commands/citations.rs` for the full contract: argv, exit codes, sidecar schema, SSRF defenses, regex tiers, idempotency rerun semantics, bounded DNS, HTTPS fetching, no proxy environment use, redirect handling, TLS verification, connection pinning, and budget cancellation.

## 2.5.3 — Sidecar schema

The sidecar is an operator-readable Markdown document. The single source of truth lives in `crates/larch-cli/src/research_commands/citations.rs`; the structural shape is:

```markdown
## Citation Validation

**Validator**: validate-citations.sh v1
**Synthesis**: <byte-count> bytes, <line-count> lines
**Claims extracted**: <total>
**Status counts**: <pass> PASS · <fail> FAIL · <unknown> UNKNOWN

| Claim | Type | Status | Reason | Cited by |
|---|---|---|---|---|
| `<excerpt>` | url | PASS |  |  |
| `<excerpt>` | doi | UNKNOWN | head-not-supported |  |
| `<excerpt>` | file-line | FAIL | line-out-of-range |  |

<details><summary>Domain credibility (advisory only)</summary>

| Domain | Tier | Notes |
|---|---|---|
| ...  | allow | well-known reputable origin |
| ...  | unknown | no allow-list entry; classification heuristic only — NOT a FAIL signal |

</details>
```

`Status` is one of `PASS` / `FAIL` / `UNKNOWN`. `Reason` is empty on `PASS` and a short token on `FAIL` / `UNKNOWN` per the reason vocabulary in `crates/larch-cli/src/research_commands.rs` (`network-error`, `timeout`, `head-not-supported`, `redirect-not-followed`, `ssrf-private-host`, `ssrf-private-resolved`, `git-root-unavailable`, `file-not-found`, `line-out-of-range`, `doi-syntax`, `doi-unresolved`, and related file-line tokens). URL and DOI claims are deduplicated — a single fetch produces one ledger row. The `Cited by` column is reserved for a future enhancement that will list every claim-index reference (`claim-<N>` matching the synthesis-walk index); v1 of the validator emits an empty `Cited by` cell while preserving the 1:1 fetch-to-row contract. Operators inspecting the sidecar can grep the synthesis directly for now.

## 2.5.4 — Idempotency rerun

The sidecar path (`--output`) is overwritten on every invocation. Two consecutive runs against an unchanged synthesis MUST produce byte-identical sidecars (deterministic stdout ordering, no timestamps in the body — the audit-context line is captured externally by the orchestrator's prelude prints). Operators can re-invoke the validator against the same `$RESEARCH_TMPDIR/research-report.txt` to re-validate after a transient network failure without polluting the audit trail.

## 2.5.5 — Failure surfaces

Per-claim failures are written into the sidecar's `Status` column. The orchestrator does NOT print per-claim failures to stdout; instead, Step 2.5 records this summary:

```
Record citation-validation summary: `<pass> PASS, <fail> FAIL, <unknown> UNKNOWN (<total> claims)`.
```

When `<fail> > 0` OR `<unknown> > 0`, ALSO print one of these advisory warnings (not errors — fail-soft contract):

- `<fail> > 0`: `**⚠ 2.5: citation-validation — <fail> claim(s) FAILED. See ## Citation Validation in the report.**`
- `<unknown> > 0` (regardless of `<fail>`): `**ℹ 2.5: citation-validation — <unknown> claim(s) UNKNOWN. Common reasons: HEAD not supported (try GET manually), DNS resolution unavailable, git tree not detected. See ## Citation Validation in the report.**`

The script's stdout summary (parsed by the orchestrator from the validator's last line `SUMMARY=PASS=<n> FAIL=<n> UNKNOWN=<n> TOTAL=<n>`) drives the conditional warnings.

## 2.5.6 — Step 3 splice contract

Step 3 (final-report write) is the sole consumer of the sidecar. After writing the report block to `research-report-final.md` and BEFORE the helper-driven sidecar generation (`scripts/larch.sh research render-findings-batch`), Step 3:

1. Checks `$RESEARCH_TMPDIR/citation-validation.md` exists and is non-empty.
2. Appends the sidecar's full content to `research-report-final.md` with a single blank line separator. The sidecar already opens with `## Citation Validation` so no extra header is added.
3. On missing or empty sidecar (Step 2.5 was skipped per § 2.5.1): no splice, no warning. The skip breadcrumb at Step 2.5 already informed the operator.

The splice happens BEFORE `cat`-ing the report for user-visible output, so the final report displayed to the operator includes the citation-validation section.

## Why a separate phase, not a 6th Step 2 reviewer

Per the design discussion on issue #516 DECISION_1 (resolved via the plan-review panel's 2-1 sidecar vote, user-confirmed at Step 3.5 round 2), Step 2.5 is a separate phase that writes a sidecar — NOT a 6th reviewer added to Step 2's validation panel. Phase separation:

1. Keeps Step 2's voting machinery focused on the synthesis content and accept/reject votes; citation validation has no vote — it is mechanical.
2. Lets the validator be deterministic Rust with no LLM call, costing zero measurable Claude tokens (parallel to Step 0.5's classifier).
3. Keeps the validator failure mode local — a transient network failure during URL HEAD-fetch surfaces as `UNKNOWN(network-error)` rows in the sidecar, NOT as a vote-skewing reviewer fallback.

## Failure modes and fail-soft posture

The validator script always exits 0 on validation paths; exit 2 only for argument/flag errors. Failure modes that would otherwise abort a strict validator are reclassified into `UNKNOWN` reasons in the per-claim ledger:

| Failure mode | Sidecar reason |
|---|---|
| `git rev-parse --show-toplevel` fails (not a git tree) | `UNKNOWN(git-root-unavailable)` for every file:line claim |
| Hostname pre-rejected by RFC1918/IPv6 link-local/RFC6598 rules | `FAIL(ssrf-private-host)` |
| DNS resolves to a private IP range | `FAIL(ssrf-private-resolved)` |
| Multi-answer DNS where ANY answer is private (rebinding defense) | `FAIL(ssrf-private-resolved)` |
| HEAD returns 4xx/5xx that does not indicate non-support (e.g., 404, 410) | `FAIL(head-not-found)` for 404/410; `FAIL(head-server-error)` for ≥500 |
| HEAD returns 403/405/501 | `UNKNOWN(head-not-supported)` (some servers reject HEAD; an optional constrained GET retry MAY upgrade to PASS — see `crates/larch-cli/src/research_commands.rs`) |
| HEAD 2xx response inside per-fetch timeout window | `PASS` |
| HEAD 3xx response inside per-fetch timeout window | `UNKNOWN(redirect-not-followed)` (redirect destination not fetched; `--max-redirs 0`) |
| HEAD response after timeout (per-claim or overall budget) | `UNKNOWN(timeout)` |
| Realpath escape (`..`-traversal or symlink-escape outside repo root) | `UNKNOWN(out-of-tree-path-after-realpath)` |
| Broken symlink on the resolved path | `UNKNOWN(broken-symlink)` |
| File exists but the cited line range exceeds the file length | `FAIL(line-out-of-range)` |
| File exists, line range valid, but range is empty (start > end) | `FAIL(line-range-empty)` |
| DOI fails syntactic validation (e.g., not `10.NNNN/...`) | `FAIL(doi-syntax)` |
| DOI is syntactically valid but doi.org HEAD does not resolve to a permanent URL | `UNKNOWN(doi-unresolved)` (a 3xx HEAD on `https://doi.org/<doi>` IS the registry's success signal — the DOI path interprets `UNKNOWN(redirect-not-followed)` as PASS, not as `doi-unresolved`) |

The `UNKNOWN` bucket is deliberately broad: every transient or environment-dependent failure ends there so the validator's strictness scales with the operator's environment without false negatives skewing the audit.

## Budget and network contract

When the overall validator budget elapses (`--budget-seconds`, default 300), in-flight URL and DOI work is canceled where possible and every claim without a result is backfilled as `UNKNOWN(timeout)`. The command still writes the sidecar, emits exactly one trailing `SUMMARY=...` line, and exits 0 on validation paths. Inline tests in `crates/larch-cli/src/research_commands.rs` cover this through injected fetcher seams instead of real sleeps or network calls.

The Rust HTTPS implementation uses default CA verification. It does not honor proxy environment variables, does not follow normal URL redirects, treats DOI redirects as success, fails closed on private host literals and private resolved IPs, preserves the original hostname for TLS SNI and hostname verification, preserves the original `Host` header, and connects to the checked public IP when pinning is available.

## Domain-credibility heuristic (advisory only)

A small allow-list of widely-recognized reputable hosts (e.g., `*.wikipedia.org`, `*.arxiv.org`, `*.acm.org`, `*.ietf.org`, `doi.org`, `github.com`, `*.python.org`, `*.rust-lang.org`) tags matching domains as `allow` in the credibility table. Other domains are tagged `unknown`. The credibility tier NEVER flips a claim's primary status (`PASS` stays `PASS` even for an `unknown` domain). The operator allow-list flag (`--trusted-domains=`) is deferred to issue #514 — that flag will, when shipped, expand the heuristic into operator-supplied policy.

## Step 2.5 → Step 3 control-flow summary

```
2.5 entry breadcrumb (SKILL.md)
  → § 2.5.1 input gates (evaluated in order):
      empty-synthesis gate → skip 2.5 → Step 3
    → § 2.5.2 validator invocation (exits 0 on validation paths; exit 2 only on argument errors)
    → § 2.5.5 conditional advisory warnings
  → Step 3 splice (§ 2.5.6) appends sidecar to research-report-final.md
  → Step 3 cat displays the spliced report to stdout
```
