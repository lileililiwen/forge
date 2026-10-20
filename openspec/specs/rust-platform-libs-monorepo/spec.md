# rust-platform-libs-monorepo Specification

## Purpose
The Rust platform workspace was a separate sibling checkout, so Forge could not
build it, audit it, or point its contract parity tests at one source of truth.
The imported workspace lives as ordinary tracked source under
`modules/rust-platform-libs/`, with its Cargo root, crate identity, toolchain
pin and history verified against the recorded revision. Module-scoped quality
runs from the module root and cannot publish, crates.io release stays gated on
explicit approval, and the parity tests read Forge's own `contracts/` root.
## Requirements
### Requirement: Forge contains the provenance-verified Rust platform workspace

Forge SHALL contain the complete committed `rust-platform-libs` source at the
recorded revision under `modules/rust-platform-libs/` as ordinary tracked
files. The import SHALL preserve the source workspace as an independent Cargo
root, include all source commits in Forge ancestry, and match tracked source
paths, modes and contents except for explicit, tested monorepo adaptations.
The import SHALL NOT require the old sibling checkout or add a Git submodule.

#### Scenario: Source import and ancestry are checked

- **WHEN** Forge verifies its Rust platform module
- **THEN** the verifier checks the immutable source revision, all 30 source
  commits as ancestors, the complete tracked path/mode/blob inventory and the
  declared adaptation list, and fails on missing or undeclared differences

#### Scenario: Native Rust workspace remains independent

- **WHEN** a maintainer runs the module's Cargo commands from
  `modules/rust-platform-libs/`
- **THEN** Cargo uses the module's pinned toolchain and workspace members
  without adding these crates to Forge's root Cargo workspace or resolving
  source through the old sibling checkout

#### Scenario: Crate identity and policy are retained

- **WHEN** a consumer reads the imported Rust package metadata
- **THEN** the eight crate names, versions, feature defaults, MSRV, edition,
  Apache-2.0 licensing, dependency-direction policy and publish order match
  the recorded source except for repository and homepage links to Forge

### Requirement: Forge runs module-scoped Rust quality and gated publication

Forge SHALL load Rust quality and publication workflows from the repository
root, scope quality checks to Rust module changes, and run the native pinned
toolchain checks from the module root. Quality CI SHALL be read-only. Crates.io
publication SHALL retain the source owner-approval, token, version and
operator-confirmation gates and SHALL use module-namespaced tags.

#### Scenario: Rust module change runs native quality checks

- **WHEN** a push or pull request changes `modules/rust-platform-libs/**` or
  the Rust quality workflow
- **THEN** Forge schedules formatting, Clippy, dependency policy, workspace
  build/tests and consumer conformance from the module root using Rust 1.95.0

#### Scenario: Quality CI cannot publish

- **WHEN** the Rust quality workflow runs on a push, pull request or manual
  dispatch
- **THEN** it performs no crates.io or other remote write and cannot invoke
  the publication workflow

#### Scenario: Crates.io release requires explicit module approval

- **WHEN** an operator pushes a `rust-platform-libs-v<SemVer>` tag or manually
  dispatches publication with an exact version and explicit approval
- **THEN** the publisher validates the module version, tracked approval
  marker, registry token and all release checks before publishing crates in
  `.publish-order`, and a missing or mismatched precondition prevents all
  publish steps

### Requirement: Rust contract parity respects Forge contract ownership

The Rust `platform-contracts` parity tests SHALL read Forge's canonical
`contracts/` registry, schemas, fixtures and family-support declaration by
default. They SHALL validate all six Forge-supported families and assert that
identity-subject, job-outcome, permission-decision and tenant-context remain
explicitly unsupported. They SHALL preserve `PLATFORM_CONTRACTS_DIR` for
standalone consumers, including the legacy nine-family registry layout, and
fail closed when the selected root or required schema is missing or invalid.

#### Scenario: In-tree contract fixture parity runs without a sibling

- **WHEN** Rust workspace parity tests run in the Forge checkout without
  `PLATFORM_CONTRACTS_DIR`
- **THEN** they resolve `<forge-root>/contracts`, validate valid and invalid
  fixture pairs for all six supported contract families, assert the four
  unsupported family statuses, and run typed payload checks without reading
  `/home/paul/code/platform-contracts`

#### Scenario: Explicit contract directory override is honored

- **WHEN** `PLATFORM_CONTRACTS_DIR` names a contract root
- **THEN** both parity test suites use that exact Forge-layout or legacy-layout
  directory, and a missing or malformed root fails with an actionable
  diagnostic instead of falling back

### Requirement: Forge Governance resolves the imported Rust module portably

Forge-owned project registration and path reconciliation SHALL resolve
`rust-platform-libs` to `forge/modules/rust-platform-libs` while preserving
its project ID and Rust profile. Module Gate metadata SHALL identify Forge as
the Gate runtime, and the module SHALL remain within the single Forge Git
boundary.

#### Scenario: Workspace Governance reads the in-tree project

- **WHEN** Forge Governance audits the registered `rust-platform-libs`
  project
- **THEN** it resolves the declaration and files at
  `forge/modules/rust-platform-libs` without a sibling source checkout

#### Scenario: Forge Gate remains repository-root scoped

- **WHEN** the Forge Gate evaluates the imported Rust module
- **THEN** the Git boundary remains the Forge repository root, while native
  Rust checks execute from the nested Cargo workspace root
