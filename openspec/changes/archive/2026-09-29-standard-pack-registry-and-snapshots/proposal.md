# Proposal: versioned standard pack registry and standalone snapshots

## Why

Forge already generates profile projects, while Workspace Governance and
runtime templates provide related reusable conventions. The conventions need
to cover structure, local verification, CI, quality, Compose, and optional UI
tokens without making generated projects depend on sibling repositories.

## What Changes

Add versioned standard-pack descriptors to Forge. A pack selects or extends a
profile and renders a repository-local snapshot with a receipt, verification
script, CI workflow, quality configuration, and Compose family. Forge can
inspect, compare, and explicitly upgrade that snapshot; generated projects
remain standalone after creation.

## Package Boundary and Split Assessment

This package owns one lifecycle: selecting a versioned pack and materializing a
standalone project snapshot. Portfolio tags/reviews are separate persistence
and UI work. Governance auditing remains Workspace Governance's policy
boundary.

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `standalone-project-boundary` | Independence policy and findings | Workspace Governance, Python | Governance adapter observation | Existing registry | Audit fixtures |
| `standard-pack-registry-and-snapshots` | Versioned standard snapshots in generated projects | Forge, Rust/SQLite/assets | Pack descriptor and receipt | Existing profile registry, generation, runtime template contract | Render/diff/upgrade contract tests |
| `portfolio-metadata-and-review` | Horizontal project management data and UI | Forge, Rust/SQLite/HTML | Additive portfolio schema/read model | Existing inventory, provider contracts, `portal-web-ui` | Migration and HTTP contract tests |

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| Forge profile registry | `src/profile/`, `profile-registry` spec | Versioned profile descriptors and generation inputs | Does not include cross-profile standards | Forge | **Extend shared owner** |
| Forge generator | `src/generate/`, `deterministic-project-generation` | Deterministic rendering and ownership receipts | Needs pack-level files and upgrades | Forge | **Extend shared owner** |
| Workspace runtime templates | `workspace-governance/templates/runtime`, `runtime_template_library` | Typed Compose/runtime families and safe rendering | Python-owned provider and no Forge receipt | Workspace Governance | **Adapt through a generic adapter** |
| Platform libraries | `dotnet-platform-libs`, `rust-platform-libs` | Reusable implementation libraries | Libraries are not project scaffolds | Platform owners | **Adopt only as profile dependencies** |

## BFS Impact Map

| Surface | Impact |
|---|---|
| Pack registry | New versioned descriptors, compatibility, support state, and asset references |
| Generation | Additive `.standard/` snapshot and receipt; existing generated source remains ordinary source |
| Upgrade | Read-only inspect/diff first; explicit upgrade with conflict refusal |
| CI/quality/Compose | Rendered local files; no required Forge runtime during project verification |
| SQLite | Registry/operation records for pack metadata only; project source is not stored |
| CLI/API/portal | CLI first; later callers consume the same Core contracts; no portal implementation in this package |
| Failure | Unsupported pack, missing asset, modified generated file, and incompatible target are typed failures |
| Security | No secrets, network fetches, or arbitrary template commands |

## Capabilities

### New Capabilities

- `standard-pack-registry-and-snapshots`: versioned standards materialized into standalone projects.

## Non-goals

- No mandatory Forge dependency after generation.
- No submodules, sibling source imports, or machine-specific paths.
- No automatic upgrade or silent overwrite.
- No universal CSS component library in this package.
- No central CI service replacing local project verification.
