---
name: relevant-checks
description: Run repo-specific validation checks based on modified files. Use when you need to validate code quality after implementation, after code review fixes, or when fixing CI failures. This skill replaces the old /lint, /test, /format pattern.
allowed-tools: Bash
---

# Relevant Checks

Run validation checks relevant to the files modified on the current branch. This is a repo-specific skill — each repository defines its own `/relevant-checks` with checks appropriate for that repo.

## How it works

File-type detection is used for **gating** (deciding whether to run a check category), not scoping. When a check category is triggered, it runs repo-wide — not just on the modified files.

## Usage

Run the private check script:

```bash
$PWD/.claude/skills/relevant-checks/scripts/run-checks.sh
```

The script automatically detects which file types were modified on the current branch and runs only the applicable checks:

- **Shell scripts (`.sh`)**: Runs `make shellcheck` at the repo root
- **Python files (`.py`)**: Runs `make -C python lint test ruff-format validate-dataclasses validate-no-logging-exception-calls` in the python directory (where `lint` expands to `ruff pylint pyright vulture`). Each target runs individually so all failures are visible at once.

If no recognized file types are modified, the script prints a message and exits successfully.

## Retry semantics

If the script exits non-zero, one or more checks failed. The caller should:
1. Diagnose the failure from the script output
2. Fix the issue
3. Re-invoke `/relevant-checks` to confirm the fix

The script runs all applicable checks even if earlier ones fail, so you can see all failures at once.
