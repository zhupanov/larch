#!/usr/bin/env bash
# git-conflict-files.sh — List files in merge-conflict state with per-stage presence.
#
# Wraps `git ls-files -u` so callers don't invoke `git` directly. Used by
# /implement's Conflict Resolution Procedure Phase 1 to classify each
# conflicted file by which index stages (1/2/3) are present.
#
# Usage:
#   git-conflict-files.sh
#
# Output (stdout, one block per file):
#   FILE=<path>
#   STAGE_1=true|false
#   STAGE_2=true|false
#   STAGE_3=true|false
#
# Exit codes:
#   0 — success (zero or more files listed)

set -euo pipefail

# Avoid associative arrays (not available on bash 3.x / macOS default).
# Parse `git ls-files -u` output into one-line-per-file groups using awk.

git ls-files -u | awk '
{
    # ls-files -u format: <mode> <sha> <stage>\t<path>
    # split meta from path on the tab.
    tab = index($0, "\t")
    if (tab == 0) { next }
    meta = substr($0, 1, tab - 1)
    path = substr($0, tab + 1)
    n = split(meta, parts, " ")
    stage = parts[n]
    if (!(path in seen)) {
        order[++count] = path
        seen[path] = 1
        s1[path] = "false"
        s2[path] = "false"
        s3[path] = "false"
    }
    if (stage == 1) s1[path] = "true"
    else if (stage == 2) s2[path] = "true"
    else if (stage == 3) s3[path] = "true"
}
END {
    for (i = 1; i <= count; i++) {
        p = order[i]
        print "FILE=" p
        print "STAGE_1=" s1[p]
        print "STAGE_2=" s2[p]
        print "STAGE_3=" s3[p]
        print ""
    }
}
'
