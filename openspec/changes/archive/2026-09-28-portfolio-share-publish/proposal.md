# Proposal: Publish an approved public portfolio catalog

## Why

Forge needs to convert private project knowledge into a controlled public
showcase. The GitHub Pages site must receive only an explicit, reviewable
allowlist of project metadata and URLs; it must never become Forge's database,
permission system, or secret store.

## What Changes

- Add a Forge-owned share record and approval flow for project showcase data.
- Produce a deterministic public manifest for the GitHub Pages publisher.
- Reuse the existing project registry, portfolio metadata, publish engine, and
  fleet-liveness evidence instead of duplicating them.
- Support a dry-run preview, explicit approval, idempotent publish operation,
  and auditable manifest revision.

## Package Boundary and Split Assessment

The single outcome is “an administrator can approve and publish a safe public
portfolio manifest.” The package includes Forge persistence, authorization,
preview, manifest generation, and the generic publisher boundary. It excludes
Hugo rendering, GitHub credentials, project authentication, analytics, billing,
and implementation of a project's public routes.

The request spans different runtimes, so it is split into dependency-ordered
packages:

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `public-portfolio-manifest` | Stable manifest schema and fixtures | `platform-contracts`, JSON Schema | Versioned JSON manifest | none | Schema and fixture validation |
| `portfolio-share-publish` | Approved Forge export/publish | Forge, Rust/SQLite/HTTP | Manifest contract and publisher adapter | manifest, portfolio metadata | Preview/export/publish integration tests |
| `public-portfolio-catalog` | Public Hugo catalog renders | GitHub Pages, Hugo/Go templates | Checked-in manifest | manifest | Build and rendered-output tests |
| `portfolio-interest-snapshots` | Privacy-safe interest evidence informs prioritization | Forge, Rust/SQLite | Aggregate snapshot import | share-publish, analytics adapter | Import, retention, and authorization tests |

This package is the smallest independently verifiable Forge control-plane
unit. The analytics package remains separate because it has a different data
owner, privacy lifecycle, and acceptance oracle.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| Forge registry | `src/project_registry.rs`, `forge-independent-project-inventory-fleet` | Project identity, source revision, public HTTP inventory | Does not define public showcase approval | Forge release | **adopt** |
| Forge publish engine | `src/publish/`, `forge-publish-plugin-orchestration` | Idempotent operation and optional providers | Needs a GitHub Pages manifest adapter | Forge release | **extend shared owner** |
| Forge liveness | `fleet-liveness-status`, `src/liveness.rs` | Online/degraded/offline/stale evidence | Must be represented as evidence, not invented health | Forge release | **adopt** |
| Platform contracts | `/home/paul/code/platform-contracts` | Language-neutral schemas and fixtures | Manifest contract is not yet present | Platform-contracts release | **extract recurring capability** |
| GitHub Pages | `/home/paul/code/lileililiwen.github.io`, Hugo layouts/data | Public static rendering and deploy workflow | Must consume generated data, not private Forge state | Site release | **adapt through a generic adapter** |
| Workspace governance | `/home/paul/code/workspace-governance` | Policy/discovery declarations | Not a portfolio publisher or product database | Governance release | **keep local** |

## BFS Impact Map

| Area | Impact |
|---|---|
| Actor/flow | Admin selects projects and public surfaces, previews, approves, publishes; public visitor only sees exported records |
| Data | Share record, allowlisted surfaces, manifest revision, export hash, publisher result |
| Contracts | `platform-contracts` manifest; generic Forge publisher adapter; no shared auth/session |
| Integrations | GitHub Pages adapter is optional; local file export remains the default-safe path |
| Failures | Invalid URL, secret-like field, stale metadata, unauthorized approval, publisher timeout, partial publish |
| Security/privacy | Allowlist only; no local paths, credentials, internal ports, private notes, raw analytics, or admin URLs |
| Tests | Unit, authorization, manifest golden, idempotency, malformed-input, adapter contract, audit tests |
| Unaffected | Project runtime auth, project data, Hugo templates, Cloudflare Tunnel, payments |

## Capabilities

- `share-records`: admin-owned public metadata and surface allowlist.
- `public-manifest-preview`: deterministic preview and validation findings.
- `approved-manifest-publish`: idempotent publication through a generic adapter.
- `publication-audit`: actor, revision, hash, result, and failure reason.

## Non-goals

- No automatic publication from a project directory or Git push.
- No shared account, cookie, database, or secret between Forge and products.
- No claim that a reachable demo is production-ready.
- No analytics, lead scoring, payment, CRM, or visitor tracking in this change.
