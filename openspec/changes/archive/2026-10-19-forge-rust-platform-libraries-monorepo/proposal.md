# Proposal: forge-rust-platform-libraries-monorepo

## Why

Forge is consolidating the maintained platform source projects as ordinary
modules so source ownership, native verification and release automation are
coordinated from one repository without erasing each module's runtime or
package boundary. The Rust platform workspace at
`/home/paul/code/rust-platform-libs` is a clean, independent repository at
`4762746e35b3886b21e2c87bf602aed14bc9dea8`, with 161 tracked files and 30
commits. It has no configured Git remote. It contains eight Rust crates, a
pinned toolchain, a conformance consumer, OpenSpec history and a crates.io
publisher whose first release remains explicitly blocked on publication
preconditions.

The Forge Governance registry currently points at the old sibling path
`rust-platform-libs`. The source's nested GitHub workflows will not execute
after import, and its contract-parity tests currently resolve a sibling
`platform-contracts` checkout. A source copy alone would therefore leave
discovery, CI, schema evidence and future package publication tied to the old
layout.

## What Changes

- Import the complete tracked Rust source tree at `4762746` under
  `modules/rust-platform-libs/` as ordinary files, preserving its 30 source
  commits as Forge ancestry. Do not add a Git submodule or copy ignored build
  artifacts.
- Preserve the nested Cargo workspace, eight crate names, feature graph,
  pinned Rust toolchain, MSRV, license, versions, consumer fixture and native
  formatting, Clippy, test, deny and conformance commands.
- Move module quality and publication workflows to Forge-root workflows with
  module path filters and module working directories. Keep CI read-only and
  keep publication behind the existing marker, token, version and explicit
  operator-confirmation preconditions. Give module releases a unique tag
  namespace so they cannot collide with Forge releases. Do not publish.
- Point the Rust contract-parity tests at Forge's canonical `contracts/`
  directory by default, validate its six supported families, and verify its
  four unsupported families remain explicit. Preserve `PLATFORM_CONTRACTS_DIR`
  as an override for the legacy nine-family registry used by standalone
  consumers.
- Update Forge-owned project registration, monorepo path reconciliation,
  module metadata, repository links and import documentation. Add an import
  receipt/checker that verifies source ancestry and tracked path/mode/blob
  parity, allowing only named workflow and monorepo adaptations.

## Package Boundary and Split Assessment

This package imports one complete source repository while retaining its
independent Cargo workspace and crates.io boundary. Splitting the eight crates
into separate changes would break their workspace dependency graph, shared
version and publication order; importing their source without the consumer
fixture and validation tools would lose the only end-to-end workspace oracle.
CI integration, metadata and import provenance share this source ownership
transition and can be accepted together by checking the imported tree and
running the native workspace verification.

The platform-library consolidation contains two language/release owners and is
decomposed by source repository:

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `forge-dotnet-platform-libraries-monorepo` | Make the complete .NET/UI platform repository an ordinary Forge module | Forge / dotnet-platform-libs, .NET 10 and Node.js | `modules/dotnet-platform-libs/`; preserve NuGet and local UI feed identities | None | Completed and archived at `openspec/changes/archive/2026-10-19-forge-dotnet-platform-libraries-monorepo/` |
| `forge-rust-platform-libraries-monorepo` | Make the Rust platform workspace an ordinary Forge module | Forge / rust-platform-libs, pinned Rust toolchain | `modules/rust-platform-libs/`; preserve crate identities and crates.io publication preconditions | Dotnet import is complete; no code dependency | Source ancestry/tree parity; pinned-toolchain Cargo tests, formatting, Clippy, deny and conformance; root workflow path assertions |

The imports are technically independent and are serialized by Forge's
single-active-change workflow. Consumer migration and retirement of source
origins remain later roadmap work.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| Rust platform source | `/home/paul/code/rust-platform-libs/Cargo.toml`, eight `crates/platform-*` workspaces, `.publish-order`, `tests/consumer-fixture`, `.project.json`; source HEAD `4762746`, 30 commits, no remote, clean worktree | Complete Rust crates, architecture tests, Rust 1.95 pin, MSRV 1.75 declaration, conformance checks, publication order and owner-gated publisher | Nested workflows are inert after relocation; repository metadata and registry use the old sibling; contract tests assume a sibling `platform-contracts` checkout; `vX.Y.Z` tags would collide in the Forge repository | Forge owns the consolidated source; nested Cargo workspace and crates.io publication remain Rust module boundaries | **adopt** the complete committed tree and retain its native boundaries |
| Forge language-neutral contracts | `contracts/registry.json`, `contracts/family-support.json`, six supported schema families and fixtures, Rust `platform-contracts` crate parity tests | Canonical Forge registry, schemas and fixture pairs; explicit `PLATFORM_CONTRACTS_DIR` override exists | Forge intentionally does not own identity-subject, job-outcome, permission-decision or tenant-context schemas; Rust's legacy registry has nine families and a different on-disk layout | Forge owns six currently supported wire families; Rust crate owns Rust types and validation and remains compatible with a supplied legacy nine-family registry | **adapt through a generic adapter** to validate Forge's six supported families and assert its four unsupported families without widening Forge contract ownership |
| .NET platform module | `modules/dotnet-platform-libs/`, `openspec/specs/dotnet-platform-libs-monorepo/spec.md`, module-scoped workflows | Demonstrates ordinary module import, path-filtered root CI, explicit release workflows, and Forge Governance path reconciliation | Different language, package ecosystem and release lifecycle | Forge owns repository orchestration; each module owns its native build and package contract | **adopt** the already-proven monorepo integration pattern where compatible, with Rust-specific crate and tag behavior |
| Workspace Governance | `modules/workspace-governance/projects.json`, `.project.json`, `migration/forge-monorepo-path-reconciliation.json`, `profiles/rust-platform.json` | Existing project ID, Rust profile, identity registry, declaration validation and project-path migration | Registry path is currently `rust-platform-libs`; its status evidence and homepage are stale after import | Forge-internal Governance owns the registry and path map; consumers do not need a Governance checkout | **extend shared owner** by updating the Forge-owned registry and path map |
| Forge Gate and CI | `src/gate/`, `.github/workflows/ci.yml`, root `Cargo.toml`, `docs/architecture.md` | Forge-root Gate is the single Git repository quality entry; existing CI supports module-specific jobs | Root Cargo workspace intentionally does not absorb nested crates; GitHub ignores nested workflow definitions; publication tags need namespacing | Forge root schedules checks; the Rust module owns Cargo commands and crates.io policy | **extend shared owner** with root workflows while keeping the module Cargo workspace separate |

## User Experience and Interface Impact

Actors are Rust platform maintainers and release operators. Maintainers find
the module from the Forge README and the module README, then run its native
Cargo commands from `modules/rust-platform-libs/`. CI results and release
precondition failures remain visible in GitHub Actions. Release operators use
the root workflow with a namespaced version tag or explicit workflow dispatch;
the existing `PUBLICATION-APPROVED` marker, `CARGO_REGISTRY_TOKEN`, exact
workspace-version match and typed dispatch confirmation remain visible
preconditions. This change adds no human-facing Forge web route or screen and
does not change a product user journey. UI/UX is N/A because the capability is
source ownership and maintainer automation, with no human-operated Forge
application surface; the existing GitHub Actions interface remains the
operator surface. No browser/device flow or accessibility change is introduced.

## BFS Impact Map

| Surface | Current evidence | Change and verification |
|---|---|---|
| Source and Git history | 161 tracked files, source HEAD `4762746`, 30 source commits, no remote, clean source worktree | Import committed files only; preserve all 30 commits in Forge ancestry; exact tracked path/mode/blob comparison with named adaptations; prove no submodule or ignored target/lockfile artifacts |
| Cargo workspace and crates | Eight crates under `crates/`, nested `Cargo.toml`, `rust-toolchain.toml` pins 1.95.0 and workspace MSRV is 1.75 | Keep nested workspace and crate identities; run format, Clippy, build, tests, deny and consumer conformance from module root |
| Canonical contracts | Rust legacy registry has nine families; Forge `contracts/registry.json` and `family-support.json` own six and explicitly refuse four | Resolve the Forge canonical tree by default, adapt its registry and fixtures, verify all six supported families and the four unsupported statuses; preserve explicit legacy directory override |
| CI and publication | Nested `ci.yml` and `publish.yml`; publisher accepts generic `vX.Y.Z` and existing marker/token/version/confirmation gates | Relocate to unique Forge-root workflow names, path-filter CI, set module working directory and Rust toolchain explicitly, namespace crate release tags, validate YAML and release refusal branches; no publication |
| Forge Governance and metadata | Registry ID `rust-platform-libs` points at `rust-platform-libs`; metadata homepage names the old repository | Point to `forge/modules/rust-platform-libs`, retain Rust profile and ID, reconcile migration data and verify registry resolution |
| Forge Gate | Root Forge Git boundary and root Gate; nested `.ai-gate` describes source project checks | Keep Forge Gate repository-root scoped; run its dry/full check from Forge root and native Rust commands from module root; do not claim the nested module is a second Git root |
| Consumers and origin repository | Rust consumers currently use path or git dependencies; release is blocked on owner approval; source repository has no remote | Do not edit sibling consumers, add dependencies to Forge root, create or push a remote, add the approval marker, tag, or publish; origin retirement remains a later explicit operation |
| Forge browser UI | Existing project workbench does not expose source import or package publication administration | N/A for this source/workflow change; test README/registry discoverability and truthful workflow controls, with no new route or browser flow |

## Capabilities

- `rust-platform-libs-monorepo`: Forge contains a provenance-checked, natively
  verifiable Rust platform module with namespaced, explicitly approved
  crates.io release automation.

## Non-goals

- Migrating product consumers from path/git dependencies or changing crate
  APIs, versions, features, MSRV, license or dependency policy.
- Publishing any crate, creating the approval marker, configuring secrets,
  pushing tags, creating a source remote or archiving/deleting the old source
  repository.
- Merging the nested Cargo workspace into Forge's root Cargo workspace.
- Moving canonical contract schemas out of Forge or making the Rust crate own
  those schemas.
- Adding product behavior, provider integrations, a Forge UI or a generic
  release platform.
- Replacing Forge Gate with a nested module Gate or adding source-level
  sibling-checkout dependencies.
