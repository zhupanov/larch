# Complete Umbrella Recovery

Use the same command to start or recover a managed umbrella:

```text
/complete-umbrella <umbrella-issue-N>
```

Do not remove `[IMPLEMENTING]` from a leaf by hand. The command checks for a
durable run pointer before it starts a new run. Recovery keeps the original
session tmpdir, including every `complete-umbrella-leaf-<N>/` handoff file.

## Sleep and offline behavior

A sleeping laptop may leave the whole-loop bgjob and its leaf child alive. A
lost Claude session does not own the only recovery state. The pointer under
`~/.cache/larch/sessions/` records the repository, umbrella, tmpdir, current
leaf and step, transient-attempt count, and bgjob step.

After the laptop wakes, run `/complete-umbrella <N>` once. The resume owner:

- rebinds a live bgjob wait lease to the current session and returns to the
  same wait;
- consumes an identity-bound durable child result through the existing retry
  or failure route, including `transient-api` and `incomplete-envelope-ship`
  (an incomplete child envelope that still left durable ship progress with a
  positive `PR_NUMBER`);
- treats a present durable child result with no `CHILD_ISSUE` as an interrupted
  write, then reconciles the pointer leaf against GitHub. A closed `[DONE]`
  leaf advances selection. An open in-flight leaf follows the existing reset
  and reselection route;
- ignores the immediately prior transient result after its next-attempt
  checkpoint has already been persisted, so a wake cannot consume one retry
  twice;
- resets an open leaf's stale `[IMPLEMENTING]` prefix when no live job remains,
  then selects it again without replacing its handoff root.

An offline wake can still make the GitHub validation or title reset fail. Wait
until connectivity returns, then run the same command again. Recovery does not
loosen remote read-back or mutation checks.

## Fail-closed cases

Recovery stops without choosing a candidate when it finds multiple matching
pointers, a missing tmpdir, a repository mismatch, malformed pointer state, or
an unsafe path. Keep the pointer and tmpdir intact while investigating.

The normal success path removes the pointer after the parent closes. A
terminal hard failure removes it only after lifecycle diagnostics are written.
Session cleanup treats a valid complete-umbrella pointer as an active tmpdir
reference, so age cleanup does not discard recoverable handoffs.

## Harness false-denies

Recovery assumes the verified runtime entrypoint can run. Two co-installed
production-guard failures can deny an unrelated larch driver before execution:

- `smarts` versions before v2.0.3 can misclassify Cursor `Shell` input and match
  the short PagerDuty marker `pd` inside `--tmpdir`. Version v2.0.3 bounds that
  classifier path and marker.
- A current guard classifier can exit nonzero and return `guard is unavailable`.
  The pagerduty, hyperdx, changes, and log-evidence guards match both Claude
  Code `Bash` and Cursor `Shell`, so this transient is host-agnostic. Its
  upstream owner is [character-tech/smarts#909](https://github.com/character-tech/smarts/issues/909).

For the exact transient `guard is unavailable` shape, repeat the identical
denied workflow-driver command once. Claude Code has no
`request_smart_mode_approval` API, and Cursor approval cannot override a
PreToolUse `permissionDecision: deny`, so do not request approval on either
host. If that one retry is denied, or the first denial is a positive policy
decision such as `not approved` or `use the bounded packaged reader`, hard-fail
through the ordinary Failure rule. Attempt each remaining lifecycle diagnostic
and pointer-cleanup command once, with no guard retry. If a guard denies one,
preserve any pointer and the session tmpdir. Report the unexecuted postcondition
without claiming terminal success.

Upgrade older `smarts` installs before retrying the workflow. For a current
install, repair or disable the failing co-installed guards, report a regression
upstream, or run `/complete-umbrella` in a session without those guards. The
same `/complete-umbrella <N>` command can then recover retained state. Do not
hand-edit lifecycle titles, invent an alternate shell entrypoint, or rephrase
the driver as `gh`, curl, or wget. See the canonical
[co-installed PreToolUse gate contract](security/workflow-trust-and-mutations.md#co-installed-pretooluse-gates).

When `bgjob wait` reports `BGJOB_STATUS=DEAD`, it may also report the mutually
exclusive `DAEMON_EXIT` or `DAEMON_SIGNAL` and separate `STDOUT_TAIL` and
`STDERR_TAIL` values. Empty termination fields mean the supervisor evidence was
unavailable, not that the daemon exited cleanly. The tails are bounded and
redacted operator diagnostics. Keep the raw session logs private.

## Diagnostic helper

The skill calls the Rust helper through the verified runtime entrypoint. A
maintainer can inspect the same route directly from the target repository:

```bash
"${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh" complete-umbrella resume \
  --repository OWNER/REPO \
  --issue N \
  --claude-pid "$PPID" \
  --operator-invoked
```

`RESUME_ACTION=wait` means re-enter the documented `bgjob wait` command.
`RESUME_ACTION=reselect` means launch the same whole-loop step against the
returned `COMPLETE_UMBRELLA_TMPDIR`. `needs-design` and `failed` remain terminal
for that invocation and follow the ordinary diagnostic cleanup path.
