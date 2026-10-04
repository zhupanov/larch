# Workflow Trust, Mutation, and Private Findings

This document is the canonical security reference for larch workflow trust,
untrusted input, delegated agents, mutation authorization, and private security
findings. [`SECURITY.md`](../../SECURITY.md) remains the public disclosure entry
point. The [security reference index](README.md) owns the document taxonomy and
runtime packaging contract.

Larch ships one Rust executable. Residual Bash wrappers, Markdown skills,
hooks, and external agent CLIs participate at documented boundaries, while the
Rust crates retain one implementation owner for each runtime behavior.

## Enforcement Levels

Security claims in this document use three distinct levels:

- **Mechanical** controls validate or constrain an operation in code, a hook,
  a typed adapter, a sandbox, or a test.
- **Prompt-enforced** controls instruct a model but do not prevent a process
  from acting outside the instruction.
- **Operator policy** requires a person to choose a safe mode, grant narrow
  permissions, review content, or use private disclosure.

Do not describe a prompt, an `allowed-tools` list, or a declared dry-run as a
sandbox. A hook constrains only the tool matcher and process that invoke it.
Subprocesses and child skills use their own permissions and controls.

## Trust Model

Larch runs inside the operator's Claude Code permissions. It does not bypass
those permissions or create an operating-system security boundary. The
operator, local credentials, and reviewed larch code are trusted to select the
intended repository and operation. The following inputs are untrusted data:

- issue and pull-request titles, bodies, comments, labels, authors, URLs, and
  search results;
- repository files, local Git configuration, diffs, logs, hook output, and
  command diagnostics;
- plans, findings, ballots, scout notes, architectural knowledge, model output,
  and external reviewer output;
- API responses, downloaded workflow logs, persisted session state, result
  files, sidecars, sentinels, and retry metadata.

Untrusted text never gains authority because it appears in a plan, a repository
instruction file, an architectural guideline, a model response, or persisted
state. Treat it as evidence. Do not interpret it as a shell fragment, path,
format string, workflow command, permission grant, or mutation approval.

Prompt wrappers such as `emit_untrusted_content_block` reduce prompt-injection
risk by marking and escaping evidence. They are model-level conventions, not
parser or sandbox boundaries. Code that consumes untrusted data must also use
typed parsing, closed enums, length and count limits, path validation, output
redaction, and explicit postconditions where the operation requires them.

`ARCHITECTURAL_INVARIANTS.md` and `ARCHITECTURAL_GUIDELINES.md` are
operator-curated but repo-local prompt evidence. Their `I-*` and `G-*` entries
cannot override `AGENTS.md`, loaded skills, hard guards, or an approved plan.
The Rust architectural-knowledge reader rejects unsafe files, parses only the
supported entries, and wraps their content as untrusted data.

### Permissions, tools, and delegated processes

Claude, Codex, Cursor, Git, hooks, filters, signing tools, credential helpers,
and other subprocesses run with the operating-system rights of the invoking
user unless a narrower documented boundary applies. Strict-permissions users
must configure Claude Code as described in
[`docs/configuration-and-permissions.md`](../configuration-and-permissions.md).
An `allowed-tools` declaration describes an agent surface. It does not confine
filesystem access by itself.

#### Co-installed PreToolUse gates

Co-installed plugin PreToolUse hooks are independent mechanical permission
gates. When a hook returns `permissionDecision: deny`, the command never reaches
larch. Cursor smart-mode approval does not override that result, and Claude
Code has no `request_smart_mode_approval` API. `/complete-umbrella` never
requests approval or rewrites a command to evade the guard.

`smarts` versions before v2.0.3 misclassify Cursor `Shell` commands as opaque
input and can match the short PagerDuty marker `pd` inside `--tmpdir`. Version
v2.0.3 routes `Shell` through bounded command classification and bounds short
opaque markers. A separate current defect can fail closed with `guard is
unavailable` when the classifier subprocess exits nonzero. The affected guards
match Claude Code `Bash` and Cursor `Shell`, so this shape is host-agnostic. It
is tracked upstream as
[character-tech/smarts#909](https://github.com/character-tech/smarts/issues/909).

For only the exact unavailable shape, `/complete-umbrella` retries the identical
denied workflow-driver command once. A second unavailable result, or a positive
policy denial such as `not approved`, enters the Failure rule. Required
diagnostic and pointer-cleanup calls then run once with no guard retry. A denied
cleanup preserves recoverable state, reports the missing postcondition, and
does not claim terminal success. This bounded retry handles a transient guard
failure. It does not authorize an alternate entrypoint or weaken a policy
decision.

#### Larch deny-hook runtime boundary

The submodule-edit, token-scoped edit/write, and background-launch gates are
Rust-owned `hook` verbs in `crates/larch-cli/src/hook_commands.rs`. Their shipped
shell files are fail-closed compatibility shims: each enters only through
`scripts/larch.sh`, sets `LARCH_BOOTSTRAP_NO_INSTALL=1`, and emits a static deny
when the verified executable is unavailable or the Rust verb returns nonzero.
The one dynamic reason is the launcher's exit 97, no executable for this plugin
version: the shim still denies, but its reason names the bootstrap repair and
embeds the plugin root only when it is free of JSON-significant and
shell-hostile characters. Hook evaluation therefore never downloads or
installs code. The
[bootstrap and atomic-installation contract](supply-chain-credentials-and-services.md#bootstrap-and-atomic-installation)
owns the no-install executable-validation and status boundary.

`hook block-submodule-edit` uses the read-only `GixRepository` adapter. It
denies only when the resolved target repository matches both locations of an
initialized direct submodule checkout, so an unrelated nested repository does
not inherit submodule policy. Malformed input and relative targets deny before
repository discovery; symlink cycles deny after a repository root is found.
Clearly non-Git paths and unavailable Git metadata fail open.

`hook deny-edit-write` checks its token-scoped, TTL-bounded activation before
reading the tool event. While active, only a positively resolved absolute path
under canonical `/tmp` or the existing larch sessions cache is allowed. The
Rust deny envelope is fixed and byte-stable. An inactive Rust command emits nothing,
but a delegation failure denies because the shim cannot establish inactivity.

`hook deny-run-in-background` reads `CLONE_PATH` through the shared registry KV
codec and denies an overlapping clone while any regular registry row remains.
Malformed events, unresolved cwd identity, and unreadable regular rows fail
closed. The combinator-free `scripts/larch.sh bgjob wait` form remains the only
active-registry carve-out.

#### Advisory-hook runtime boundary

Session health, statusline installation, SessionStart cleanup, the opt-in
edit/write audit, and the `/implement` Stop boundary are Rust-owned `hook` verbs
in `crates/larch-cli/src/hook_commands.rs`. Their shipped shell files are thin
compatibility shims. Each enters through `scripts/larch.sh`, sets
`LARCH_BOOTSTRAP_NO_INSTALL=1`, and exits 0 without output when the verified
executable is unavailable or the Rust verb fails. Hook evaluation never
downloads or installs code. `sessionstart-health.sh` alone retains fixed JSON
literals for stripped-`PATH` environments where `jq` is unavailable; those
literals interpolate no event or environment data.

The Rust health and Stop owners read each hook payload once and use the shared
in-process session resolver, so a missing payload session identity cannot
inherit stale process state. Health repository reads use `GixRepository` and
remain advisory. The Stop owner emits its existing block envelope only for an
unreleased post-`/review` boundary; a re-entrant Stop and unavailable runtime
fail open. Statusline installation reuses the Rust progress owner. Cleanup
reaps background jobs synchronously before launching the age-based sweep as a
detached, no-install child with a newly created diagnostic log. The audit owner
accepts only object JSON and refuses symlinked, multiply linked, or non-regular
audit paths before appending one compact JSONL record.

Review launchers use the narrowest available CLI posture. Codex review runs use
`--sandbox read-only`. Cursor review runs use `--mode ask`. Their launchers also
compare the working tree with a pre-launch baseline and discard results after a
dirty or unknown post-run state. This backstop detects writes after they occur;
it does not prevent all writes during execution. Some Claude review and voter
subprocesses have no mechanical read-only CLI sandbox. Their read-only behavior
is prompt-enforced, with path validation and later tree or publication checks.

Implementer and fixer lanes are intentionally write-capable. Codex implementers
use a workspace-write sandbox with explicit repository and output-directory
grants. Cursor implementers run with `--trust` and can reach any path allowed to
the user. The dispatcher, not the external implementer, owns staging and commit.
It rejects history drift, protected-path changes, dirty submodules, invalid
manifests, and unsafe paths before committing. External process writes bypass
Claude's `Edit` and `Write` hooks, so the normal pre-commit checks remain a
required second line of defense.

Rust vendor modules build product argv, and `ExternalProcessRunner` owns the
environment allowlist, timeout, bounded capture, termination, and reaping for
every larch-owned vendor child. There is no Python launcher or fallback.
Captured external output remains untrusted and may contain secrets. Keep raw
streams in session-local state and use the owning redaction and publication
boundary before egress. Rust Cursor isolation creates a private config
directory and injects `CURSOR_CONFIG_DIR` only into the child `ProcessRequest`
environment; it does not mutate the parent process environment, so parallel
tests and parallel clones stay isolated.

The Rust `plan-review step3-review` owner also uses the env-clearing process
boundary for nested verified larch commands. It forwards only its fixed,
non-secret design-session context through typed `ChildEnvironment` entries:
the design and session roots, session and run identities, repository and issue
identity, reviewer availability, external health-check timeout, plugin root,
Claude source path, and explicit session-env path. It does not forward
`LARCH_LIVE_MUTATION_OK`. A nested mutation must validate its session context
through the scoped live-mutation gate below.

`analyze-bugs runtime` uses closed `HostUtilityProgram` cases: one fixed legacy
test runner for Gix-discovered, live, repository-relative retired-runtime test
paths, plus fixed Make harness targets selected from touched-path prefixes. It
accepts no generic executable or target, has a 300-second deadline, five-second
shutdown grace, and 64 KiB capture cap, and caps normalized failure evidence
before it reaches private artifacts. `analyze-bugs report` is local only: it may
render a follow-up issue body, but never mutates GitHub.

`checks run-relevant` uses the closed `HostUtilityProgram::PreCommit` case
(#8616) to run the repository's `pre-commit` hooks over the changed-file
selection. The child inherits only the non-secret `ChildEnvironment::production`
allowlist plus the bounded Cargo build selectors, the pre-commit hook-skip
token, and the XDG cache/config roots; no credential variable is forwarded. The
captured log is redacted before it reaches any private artifact, and the
bounded Clippy fallback routes through the Rust-owned `checks rust-clippy`
verb. The contains-pin probe never executes an
external program: it reads the repository's `scripts/test-*.sh` harnesses and
their pinned target files directly.

Rust Codex-home preparation likewise creates a fresh confined directory below
the caller's private root. It strips inherited API settings and prior trusted
instructions from copied configuration, accepts a trusted-instructions file
only when it is a regular non-symlink, and copies a regular `auth.json` into
the private home when environment-key auth is absent. It never places a
symlink in the prepared home or points `CODEX_HOME` outside that root; the
typed `CODEX_HOME` override reaches only the vendor child request.

Rust owns the `agent launch-review` lifecycle for Codex and Cursor reviewers.
It preserves cap, preflight, execution, retry, and postprocessing order while
the shared process runner remains the only Rust product-spawn and live
process-tree owner. On cancellation or timeout, it snapshots and validates
separate descendant groups before signaling them, so reparenting cannot let a
surviving descendant escape cleanup.
Darwin startup locking uses a caller-selected temporary root, a bounded retry
budget, a confined lock directory, and an owned delayed-release handle. Stall
writers reuse `LauncherArtifactPaths` for the `.stall.json` path, bound and
redact captured transcript and Git status text, and publish through the shared
confined atomic writer. A detailed Cursor compatibility sidecar remains
best-effort and cannot turn a successfully published primary artifact into a
launch failure.

### Same-user state and sandbox limits

Session directories, cache files, startup locks, `.meta` files, result envs,
sentinels, and PID or process-group records are not authentication boundaries.
Their readers validate shape, ownership assumptions, containment, file type,
identity, and freshness before use. Writers use the owning guarded and atomic
helpers. Do not `source`, `eval`, or execute a persisted data file unless its
specific contract defines a closed sourceable format and validates every key.

These controls do not defend against a hostile process running as the same
operating-system user. Such a process may inspect environments, alter writable
session state, race pathname checks, or modify plugin cache data. Temporary
directories are private workflow state, not a confidentiality boundary against
the same user. `/tmp` is shared scratch and provides no cross-skill secrecy.

Rust filesystem adapters require explicit absolute roots, reject escapes,
symlinks, special files, and multiply linked write targets, and revalidate near
mutation. Shared Rust session-state foundations own KV parsing, session-root
derivation, path-confinement checks, and private atomic publication. These are
confinement controls for larch mistakes and
untrusted paths, not a sandbox against hostile same-UID parent replacement.
The Rust-owned review-phase detail commands use the same confined atomic writer
for per-round metadata and caller-selected rendered-report output; unsafe
destination shapes fail before replacement.

The ancestor walks that refuse a symlinked write path exempt a symlink owned by
uid 0, and only that case. macOS spells its platform temporary roots as
root-owned symlinks, `/tmp` to `private/tmp` and `/var` to `private/var`, so a
walk that refuses every symlink up to the filesystem root refuses every larch
write below `$TMPDIR` or `/tmp` on that platform, including the session-tmpdir
fallback. Creating or replacing a root-owned symlink requires privileges the
threat model above already places outside these controls, while a symlink
planted by the same user or by any other unprivileged user stays refused. The
confining owner still canonicalizes the write parent, rejects a symlinked parent
or leaf, and revalidates before replacement, so the exemption widens the accepted
path spellings and not the set of accepted write destinations.

### Advisory anti-read-poll hook

`hook anti-read-poll` is Rust-owned by
`crates/larch-cli/src/hook_commands.rs`. Its shipped wrapper only forwards
stdin to `${CLAUDE_PLUGIN_ROOT}/scripts/larch.sh hook anti-read-poll`, the sole
verified runtime entrypoint. The hook treats
the JSON event and existing local state as untrusted data. It records only
fixed-format rows containing hashes of the cwd, session key, and requested
path, never a raw requested path, and its sole visible output is the fixed
advisory reminder envelope.

On supported Unix platforms, the owner opens the temporary root and
`larch-read-poll` state directory with no-follow directory descriptors,
revalidates directory identity around mutation, accepts only regular state
leaves, and replaces rows through collision-resistant, mode-0600 temporary
files and a same-directory atomic rename. It rejects swapped, symlinked,
non-regular, and multiply linked write entries before replacement. Missing
Unix primitives, malformed input or rows, local filesystem failures, and brief
lock contention are advisory failures: the hook emits nothing and exits zero
rather than blocking a `Read` event. Only the third matching in-window read
emits the reminder.

## Mutation Authorization and State Integrity

Every external mutation requires authority from the current workflow step or a
direct operator request. Issue text, model output, repository content, a result
file, or an earlier successful run cannot grant authority. A dry-run must avoid
the mutation entirely, not merely label its output as a simulation.

### Scoped live-mutation gate

GitHub issue creation, comments, closes, labels, and the callers covered by the
shared issue-mutation boundary accept only one of these routes:

- A session-backed `/implement` or `/design` run passes a regular non-symlink
  context file directly below its canonical session root. The file must contain
  `LARCH_LIVE_MUTATION_OK=true` and the matching run identity.
- A direct operator command passes `--operator-invoked` at the guarded CLI
  boundary.
- A dry-run passes neither route and makes no GitHub mutation call.

`crates/larch-adapters/src/github/mutation_auth.rs` owns validation of the
session route, and `session check-live-mutation-auth` is the Rust command shell
callers use to reach it. The canonical roots it accepts are the shared session
allowlist: `/tmp`, `/private/tmp`, `/var/folders`, `/private/var/folders`, and
the `XDG_CACHE_HOME` or `HOME` cache root. A caller-supplied `TMPDIR` does not
widen that set. `crates/larch-adapters/src/github/issue_mutation.rs` owns every
live issue mutation. Unauthorized calls fail before any GitHub request,
emit the documented refusal result, and do not retry through another route.
Every refusal keeps the `unauthorized-mutation` prefix and adds one bounded,
static reason: `no-context`, `root-not-canonical`,
`context-parent-mismatch`, `auth-flag-false`, `run-id-mismatch`, or the
test-only `test-denied`. These reasons expose no path or context-file content.
Generated session context is decoded through the shared allowlisted shell
assignment parser, including its `export KEY=value` form, before the run
identity is compared.
`issue create-one` applies the check in the Rust owner before the create request
is built. `issue create-batch` validates the same route before it reads any
remote state, then reuses the authorized create and dependency owners for each
effect. Before its first mutation, it confines and reads the parser output,
validated decision file, and every CREATE body below one declared temporary
root. A malformed row, duplicate key, carriage return, escaping or symlinked
path, unreadable body, unusable reference, or dependency cycle therefore stops
the whole batch before a partial graph exists. Its decision file describes
work but never grants authority. `/issue`, including every `/learn-from-bugs` filing route, requests
authenticated-user assignment on every create. `/audit-umbrella` requests the
same assignment on every direct corrective-leaf create. The shared owner
resolves the authenticated GitHub login, includes it as the issue assignee, and
requires the exact assignee on read-back. A missing login or dropped assignment
fails closed; the existing create rollback closes an issue whose response
cannot satisfy that postcondition. When `/audit-umbrella` cannot resolve the
login before a create begins, it restores and persists the corrective leaf as
pending. Failures that cannot prove whether a create began stay in flight for
exact-match recovery.
`issue add-blocked-by`, `issue add-sub-issue`, and both `/block-issue`
dependency mutations apply it in
`crates/larch-cli/src/issue_dependency_commands.rs` before any lookup, and the
typed issue-graph adapter operations re-apply it before their own first read.
`issue cleanup-failed` deliberately carries no gate: it closes an issue the same
caller has just created, and its predecessor took no authorization either.
The in-process batch cleanup uses that same narrow recovery owner after an edge
failure. It closes only the issue identity returned by the current create,
marks transitive descendants failed, and continues only independent siblings.
Dry-run never enters create, edge, or cleanup effects and emits no remote id.

Rust mutation tests exercise the boundary through injected services and scoped
environment fixtures. Denial overrides a valid parent session.

The gate does not cover every process or every GitHub surface. It cannot stop a
process that can rewrite code, credentials, or validated session state. Treat
its scope as an explicit operation allowlist, not a general capability system.

### Freshness, identity, and read-back

Before a security or integrity-sensitive mutation, re-read mutable targets and
validate the exact expected repository, issue or pull request, state, revision,
head SHA, timestamp, lease, and input fingerprint required by that operation.
Persisted results carry the identity of the inputs that produced them. Consumers
reject or recompute stale results as required by I-Stale-1.

After mutation, read back the owning surface and verify the requested
postcondition. An uncertain mutation is not blindly retried. Idempotent writes
may reconcile and retry only after proving absence. Creates without a stable
collision key return an ambiguous outcome instead of risking a duplicate.

Protected issue-body updates require an expected `updatedAt`, the expected
state, a matching lease, one named block, redaction, and a strictly newer exact
read-back. Named-block writers resolve the lease identity from `RUN_ID`, then
the rehydrated `LARCH_RUN_ID` and `SESSION_ID`; missing all three still fails
closed. A composing parent that launches a named-block writer through the
verified child boundary forwards that resolved identity through the closed
`ChildEnvironment::RunId` allowlist. The `/design` publish paths use their
explicit or rehydrated session id only when the parent has no run-id key.
Dependency and migration-governance paths bind blocker, owner, plan, base, and
lease evidence. `/implement` evaluates receipt base-scope freshness
between the receipt base and the current base target (`origin/main`, or
`upstream/main` for a fork run), never the implementation-branch `HEAD`.
Plan, owner, and blocker hashes remain live checks, and in-scope base-target
drift fails closed at Step 2 dispatch. A sole `stale-plan-base-scope` finding
at Preflight, or at the Step 8 ship gates through the bounded
`governance-refresh` route, reaches a bounded independent
semantic-materiality probe; only a current result may invoke the Rust
`plan-receipt refresh` adapter, which binds the current base SHA, requires the
implementation run lease once the title carries a managed prefix, and
read-verifies the protected issue-body mutation. It also binds the checked plan
and prior receipt before writing the exact read-back snapshot that Step 0
compares and a bounded JSON-quoted path-only drift record. Plan, owner, blocker,
malformed-receipt, and unavailable-evidence defects still fail closed, and
later base drift is never silently carried past Step 0 or past a ship gate.

`crates/larch-adapters/src/github/issue_mutation.rs` is the single Rust owner
for issue title, body, label, comment, and close writes. Tracking comment
creation, replacement, and deletion pass through this owner, which verifies the
returned comment identity and body, then verifies create and replacement with a
same-surface comment-list read-back; deletion is verified by absence from that
list. Issue creation accepts a canonical create response only after a same-issue
GET proves its title, body, labels, identity, and open state; a failed proof
names the orphan for best-effort closure.
Tracking lease activation also applies the lease body and `[IMPLEMENTING]`
title together, bound to the preflight title, body, and admission-relevant
label hashes, timestamp lower bound, and current base-target SHA. Metadata-only
timestamp advancement is accepted only while those admission-relevant issue
fields remain exact; pre- and post-mutation governance checks consume a fresh
Rust-materialized post-mutation body and retain blocker and owner race
detection. Workflow callers reach those operations through `scripts/larch.sh`,
and Rust callers use the owner in process. `final-report write` calls the same
Rust tracking owner in
process to preserve its own output envelope. Later Rust callers use
`larch_adapters::github::IssueMutationOwner`, which applies the shared
live-mutation gate before its first read, serializes through the shared GitHub
runtime lock, redacts outbound titles, bodies, and comments, and proves a
fresh exact read-back without a blind retry. `/combine-issues` candidate
discovery reads at most the 100 most recent open issues. Its Rust-owned apply
path reads every selected source's native blockers before creation, re-adds those
blockers to the combined issue, and verifies the full set before it can
close a source. A partial transfer leaves sources open and reports the durable
combined issue URL. Deferred source closure re-reads the combined host and each
source's active blockers; missing or unverifiable inherited edges leave that
source open. If a source-close batch becomes partial after the combined issue
is durable, it also reports that URL and its exact closure tally. A close
comment is published before the close while holding the same mutation lock;
comment, close, or closed-state failure is never reported as a successful
close.

`audit-runs close-priors` first performs a bounded labeled issue read and
refuses without commenting or closing when more than one matching prior report
is present. Its one permitted close goes through that same owner and emits a
verified close result only after the comment and closed-state read-backs. The
related backlog advisory is read-only and never posts a comment.
`audit-runs comment`, the skill's supplementary augmentation and
session-summary mutation, requires `--operator-invoked` and refuses before any
network access without it; the post itself goes through the same shared
issue-mutation owner, which authorizes, redacts outbound text, and proves the
exact comment body by read-back. The remaining audit-runs helpers —
`issue-search`, `label-check`, `fix-merge`'s issue-timing and merged-PR reads,
and the local `version-window` Git-history walk — are read-only and perform no
mutation. The proposal-classification `issue-search` is capped at the 100
newest matching issues.

### Local mutation safety

Wire files use closed key sets, single-line values, explicit size limits,
non-symlink regular files, and atomic publication. The Rust CLI owns `kv get`,
`session read-key`, and `session read-keys`; their parsing and filesystem
primitives live in `larch-core` and `larch-adapters`.
`crates/larch-cli/src/session_env_commands.rs` owns approved session-writer
destinations and key allowlists. Prompt-side orchestration must not write or
repair trusted result or session files directly.

Destructive cleanup or synchronization validates the exact root and target,
rechecks mutable identity immediately before acting, and limits deletion to an
operation-owned allowlist. A persisted PID or process group is signaled only
after process identity is re-verified. The Rust session and background-job
runtime owns process-identity capture, validated process-group termination, and
`session kill-background-processes` (`crates/larch-core/src/process_identity.rs`,
`crates/larch-adapters/src/process_identity.rs`,
`crates/larch-cli/src/kill_background.rs`). Rust-owned stall-state clearing
consumes the Rust bgjob registry and process-identity validation directly.
Rust-owned stall classification also consumes the Rust bgjob registry directly.
The Rust `review-and-fix` and `plan-review` `write-loop-identity`,
`await-loop-identity`, and `teardown-loop-identity` commands compose this same
owner through `crates/larch-cli/src/review_loop_identity_commands.rs`.
Teardown records signal intent, revalidates the PID, PGID, start time, command,
and kernel birth identity immediately before group signaling, and retains the
identity sidecar until the process-group absence probe succeeds. This shared
boundary is the sole process-identity and kill-log implementation.
Before a stall clear removes or rewrites anything, it preflights every fixed
state layer and derived classification or issue artifact below the validated
temporary root, and it completes the required abandoned-bgjob recovery proof.
It then publishes a private, versioned pending-clear marker. The marker records
which fixed files existed at the start; classification and outcome normalization
refuse to treat the session as cleared while it remains. Each state-layer write
preserves unrelated keys and has a read-back check. The marker is removed only
after every intended state change and artifact removal verifies. An interruption
therefore leaves a recognizable pending transaction: a retry requires each
expected state layer to remain confined and parseable, treats an already-removed
derived artifact as a completed phase, and converges without deleting unrelated
state keys.

Classification and attempt artifacts are published atomically below the
validated temporary root, and attempt values reject line breaks. Attempt-ledger
read-modify-replace operations hold a stable private companion lock rather than
locking the inode that atomic replacement swaps out; every successful append is
read back before its count is returned. Escalation rows are appended under an
exclusive lock through a non-symlink file descriptor; unsafe detail filenames
cannot forge TSV fields, and the append repairs a missing terminal newline
before writing one complete row.
Unsafe canonical or fallback paths fail closed, while a genuine canonical write
failure may use the existing bounded fallback artifacts. A fixed-string
comparison, field equality, or closed parser must handle
interpolated labels, markers, refs, and identifiers. Do not interpolate
untrusted data into a regular expression or shell program.

Session setup records its private uncommitted owner as a PID, normalized
process start time, and the same kernel birth identity used by process cleanup,
not as a bare PID. Cleanup retains any marker whose live identity is
unverifiable or matches. Markers written before the birth field, including ones
with only a PID and start time, are legacy: cleanup may reclaim them only when
the PID is absent and otherwise retains them rather than treating a
same-second `lstart` comparison as proof of reuse. The setup writer uses one
cancellation-versus-transfer decision after the ordered stdout envelope is
written and flushed, so a signal before transfer cannot become a silent
successful commit and a signal after transfer cannot retract a published
session.

Current session pointers are a separate `$HOME/.cache/larch/sessions/`
authority even when session artifacts use `XDG_CACHE_HOME`. Pointer publishers,
pointer reapers, and cleanup's recursive deletion boundary share a confined
advisory lease. Cleanup re-reads pointer state and revalidates the target while
holding that lease; a malformed pointer or unavailable lease retains the
candidate. The lease coordinates larch's cooperating processes and is not a
same-user authentication boundary. Implement-tempdir routing likewise accepts
only direct non-symlinked candidates with non-symlinked sentinel and keepalive
components beneath an approved root.

Rust owns every bgjob command: durable registry records, `bgjob adapt`, the
daemon `start`, `wait`, `status`, and `reap` surfaces, and
`bgjob write-merge-result-env`
(`crates/larch-core/src/bgjob.rs`, `crates/larch-core/src/bgjob_daemon.rs`,
`crates/larch-cli/src/bgjob_adapt.rs`, and
`crates/larch-cli/src/bgjob_commands.rs`). The adapter confines its state files
and holds a pinned decision lock before it reattaches or launches. `start`
and `adapt` reject a bare worker program unless it resolves to an executable on
`PATH`. Path-bearing programs remain subject to the worker's spawn check. A
spawn failure writes the operating-system error to the confined step stderr log
before recording exit 2. For an input-fingerprinted job, the next matching
adapter call invalidates a nonzero completed result and starts a fresh attempt.
Jobs without an input fingerprint retain nonzero results for explicit routing.
`start` re-executes the same verified binary as a detached supervisor. The
supervisor starts a private-gated daemon monitor,
binds a confined status sidecar to that monitor's PID, then releases its gate.
It remains the monitor's direct parent so it can reap the monitor and atomically
record either its exit code or terminating signal. A DEAD recovery consumes the
status only when its PID matches the daemon identity in the registry row. A
missing or mismatched sidecar grants no process claim and yields empty
termination fields.

The merge-result writer accepts explicit rows or copies one confined regular
source envelope. It rejects carriage returns, multiline keys and values,
symlinked or non-regular inputs and destinations, paths outside the caller's
validated session tmpdir, and missing routing keys requested by the caller.
Publication creates a confined parent when needed and uses the shared no-follow
atomic writer with mode `0600`. The Step 3 copy path preserves first-key order
and the last value for duplicates, matching the retired Python reader.

The daemon monitor binds the owner's recorded process identity, never a bare
pid, so a reused pid never keeps an orphaned job alive (#6604). It terminates a
timed-out or orphaned child only through validated process-group termination.
Before it reports `STARTED`, the monitor creates a private-gated persistent
worker as the process-group leader, captures its and the monitor's identities,
and atomically publishes the complete registry record. It then writes the
identity-bearing `registry-published` startup marker, releases the worker gate,
and acknowledges the launcher. The adapter retains its decision lock through
that acknowledgement, so another adapter sees the one durable launch rather
than starting a duplicate. A bounded launcher acknowledgement timeout, closed
pipe, or malformed acknowledgement terminates the detached supervisor group and
enters the same durable recovery path. Recovery either proves the worker group
absent or leaves the registry record retryable.
Each Rust-owned persisted identity records PID, process group, normalized `ps`
start time, command text, and a kernel birth identity. Darwin uses the `proc_pidinfo`
BSD-process creation seconds and microseconds; Linux combines its boot UUID
with `/proc/<pid>/stat` start ticks. Capture brackets the `ps` and
process-group fields with birth probes and fails closed if either probe is
unavailable or differs. A child may legitimately replace its wrapper through `exec`; that
transition retains the recorded kernel birth identity, PID, process group, and
start time, so `AllowCommandTransition` permits the changed command only after
those fields all match. Command text alone is never proof of continuity.
Records that predate the birth field, including registry rows, recovery leases,
and loop sidecars, remain readable for diagnostics and recovery but cannot
authorize a live result or a signal. A legacy recovery lease with a live PID
also remains held until absence proves it stale, rather than admitting an
overlapping cleaner. Other persisted identities retain exact command validation.

The runtime logs signal intent, then revalidates the full identity immediately
before each `killpg` call. The platform APIs cannot make comparison and group
signal one atomic operation, and neither platform exposes a serializable kernel
handle with unbounded birth precision. A small scheduling or theoretical
same-tick reuse gap remains; the second check and strongest durable metadata
narrow it without claiming an absolute guarantee. Cleanup never individually
signals an enumerated descendant PID, because that PID has no durable identity
and could have been recycled. A validated group signal reaches members that
remain in the recorded group. The persistent worker never `exec`s the requested
command and does not exit until its group is empty, so a requested shell or
wrapper can exit while its descendants remain owned. During TERM-to-KILL
escalation, larch captures and revalidates a live non-leader group member before
using it as a one-call escalation anchor. If neither the leader nor such a
validated anchor remains, larch retains the record instead of signaling a bare,
possibly recycled numeric group.

When a daemon dies, `wait` and `reap` share an exclusive, process-identity-bound
recovery lease for its durable registry row. The lease holder validates the
recorded child group, logs each intended signal, and proves that the full group
is absent before it removes the row. A signal attempt, bounded child reap, or
missing leader alone is not proof of teardown. If that proof fails, larch keeps
the registry state and an actionable teardown diagnostic; it does not publish a
`BGJOB_RC` result envelope or claim `DONE`. `wait` reports that condition as a
retryable `WAIT` with `BGJOB_RECOVERY=retryable`, never terminal `DEAD`; `reap`
returns a nonzero result with `BGJOB_RECOVERY_FAILED` rather than silently
reporting zero work. The shared recovery owner is also the only clear-stall
registry cleaner: `clear-stall` must claim, validate, terminate, and prove
absence before it clears state or removes a row. This fail-closed retention is
an explicit safety difference from the retired Python behavior, which could
discard the row after an unverified timeout or orphan cleanup.

Active `bgjob wait` refreshes a session-local wait lease on every poll so an
ephemeral start-time owner cannot orphan a live child mid-wait. The default
wait chunk remains 270 seconds; the hard maximum is 7200 seconds for
documented long leaf waits. While a registry row is live for the clone,
the Rust-owned `hook deny-run-in-background` command, reached through
`scripts/hook-deny-run-in-background.sh`, denies Bash `run_in_background`
launches. A combinator-free documented `scripts/larch.sh bgjob wait` command
that owns its own `--max-wait-s` deadline remains allowed.

The daemon owns elapsed-time decisions with a suspend-pausing monotonic clock.
It refreshes the registry row's wall-clock `HEARTBEAT_EPOCH` on every monitor
poll; other processes use that field only as a bounded cross-process liveness
TTL. A detected suspend resets owner validation and grants exactly one
wait-lease window before orphaning can proceed. The same wall-clock jump cannot
renew that grace, and a wall-clock jump cannot consume the runtime budget.
Legacy registry rows remain readable by treating their start epoch as their
initial heartbeat. On macOS, the default-off `LARCH_BGJOB_CAFFEINATE=true`
option wraps only the requested command with fixed `/usr/bin/caffeinate -i`.
It prevents idle sleep while that command runs, not lid-close or forced sleep.

After a worker observes that its requested child exited and its group drained,
it atomically retains a confined completion witness until the daemon commits the
 whole result transaction. The registry retains one versioned, complete recovery
 input set: the confined merge envelope, declared sentinels, and any explicitly
configured terminal stdout marker. A dead-daemon claimant rebuilds the
transaction only after independently proving group absence again. It reads a
bounded, no-follow tail of the owned
stdout log only for that opt-in marker and only accepts a contiguous final
`KEY=value` block containing it. Missing markers, malformed output, a tail
that cuts through the candidate block, or a failed publication keep the
witness and registry retryable instead of silently dropping the terminal child
envelope. An unsafe registry artifact never authorizes reconstruction or
publication.

Recovery-lease publication writes and syncs the claimant identity in a private
temporary file before atomically linking it into the final no-replace lease
path. A short-lived per-row advisory lock serializes inspection, stale-lease
removal, and publication; the durable identity lease remains the recovery owner
after that lock drops. The final lease is therefore either absent or a complete,
parseable identity, never a partially written claimant. A fresh malformed
regular lease is retained as a possible in-progress legacy publication and
returns a retryable busy state. Once it passes the bounded staleness window,
the next `wait` or `reap` reconciles it under the lock and retries acquisition.
Unsafe paths, unreadable metadata, and unprovable live identities retain the
registry row and fail closed. If a claimant is cancelled or dies after a valid
claim, a later waiter validates the dead identity before safely taking recovery
ownership. If it dies while reconciliation still holds the advisory lock, the
operating system releases that lock first.

## Workflow Boundaries

### CI cache trust

The [CI tool bootstrap and caches](supply-chain-credentials-and-services.md#ci-tool-bootstrap-and-caches)
section is the canonical cache-class and publication contract. The `CI`
workflow validates pull requests, merge groups, and manual dispatches with
read-only cache restores. A normal `main` push starts the separate trusted
publisher, which refuses every other event and ref, has only `actions: read`
and `contents: read` permissions, and is serialized by its own newest-wins
concurrency group. Validation events do not gain authority to publish a
compiler-output cache or trusted policy cache.

When the publisher resolves an expensive Rust candidate source, its
`main-cache-merge-group-source` job may restore only the canonical exact Cargo
input and pruned lint-dependency caches. It records their two hit states and
the typed resolver's wall-clock seconds, but runs the resolver from the checked
out final `main`; a cache miss rebuilds from that checkout and does not weaken
final-SHA, event, workflow, producer, or ambiguity verification.

For an expensive Rust cache miss, the publisher treats a merge-group artifact
as untrusted input. It requires exactly one successful `CI` merge-group run for
the final `main` SHA, the `rust-coverage` aggregate, and the named
`rust-full shard 1` and `rust-lint` producers, then verifies the
candidate manifest, source SHA, canonical key and key-input digest, byte bound,
regular-file tree, checksums, modes, bounded nanosecond modification times,
artifact identity, and declared tool versions before saving. It restores and
verifies member modes and modification times through pinned regular-file
descriptors before the payload reaches an Actions cache. Cargo cache payload
names may include ordinary package build metadata and generated punctuation,
but every member must still be below
the payload root with nonempty components that are neither `.` nor `..`, and
must exclude path separators, control characters, and portable
Windows-reserved filename characters. Symlinks, stale source identity,
unexpected paths, or a missing producer fail closed. The publisher may rewrite
only Rust-policy provenance,
and only after that final-SHA verification. A
candidate manifest is separately bounded at a reviewed 32 MiB so Cargo's
per-file integrity records remain accepted without giving untrusted artifacts
an unbounded parser allocation. A
verified Cargo-input candidate may omit its optional empty `git` directory
because artifact transport carries regular members rather than empty
directories; the trusted publisher recreates only that empty canonical cache
path and rejects a non-directory replacement. A
coverage target cache is dependency-only, bound at a reviewed 1,400,000,000
bytes, and enabled only after independent end-to-end measurements prove it
helps. Neither a cache restore nor its diagnostic metadata waives the coverage,
artifact, executable, repository-policy, or plugin-validation gates.

The manual target-cache benchmark is isolated from that production cache
contract. Its fixed workflow condition requires a direct `workflow_dispatch`
of `refs/heads/main`, its benchmark-only key cannot be restored by the normal
coverage lane, and its decimal size input is capped before the shared action
can save. During that exact dispatch, the full shard and policy path stays
cache-off as the paired control. The benchmark exists only to collect the
independent warm-cache comparison; it does not authorize a pull request or
normal manual run to publish compiler output.

### CI Rust selection trust

The pull-request `rust-selection` job has read-only workflow permissions. The
checkout action supplies GitHub's tested merge candidate with full history, and
a separate checkout supplies the pull-request base. The job uses that base's
`scripts/larch.sh` wrapper but does not build an executable there. It restores
the exact `trusted-main-rust-policy` cache entry, validates its content-derived
identity, and supplies that executable to the base wrapper's `ci rust-select`
command. The command proves the base commit, candidate commit, and base ancestry
before it reads the candidate diff. An invalid identity, unavailable history
proof, missing or invalid trusted-main executable, or selector failure selects
`full`. Candidate code can supply the tree being classified, but cannot author
or execute the classifier that authorizes a non-full lane. Selector, workflow,
coverage-action, Rust-input, and selector-redaction/workspace-metadata changes
are explicit global `full` triggers or exact-cache misses.

The selector validates commit identity, checked-out state, and base ancestry
before it inspects a diff. Missing history, a malformed or empty diff, unknown
path, metadata failure, unsupported workspace shape, and selector failure all
become `full`. The partial decision is a strict Rust-source package closure
derived from locked offline Cargo metadata. It includes normal, build, and dev
reverse dependency edges; it must contain `larch-cli` and be smaller than the
workspace. The selected lane builds that candidate executable, runs repository
policy, plugin validation, and bootstrap integration with it, so it does not
mistake an all-workspace closure for a partial path.

Skip ownership is explicit rather than extension-based. Each root or path
family in the allowlist names the normal lint, agent, plugin, and/or
trusted-main repository-policy job that continues to validate it. The
`trusted-main-rust-policy` cache trust contract is canonical in
[Supply Chain, Credentials, and Services](supply-chain-credentials-and-services.md#ci-tool-bootstrap-and-caches).
The selection job and skip job both verify that content-derived identity before
they execute it. The selection job additionally executes it only through the
trusted base wrapper. Its trusted base cache-key action derives the lookup key
and expected Rust-input digest from that same isolated checkout, so candidate
Rust files cannot name or validate an executable. The job uploads the verified
handoff only for an effective `skip` decision. A cache miss or failed
verification selects `full`; no pull-request-provided Rust binary is accepted
for selection or `skip`.

`RUST_CI_PARTIAL_ENFORCEMENT` and `RUST_CI_SKIP_ENFORCEMENT` are `true` only
because each durable live record has at least three independent non-full
proposals, successful full backstops, and zero false-safe results. Either class
still falls back to `full` if trusted-main policy validation fails. Only a
reviewed workflow update may set a class-specific value to `true`. A candidate
checkout, selector output, cache result, or pull-request label cannot promote a
class.

Every dynamic JSON and summary string passes through the Rust core redaction
boundary and a residual-secret rescan; redaction failure emits a static `full`
result without changed-path data. The step summary HTML-escapes those redacted
fields. The structured result preserves the classifier proposal and adds the
effective execution mode, reason, rollout state, and observation flag after
cache validation and any safe override; it is an artifact for audit, not an
authorization token. The stable required `rust-coverage` status accepts only
both successful full-mode producers or one successful alternative
(`rust-partial` or `rust-skip`), with every unselected producer skipped. In
full mode it requires every `rust-full-shards` matrix cell, the parallel
`rust-full-policy` producer, and the same-run pinned LCOV runtime preparation.
The aggregate verifies that runtime's checksum, metadata, archive paths,
extraction boundary, and reported version before an exact-count LCOV merge that
includes the policy profile and the combined line gate. The separately required `rust-gate`
validates lint, dependency policy, and the raw producer-result shape without
waiting for `rust-coverage`; either required status fails closed. An
unavailable selector requires the full path, which must succeed before the
stable status can pass. Main, manual,
scheduled, merge-queue, and unknown events continue to run the full lane. The
`full-rust-ci` label is a
safe pull-request override because it can only force that same full path.

### Design

Issue text, feature text, plan text, findings, ballots, scout output,
architectural guidance, and operator refinement text are untrusted evidence.
Inline prompt renderers redact and escape these blocks. Path-only handoffs pass
validated paths and never relay file bytes through `KEY=value` output. Rust
`render specialist` and `render plan-review` composition lives in
`crates/larch-cli/src/rendering_commands.rs` and
`crates/larch-cli/src/plan_prompt_commands.rs`. Both reuse the canonical
`larch-core` untrusted-content wrapper.
`crates/larch-cli/src/rendering_commands.rs` also owns `render voter`.

The Step 1d.7 outline is binding only after operator approval. `--skip-approve`
removes that human review for the outline and final plan. Use it only when issue
and refinement input are trusted or generated by a controlled pipeline. It does
not disable size, validation, finding-apply, or persistence gates.

The plan-size override becomes authority only through the typed operator-action
command, which records the plan hash and independently computed trigger reasons
under the validated design root. Gate B may re-arm that hash after an automated
rewrite only when every live trigger reason was recorded with the decision. A
new reason or direct review re-entry revokes the authority. The plan-authored
`oversize_override` trailer alone never suppresses the gate.

`larch_adapters::validate_design_tmpdir` confines design state before any
quiet-log, result, pause, or publish write. Pause markers bind
the issue, repository, run, snapshot, and allowed recovery branch. Restore uses
a staged tree, validates every path and required artifact, and installs only
after the complete snapshot verifies. GitHub issue markers remain editable by
collaborators and are not an authenticity proof.

The Rust `plan-review step3-mav` transaction reads only allowlisted session and
result-envelope keys. PID-keyed current-session links must pass the shared
trusted-link check, result envelopes must be regular files, and its warning,
phase, and tally-support writes remain confined to the validated design root.

Dialectic drafter, debater, judge, and assessor output is advisory model data.
It cannot edit `plan.txt` or clear a gate by declaring itself safe. Compact
digests reach approval surfaces through untrusted framing. Operator text uses a
file-backed request and never enters shell argv through interpolation.

### Implementation and shipping

The approved plan limits scope but remains untrusted text. `/implement` passes
it to coders and reviewers as evidence. The dispatcher validates the manifest,
branch, history, changed paths, submodules, and worktree before it stages or
commits. Model-authored commit text passes through secret redaction before Git
receives it.

Preflight rejects closed or managed issues, audit reports, live blockers, and
missing design state before session setup. A `[DESIGNED]` title is mutable
GitHub metadata, not proof of plan identity. `--force` may skip semantic plan
review and the designed-prefix check. It cannot admit a missing or malformed
`larch:plan`, suppress later branch or worktree gates, or turn issue prose into
an execution plan. Every admitted bypass is recorded. Blocker lookup still has
its documented fail-open behavior when GitHub dependency reads fail, so an API
outage can produce a false negative.

`crates/larch-cli/src/ship_pr_commands.rs` owns the active post-review
pull-request, CI, merge, resume, and post-merge state machine. It consumes typed,
bounded, redacted result envelopes. Pull-request creation and updates require
current scope and coverage artifacts. CI fixes receive a bounded redacted
digest, not raw failed logs. Conflict fixers receive validated
repository-relative paths and may edit only the named conflict files. The Rust
owner controls staging, rebase continuation, and lease-protected push. Rust
owns `merge pr` and `merge wait` through the verified process boundary.
`implement-finalize postmerge` runs in process, while terminal teardown applies
its issue-title rename through the typed tracking-issue owner. Every create,
merge, queue, and title mutation is followed by a typed GitHub read-back.

Rust owns the canonical initial ship-state schema and the Step 8 result-env
schema. `ship seed-initial-state` validates the session root, contained state
path, identity fields, manifest shape, line-safe values, and create-only gate
before a private atomic write. The Rust Step 8 child invokes the lifecycle owner
in process, requires a typed outcome, normalizes scalars, and writes the
contained result env before it forwards JSON. The Rust `ship pr` command is the
sole lifecycle owner. Symlinked destinations, unsafe parents, relative or
escaping result paths, and missing outcomes fail closed before publication.

Recovery never applies pre-merge mutations to a merged or closed pull request.
A resumed lifecycle performs a typed pull-request read before checkout or Git
mutation. Rust-owned manual reconciliation first proves the run, manifest,
repository, and merged pull-request identity, then rewrites all three durable
state layers, clears every stall and bail overlay, publishes the sentinel and
manifest fields, and verifies the postconditions. Assessment waivers and state
artifacts stay inside the validated run root and bind to the current run
identity.

### Review

Plans, diffs, findings, reviewer prose, votes, and dynamic scout notes are
untrusted. Review prompts use fixed trusted templates. Scout output can supply
file or aspect hints but cannot add commands, tools, scope, or output grammar.
Accepted findings still pass through the fix-coder contract. Unsafe or
out-of-scope instructions in finding prose are ignored.

The code-reviewer security lane covers injection, authorization, secret
handling, cryptography, deserialization, SSRF, path traversal, and dependency
risk. Namespaced context tags and a data-not-instructions preamble reduce prompt
injection but remain model-level conventions. Dynamic prompt bodies with
reserved slugs, unsupported focus areas, unsafe closers, or standalone YAML
fences are rejected before reviewer dispatch.

Review retry metadata is parsed as JSON arrays and closed typed fields, never
with `eval`. Tool-specific argv shapes, timeouts, prompt paths, workdirs, output
containment, and sentinels are validated before replay. Invalid outer-launcher
metadata cannot fall back to an inner command that skips launcher-owned checks.

Review and plan-coverage snapshots are untrusted local state. Readers require
a contained non-symlink root, regular no-follow files, complete artifact sets,
and identity matching the live plan, diff, and run. A partial, stale, malformed,
or unsafe present set fails closed. Snapshot creation and cleanup never rewrite
an unsafe pre-existing tree.

### Research

`/research` is best-effort read-only for the repository. Its skill-scoped
`scripts/deny-edit-write.sh research` shim delegates to the Rust-owned
`hook deny-edit-write research` gate. While a fresh activation sentinel exists,
it mechanically confines only Claude's matched `Edit`, `Write`, and
`NotebookEdit` calls to canonical `/tmp` and the larch cache sessions root
(`~/.cache/larch/sessions`, the larch-owned session scratch tree, so a nested
`/issue` can write its session-setup tmpdir body files). It does not cover
`Bash`, child `Skill` invocations, or external subprocesses. `allowed-tools`
does not add confinement.

Research Cursor and Codex lanes run against the working tree with write-capable
user privileges. Their non-modification rule is prompt-enforced. Synthesis and
revision subagents have their own permissions, and the parent hook does not
propagate. Operators who require a stronger read-only posture must constrain
Claude Code permissions and external tool visibility or avoid those lanes.

Successful research publishes the full report to GitHub unless `--no-issue` is
set. Reports can contain internal architecture, private infrastructure, or
security-sensitive analysis. Use `--no-issue` for sensitive work. Outbound
secret redaction is a backstop, not a classifier for internal URLs, PII, or
domain-specific sensitive content.

### Triage and issue filing

`/triage` treats issue content, repository content, Git output, probe output,
and model verdicts as untrusted. It activates scratch writes only after the
security, repository-target, and immutable-main gates pass. Reproduction uses
the named fixed probes from the triage contract. Issue-supplied commands,
credentials, destinations, and mutations are forbidden.

Duplicate and dependency triage reuses `/issue`'s newest-first snapshot helper,
admits open rows from at most 100 issue records, and does not paginate into older
history.

Before every public mutation, triage rechecks security classification and
freshness. Uncertain security classification routes to private disclosure and
no public issue mutation. Allowed edits and closes pass the expected
`updated_at`, current state, redaction, operator authorization, and exact
read-back contracts in `crates/larch-cli/src/triage_commands.rs`, over the
grammar in `crates/larch-core/src/issue/triage.rs`. The typed Rust
issue-dependency adapter applies the same security terms and exact security
labels across the target title, body, and every bounded comment page before a
triage-controlled public dependency mutation. The service transport and
pagination contract is canonical in
[`supply-chain-credentials-and-services.md`](supply-chain-credentials-and-services.md#pull-request-review-and-dependency-operations).

`/issue` treats fetched issue content as an untrusted corpus. Its delimiter
wrappers are prompt-level defenses. Its candidate snapshot is a mechanically
bounded, newest-first list of at most 100 issue records. Open issues and closed
issues inside the configured window are admitted from that list; a remaining
older tail is explicitly reported as omitted from deduplication and dependency
analysis. A refused fetch names its real class, including the transport-limit
case, and never mislabels the transport bound as a network, authentication, or
rate-limit failure. Deduplication runs through a
read-only verdict agent that cannot mutate the repository or GitHub. Issue
creation uses the scoped live-mutation gate and outbound redaction. Public issue
text still requires prompt-level removal of internal URLs, PII, and sensitive
context that token-pattern redaction does not cover.

`/deps` applies the same untrusted-corpus treatment to open issue titles,
bodies, and comments. It validates rewrite, close, and dependency targets
against the fetched snapshot, requires operator approval before mutation, and
revalidates issue state during apply. Delimiter wrapping and endpoint checks
reduce prompt-injection risk but do not create a parser-enforced sandbox.

### Rejected analysis

`/rejected-analysis` treats published findings and run-log prose as untrusted.
Preparation reads a newest-first snapshot of at most 100 open issues through
the typed GitHub service and reports when older issues are omitted from overlap
checks. An unavailable snapshot still fails closed. It reads only contained regular run-log files and
bounds each input before parsing. Verifier prompts wrap the candidate, pin the
expected file location, and demand the closed verdict format. Launchers use
their read-only posture and dirty-tree backstop. Replies must bind to the
candidate path. Ledger, sidecar, and issue batch fields are TSV-sanitized
before persistence.

Confirmed non-security findings are filed only through `/issue`, preserving
redaction, deduplication, and dependency handling. Finalization reruns the
security classifier. Confirmed or uncertain security findings never enter the
public filing batch.

### Architectural assessment

Architectural knowledge, materialized diffs, assessor output, route detail, and
diagnostics are untrusted evidence. `architectural-assessment materialize` owns
deterministic diff filtering, input fingerprints, durable state, coverage
reuse, and reassessment requests. The read-only `larch:arch-assessor` reads only
the supplied paths and authors every requested assessment kind.

`architectural-assessment submit` revalidates HEAD and diff identity, parses the
closed state and note grammar, redacts the note, reapplies its size cap, and
publishes atomically. A first-submission guideline deviation cannot inject an
`Exception:` block. Only the documented decline path may add a validated block
with rationale, author tier, and date.

The main workflow does not author, repair, or inspect assessment prose on this
path. Stale, malformed, incomplete, unavailable, or mismatched results do not
clear the gate. A fresh assessor judges every repair. An invariant violation
hard-stops after the bounded fix ladder; no waiver or operator override accepts
it. Rust owns architectural preparation, design-time knowledge reads, note
presentation, design-assessment persistence, and implement assessment writes.
All render and run-log consumers use the Rust `read` commands and do not parse
the repo-root knowledge files. The active Rust `ship pr` owner
consumes only identity-validated durable notes, follows the closed outcome
grammar, binds fork assessments to `upstream/main`, and redacts note text before
PR composition. The Rust Step 8 dispatcher carries
closed route requests without interpreting assessment prose.

### Destructive and background workflows

`/set-up-forked-open-source-repo` verifies the immediate fork parent through the
typed GitHub service, reads repository state through `RepositoryRead`, and uses
only closed `GitCli` mutations. It prints exact branch identities, requires
explicit confirmation, and reprobes immediately before its scoped destructive
mirror push. A confirmed sync can delete or overwrite fork branches and tags.
Remote renames record typed inverse operations, changed config values are
snapshotted, and both are reverified after rollback. URL overrides remain
operator-supplied test-seam trust inputs.

Rust-owned `/cleanup` and SessionStart cleanup use fixed roots, name
allowlists, age gates, bounded nested-activity checks, and symlink rejection.
The sweep collects session directories named by live environment state and
current pointers before it considers deletion, then re-reads that pointer
authority under the activity lease immediately before recursive removal; those
live directories remain protected regardless of age. An unreadable current
pointer or unavailable lease makes age-based cleanup fail closed. Retention is
not a lock for an unknown or unreferenced stale session. Stale private session
state can be deleted permanently when it passes those gates.

SessionStart maintenance hooks are fail-soft and non-blocking. They must not
turn local paths, logs, or subprocess diagnostics into advisory instructions.
Automated merge remains gated on validated pull-request state and green
required checks. Immediately before mutation, the Rust pull-request owner uses
a fixed GraphQL read to determine whether the PR's base requires a merge queue.
An enabled queue receives the fixed `enqueuePullRequest` mutation with the
verified head object ID and without an admin bypass or branch-deletion request.
Durable state records queue acceptance only after bounded GraphQL read-back
observes the same head in a queue entry or completed merge.
It distinguishes that acceptance from completion, and post-merge work waits
for an observed `MERGED` state. A policy-read failure stops before mutation.
Direct admin merge remains the no-queue fallback. The development-only release
command follows the normal queue path, then resolves and tags GitHub's recorded
post-merge commit. It does not request an admin merge, a queue bypass, or a
repository or ruleset change.

The active Rust `ship pr` path reads repository state through `gix`,
executes fetch, rebase, and push only through the closed Git CLI adapter, and
pushes with an exact observed remote-tip lease. Its GitHub owner reconciles an
ambiguous create before retrying, repairs an adopted PR body without replacing
its title, and performs a fresh typed read-back after every mutation. A stale
qualified head owner or branch, base, state, draft flag, or redacted body fails
closed.

## Security Findings in OOS Workflows

Security-sensitive or uncertain findings are private. Never file them through
`/issue`, copy them into a public issue or pull request, include them in
published run logs, or fold them into an unrelated implementation. Follow the
responsible disclosure instructions in [`SECURITY.md`](../../SECURITY.md).

Review and design tally paths route security-tagged OOS blocks to the
session-local `security-oos-observations.md` sidecar. The sidecar never merges
into `oos-accepted-design.md`, `oos-accepted-review.md`, `oos.md`, or a public
issue batch. A non-empty sidecar keeps `OOS_PENDING=true` and blocks pull-request
creation until private disposition completes.

Public-boundary classifiers recognize these structured security signals:

- an unfenced canonical `focus-area=security` token;
- a dedicated line-start `focus-area` field whose value begins with `security`,
  including values such as `security-hardening` and supported markup or
  separator variants;
- a block-opening heading that begins with `[security]` or `<security>`,
  optionally after `[OUT_OF_SCOPE]` or `[OOS]`.

Canonical tokens that appear only inside inline code or triple-backtick fences
are meta-discussion, not a security route. A later prose heading that merely
contains `[security]` is not an opening tag. External implementer manifests use
the same dedicated-field predicate after sanitization. A security-looking title
alone does not route a manifest item.

Classifier failure is private by default. It must never fall back to the public
OOS path. Manifest materialization and private-sidecar routing are Rust-owned by
`crates/larch-cli/src/oos_commands.rs` and
`crates/larch-core/src/issue/oos_batch.rs` behind
`scripts/larch.sh oos materialize-manifest`. Rust checkpoint enforcement is
owned by `oos_commands.rs` and `larch_core::issue::oos_disposition` behind
`scripts/larch.sh oos disposition-checkpoint`; the Rust filing driver in
`crates/larch-cli/src/oos_file_commands.rs` refuses post-checkpoint completion
while the sidecar remains.

Design Step 5b preparation and annotation are Rust-owned by
`crates/larch-cli/src/design_oos_commands.rs` behind
`scripts/larch.sh design file-oos-prepare|file-oos-annotate`. The owner keeps
accepted files, sentinels, and retry sidecars under validated, non-symlinked
temporary or cache roots. High-risk `oos-correctness` provisioning uses the
typed GitHub label service after the shared live-mutation gate accepts the
session context, matching run ID, and trusted design root; direct recovery must
instead name `--operator-invoked`. Label application reads the issue snapshot,
applies the field-scoped label replacement through `IssueMutationOwner`, and
accepts success only after read-back. Missing authorization or repository
identity, ambiguous slot mapping, or any label failure preserves the pending
marker and fails closed.

Review and design tally/aggregation modules retain their distinct source-side
classification responsibilities. Under receiving umbrella #7681,
`crates/larch-cli/src/implement_ship_commands.rs` owns Step 8 dispatch, typed
result-env publication, checkpoint exit mapping, canonical run-id resolution,
manifest stamping, run statistics, and the allowlisted atomic
`OOS_PENDING=false` transition. `crates/larch-cli/src/ship_commands.rs` and
`larch_core::implement::{ship_state, ship_result}` own initial state and result
wire validation and private publication. The sibling pre-driver,
pre-fix-rebase, route-exit, and assessment-handoff commands are Rust-owned by
`crates/larch-cli/src/ship_pre_driver_commands.rs`, including confined
result/handoff reads and shared-core ship-state patches. Rust OOS owners provide
block parsing, counting, title normalization, and post-checkpoint behavior.
These Rust owners are the sole OOS implementations. Rust tests in `implement_ship_parity.rs`,
`ship_pre_driver_parity.rs`, `ship_state_parity.rs`,
`design_oos_migrated_parity.rs`, `design_oos_commands.rs`, `oos_commands.rs`,
`oos_file_commands.rs`, `oos_batch.rs`,
`oos_disposition.rs`, and `oos_record.rs` cover Step 8 wire parity, manifest
materialization, field variants, private routing, and checkpoint refusal.

## Major Residual Risks

- A hostile same-UID process can inspect credentials in child environments,
  tamper with writable state, race path checks, and alter plugin data.
- Prompt wrappers reduce but do not eliminate prompt injection. A model can
  still misunderstand or disobey evidence framing.
- Some Claude, Cursor, Codex, Git, hook, and helper paths retain user-level
  filesystem access. Prompt-only read-only rules are not sandboxes.
- Pattern redaction does not cover every credential, internal hostname, PII,
  partial token, or domain-specific secret. Minimize captured and published
  text even when a redactor runs.
- GitHub collaborators can edit issue bodies and workflow markers. Freshness,
  leases, hashes, and read-back reduce risk but do not prove authorship.
- Dry-run, mutation authorization, and test-deny controls apply only to their
  documented entry points. They do not restrict arbitrary code with the same
  credentials.

## Umbrella

`/umbrella` treats input issue text, draft records, agent output, and child `/issue` output as untrusted. It applies one explicit approval gate; `--skip-approve` changes only that presentation wait. Normal filing uses `/issue`'s shared snapshot of at most the 100 newest issues. The standard proposal composer confines its snapshot, generic batch, optional dependency file, and three outputs below the snapshot's resolved scratch root. It rejects input/output collisions, derives exact leaf identities and parser-normalized bodies in Rust, and publishes the durable proposal only after both companion outputs succeed. In-flight recovery admits only the 100 newest open issue candidates and mechanically ignores any older rows before exact-match reconciliation. The skill persists immutable leaf identities and in-flight state before filing, confirms the child sentinel and machine counters, performs live authorization and freshness checks for every mutation, redacts outbound public content, and reads back the final native graph. Ambiguous recovery, incomplete dependency analysis, failed redaction, or missing verification stops the run without a replacement create.

Nested `/design` and `/implement` partitions use a narrower prepared-artifact path. The child accepts it only with immutable parent lifecycle context, one numeric managed issue, `--skip-approve`, and the complete internal flag group. Input and dependency files must be contained regular files under the declared parent scratch root. `/umbrella` parses and bounds the exact generic batch, rejects malformed or cyclic dependency graphs, persists deterministic leaf identities and an atomic child-local dependency copy before any create, and keeps `/issue` duplicate detection enabled. Filing consumes that copy instead of rereading the parent TSV. The parent approval covers only those exact leaves and edges; the child cannot re-decompose them or ask a broader second question.

Final conversion is one centralized issue-mutation operation. It accepts an open `[DESIGNING]` or `[IMPLEMENTING]` source only when the target title preserves the complete source title after replacing its lifecycle prefix with `[UMBRELLA]`, and it requires the complete prior body to survive inside the new body. A separate adoption mode accepts a record-less open `[UMBRELLA]` only after typed GitHub reads prove no direct sub-issues and no open blockers. Closed blockers are already satisfied, and adoption never fetches blocker bodies. That mode keeps the exact existing title and preserves the complete original body in the new body. Both paths redact before write and verify a fresh read-back. The child compares the live prepared-artifact hashes and deterministic leaf/edge shape to the persisted proposal, then writes a repository-, issue-, artifact-, and graph-SHA-256-bound parent completion sentinel atomically only after leaf and graph verification. The parent rehashes both live artifacts and recomputes the graph fingerprint through the umbrella owner before consuming that sentinel. Missing context, unsafe, invalid, or stale artifacts, stale source content, partial filing, or failed verification preserves the original issue and parent scratch state without a success claim.

## Complete umbrella

`/complete-umbrella` does not use the `/umbrella` durable proposal marker as admission or mutation authority. A fresh start accepts an open `[UMBRELLA]` lifecycle parent only after the live native graph supplies at least one selectable direct leaf with the exact reciprocal `[LEAF OF N]` title and first body line, and proves every direct child also blocks the parent. Direct native children must be leaves, not umbrellas. A native parent on the target, including the chief program umbrella, is allowed. Its Rust `run-leaves` owner reads native direct sub-issues and blocked-by edges once per normal iteration. When a prior child exists, the owner first verifies that child's remote lifecycle from the same fresh graph, then rejects nested umbrella children, missing parent-blocker relations, and open parent blockers that are not direct leaves before selecting the smallest-numbered open leaf whose live blocker set is empty. An open leaf may have the exact idle, `[DESIGNED]`, or `[IMPLEMENTING]` managed-leaf lifecycle prefix. Issue titles and bodies are untrusted data. They are redacted into a session-local audit snapshot and are never interpolated into the child command or prompt; only validated repository and issue-number identifiers enter the fixed prompt.

The Rust-owned `complete-umbrella bootstrap` command is the sole model-facing Step 0 process. It starts the shared lifecycle, resolves the repository, asks the pointer owner to resume, creates and starts a fresh session only when no pointer exists, activates the scoped Write-hook sentinel, and resolves or reuses the pinned Claude model. Production callers enter through `scripts/larch.sh`. The composition reuses the existing session and run-log owners in process, rejects duplicate or malformed machine keys, validates each stage before starting the next one, and names the failed stage on a non-zero exit. On success, it emits one consolidated KV block and writes the same block plus any newly resolved model below the private session tmpdir. Prompt-side orchestration consumes stdout; the file copies are diagnostic and resume evidence only.

One durable bgjob invokes `run-leaves` for the complete current leaf loop. The owner requires a clean worktree on `main`, fetches and rebases onto `origin/main` through typed Git operations, and proves clean `HEAD` equality before every child launch and after every successful child. Each selected leaf then runs serially through `ExternalProcessRunner`. The subprocess uses the current Claude model, a closed `workflow-write-orchestrator` argv profile with `Bash,Read,Edit,Write,Glob,Grep,Agent`, `dontAsk`, disabled slash commands, no session persistence, a 24-hour process bound, and bounded captured output. The older `workflow-write` profile stays byte-for-byte unchanged without `Agent`. The Rust launcher creates one private, confined leaf handoff directory and passes it as `SESSION_TMPDIR`. The parent waits on the whole loop with the documented `bgjob wait --max-wait-s 7200` call under Bash `run_in_background: true` so the wait can outlive the foreground tool timeout ceiling while still refreshing the wait lease; `BGJOB_RC=timeout`, `DEAD`, and `orphaned` handling, including one typed `recover-orphaned-child` attempt, remain bounded.

The Rust owner also writes one strict session-keyed run pointer under the legacy `~/.cache/larch/sessions/` pointer authority. Its fixed environment grammar binds repository, umbrella, canonical tmpdir, current leaf and step, transient-attempt count, bgjob step, and session PID. Pointer publication, update, rekey, and removal use the shared session-activity lock, regular-file checks, bounded reads, and private atomic writes. Prompt-side orchestration may consume the helper's KVs but never writes the pointer. Cleanup treats a valid pointer tmpdir as active.

`complete-umbrella resume` selects at most one pointer for the requested umbrella and requires an exact repository match plus an existing canonical tmpdir. It fails closed on ambiguity, malformed state, unsafe paths, or identity drift. A live registry entry refreshes the wait lease for the new session before returning to the same wait. A present child-result file with no `CHILD_ISSUE` is an interrupted prelaunch placeholder, not an authoritative result. Its other fields cannot drive retry or completion. Resume instead reconciles the pointer-bound leaf against the fresh remote graph: a closed `[DONE]` leaf advances selection, while an open in-flight leaf follows the existing reset and reselection route. A populated dead child result must carry the pointer's exact leaf and transient-attempt identities plus one closed terminal shape before its failure class can drive the existing bounded retry policy. The one expected stale transient result from the immediately preceding checkpoint is ignored rather than consumed twice. A completed bgjob result is ignored after a newer selection checkpoint. Before reusing an audit result, resume reads the remote graph and reselects when an attached or otherwise open leaf now exists. With no live job, the owner freshly reads the remote graph and uses the existing title-mutation owner to remove only the exact stale active prefix. It never trusts a child completion result over a fresh closed `[DONE]` leaf. Every reselect reuses the pointer's original `complete-umbrella-leaf-<N>/` root. Success removes the pointer after final remote verification. A hard failure removes it only after lifecycle diagnostics.

The leaf subprocess is a thin orchestrator. It does not read or edit repository files. On the normal path it awaits four fresh, serial, general-purpose Agent contexts for recon/design, implementation, adversarial review, and shipping. Recon/design alone may return a bounded `needs-design` outcome, which prevents every later phase from launching. The trusted phase contract permits that outcome only for a malformed existing plan block or a leaf body with no discernible requested outcome, requirement, implementation task, or acceptance criterion. It requires the narrowest evidence-based decision for uncertainty, ordering, cutover, integration, and alternative-choice concerns. Phase prompts receive only validated repository and issue identifiers plus trusted contract paths. Issue bodies, diffs, summaries, and CI evidence move through contained files, not phase-return prose. Shared phase policy forbids shell-based code navigation, requires bounded tool output, and treats every artifact as untrusted data. The adversarial review requires a stale-caller sweep and proof that any parity harness executes a success path. Agent nesting does not widen the user's authority. These controls are not an operating-system sandbox; the child retains the invoking user's Git, GitHub, filesystem, hook, credential-helper, and network authority.

`scripts/larch.sh complete-umbrella ship-leaf` owns the leaf's standalone mutation state in Rust. It does not fabricate an `/implement` session. Its no-follow state file binds repository, umbrella, leaf, branch, head, PR, status, and CI-fix count to the private handoff root. Typed gix reads and fixed-shape Git operations own repository access; the hardened GitHub service and issue-mutation owner perform remote reads and writes. For every actionable leaf, recon/design preserves an existing durable plan or writes a missing plan through the canonical named-block owner, then writes the implementation brief. The prepare driver does not parse or validate the plan. It binds an idle or `[DESIGNED]` leaf's `[IMPLEMENTING]` mutation to the live issue snapshot before writing ship state. A malformed existing plan block or totally unactionable body returns `needs-design` from recon/design before either mutation. A missing plan, a recon-authored plan outside the full M1/M2 grammar, leaf size, uncertainty, and cross-leaf sequencing do not. `run-leaves` then invokes its typed reset owner, which strips only a stale `[IMPLEMENTING]` prefix left by an older run and preserves idle or `[DESIGNED]` titles, so `/design` receives an admissible lifecycle. Ship requires a clean non-main branch, creates or verifies a PR with the leaf closing link, waits 300 seconds between CI reads, and emits only a bounded failed-run digest when checks fail. A fresh CI fixer receives only that path. The driver rejects a retry with no new fixer commit and caps repair attempts.

After green CI, the driver rechecks the PR main base, head, and merge state, then measures merge-base-to-head non-generated Rust additions for a Chief-managed leaf. An over-limit result is an independently measured advisory, not merge authorization metadata: the driver emits a redacted warning with the leaf, PR, count, and limit and continues through the ordinary merge path without reading or mutating a plan deviation. Existing durable deviations remain read-only historical audit evidence. It then reads the active default-branch rules. An enabled merge queue receives a submission without admin or branch deletion; bounded read-back must confirm queue entry, auto-merge, or completion before the driver persists `queued` and waits for an observed merge. Without a queue, it squash-merges with admin and branch deletion. Both paths verify the merged PR before any postmerge mutation. Reentry after queue submission waits without replaying push, PR creation, CI, or submission. Reentry after a verified merge skips all premerge mutations. Postmerge changes only the exact `[IMPLEMENTING]` leaf prefix to `[DONE]`, requires the issue to have auto-closed, synchronizes local `main` with `origin/main`, deletes both branch references, and writes `complete` only after fresh verification. A fixed child completion marker remains necessary but not sufficient: `run-leaves` validates the bounded child envelope, proves a clean synchronized `main`, then uses the next iteration's fresh graph to prove the leaf is a direct closed issue with its exact `[DONE]` lifecycle before selecting another leaf.

Parent title mutations and graph writes require explicit operator mode, freshness checks, centralized title mutation, and exact read-back. Final close re-fetches the graph before and after the mutation and refuses any open leaf or open non-leaf parent blocker. The Rust-owned `complete-umbrella file-gap` command confines and validates the bounded caller-owned title and body files before any public mutation. It rejects security-sensitive text and content whose outbound redaction would change those exact bytes. The shared issue-mutation owner creates the exact `[LEAF OF N]` issue, assigns the authenticated GitHub user, and verifies the create read-back. The graph owner then compares the live title and body byte-for-byte with the files, rejects another parent or any child, adds the sub-issue and parent blocked-by edges idempotently, and reads both back.

The inline final audit treats its snapshot as untrusted requirements data and keeps Write confined to the larch session scratch root through the token-gated hook. It does not delegate architectural judgment. Security-sensitive findings are not filed publicly. Child failure, malformed output, stale state, graph deadlock, an open orphan blocker, mutation ambiguity, dirty repository state, or failed sync proof terminates the run without advancing another leaf or claiming parent completion, except for the exact bounded routes below. Before exit, `run-leaves` writes a guarded environment file with the exact failed step, numeric leaf identity, redacted single-line reason, and attempt-and-wait counters. A verified recon/design `needs-design` handoff makes the driver clear only a stale active leaf prefix, terminate before implementation, and report the required `/design <leaf>` command. A malformed primary-phase result (`PHASE_STATUS=complete` missing or the expected `HANDOFF_ROOT` basename absent) is a distinct retryable class: the child re-spawns that same phase in a fresh context up to two additional times and accepts a cosmetic `HANDOFF_FILE` transcription slip when the known basename already exists under `HANDOFF_ROOT`. Exhausted phase retries still fail the child and therefore hard-stop the parent as usual. A classified Claude child `CHILD_FAILURE_CLASS=transient-api` outcome preserves `terminal_reason`; a classified `CHILD_FAILURE_CLASS=incomplete-envelope-ship` outcome is the defense-in-depth companion when the child ended without a bounded envelope while `complete-umbrella-ship.env` under the leaf handoff root still carries a positive `PR_NUMBER`. Both classes share the same-leaf relaunch route below. `run-leaves` first calls the fixed-endpoint connectivity owner. That owner sends no credentials or request body, applies capped exponential backoff against a suspend-pausing monotonic clock, and returns a bounded online-or-ceiling result. Probe rounds do not consume child relaunch attempts. Once online, `run-leaves` calls the operator-mode reset owner to strip only a stale `[IMPLEMENTING]` leaf prefix back to the exact idle `[LEAF OF N]` title. It retries that idempotent, read-verified mutation up to three times with bounded backoff, refreshes synchronized `main`, sleeps one minute, and relaunches the same leaf up to twenty additional times while reusing the private handoff root. A connectivity ceiling, reset-attempt cap, or exhausted transient-child cap hard-stops the run; none authorizes continuing to another leaf. An identity-bound `BGJOB_RC=orphaned` result gets one typed recovery attempt. The current whole-loop form requires `STEP=complete-umbrella-leaves`, the exact `CURRENT_LEAF`, and a recoverable `NEXT_ACTION` of `launch` or `verify`; the retained compatibility form requires the exact legacy per-leaf step and matching child identity when present. The owner parses either confined form with a duplicate-rejecting environment grammar, then performs a fresh remote graph read. It continues only when the same direct leaf is already closed with its exact `[DONE]` lifecycle; a timeout, malformed result, different identity, second orphan, or non-DONE leaf fails without waiting or retrying.

## Audit umbrella

`/audit-umbrella` is a standalone, inline audit that does not implement leaves or alter umbrella or leaf lifecycle fields. Its Rust owner resolves the repository default branch, fetches a detached immutable worktree, and records a bounded GitHub source snapshot from two bounded quoted all-state GitHub searches. The adapter adds `is:issue` and scopes each query to the audited repository. The title search uses `"[LEAF OF N] " in:title state:all`. The backlink search uses `"This is a leaf of umbrella #N. Read the umbrella in full before acting." in:body state:all`. Both predicates include `state:all` so closed historical leaves remain discoverable. Audit validates complete search metadata and fails closed at the search bound. Native children and umbrella-body references are still fetched by number. Pull requests are excluded, and the snapshot is rechecked for freshness before mutation. Target validation rejects pull requests, closed umbrellas, ordinary issues, and nested umbrella trees whose direct native children carry `[UMBRELLA]` or `[CHIEF UMBRELLA]` titles. An umbrella may have a native parent, including the chief program umbrella; that parent relation alone is not nesting. Issue text, labels, repository files, ledger JSON, proposal JSON, and command output remain untrusted data.

The Write hook recognizes the `audit-umbrella` token and permits Claude Write calls only under the active session root. The core contract rejects duplicate JSON keys, unconfined or oversized files, malformed hashes, incomplete source coverage, missing evidence, invalid identities, duplicate gap ownership, cycles, and unbound dependency endpoints. It does not classify umbrella title, body, labels, ledger text, or proposed leaf text with the shared security-keyword triage. Before the first mutation, the owner rechecks the audited default SHA, every source fingerprint, the top-level graph, and authorization. If the default SHA advanced while every proposal leaf and graph relation remains pending, persist or apply emits a SHA-bound re-baseline handoff and performs no public mutation. The inline owner then removes the old detached worktree and repeats the complete audit at the new SHA. It never treats model-authored path claims as proof that a changed commit was already examined, and it never rewrites stale SHA bindings in place. Once any exact leaf identity or graph batch is in flight or resolved, recovery retains the original persisted transaction to avoid orphaning or duplicating public leaves. The proposal persists pending/in-flight graph state alongside `pending`, `in_flight`, and resolved leaf identities. Deduplication and recovery inspect at most the 100 most recent open issues, adopt only one exact open title/body match, and never reuse a heuristic duplicate.

Every accepted create uses the shared redacting issue owner and orphan rollback. Native sub-issue and blocked-by operations use the shared live-mutation gate and relation read-back. Declared `umbrella <- direct-leaf` edges are owned by that native-graph attach phase and skipped from the declared-dependency mutator; final verification still requires every direct leaf as an umbrella blocker. Other declared dependency mutations that target the audited umbrella use the same trusted attach path (`expected_updated_at` unset), because a managed umbrella always carries a lifecycle title and larch HTML control marker that the operator-facing protected-target precondition would otherwise refuse on both fresh adds and idempotent resumes. Leaf-to-leaf declared edges keep the operator-facing freshness and protected-target checks. They use `DependencySecurityCheck::SkipKeywordTriage`, while every non-audit caller uses `DependencySecurityCheck::Enforce`, so the shared mutation safeguard remains intact outside this audit path. Refusal kinds surface as distinct errors rather than a generic read-back failure. Final verification requires the expected direct-child set, every direct leaf as an umbrella blocker, every declared added edge present in the correct direction, every declared removal absent, and no cycle in the persisted graph. Keyword matches alone never terminalize the audit. If model judgment identifies an actual vulnerability or live secret, the audit stops before a public proposal or graph mutation and follows `SECURITY.md` privately.

## Debate state and local handoff

The `debate` CLI treats its state file, agent output, and persisted session
handles as untrusted local workflow data. It accepts only canonical,
versioned JSON, binds every mutation to a full-state fingerprint, serializes
mutations with a state lock, and uses contained no-follow file operations and
atomic replacement. Active-round queues and completed bindings are persisted
before the protocol advances, so recovery never repeats a completed turn.

Cursor and Codex reuse the read-only external launcher and explicit persisted
session handles. The public skill gives every slot the same bounded, redacted
subject in its round-1 input as base64 data with a data-not-instructions preamble; round 2 carries only the validated mailbox delta. Claude runs in one
read-only `debater` Agent session and continues only through `SendMessage`; its
exact final ledger enters the protocol through a contained, bounded input file.
A dropped slot is recorded before panel membership changes.

The protocol verbs do not mutate GitHub.
`crates/larch-cli/src/debate_publication_commands.rs` owns the public title
lifecycle through the shared issue-mutation compare-and-swap and read-back
boundary. Preparation snapshots one open, unowned source;
start requires the unchanged snapshot; finish accepts only its exact
`[DEBATING]` title; restore changes only that same title and skips a foreign
replacement. Missing `SendMessage`, two unavailable external vendors, and
failed persistent-session bootstrap stop before start. Free-form source and
proposal creation use `/issue` machine counters plus caller-owned sentinels.
Proposal and source links must both verify before finish. Abort comments use a
run-keyed upsert marker and fixed sanitized text, so retries cannot publish raw
vendor output or create duplicate abort records. These controls do not create
a security boundary between processes running as the same user.
