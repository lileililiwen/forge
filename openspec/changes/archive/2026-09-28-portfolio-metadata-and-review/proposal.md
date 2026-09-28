# Proposal: portfolio metadata and review

## Why

Repository metadata describes local technical facts but cannot express the
horizontal information needed to manage a portfolio: tags, goals, project
relationships, confidence, blockers, next actions, and review history. Forge's
portal needs a durable local read model that combines these user-owned facts
with imported governance and verification observations.

## What Changes

Extend Forge's SQLite registry with a separate portfolio metadata domain and
projected portal views. The domain stores user classifications and timestamped
external evidence snapshots. Repository declarations, Workspace Governance
findings, Driftwatchdog results, and runtime observations remain source-owned
and are imported with source/revision/time metadata.

## Package Boundary and Split Assessment

This package has one owner and one persistence lifecycle: local portfolio
classification and review. It depends on the existing `portal-web-ui` package
for browser delivery and does not reimplement governance, gate execution, or
deployment. The SQLite schema is deliberately local-first; PostgreSQL is a
future backend implementation only after a multi-user requirement exists.

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `standalone-project-boundary` | Independence classification | Workspace Governance, Python | Governance observation | Registry/checker | Audit fixtures |
| `standard-pack-registry-and-snapshots` | Versioned standalone project standards | Forge, Rust/assets | Pack/receipt contract | Profile/generator | Generation tests |
| `portfolio-metadata-and-review` | Tags, relations, reviews, and imported evidence in Forge | Forge, Rust/SQLite/HTML | Additive schema and portal view model | Inventory, provider contracts, `portal-web-ui` | Migration/API/portal tests |

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| Forge project registry | SQLite registry and `src/registry/` | Project identity, migrations, operation persistence | Portfolio fields must not alter governance identity | Forge | **Extend shared owner** with separate tables |
| Existing portal | `portal-web-ui`, `control-plane-portal` | HTML routes, auth, read models, accessibility/error conventions | No user-owned portfolio metadata yet | Forge | **Extend shared owner** after portal change |
| Governance adapter | `src/governance/` and provider contracts | Normalized external observations, unavailable semantics | It is read-only and source-owned | Forge | **Adopt** without copying policy |
| Driftwatchdog | `gate-runtime-evidence` and evidence-export contract | Verification evidence and stale revision semantics | Execution belongs to Driftwatchdog | Driftwatchdog | **Adapt through generic provider** |
| Separate portfolio manager | None required | No existing compatible persistence surface | Would create another registry/UI | New project | **Keep local** in Forge to avoid duplication |

## BFS Impact Map

| Surface | Impact |
|---|---|
| SQLite | Additive tables/migrations for tags, relations, goals, reviews, next actions, blockers, and evidence snapshots |
| Import | Read-only import from local inventory and configured providers; source revision/time retained |
| Portal | Extend existing list/detail views with filters, labels, confidence, next action, stale evidence, and relationship links |
| CLI/API | Add Core query/mutation contracts before or alongside portal projection; existing outputs remain compatible |
| State | User metadata is editable; external observations are append-only snapshots and never edited as source facts |
| Failure | Duplicate tag, invalid relation, unavailable provider, stale snapshot, malformed import, and unknown project are typed outcomes |
| Security/privacy | Local database by default; no secrets or source contents persisted; authorization follows existing Forge API rules |

## Capabilities

### New Capabilities

- `portfolio-metadata-and-review`: user-owned horizontal project management data and evidence snapshots.

## Non-goals

- No PostgreSQL server, multi-user collaboration, SSO, or remote synchronization in this package.
- No replacement for `.project.json`, Workspace Governance, Driftwatchdog, or runtime providers.
- No automatic project prioritization or AI-generated classifications.
- No source-code editing from the portfolio UI.
- No requirement for sibling repositories to be present.
