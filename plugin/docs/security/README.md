# Security Reference Index

This index owns larch's security document taxonomy and ownership map. It does
not duplicate behavioral guarantees. The root [`SECURITY.md`](../../SECURITY.md)
owns the public policy, supported versions, responsible disclosure, scope, and
high-level trust statement.

## Ownership Rules

- Give each security guarantee one canonical document.
- Link to the canonical document instead of copying its normative text.
- Keep disclosure and security-sensitive triage instructions useful without
  opening a secondary link.
- Use stable headings for important security topics so live links remain
  useful.
- Record current implementation owners and enforcement status in the canonical
  document.

## Document Taxonomy

| Area | Canonical owner |
|------|-----------------|
| Public policy, supported versions, disclosure, scope, and high-level trust | [`SECURITY.md`](../../SECURITY.md) |
| Release provenance, bootstrap, dependencies, credentials, and external service boundaries | [Supply Chain, Credentials, and Services](supply-chain-credentials-and-services.md) |
| Workflow trust, untrusted input, agent access, authorization, and mutation controls | [Workflow Trust, Mutation, and Private Findings](workflow-trust-and-mutations.md) |
| Temporary and published artifacts, redaction, retention, and public publication | [Artifacts, Redaction, and Publication](artifacts-redaction-and-publication.md) |

## Runtime Packaging Contract

`release stage` generates the runtime-only `plugin/` projection in the tagged
projection commit. It is not committed to `main`. The generated tree must
contain the root `SECURITY.md`, this index, and every tracked Markdown file
under `docs/security/`. Projection generation also scans shipped skill
Markdown for `docs/security/*.md` references and fails if a target is absent.
CI generates the same projection in a temporary directory and validates it;
there is no committed projection copy to compare for drift.

Projection generation refuses symlinked output, the repository root or its
ancestors, and repository-local output outside a child of `target/`. The
repo-local `plugin/` target uses the repository confinement guard.

The projection also includes `ARCHITECTURE.md`, its linked Rust references, the
Git, GitHub, and Google service inventories, and the operator documents linked
by the focused security references. Those links therefore resolve in both a
source checkout and an installed plugin.

`crates/larch-cli/src/release_plugin_runtime.rs` is the single projection
content and generation owner. `crates/larch-cli/src/release_stage.rs` places
that output in the tagged projection commit. The canonical release pin contract
lives in [Release content pin](supply-chain-credentials-and-services.md#release-content-pin).

## Live Reference Audit

Use these stable entry points when editing security references. Do not rely on
a fixed repository-wide count.

| Concern | Live entry points | Required destination |
|---------|-------------------|----------------------|
| Vulnerability disclosure | `skills/file-bug/SKILL.md`, `skills/triage/SKILL.md` | Root `SECURITY.md`; keep the no-public-issue instruction inline |
| Installation and runtime loading | `docs/installation-and-setup.md`, `.claude-plugin/marketplace.json`, `crates/larch-cli/src/release_plugin_runtime.rs` | Root policy and all focused references ship in `plugin/` |
| Runtime security decisions | Shipped Markdown under `skills/` | The focused reference that owns the behavior in the installed plugin |
| Contributor policy | `AGENTS.md`, `docs/preparing-your-repo.md` | Root policy for disclosure and scope; focused references for technical behavior; this index for ownership |
| Run-log and breadcrumb operation | `docs/run-logs.md`, `docs/run-log-cli.md`, `docs/run-log-batches.md` | Operator mechanics stay in run-log docs; confidentiality, redaction, and publication rules live in the artifact reference |
| Lint and scanner operation | `docs/linting.md`, `.pre-commit-config.yaml`, `.github/workflows/ci.yaml` | Operator commands stay in lint docs; scanner guarantees and limits live in the artifact reference |
| Public reports and diagnostics | Workflow docs, configuration docs, and shipped skill Markdown | Workflow routing stays with its owner; egress classification and redaction live in the artifact reference |

When editing security references, audit the relevant entry points. Preserve
self-contained safety instructions at public-disclosure and security-sensitive
triage boundaries.
