# Security Policy

## Policy Scope

This policy covers the latest released larch plugin, including its runtime-only
plugin projection. It is the stable public entry point for supported versions,
responsible disclosure, security scope, and the high-level trust model. The
[security reference index](docs/security/README.md) maps detailed technical
security contracts to one canonical owner.

## Supported Versions

| Version | Supported |
|---------|-----------|
| Latest  | Yes       |
| Older   | No        |

Only the latest released version receives security updates.

## Reporting a Vulnerability

If you discover a security vulnerability in larch, please report it responsibly:

1. **Email**: Send details to <zhupanov@yahoo.com>
2. **Do not** open a public GitHub issue for security vulnerabilities
3. Include steps to reproduce the issue and any relevant context

You should receive an acknowledgment within 72 hours. We will work with you to
understand the issue and coordinate a fix before any public disclosure.

## Security Overview

Larch runs with the operator's permissions inside Claude Code. It treats
repository content, GitHub content, model output, and external-tool output as
untrusted data at workflow boundaries. Mutation and publication paths use
explicit authorization, bounded inputs, validation, and redaction. See
[Workflow Trust, Mutation, and Private Findings](docs/security/workflow-trust-and-mutations.md)
for the canonical technical contracts.

Larch verifies release provenance, dependency policy, archives, executable
identity, and atomic installation before it runs a downloaded binary. Operators
provide credentials through documented environment variables or standard
Application Default Credentials. Typed service adapters constrain credentials,
hosts, operations, redirects, retries, response sizes, and diagnostics. See
[Supply Chain, Credentials, and Services](docs/security/supply-chain-credentials-and-services.md)
for the canonical technical contracts.

Larch-owned Claude, Codex, and Cursor binary children use one typed Rust process
boundary. Ambient child environments are cleared, vendor credentials require
explicit typed overrides, output and diagnostics are bounded and redacted, and
timeouts or cancellation terminate and reap the owned process group, including
descendants on Unix. Skills, hooks, and scripts do not provide an alternate
vendor-launch path.

Session artifacts, operator diagnostics, remotely archived run logs, and public
GitHub content have distinct confidentiality rules. The universal skill
lifecycle sanitizes every terminal outcome before create-only publication and
keeps child invocations in separate archives. Redaction and scanning are
egress backstops, not complete content classifiers. See
[Artifacts, Redaction, and Publication](docs/security/artifacts-redaction-and-publication.md)
for the canonical technical contracts.

These controls do not make larch a sandbox against hostile processes running as
the same operating-system user. Provenance proves how release bytes were built,
not that the source or build infrastructure is trustworthy. Checksums prove
integrity, not trust. Delegated tools may receive workspace access when a
workflow permits it. The [security reference index](docs/security/README.md)
maps the remaining trust boundaries and known limitations.
