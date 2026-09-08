# Rust Async Runtime

Larch uses one Tokio runtime per `larch` process. `larch-cli` owns that runtime
as part of the composition root. Synchronous command parsing may call
`LarchRuntime::block_on` once, at the top level. Libraries and command handlers
must stay asynchronous and must not create or nest runtimes.

## Decision

Use Tokio 1.53.0 with only the I/O utility, macros, process, runtime, signal,
synchronization, test-time, and time features. Use `tokio-util` 0.7.18 for
hierarchical cancellation tokens. Both crates are maintained, pure Rust, MIT
licensed, and support an older Rust version than larch requires.

Tokio owns the facilities larch needs as one integrated stack: a multi-thread
executor, deterministic timers, signals, synchronization, and task sets. Later
process and I/O leaves may add their Tokio features only when they add a real
consumer.

Rejected alternatives:

- `async-std` 1.13.2 is deprecated in favor of `smol`.
- `smol` 2.0.2 is small, but larch would need separate crates and ownership
  conventions for signals, process handling, cancellation, and task tracking.
- A custom standard-library executor would duplicate scheduling, waking,
  timers, and platform signal behavior without improving larch's contracts.

## Cancellation and deadlines

Every command creates one root `Cancellation`. Each concurrent operation gets
a child token. Cancellation flows from parent to child, never from child to
parent or sibling. Cancellation is cooperative. Async loops and external I/O
must observe their token at bounded intervals.

`run_until` gives cancellation priority over a simultaneously ready timeout.
Timeouts remain caller-owned policy. A shared primitive never chooses a domain
timeout. `larch-core` defines injectable `BusinessClock`, `MonotonicClock`, and
`AsyncClock` ports. A propagated `Deadline` can create a shorter child deadline
but can never outlive its parent. Production adapters use `SystemClock` for
business timestamps and `TokioClock` for deadlines and sleeps.

`RetryExecutor` owns the async retry loop. Domain code classifies typed errors
as a closed `RetryDecision`; usage, authorization, and known permanent failures
stop without another attempt. `RetryPolicy::default` owns the bounded attempt
and full-jitter exponential-backoff defaults. Callers inject the jitter source,
clock, cancellation token, optional deadline, and observation sink. Retry
observations contain only closed classes, attempt counts, and bounded delays.
They never contain the operation error or response payload.

The inactive vendor migration surface keeps one narrower compatibility loop in
`larch-core::run_with_vendor_retries`. It preserves the legacy independent
authentication, transient, and empty-response budgets and their precedence.
Execution and delay remain injected effects. This domain contract does not
select a runtime, spawn a process, or replace `RetryExecutor` for services.

## Task ownership and concurrency

Use `TaskSet` for concurrent work. Its non-zero capacity is the explicit
concurrency bound. The caller owns the set and must join results or call
`shutdown`. Dropping a set cancels and aborts its tasks, so a library task
cannot detach from its owner.

Shutdown follows one sequence:

1. Cancel the task-set token.
2. Allow cooperative cleanup until the caller's grace period expires.
3. Abort remaining tasks.
4. Join every task and return completion, abort, and panic counts.

Use a separate `TaskSet` for each ownership domain. Do not place unrelated
background jobs and request work in one set merely to share a limit.

## Signals and child processes

The composition root listens for SIGINT and, on Unix, SIGTERM. The first signal
cancels the command root. Normal task-set shutdown then enforces the command's
grace period.

The process adapter must implement `ChildProcess` for an owned child process
group. It must capture group identity at spawn time. On cancellation it asks
the group to stop, waits for the caller's grace period, kills the group if
needed, and reaps the child. Persisted process identities require the
[local mutation safety](security/workflow-trust-and-mutations.md#local-mutation-safety)
re-verification rules before any signal. The generic
`shutdown_child` helper owns only the graceful, deadline, force, and reap
sequence. It does not choose platform signals or accept arbitrary process IDs.

`TokioProcessRunner` is the sole product owner of process construction. The
core `ExternalProcessRunner` port accepts a typed executable and argument
array, never a command string. The request owns an absolute working directory,
typed environment overrides, stdin bytes, per-stream capture limit, timeout,
and shutdown grace period. The adapter clears the ambient environment and
copies only common allowlisted keys. Vendor credentials require an explicit
typed override and never enter the common inheritance list. On Unix, each
child starts in its own process group. Before cancellation or timeout sends
SIGTERM, the adapter snapshots the descendant tree and binds each separate
group to kernel process-birth identities. It signals descendant groups from
deepest to shallowest, then the direct child's group. Forced cleanup refreshes
that snapshot and revalidates a saved member before each SIGKILL, so
reparenting cannot let a nested group escape and PID reuse cannot redirect a
signal. It then reaps the direct child. Other platforms use Tokio's safest
available direct-child kill and reap path.

## Deterministic tests

Production uses the multi-thread runtime. Tests use
`LarchRuntime::paused_current_thread`. Tokio advances paused time only when no
other work can make progress, so timeout and cleanup tests complete without
wall-clock sleep. Inject futures and `ChildProcess` fakes at boundaries. Do not
mock Tokio's scheduler or use global runtime state.
