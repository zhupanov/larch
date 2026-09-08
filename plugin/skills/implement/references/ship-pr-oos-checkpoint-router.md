# Ship PR OOS checkpoint router

**Consumer**: /implement Step 8+ on NEXT_ACTION=oos-pipeline after the retained #7681 Rust security-sidecar branch is selected.
**Contract**: Owns the Step 8+ OOS checkpoint wrapper routing semantics and success bookkeeping contract.
**When to load**: **MANDATORY: READ ENTIRE FILE** only on the NEXT_ACTION=oos-pipeline branch, before invoking step-8-oos-checkpoint.sh and without assuming any prior OOS pipeline body ran.

## Security sidecar disposition

`security-oos-observations.md` is private-disposition material. Read `$IMPLEMENT_TMPDIR/security-oos-observations.md`, follow `${CLAUDE_PLUGIN_ROOT}/docs/security/workflow-trust-and-mutations.md` `## Security Findings in OOS Workflows` private disclosure with no public `/issue`, and clear the sidecar only after private disposition completes. Public `/issue` filing is forbidden on this branch. Checkpoint stall is expected until private security disposition clears the sidecar.

OOS issue cap enforcement applies only on the pre-driver `scripts/larch.sh oos file` Rust path for non-security OOS; this branch does not run cap enforcement or public issue batch emission.

`${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh implement step-8-oos-checkpoint` runs the Rust `oos disposition-checkpoint` command through the verified bootstrap, preserves the command's refusal artifact, appends a de-duplicated Tool Failures fallback when the command could not record one, owns success bookkeeping, and emits exactly one `NEXT_ACTION=` when routing succeeds. Its process rc is 0 whenever `NEXT_ACTION` is emitted. It returns non-zero only when no `NEXT_ACTION` is emitted. It never emits `OOS_CHECKPOINT_RC=0` with `NEXT_ACTION=stall`.

On disposition rc 0 and successful bookkeeping, it writes run-scoped `run-statistics.md`, stamps `steps_ran.step9a1=true`, atomically clears only the allowlisted `OOS_PENDING=false` state key, emits `OOS_CHECKPOINT_RC=0`, and emits `NEXT_ACTION=reship`. Filed count is the number of distinct issue URLs parsed from `larch-logs/implement/<RUN_ID>/oos-issues.ndjson`; an absent batch contributes zero.

On disposition rc 0 with stats, manifest-stamp, or state-patch failure, it best-effort stamps `steps_ran.step9a1=false`, leaves `OOS_PENDING` unchanged, emits non-zero `OOS_CHECKPOINT_RC`, and emits `NEXT_ACTION=stall`. On disposition rc 1, rc 2, rc 3 (private security sidecar pending), 126, 127, or other non-zero rc, it emits `NEXT_ACTION=stall`, writes no stats, and clears no state.

The checkpoint wrapper preserves non-empty child-written `oos-disposition-checkpoint.stderr.log` when captured stderr is empty. Child stdout is not forwarded on success.

OOS-checkpoint `stall` is distinct from post-driver `stall`: halt Step 8+ until the gap or bookkeeping failure is resolved. Do not continue to Step 16.
