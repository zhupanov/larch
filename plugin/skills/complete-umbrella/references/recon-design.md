# Recon and Design Phase

**Consumer**: The first fresh general-purpose Agent spawned by the `/complete-umbrella` leaf orchestrator.

**Contract**: Adopt the leaf lifecycle, gather bounded requirements and repository evidence, and write the implementation-ready design brief without returning large content.

**When to load**: **MANDATORY: READ ENTIRE FILE** only for the primary recon-design phase.

Read `phase-common.md` in this directory in full before acting.

The spawn prompt supplies `REPOSITORY`, `UMBRELLA`, `LEAF`, `REPO_ROOT`, and `HANDOFF_ROOT`. Require positive numeric issue IDs, exact `OWNER/REPO` syntax, the current working directory as `REPO_ROOT`, and `HANDOFF_ROOT=$SESSION_TMPDIR`.

The recon/design phase owns leaf actionability. The prepare driver only starts
the mutation state after this phase has produced its handoffs. It does not
validate a durable plan. Then:

1. Read `ARCHITECTURAL_INVARIANTS.md` and `ARCHITECTURAL_GUIDELINES.md` when present.
   `AGENTS.md` is already loaded through the `CLAUDE.md` import chain. Do not
   read it again. Follow all repository rules.
2. Fetch the full leaf and umbrella issue bodies into `leaf-issue.md` and `umbrella-issue.md` below `$SESSION_TMPDIR`. Redirect the `gh issue view` output to those files. Do not return issue text in tool output.
3. Read both issue files in full. Route to `needs-design` only when the existing
   plan block is malformed as described in Step 5, or when the leaf body is
   totally unactionable. A body is actionable when it contains any discernible
   requested outcome, requirement, implementation task, or acceptance
   criterion. It does not need a durable plan or every one of those fields. An
   absent plan block, M1/M2 grammar gaps, uncertainty, cross-leaf ordering,
   atomic-cutover concerns, integration risk, and competing implementation
   choices are not `needs-design` reasons. Resolve those concerns with the
   narrowest evidence-based decision, preserve backward compatibility when
   practical, and follow an umbrella-level atomic-cutover rule when one exists.
   Inspect relevant precedent pull requests and the target source. Use no more
   than five precedent PRs.
4. Inspect only enough repository context to identify the implementation. Batch independent `Read`, `Grep`, and `Glob` calls.
5. Materialize any existing durable plan before drafting a replacement:

```bash
"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" plan-block read \
  --issue "<LEAF>" \
  --output "$SESSION_TMPDIR/plan.md" \
  --repo "<REPOSITORY>" \
  >"$SESSION_TMPDIR/plan-read.env"
```

Require command success and exactly one `BLOCK_PRESENT=true|false`. If the
command instead fails with exactly one `MALFORMED=<reason>` row, write the
`needs-design` handoff described below and end without calling prepare. Any
other command failure is unrecoverable. When `BLOCK_PRESENT=true`, preserve the
extracted plan exactly. Do not replace or republish it. This includes a plan
published by a prior full `/design` run.

6. Write `$SESSION_TMPDIR/design-brief.md`. Include requirements, relevant architectural rules, file-and-line anchors, exact code and test surfaces, generated or projected companions, stale callers to sweep, local checks, and a parity plan. If a differential harness is needed, require an assertion that proves a success path executed.
7. Only when `BLOCK_PRESENT=false`, write `$SESSION_TMPDIR/plan.md` as a
   concrete executable plan. Use the issue-anchored M1/M2 structure: firm file
   headings, ordered steps, closed ownership decisions, acceptance,
   breaking-change or migration treatment, and a terminal `diff_lines:` line.
   The plan records this autonomous decision. It is not evidence of an
   approval that did not occur, and its prior absence is not an admission
   failure. Do not stop or route to `needs-design` over an M1/M2 grammar gap;
   make the narrowest concrete plan the evidence supports. Publish exactly
   that file through the canonical wire owner:

```bash
"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" named-block write \
  --marker plan \
  --issue "<LEAF>" \
  --content-file "$SESSION_TMPDIR/plan.md" \
  --repo "<REPOSITORY>"
```

8. Run the standalone driver in prepare mode:

```bash
"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" complete-umbrella ship-leaf \
  --mode prepare \
  --repository "<REPOSITORY>" \
  --repo-root "$PWD" \
  --handoff-root "$SESSION_TMPDIR" \
  --umbrella "<UMBRELLA>" \
  --leaf "<LEAF>"
```

On `SHIP_STATUS=prepared`, require the driver to have bound the
`[IMPLEMENTING]` change to the live leaf snapshot; it changes no other title
bytes. Do not echo `SHIP_STATUS` or any prepare-driver output in your final
response.

If Step 3 found a totally unactionable body, or Step 5 reported a malformed
existing durable plan block, write `$SESSION_TMPDIR/needs-design.md` with one
short line naming the leaf, the eligible reason, and the required command
`/design <LEAF>`. These are the only `needs-design` routes. They occur before
the prepare driver adds an active title or writes ship state. The parent may
later clear a stale active title from an older run. Do not implement, review,
or ship. End with only:

```text
PHASE_STATUS=needs-design
HANDOFF_FILE=needs-design.md
```

Any other prepare status is an unrecoverable failure.

Keep the brief concrete. Do not copy issue bodies into it. The next phase must be able to implement from the brief and `leaf-issue.md` without broad exploration.

End with only:

```text
PHASE_STATUS=complete
HANDOFF_FILE=design-brief.md
```
