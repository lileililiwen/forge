# Proposal: forge-workspace-governance-in-tree

## Why

Forge is the developer-facing owner and facade, but its README and active
adapters still treat Workspace Governance as an external sibling checkout.
That leaves Forge's core workflows dependent on checkout layout and leaves
the source-of-truth project outside the consolidation the user requested.
The sibling currently has 400 tracked files and six commits ahead of its
remote; importing only selected scripts would lose its independent project,
tests, docs, specs, and local history.

## What Changes

- Import all tracked Workspace Governance source into the ordinary directory
  `modules/workspace-governance/`, preserving its project root and Python 3.11
  command/test lifecycle.
- Preserve all six commits currently present on the local source branch in
  Forge's Git history; do not use a Git submodule and do not discard or
  reinterpret those commits.
- Rebase the four Workspace Governance registry paths for already-consolidated
  DriftWatchdog and cargo-* projects to their Forge crate directories while
  preserving their project IDs.
- Record source provenance, the imported source revision, origin status, and
  migration boundary in the module's import note and Forge docs.
- Update the consolidation roadmap: origin retirement includes
  Workspace Governance and requires consumer migration plus preservation of
  all local-only source commits.

## Package Boundary and Split Assessment

This package has one outcome: Workspace Governance is present in Forge as a
normal, independently runnable project with its complete source and history.
Importing and validating that project share one ownership transition and are
verified through path/blob/mode parity, ancestry checks, and the module's
self-check/tests from its new root. The four registry paths must also be
corrected because they currently point at removed origin directories. Forge
call-site migration is separate because it changes adapter resolution,
data-root semantics, packaging, and compatibility behavior independently.

The requested end state has separately verifiable follow-ups and is split:

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `forge-workspace-governance-in-tree` | Preserve the complete source project in Forge as a normal, independently runnable subproject | Forge / Workspace Governance, Python 3.11 | Ordinary `modules/workspace-governance/`; existing project CLI/test boundary | None | Source commit ancestry, 400 tracked-path parity, module self-check/tests from imported root |
| `forge-workspace-governance-gate-conformance` | Apply the built-in source-size baseline to Python scripts and split the four over-limit files | Forge Gate + Workspace Governance, Rust/Python | Existing `source-file-size` Gate baseline and module Gate | `forge-workspace-governance-in-tree` | Module Gate finds actual candidates and passes at the baseline; four script files are <=1,000 lines |
| `forge-governance-consumer-migration` | Route Forge workflows to the in-tree owner and remove Forge-owned sibling-checkout paths and duplicate maintained sources | Forge, Rust/JS/Python | Existing Forge provider, registry and standard-pack interfaces | `forge-workspace-governance-in-tree` | Search/assert no active Forge source requires `../workspace-governance`; compatibility contracts pass |
| `forge-governance-consumer-portability` | Make each sibling consumer work without a Workspace Governance checkout | Each consumer project, its native language | Versioned Forge assets or declared Forge command; no sibling source/runtime coupling | consumer migration | Per-repository native Gate and isolated checkout tests |
| `forge-workspace-governance-origin-retirement` | Mark origin read-only after preserving changes and verifying every consumer | Forge release/operator boundary | Remote repository archival | consumer portability | Remote read-only/archive state plus fleet-wide consumer scan |

Dependencies are ordered as listed. This package does not claim the later
packages are done. The local-only six commits must be retained in the imported
history before the source repository can be archived. Any later publication
or remote archive remains a separate operator action; Forge's no-push rule
applies.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| Workspace Governance | Source `README.md`, `scripts/workspace_check.py`, `scripts/forge_governance_adapter.py`, `scripts/bind_gate_commands.py`; 400 tracked files; current source HEAD `2c766c6` | Complete project registry, declaration and audit model, Python scripts, schemas, templates, specs, tests and existing Forge adapter | Forge calls several sibling paths and carries pinned copies; monorepo location changes the module root but not the module's Python interfaces | Forge owns the product/facade; this directory preserves Workspace Governance's independent Python project boundary | **adopt** |
| Forge Gate and standard packs | Forge `src/gate/`, `src/standard/`, `scripts/governance/runtime_templates.py` and versioned snapshots | Forge owns mandatory quality Gate and packaged templates; existing provider contract adapts governance observation | Current selected governance policy/data overlaps with Forge-owned Gate and generated snapshots | Forge | **extend shared owner**: keep Forge Gate authoritative; import WG policy/source first, reconcile active duplicates in follow-up |
| `platform-contracts` | `scripts/sync-contracts.mjs`, `contracts/`, `.contracts-manifest.json` | Shared wire schemas have independent producer/consumer contract | Not part of Workspace Governance source and has separate ownership/release boundary | `platform-contracts` | **adapt through a generic adapter**: keep it external in this package; separately scoped schema consolidation remains on roadmap |
| Workspace Governance consumers | `.project.json`, `src/publish/fleet.rs`, `src/cli/fleet.rs`, `scripts/sync-contracts.mjs`, baseline-service shims; 207 files mention `workspace-governance/scripts/` across workspace scan | Existing adapter and generated pinned assets | Many consumers encode sibling layout; broad rewrite would hide independent project compatibility work | Forge in this package; each sibling owns its later migration | **adapt through a generic adapter** for current Forge provider; defer external consumer rewrites |

The source branch is clean and has six commits beyond `origin/main`:
`b8808c4`, `d48ea3f`, `a55fbb7`, `abf5c37`, `d6b628e`, and `2c766c6`.
All six must be reachable from the imported history. This is source and
history evidence, not authorization to push.

## User Experience and Interface Impact

Actors are Forge operators and developers invoking governance workflows from
Forge CLI, generated project Gate scripts, or the Forge dashboard. This
package changes no command names, screens, navigation, payloads, or dashboard
states. Existing `forge governance` commands keep their current arguments and
typed errors; default provider resolution becomes independent of sibling
directory placement. Explicit provider selection remains available. No
responsive or accessibility behavior changes because there is no UI surface
change; the consuming CLI/browser flows must be exercised in their existing
tests. External sibling consumer experiences are explicitly owned by the
consumer-portability package.

## BFS Impact Map

| Surface | Impact and evidence |
|---|---|
| Source ownership | Add the entire tracked source project under `modules/workspace-governance/`; preserve Python 3.11 layout and six local commits. |
| Forge caller | Existing adapter and `.project.json` declarations remain unchanged here; consumer migration is the next package. |
| Gate / templates | Do not replace Forge Gate or its runtime-template renderer; source is imported first, duplicate ownership is reconciled in the consumer-migration package. |
| Registry / persistence | No new Forge database schema. Workspace Governance's own JSON declarations/registry remain in its subproject unchanged. |
| Contracts | Keep `governance-provider-contract/0.1.0`; do not change JSON shape or status mapping. Do not change `platform-contracts` sync in this package. |
| Build / tests | Python 3.11 project checks run from module root; Rust Forge tests run at Forge root; `cargo build` alone is not sufficient. |
| Failure boundaries | Missing source checkout, destination conflict, any path/blob/mode mismatch, or lost commit ancestry blocks import completion. |
| Compatibility | Public Forge CLI/provider behavior stays stable. Existing sibling paths and generated concern shims remain known follow-up work and are not falsely claimed portable here. |
| Consumers | Forge + sibling scan establish callers; external repositories are not rewritten in this package. |
| Security/privacy | Preserve argument-array process execution, bounded outputs/timeouts, credential redaction and path checks in the existing Rust adapter. No remote operation. |
| UX | Existing CLI and dashboard retain their workflows; no new UI is in scope. |

Repeated cross-project concern: project declarations, audit policy, gate
bindings and generated templates are consumed by many repositories. The
existing Workspace Governance project is their shared owner; importing it as
an ordinary Forge subproject preserves that ownership while Forge provides a
single facade. Consumers will move to packaged/versioned Forge assets or
commands in a follow-up; they will not import this module's source directly.

## Capabilities

- `forge-workspace-governance-in-tree`

## Non-goals

- Editing sibling repositories or their HANDOFF/Git state.
- Archiving or pushing the origin repository in this change.
- Rewriting Workspace Governance policy, schemas, specs, scripts, or Python
  interfaces during import.
- Migrating all sibling consumers or removing every sibling path in this
  first package.
- Consolidating `platform-contracts`, .NET/Rust libraries, `gitrunbykaru`,
  DriftWatchdog, or unrelated cargo tools.
- Replacing Forge Gate or making generated projects require Forge at runtime.
