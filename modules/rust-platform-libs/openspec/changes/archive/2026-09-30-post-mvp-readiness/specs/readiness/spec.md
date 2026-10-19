# Readiness (Rust platform crates)

## ADDED Requirements

### Requirement: A per-crate matrix describes every crate

The README SHALL present one matrix row per workspace crate with its purpose, production/test-only posture, version, license, publish posture, MSRV, and edition, and every value SHALL match the crate and workspace manifests.

#### Scenario: A consumer reads the matrix

- **WHEN** a consumer reads the README matrix
- **THEN** each crate's purpose, version, license, publish posture, and MSRV are stated without reading the manifests

#### Scenario: A matrix value disagrees with a manifest

- **WHEN** a row's version, license, publish, or MSRV differs from `Cargo.toml`/`crates/*/Cargo.toml`
- **THEN** the row MUST be corrected from the manifest

### Requirement: Each production crate has a compiling usage example

`docs/usage.md` SHALL give one minimal usage example per production crate, and each snippet SHALL exist as a compiling `crates/*/examples/*.rs` file.

#### Scenario: A consumer copies an example

- **WHEN** a consumer copies a documented example
- **THEN** it compiles against the crate's public API

#### Scenario: An example does not compile

- **WHEN** an example or its documented snippet fails to compile
- **THEN** it MUST be fixed or removed and MUST NOT be documented as working

### Requirement: The license claim is true on disk

A root Apache-2.0 LICENSE SHALL exist, matching `[workspace.package].license`, and the matrix SHALL state that each crate inherits it.

#### Scenario: The workspace declares Apache-2.0

- **WHEN** `Cargo.toml` declares `license = "Apache-2.0"`
- **THEN** the LICENSE text exists at the repository root

#### Scenario: A crate claims a different license

- **WHEN** a crate's license differs from the workspace declaration
- **THEN** the crate manifest and the matrix MUST be reconciled before merge

### Requirement: Adoption is documented without duplicating the baseline

The README SHALL document the current path-dependency adoption and SHALL point to `rust-platform-consumer-baseline` for the shared lint, MSRV, CI, and conformance recipe, and SHALL NOT re-declare that recipe.

#### Scenario: A consumer needs the shared recipe

- **WHEN** a consumer asks how to adopt the lint/MSRV/CI baseline
- **THEN** the README points to the owning change rather than restating it

#### Scenario: This change edits workspace configuration

- **WHEN** the diff touches `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `rustfmt.toml`, `clippy.toml`, or CI
- **THEN** the change MUST be rejected in review as overlapping the baseline change

### Requirement: The change preserves the provider-neutral boundary

The change MUST NOT modify crate public APIs or workspace configuration, MUST NOT add a forbidden dependency to any production crate, and MUST NOT edit any consumer or sibling repository.

#### Scenario: The diff is inspected

- **WHEN** `git status` is inspected
- **THEN** only `README.md`, `docs/`, LICENSE, `crates/*/examples/`, and this change's own files appear

#### Scenario: An example pulls in a forbidden dependency

- **WHEN** an example causes a production crate to depend on `axum`, `sqlx`, a provider SDK, or a product crate
- **THEN** `platform-arch-tests` MUST fail and the change MUST be rejected

#### Scenario: The change asserts registry publication

- **WHEN** the README or docs assert that a crate is published to a registry
- **THEN** the claim MUST be removed; `publish = false` stays until a separate decision
