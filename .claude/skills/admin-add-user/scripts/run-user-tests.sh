#!/usr/bin/env bash
# run-user-tests.sh — Run data integrity tests for admin-add-user.
#
# Intentionally runs only tests/test_user_data.py (the data integrity
# tests for the admin-add-user operation), not the full test suite.
# This is a deliberate scope restriction — the full suite is run by CI.
#
# Usage:
#   run-user-tests.sh
#
# Exit codes:
#   0 — tests passed
#   1 — tests failed

set -euo pipefail

REPO_ROOT=$(git rev-parse --show-toplevel 2>/dev/null || echo "$PWD")
cd "$REPO_ROOT/python"
python3 -m pytest tests/test_user_data.py -v
