# Proposal: Make runtime template rendering a portable Forge capability

## Why

Forge's project factory should be the durable entry point for runtime templates. The current source of truth lives in the sibling `workspace-governance` checkout, and its renderer only runs when that checkout and its Python files are present. That conflicts with Forge's standalone distribution and the consolidation goal. Forge already consumes the same `.project.json` declaration and has a portable binary release path, so the renderer and its assets need an owned, versioned location in Forge.

## What Changes

- Copy the existing version-1 runtime template registry, renderer, and template assets into Forge's owned `scripts/governance/runtime-templates/` tree, preserving IDs, required variables, generated files, validation, and declaration updates. Pin every bundled source file in a Forge-owned SHA-256 manifest.
- Add `forge runtime-template list|inspect|render|check` as the supported user entry point. The command delegates to the bundled renderer using argument arrays and inherited standard streams; it never discovers a sibling checkout.
- Package the renderer and assets next to the Forge binary in release archives and install them under the selected prefix. The CLI resolves only the bundled asset location or an explicit `FORGE_RUNTIME_TEMPLATES_HOME` override.
- Add contract coverage for dispatch, resource lookup, archive contents, manifest integrity, and compatibility with the existing version-1 templates.

## Package Boundary and Split Assessment

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `forge-runtime-template-internalization` | Forge can list, inspect, render and check the current runtime templates without a sibling checkout | Forge, Rust CLI plus bundled Python renderer/assets | Existing renderer CLI and version-1 `registry.json` / `.project.json` metadata | None | Render/check contract suite with no Workspace Governance checkout, plus release archive install smoke |
| `forge-project-bootstrap-internalization` | Forge owns project declaration and governance bootstrap templates | Forge, Rust | Existing bootstrap output and Forge profile/Gate contract | This package | Scaffold compatibility and conflict-preservation tests |
| `forge-workspace-audit-internalization` | Forge owns the selected workspace audit and registry/provider reconciliation | Forge, Rust/Python boundary to be decided by its design | Workspace audit finding vocabulary and registry import/export | Bootstrap package | Audit parity corpus and registry migration fixtures |

Dependency order is runtime templates → project bootstrap → workspace audit. These outcomes have separate user workflows, source lifecycles, contracts, and test oracles; bundling them would conceal unresolved registry ownership. This change is the smallest shippable slice because the runtime renderer has a closed version-1 contract and does not require bootstrap or fleet audit changes.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| Workspace Governance | `scripts/runtime_templates.py`, `templates/runtime/registry.json`, `openspec/specs/runtime-template-renderer/spec.md` | Complete version-1 renderer and 14 template families; its atomic writes, declaration metadata, and safety checks are covered by 347 lines of tests | Source path assumes sibling checkout; not included in Forge release archive | Workspace Governance currently owns source; Forge will own the consolidated copy and bundle it with its release | adopt |
| Forge project generation | `src/generate`, `src/standard`, `scripts/package.sh`, `scripts/install.sh` | Rust CLI, packaged Forge binary, deterministic project assets and release installer | Current package contains only binary/docs and has no runtime template resource resolver | Forge owns CLI and distribution; templates remain ordinary versioned files in the Forge release | extend shared owner |
| Platform contracts | Forge `contracts/manifest.json` and `.project.json` | Existing declaration and schema compatibility practices | Runtime template metadata is currently governed by the sibling spec; this slice keeps its format and promotes a Forge-owned canonical requirement | Forge owns this renderer contract; schema-source migration remains queued separately | keep local |

The sibling checkout was inspected read-only: `main` is clean at `2c766c6`; the source repository is not modified. The migration preserves the original IDs and shape so the sibling implementation can remain a read-only historical source until consumer migration evidence supports retirement.

## User Experience and Interface Impact

The actor is a developer creating or checking a local deployment shape. Entry point is `forge runtime-template`; `list` discovers IDs, `inspect` reports requirements and target evidence, `render` writes selected files, and `check` validates registry and generated output. The human text and JSON output must identify the template and exact refusal; success reports rendered paths and configured metadata. Existing-file conflicts, ambiguous detection, invalid variables, missing declaration, and unavailable Compose tooling remain visible failures. This command-line workflow has no responsive or browser surface; keyboard interaction follows normal CLI argument parsing, and output must remain usable in a terminal and machine-readable JSON. Browser/device checks are not applicable because no browser or device UI is changed.

## BFS Impact Map

- **Capability/callers:** adds the Forge runtime template CLI; does not change `forge new`, project registry operations, or Gate execution.
- **Contracts/data:** keeps registry schema version 1, template IDs, input names, generated paths, and `.project.json` runtime metadata. No SQLite migration.
- **Integration/configuration:** Python 3 remains a declared runtime prerequisite for this renderer. Release archives bundle scripts and assets; `FORGE_RUNTIME_TEMPLATES_HOME` is an explicit override only. No sibling path, parent probing, or network fetch.
- **Failure boundaries:** missing/corrupt bundled resources, missing Python, malformed templates, invalid input, file conflicts, and Compose absence are reported as errors/unknown verification, never success. Rendering preserves the renderer's preflight-before-write and atomic promotion behavior.
- **Security/compatibility:** preserve path confinement, secret rejection, `.env` ignore conflict behavior, and subprocess argument arrays. Do not print variable values that may be sensitive.
- **Verification:** renderer contract tests, CLI dispatch/resource-resolution test, package/install asset test, Python syntax check, changed Rust formatting/build, OpenSpec validation, and local Forge Gate.
- **Dependencies:** no sibling source edit. Bootstrap and audit ownership work remain explicit follow-ups in ROADMAP.

## Capabilities

- New: `forge-runtime-template-internalization` — Forge-owned, version-1 compatible runtime templates and CLI.
- Revised: `artifact-and-ci-baseline` — release archive and install behavior includes the runtime template resource tree.

## Non-goals

- Migrating project bootstrap templates, `init_project.py`, `workspace_check.py`, the portfolio registry, or the governance-provider observation protocol.
- Renaming template IDs, changing generated Docker Compose behavior, changing deployment readiness semantics, or embedding secrets.
- Editing or publishing Workspace Governance, GitHub releases, sibling repositories, or deployment state.
