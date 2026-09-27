# Proposal: Forge post-MVP readiness

## Why

Forge's Rust Core/CLI is implemented and tested (37 archived changes, contract
and cross-surface suites; latest archived `gate-runtime-evidence`), but the
repository is not yet presentable to an operator who has not read the handoff:

- `README.md` is 126 lines and names the v0.1 commands once
  (`README.md:11`), but it never maps them: there is no per-command
  description of what `forge import`, `forge list`, `forge inspect`,
  `forge new` and `forge doctor` do, what the registry is, or what `forge.yaml`
  contains. A new operator cannot tell which command to run for which task.
- `README.md:39` states plainly: "There is no Forge installation packaging
  yet." There is no install path, no released artifact and no digest; the just-
  authored (unimplemented) `artifact-and-ci-baseline` change is the packaging
  plan, but the README does not say so.
- There is **no quickstart demo transcript or terminal capture**. A reader
  cannot see `forge import`/`list`/`inspect`/`new`/`doctor` output without
  cloning and building.
- `Cargo.toml` declares `license = "MIT"` while **no `LICENSE` file exists** in
  the repository — the metadata asserts something the tree does not contain
  (also identified by `artifact-and-ci-baseline`). The sibling `driftwatchdog`
  ships `LICENSE`, `CHANGELOG.md`, `deny.toml` and a `scripts/` packaging set;
  Forge does not.
- The governance-provider integration with `workspace-governance` is documented
  (`README.md:41-71`) but the **keep-local default** and the boundary with the
  sibling's adapter are not stated as a readiness contract, and a reader cannot
  tell what happens with no `.forge/providers.yaml` beyond one sentence.

This is a documentation, demo, packaging-plan and licensing-readiness package.
It changes no Forge behavior. It is deliberately coordinated with the active
`artifact-and-ci-baseline` proposal rather than duplicating it.

## What Changes

- **README — command map.** Document the v0.1 surfaces — `forge.yaml` and the
  project/profile registries, and `forge import`, `forge list`,
  `forge inspect`, `forge new`, `forge doctor` — as a task-oriented map, each
  with its real behavior and honest limits.
- **Packaging/install plan.** Present the install plan as a plan, not a
  delivered feature: state explicitly that no packaging exists yet, name
  `artifact-and-ci-baseline` as the owning implementation package, and document
  the intended `scripts/package.sh`/`install.sh`/`checksum.sh` path and digest
  verification without claiming it works.
- **Quickstart demo + terminal capture.** Add a committed transcript and
  terminal capture of a real quickstart (import → list → inspect → new →
  doctor) generated from the built binary.
- **License.** Add the `LICENSE` file matching the declared MIT expression
  (coordinated with, not duplicated by, `artifact-and-ci-baseline`).
- **Governance-provider readiness.** Document the keep-local default (`local`
  provider with no `.forge/providers.yaml`), the `workspace-governance` preset
  invocation, and the boundary that Forge never searches parent directories or
  the network; include the sibling-owned adapter execute-bit prerequisite.
- **No behavior change.** No CLI command, flag, exit code, document,
  registry schema or provider contract changes.

## Package Boundary and Split Assessment

Keep as one readiness package: README mapping, the install *plan* presentation,
quickstart capture, license and governance-provider prose are one audience-
facing pass over the same CLI and are verified by one quickstart run.

Two overlaps are resolved explicitly rather than duplicated:

- **`artifact-and-ci-baseline` (active, unimplemented)** owns the real packaging
  scripts, `[workspace]`, `rust-version`, package metadata, `deny.toml`,
  `CHANGELOG.md` and the `LICENSE` file. This package owns the README-facing
  install *plan* and quickstart; if `artifact-and-ci-baseline` is selected and
  archived first, this package consumes its `LICENSE` and packaged-artifact
  commands instead of restating them. They must not both create `LICENSE`.
- **`gate-evidence-export-consumption` and `governance-vocabulary-consumption`**
  consume sibling contracts; this package only *describes* the governance
  provider boundary and does not consume a new sibling contract.

## Sibling and Shared Architecture Reconnaissance

Sibling repositories on this host: `workspace-governance`, `driftwatchdog`,
`platform-contracts`, `sisyphusfy`, `ariadex`, `jenkins-local`,
`dotnet-platform-libs`, `rust-platform-libs`.

| Sibling / shared surface | Owns | This package's relationship | Decision |
| --- | --- | --- | --- |
| `workspace-governance` (checked: `/home/paul/code/workspace-governance`) | project registry, `.project.json` schema, governance profiles, release-evidence vocabulary, the packaged `scripts/forge_governance_adapter.py` | Forge's `local` provider is the default; `workspace-governance` is an optional preset. README must state the keep-local default and the sibling-owned execute-bit prerequisite (`100644` observed) | **Keep-local default**: no new dependency; document the optional preset as-is |
| `driftwatchdog` (checked: `/home/paul/code/driftwatchdog`) | shared gate runtime; also the packaging/CI reference (ships `LICENSE`, `CHANGELOG.md`, `deny.toml`, `scripts/{package,install,checksum,smoke,bump}.sh`) | Forge's declared `gate_runtime`; the sibling's packaging shape is a reference for the install plan, not something Forge depends on | Keep-local: no gate change, no gate pass claimed; cite as reference |
| `platform-contracts` | versioned JSON contracts and the pinned revision manifest | `artifact-and-ci-baseline` owns the parity walk; this package does not consume it | Out of scope |
| `sisyphusfy`, `ariadex` | agent runtimes | Already integrated (`supervised-agent-adapters`); not re-documented here | Out of scope |
| `jenkins-local` | deployment execution | `deployable: false`; no deployment requested | Out of scope |
| `dotnet-platform-libs`, `rust-platform-libs` | reusable language libraries | Forge is a standalone Rust workspace | Keep-local: no library adoption |

Default is **keep-local**: Forge remains standalone and network-free by
default; this package adds no cross-repository dependency and changes no shared
contract.

## BFS Impact Map

- **Capabilities:** new `readiness`. Consumes `core-manifest-registry`
  (`forge.yaml`, registry), `profile-registry`, `project-import`,
  `deterministic-project-generation`, `doctor-maturity-assessment`,
  `governance-provider-contract` (keep-local default) and the reference
  `artifact-and-ci-baseline` packaging plan.
- **Users and flows:** an operator sees how to import an existing project,
  inspect it, scaffold a new one and assess it; a reader learns there is no
  install yet and what the plan is; a governance user learns the sibling
  boundary.
- **Contracts/data/persistence:** none. No command, flag, exit code, emitted
  document, registry schema or provider contract changes. The transcript and
  capture are documentation assets.
- **Integrations/configuration:** documents the existing `local` provider
  default and the optional `workspace-governance` preset; adds no runtime
  integration.
- **Callers:** `README.md`, `docs/provider-evidence.md` (link only),
  `docs/release-readiness.md` (link only), a committed quickstart capture, the
  pending `LICENSE` from `artifact-and-ci-baseline`.
- **Failure/boundary behavior:** a documented command that no longer behaves as
  written is corrected from a real run; an install claim that implies delivered
  packaging is rejected; the `workspace-governance` non-executable candidate is
  documented honestly as a sibling-owned prerequisite, not worked around.
- **Tests:** existing contract/cross-surface suites and `scripts/release-check.sh`
  are the behavioral oracle; the quickstart run is the demo oracle.
- **Dependencies:** README readiness depends on `artifact-and-ci-baseline` for
  the real install commands; until then the documented path is a plan.
- **Compatibility/security/privacy:** no behavior change; captures contain no
  host paths or secrets (the policy plane already redacts and host-path-scrubs);
  the MIT license matches the declaration; no push/publish/deploy claim.

## Capabilities

### New Capabilities

- `readiness`: Forge's README maps the v0.1 registry/import/list/inspect/new/
  doctor surfaces, presents the install path as an explicit plan, ships a real
  quickstart capture, states the keep-local governance default and the sibling
  boundary, and has a license matching its declared expression.

### Modified Capabilities

- None.

## Non-goals

- No packaging implementation, release artifact, digest publication, container
  image or installation in this package (owned by `artifact-and-ci-baseline`).
- No Forge command, flag, exit-code, document, registry-schema or provider
  contract change.
- No publication, tagging, push or deploy (AGENTS.md invariant).
- No new sibling consumption; `workspace-governance` stays an optional preset
  and the local provider stays the default.
- No gate pass claim; no shared gate runtime is configured for this repository.
- No maturity promotion (requirement.md §25); this package supplies
  presentation, not evidence.