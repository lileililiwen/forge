# Proposal: forge-dotnet-platform-libraries-monorepo

## Why

Forge is consolidating sibling development projects into ordinary in-tree
modules so each project keeps its native build, tests and release boundary
without requiring a sibling checkout. The live `dotnet-platform-libs` main
branch contains 1,604 tracked files, 79 .NET library projects, 70 test
projects and six npm package manifests for the co-owned UI kit. It is at
`d11fd9b` and is 38 commits ahead of its tracked origin branch. Its working
tree also contains two untracked user files, which must remain untouched and
must not be copied as committed source.

The library repository already has its own identity, `.project.json`, Gate
manifest, .NET 10 SDK pin, central package versions, solution, package feed,
tests, OpenSpec history, UI kit and release workflows. Copying only a few
packages would break the repository's standalone build and its provenance.

## What Changes

- Import the tracked source at `d11fd9b` as an ordinary project under
  `modules/dotnet-platform-libs/`, preserving its project root and all
  source-branch commits in Forge's Git ancestry. Do not add a Git submodule.
- Keep the native .NET 10 solution and npm UI kit independently runnable from
  the module directory. Preserve NuGet package IDs, versions, target
  frameworks, UI package names, licenses, tests and local artifact
  conformance.
- Move the .NET quality and manual release workflows to Forge's root
  `.github/workflows/`, with module path filters and explicit working
  directories. Preserve the manual release boundary and direct future static
  NuGet releases to the Forge Pages endpoint. Retain the old feed URL as a
  consumer fallback; its bytes are not copied because no source `gh-pages`
  branch was available. This change does not dispatch a release or publish
  an artifact.
- Update the in-tree Workspace Governance registry and UI token checks to
  resolve the imported module paths, without requiring the archived source
  checkout.
- Add an import receipt and a deterministic checker for source ancestry,
  tracked path/mode/blob parity and the small allowlist of intentional
  monorepo adaptations.
- Document the module's local verification and release commands in Forge.

## Package Boundary and Split Assessment

This package moves one existing source repository as a complete, independently
verifiable module. Its .NET packages and UI kit have separate artifact formats,
but they share one source root, maintainer, central quality gate and adoption
project; relocating only one side would leave `Platform.UI.Razor`, UI tokens,
package fixtures and their conformance checks separated from their owner.
The .NET NuGet feed and UI's versioned local npm feed remain distinct outputs.
No npm registry publishing behavior is invented.

The requested platform-library consolidation is decomposed by source
repository, language runtime and release contract:

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `forge-dotnet-platform-libraries-monorepo` | Make the complete .NET/UI platform repository an ordinary, independently buildable Forge module | Forge / dotnet-platform-libs, .NET 10 and Node.js | `modules/dotnet-platform-libs/`; preserve NuGet and local UI feed identities, move active GitHub workflows to Forge root | None | Preserved source ancestry and path/blob/mode parity; .NET quality/conformance checks and UI verifier pass from the module; root workflow paths are valid |
| `forge-rust-platform-libraries-monorepo` | Make the Rust platform workspace an ordinary, independently buildable Forge module | Forge / rust-platform-libs, pinned Rust toolchain | `modules/rust-platform-libs/`; preserve crate names, feature graph, Apache-2.0 license, publish order and crates.io boundary | None; serialized after the .NET import by the one-active-change workflow | Preserved source ancestry and path/blob/mode parity; pinned-toolchain workspace tests, formatting, Clippy and conformance pass from the module |

These are independent imports with no technical dependency; they are delivered
serially because Forge's HANDOFF and archive workflow admits one active change
at a time. Consumer migration and source-origin retirement remain later
roadmap work. This package is not complete until its own source import,
workflows and adoption metadata pass their local verification.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| .NET platform source | `/home/paul/code/dotnet-platform-libs/Platform.sln`, `Directory.Build.props`, `src/` (79 projects), `tests/` (70 projects), `ui/` (6 package manifests), `.project.json`, `.ai-gate/gate.yaml`; source HEAD `d11fd9b` | Complete platform implementations, tests, package/feed tooling, UI source and consumer fixtures; .NET 10.0.400 pin, nullable and warnings-as-errors | Existing GitHub workflows are nested and would not run after relocation; repository/page metadata names the origin; in-tree consumers and token tests contain source-checkout paths | Forge owns the consolidated source repository; keep module-native project and package identities | **adopt** into an ordinary Forge subdirectory and retain native module roots |
| Rust platform source | `/home/paul/code/rust-platform-libs/Cargo.toml`, `rust-toolchain.toml`, `.publish-order`, eight crates and `tests/consumer-fixture`; source HEAD `4762746`, no configured Git remote | Independent Cargo workspace, pinned Rust 1.95.0, crate architecture tests, consumer conformance and a gated crates.io publisher | Different language and release contract; merging it into the Forge root Cargo workspace would combine dependency graphs and release commands | Forge owns the monorepo; Rust workspace retains its nested Cargo root and crates.io publication boundary | **adopt** in the separate follow-up package |
| Workspace Governance in Forge | `modules/workspace-governance/projects.json`, `scripts/ui_conformance.py`, `tests/test_ui_conformance.py`, `migration/forge-monorepo-path-reconciliation.json` | Existing declaration registry, project audit, imported module layout and shared UI-token consumer | Registry currently points to the old sibling path; two UI tests use absolute source-checkout paths | Forge's in-tree governance module owns internal registry paths; external products are not coupled to it | **extend shared owner** by updating the Forge-owned registry, migration map and tests for the new module location |
| Forge build and release root | `Cargo.toml`, `.github/workflows/ci.yml`, `src/gate/`, `docs/workspace-governance-import.md` | Forge root already owns CI, the shared Gate, modular project declarations and a tested source-import receipt pattern | Nested GitHub workflow files are inert; a root-wide workflow must keep the .NET module isolated and manual release explicit | Forge root dispatches; the nested .NET module owns its source, package policy and command scripts | **extend shared owner** with module-scoped root workflow adapters and reuse Forge Gate |

## User Experience and Interface Impact

Actors are platform maintainers and release operators. Maintainers discover
the module in the Forge README and module-local `README.md`/`AGENTS.md`, then
run its native commands from `modules/dotnet-platform-libs/`. CI appears as a
path-filtered Forge GitHub Actions check. Release operators use the existing
manual GitHub Actions version input; validation, packaging, feed upload and
Pages status remain explicit workflow stages. No Forge web screen or new
browser action is needed: this change moves source ownership and preserves the
existing command and release interfaces. The workflow's queued, running,
failed, successful and permission-denied states remain visible through GitHub
Actions, and release dispatch remains separate from CI.

## BFS Impact Map

| Surface | Current evidence | Change and verification |
|---|---|---|
| Source ownership and Git | 1,604 tracked files at source revision `d11fd9b`; source main is 38 commits ahead; source contains two untracked user files | Import only committed source under the module prefix; preserve all source ancestry; compare imported tracked paths, modes and blobs and prove the two source user files remain untouched |
| .NET build and packages | `Platform.sln`, `global.json` (10.0.400), central package props, NuGet package metadata and feed scripts | Keep nested SDK/build configuration; update repository metadata; run quality gate, API baseline, packed-consumer conformance and audit |
| UI kit and UI-backed .NET packages | `ui/` token generator, 6 package manifests, `Platform.UI.Razor`, feed and distribution tests | Keep the UI kit under its current module path; run verifier, verifier self-test and distribution/conformance tests |
| GitHub CI/release | Two source-root workflows under `dotnet-platform-libs/.github/workflows/` | Install CI and manual release workflows at Forge root, constrain CI to module/workflow paths and set module working directory; validate YAML and ensure no automatic publication |
| Forge Gate and module discovery | Forge root `.ai-gate/gate.yaml`; nested module `.project.json`/`.ai-gate/gate.yaml` | Run the repository-root Forge Gate; run module-specific .NET checks from the module root workflow; preserve the nested declaration for standalone export without treating it as a second Git boundary |
| Workspace Governance consumers | Registry path `dotnet-platform-libs`; UI script relative defaults; UI test constants use `/home/paul/code/dotnet-platform-libs` | Point the in-tree registry at the in-tree module, keep module-relative UI defaults and replace absolute test paths with paths derived from the Forge module layout |
| Package identity and publication | NuGet IDs and versions; static NuGet Pages feed under the origin repository; local npm feed; no npm publish workflow | Preserve package identities and manual .NET release; repoint package/source and Pages metadata to Forge; do not run publication or move the existing feed contents in this change |
| External consumers and origins | Existing sibling consumers and the source repository remain available | Do not edit sibling repositories or archive the origin here; origin retirement follows the roadmap after consumer migration |
| Forge browser UI | Existing project workbench and command catalog; no new operator action is introduced | N/A: use existing GitHub Actions and native project commands; verify module discoverability and truthful release controls, with no new UI route |

## Capabilities

- `dotnet-platform-libs-monorepo`: Forge contains a provenance-checked,
  independently runnable .NET/UI platform module with module-scoped CI and
  explicit, manually dispatched NuGet feed release.

## Non-goals

- Importing Rust crates in this change; that is the separate, queued Rust
  package above.
- Migrating sibling product consumers, removing their package feeds, or
  changing their project files.
- Archiving the source origin or performing any remote repository operation.
- Publishing NuGet/npm packages or deploying Forge Pages during local work.
- Combining the nested .NET solution or Cargo workspace with Forge's root
  language workspace.
- Replacing the library's native scripts, contracts, source code or release
  policy with a Forge-only implementation.
- Adding a Forge web UI workflow for package administration.
