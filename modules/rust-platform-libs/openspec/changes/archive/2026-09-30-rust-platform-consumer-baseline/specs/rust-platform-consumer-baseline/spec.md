# rust-platform-consumer-baseline Specification

## Purpose

Give the Rust platform a committed, CI-enforced baseline that a consumer can
adopt verbatim, so adopting the crates no longer depends on a hand-written
sibling path and an undocumented toolchain.

## ADDED Requirements

### Requirement: A shared lint and format baseline SHALL exist

The repository MUST commit a formatting configuration, a clippy configuration,
a dependency-policy configuration, and a workspace lint set that a consumer can
adopt verbatim.

#### Scenario: Baseline present

- **WHEN** the repository configuration is inspected
- **THEN** `rustfmt.toml`, `clippy.toml`, `deny.toml` and `[workspace.lints]`
  are present and consistent

#### Scenario: Consumer adopts the baseline

- **WHEN** a consumer copies the baseline
- **THEN** the same formatting and lint outcomes apply to it

### Requirement: The MSRV SHALL be declared and pinned

The repository MUST declare one minimum supported Rust version in the workspace
package metadata and MUST pin it with `rust-toolchain.toml`, and MUST NOT raise
it without a separate change and a migration note.

#### Scenario: Pinned toolchain

- **WHEN** the repository is built
- **THEN** the pinned toolchain is selected

#### Scenario: Consumer below the MSRV

- **WHEN** a consumer builds with a compiler older than the declared MSRV
- **THEN** the build fails with a message naming the required version

### Requirement: CI SHALL enforce the baseline

Continuous integration MUST run formatting, clippy with warnings denied,
dependency-policy checks, and the test suite, and MUST fail on any regression.

#### Scenario: Lint regression

- **WHEN** a change introduces a lint warning
- **THEN** CI fails naming the lint

#### Scenario: Denied dependency

- **WHEN** a change introduces a dependency with a denied advisory or licence
- **THEN** CI fails naming the dependency and the policy

### Requirement: A consumer recipe SHALL replace hand-written paths

The repository MUST document one consumer recipe covering in-workspace
development and out-of-workspace adoption, and MUST NOT require a hard-coded
sibling path.

#### Scenario: Out-of-workspace consumer

- **WHEN** a consumer outside the workspace follows the recipe
- **THEN** it resolves the platform crates without a hard-coded sibling path

#### Scenario: In-workspace consumer

- **WHEN** a crate lives in the same workspace as the platform
- **THEN** the documented path-dependency form applies

### Requirement: A conformance harness SHALL exercise a consumer

The repository MUST provide a harness that builds and tests a consumer fixture
against the platform under the pinned toolchain.

#### Scenario: Conformance passes

- **WHEN** the harness runs
- **THEN** the consumer fixture resolves the platform crates, builds and tests

#### Scenario: Conformance fails loudly

- **WHEN** the fixture cannot resolve a platform crate
- **THEN** the harness fails naming the crate and the resolution mode
