# Adversarial audit prompt

**Consumer**: `/audit-umbrella` after it has written a fresh immutable snapshot.

**Contract**: Replace `<UMBRELLA>` and `<AUDITED_SHA>` with the validated values, then use the fenced text below verbatim as the audit judgment prompt.

**When to load**: Load only after the command has validated the target and emitted its immutable snapshot binding.

```text
Adversarially audit umbrella #<UMBRELLA> for the actual correctness and completeness of its implemented specifications at commit <AUDITED_SHA>. Ignore issue state, dependency structure, closed leaves, merged PRs, and passing test names as evidence of correctness. Independently trace every requirement and acceptance criterion from the umbrella, every historical leaf, and every explicitly controlling specification through production code, observable behavior, documentation, and the assertions and fault points in tests.

First build a complete requirement-to-code-to-test ledger. Assign every normative source item a stable identifier. Do not sample. Do not begin gap partitioning until every item is marked satisfied with concrete evidence, gap with concrete evidence, not applicable with a reason, or blocked from verification with a reason. An unresolved or omitted item means the audit is incomplete.

For every applicable behavior, inspect the happy path and all failure, cancellation, signal, crash, restart, retry, recovery, concurrency, race, atomicity, durability, ownership-transfer, TOCTOU, path-confinement, symlink, process-identity, teardown, trust-boundary, redaction, publication, compatibility, platform, and byte-level wire-contract paths. Read the actual test setup and assertions. A test name, prior audit claim, or successful test run does not prove that the relevant boundary is exercised. Run bounded targeted checks where they materially improve the evidence.

Finish the whole audit before proposing issues. Search for integration gaps, incomplete acceptance criteria, contradictions, unsafe compositions, and regressions introduced across leaves. Deduplicate against all existing leaves and among the findings. Do not stop after finding major gaps, and do not omit small correctness or completeness gaps.

If residual gaps exist, produce the smallest exhaustive batch of independently implementable corrective leaves. Group gaps only when they share a root cause, implementation owner, transaction boundary, and verification strategy and can land as one focused leaf within the umbrella's size convention. Split them when ownership, trust boundary, prerequisite order, or deployability differs. Minimizing issue count never permits skipped scope or a bloated leaf.

For each proposed leaf, provide current implementation evidence, bounded scope, testable acceptance criteria, and its exact prerequisites. Then define the complete acyclic dependency graph. The umbrella must be blocked by every leaf; apply's native-graph phase owns those umbrella<-leaf edges, so declare only genuine leaf-to-leaf (or other non-umbrella) prerequisites. Every old and new leaf must be a direct native sub-issue. Every real prerequisite must use native GitHub blocked-by in the direction dependent <- prerequisite. Add no convenience-only serialization, and keep every genuine root unblocked.

If your judgment identifies an actual vulnerability or live secret, keep it private and follow SECURITY.md. Security-related words alone do not stop the audit. Do not implement fixes or close the umbrella. Report completeness only when every ledger row is resolved and the final GitHub graph has been read back and verified.
```
