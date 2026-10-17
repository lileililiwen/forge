# Forge CI and pre-commit templates

## Why

Forge now owns mandatory Gate checks, but generated and existing projects do
not yet receive a Forge Gate CI workflow or a pre-commit hook through the
Forge-owned standard pack. This leaves the baseline dependent on a maintainer
remembering to wire the same command into each repository.

This is the first bounded source-consolidation slice: make Forge's existing
versioned standard-pack system own and distribute the common CI and hook
templates. Keep sibling source migration and repository retirement as later
packages with separate evidence.

## What Changes

- Add a supported `baseline-service@1.3.0` pack for the four verified service
  profiles, with a GitHub Actions workflow that runs the existing shared
  verification entry point and `forge gate .`.
- Add a supported `baseline-client@1.0.0` Gate-only pack for React and Flutter
  profiles, so all six current Forge profiles receive the same baseline Gate
  workflow and pre-commit hook without pretending a service verifier supports
  those client stacks.
- Add pre-commit framework configurations that call `forge gate .` with no
  file-list narrowing that could skip repository-wide checks.
- Default compatible Forge-created projects to their profile's verified pack;
  allow existing projects to adopt or upgrade through the existing explicit,
  conflict-safe `forge standard install` and `upgrade` operations.
- Keep project-specific checks in the existing Forge Gate extension mechanism;
  templates do not replace or weaken the built-in baseline.
- Pin the GitHub Actions Forge source install to an immutable Forge revision.
  Forge release publication is not implemented, so release-based installation
  is deferred rather than assumed.

## Package Boundary and Split Assessment

The broader consolidation has independent ownership and verification
boundaries. It is intentionally decomposed rather than combined into this
template change:

| Order | Package | Scope and completion evidence | State |
| --- | --- | --- | --- |
| 1 | `forge-ci-precommit-templates` | Versioned service and client templates; all current profile defaults; legacy install/upgrade; same Forge Gate command in local hook and CI | Complete; archived 2026-10-17 |
| 2 | `forge-portable-binary-distribution` | Publish versioned, checksummed Forge binaries so templates can install a pinned release without compiling from a source checkout | Remaining |
| 3 | `forge-driftwatchdog-test-reconciliation` | Reconcile the in-tree DriftWatchdog Gate tests with Forge's authoritative Gate behavior; current local workspace suite has 10 failures in this crate | Remaining |
| 4 | `forge-workspace-governance-internalization` | Move selected bootstrap, workspace-check and runtime-template sources into Forge; remove sibling-checkout runtime dependencies; reconcile duplicate registry/provider ownership | Remaining |
| 5 | `forge-platform-schema-ownership` | Inventory schemas actually consumed by Forge, bring those sources and validators under Forge-owned maintenance, remove sibling-path parity requirements, and mark unsupported families explicitly | Remaining |
| 6 | `forge-multiservice-local-runtime` | Model multiple services and entry points, including Docker Compose and local processes; integrate the existing `gitrunbykaru` process manager behind Forge commands | Remaining |
| 7 | `forge-platform-libraries-monorepo` | Relocate .NET and Rust library sources as ordinary directories/workspaces while retaining package identities, independent builds and publication boundaries | Remaining |
| 8 | `forge-developer-workflow-facade` | Define and deliver Forge's stable end-to-end developer workflows across the consolidated modules; reconcile product language that currently excludes a generic CI/CD replacement | Remaining |
| 9 | `forge-origin-repository-retirement` | Verify Forge's in-tree `driftwatchdog` and `cargo-*` crates are authoritative, then migrate any remaining sources and make each origin repository read-only/archive it; this is an external repository operation and final migration step | Remaining |

The exact source inventory and migration order must be rechecked against each
live sibling HANDOFF and Git state when those packages start. The
`dotnet-platform-libs` checkout currently has unrelated untracked local
verification files, which are outside this change and must be preserved.

## Sibling Reconnaissance (2026-10-17 snapshot)

These read-only checkout facts establish initial migration boundaries, not
permission to alter or archive those repositories:

| Checkout | Observed state | Relevance |
| --- | --- | --- |
| `../workspace-governance` | `main`, six commits ahead of `origin/main`, clean | Owns bootstrap, workspace checks, runtime templates and provider adapter sources currently consumed by Forge. |
| `../platform-contracts` | `main`, clean; this is the actual checkout name/path (not `../contracts`) | Contains the external schema source mirrored under Forge's `contracts/`; only consumed families should migrate. |
| `../gitrunbykaru` | `main`, eight commits ahead of `upstream/main`, clean | Existing process manager to assess for Forge's multi-service local-run facade. |
| `../rust-platform-libs` | `main`, has untracked `driftwatch.toml` and `verify-local.sh` | Potential ordinary workspace relocation; preserve local files and package build/publish identities. |
| `../dotnet-platform-libs` | `main`, 38 commits ahead of `origin/main`, has untracked `driftwatch.toml` and `verify-local.sh` | Potential ordinary solution relocation; preserve local files and NuGet identities/feeds. |

The sibling snapshots may have changed since this reconnaissance. Each
migration package must refresh these facts and inspect its source, checks and
HANDOFF before moving files.

## BFS Impact Map

- **Requirements:** Forge Gate baseline and extension union; versioned
  standard packs; deterministic project generation; explicit legacy install
  and upgrade; portable operation without a sibling checkout.
- **Modules:** `src/standard` descriptors/rendering/receipts; project
  generation request defaults; CLI standard install/upgrade; versioned pack
  assets under `standards/baseline-service`; Forge Gate invocation.
- **Callers:** `forge new`, `forge standard install`, `forge standard upgrade`,
  generated GitHub Actions workflows, and pre-commit framework.
- **Compatibility:** Existing `1.0.0`, `1.1.0`, and `1.2.0` snapshots remain
  immutable; upgrade remains explicit and conflict-safe. Unsupported profiles
  must not receive a service pack they cannot select.
- **Failures/security:** Missing Forge executable, unavailable pinned source,
  failed Gate, absent extension dependencies, and template path conflicts must
  fail closed with nonzero status and preserve user files. CI uses read-only
  repository permissions and does not publish, deploy, or mutate remotes.
- **Verification:** Pack descriptor/asset digest checks, generated-project
  behavior, legacy install and upgrade conflict behavior, CI/hook command
  parity, strict OpenSpec validation, and local Forge Gate evidence.

## Capabilities

### New project baseline automation

All six current Forge profiles receive a compatible Forge-owned CI and
pre-commit pack by default. Service profiles also run the shared language
verifier; client profiles receive the Gate-only baseline and leave language
specific build/test commands to explicit Gate extensions until verified client
verifiers are added.

### Legacy project adoption

An existing repository can install or explicitly upgrade the same pack through
Forge's existing receipt-backed operations. Modified owned files remain
conflicts and unrelated files remain untouched.

### Extension preservation

Project-specific Gate extensions continue to run in addition to the mandatory
Forge baseline from CI and pre-commit.

## Non-goals

- Migrating or archiving sibling repositories in this package.
- Publishing a Forge release or changing Forge's release/tag workflow.
- Creating a new schema/contract format for template ownership.
- Migrating Compose or multi-service local execution into the pack.
- Moving Rust or .NET platform libraries.
- Deploying, publishing, pushing, or changing any sibling checkout.
