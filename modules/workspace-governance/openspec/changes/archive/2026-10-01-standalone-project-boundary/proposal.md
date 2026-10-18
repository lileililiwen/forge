# Proposal: standalone project boundary

## Why

The workspace contains many related repositories, but repository metadata does
not currently provide one explicit, auditable distinction between a standalone
project, a library consumer, and a project that requires sibling checkouts or
private services. That ambiguity makes open-source donation and independent
maintenance difficult.

## What Changes

Add a fail-closed standalone-readiness contract to Workspace Governance. The
audit will classify a project using repository-relative inspection and declared
metadata, while preserving the existing distinction that declarations are
intent and native commands/evidence prove operation.

The contract will expose a stable status, findings, observed revision, and
optional-integration declarations for Forge and other consumers.

## Package Boundary and Split Assessment

This package has one owner, one policy lifecycle, and one audit oracle: whether
a repository can operate without sibling project checkouts. It does not add
Forge storage/UI, execute Driftwatchdog, publish libraries, or modify product
repositories. Forge consumption is a separate package because it has a Rust
SQLite/web lifecycle.

Dependency order:

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `standalone-project-boundary` | A repository-relative independence classification and findings | Workspace Governance, Python | JSON audit result and `.project.json` optional-integration fields | Existing registry/schema | Fixture audit tests and self-check |
| `standard-pack-registry-and-snapshots` | Versioned structure/CI/quality/Compose packs generate standalone projects | Forge, Rust | Profile/pack descriptors and snapshot receipt | Existing profile registry and generation | Rust contract tests and generated-project checks |
| `portfolio-metadata-and-review` | Forge stores horizontal tags, relationships, reviews, and imported evidence | Forge, Rust/SQLite/HTML | Additive SQLite schema and portal projection | Existing inventory, provider contract, and `portal-web-ui` | Migration, API, and portal contract tests |

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| Workspace registry/checker | `scripts/workspace_check.py`, `schemas/project.schema.json` | Discovery, declarations, findings, root-relative audit | No independence vocabulary | Workspace Governance | **Extend shared owner** |
| Forge governance adapter | `scripts/forge_governance_adapter.py` | JSON-line observation boundary and redaction | It currently carries governance findings only | Workspace Governance producer / Forge consumer | **Adopt** through the existing adapter |
| Driftwatchdog | `docs/capabilities.md` and Forge provider contracts | Gate execution and evidence export | It does not own repository independence policy | Driftwatchdog | **Adapt through a generic adapter**; no code change here |
| Runtime template library | `templates/runtime`, `scripts/runtime_templates.py` | Compose/runtime families and rendering safeguards | It does not define project independence | Workspace Governance | **Adopt** as a referenced runtime-pack source |

## BFS Impact Map

| Surface | Impact |
|---|---|
| Declaration schema | Additive optional `standalone` and `integrations` fields; existing declarations remain valid |
| Audit | New findings for sibling paths, absolute paths, mandatory external services, and missing local defaults |
| Adapter | Additive normalized observation fields; existing governance statuses remain compatible |
| Forge | Consumer only; no Forge implementation in this package |
| Deployment | Standalone status does not promote `deployable` and does not replace release evidence |
| Tests | Positive, negative, malformed, unavailable, and legacy-declaration fixtures |
| Security | Never execute discovered commands during this audit; redact credential-shaped values |

## Capabilities

### New Capabilities

- `standalone-project-boundary`: repository-relative independence classification.

## Non-goals

- No automatic refactoring of coupled projects.
- No dependency installation or sibling checkout discovery beyond reporting.
- No runtime health or release verification.
- No Forge database or web UI.
- No mandatory use of Workspace Governance by Forge or product repositories.
