# step-architectural-invariants-write-compose

Thin `/implement` architectural-invariants helper.

## Contract

Delegates through `scripts/larch.sh architectural-invariants write-compose-assessment` for the Step 8 compose-time invariant note contract.

The helper reads the prompt-authored assessment file, validates that the current `HEAD` still matches the compose-time materialization metadata, writes `architectural-invariant-note.md` plus metadata, and clears retired staged or dropped-note artifacts.

## Tests

`skills/implement/scripts/test-architectural-guidelines-step.sh` pins the prompt-side compose-time prose and durable-note write behavior.
