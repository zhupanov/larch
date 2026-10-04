# Shared `--run-id` Flag

`--run-id <ID>` supplies an optional stable run identifier for this invocation.
When omitted or empty, the caller auto-generates one.

The consuming skill or Rust parser owns validation for the accepted ID shape.
Some skills consume `--run-id` locally instead of forwarding it to child CLIs.

## Update triggers

Update this file when the shared `--run-id` purpose, default, validation ownership, or forwarding convention changes.
