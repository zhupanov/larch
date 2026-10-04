# Plan Review Maintainer Reference

**Consumer**: maintainers editing the Step 3 review surface.

**Contract**: editing-only authority for producer ownership, harness inventory, byte-preserved templates, and prompt-source maintenance. Runtime orchestration reads `plan-review-runtime.md`, not this file.

**When to load**: only while editing or maintaining plan-review prompts, renderers, tests, or topology. Do not load during `/design` Steps 0 through 5.

The runtime contracts, slot identities, returned-artifact interpretation, fallback adjudication, and panel tiers live in `plan-review-runtime.md`. Rust producer internals live in `crates/larch-cli/src/plan_review_commands.rs`; prompt bodies are rendered by `scripts/larch.sh render plan-review` and `scripts/larch.sh render voter`. Preserve the accepted finding and OOS templates in `plan-review-runtime.md` byte-for-byte when changing their producers or tests.

Harness authorities include `skills/design/scripts/test-step3-orchestrator-fence.sh`, `skills/design/scripts/test-step3-review-cap.sh`, `crates/larch-cli/tests/plan_review_mav_commands.rs`, and `crates/larch-cli/tests/plan_review_loop_commands.rs`.

<!-- Retained migration inventory for agent-lint S030: test-step3-orchestrator-fence.sh test-step3-review-cap.sh -->
