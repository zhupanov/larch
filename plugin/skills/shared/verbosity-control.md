# Verbosity Control

Skill files keep their own preserved/suppressed category lists and step-specific carve-outs. This anchor owns only the universal verbosity rules shared across callers.

- Use empty string for the `description` parameter on all Bash tool calls.
- Use terse 3-5-word descriptions for Agent tool calls.
- Do not produce explanatory prose between tool outputs beyond the preserved categories listed in the calling skill.
- Verbosity suppression is prompt-enforced and best-effort.

## Update triggers

Update this file when `/design` or `/implement` shared verbosity rules change. Keep skill-specific preserved/suppressed categories and step-specific carve-outs in the calling skill files.
