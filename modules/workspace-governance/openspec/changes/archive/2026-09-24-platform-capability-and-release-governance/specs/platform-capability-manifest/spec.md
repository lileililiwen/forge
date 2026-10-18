## ADDED Requirements

### Requirement: Declare reusable capabilities explicitly

Projects MAY declare identity, admin, tenancy, billing, jobs, storage, and
release capabilities. Governance MUST treat those declarations as intent and
must not imply that the capabilities are implemented or production-ready.

#### Scenario: Declared capability

- **WHEN** a project declares a capability with an owner and evidence state
- **THEN** governance reports the declaration without fabricating verification

#### Scenario: Missing evidence

- **WHEN** a declared capability has no required native evidence
- **THEN** governance reports it as unverified or review-required

### Requirement: Preserve deployment evidence boundaries

Governance MUST NOT mark a project deployable from metadata, package presence,
OpenSpec validation, or build/test success alone.

#### Scenario: Metadata-only project

- **WHEN** a project has valid metadata but no release or deployment evidence
- **THEN** `deployment.deployable` remains false

### Requirement: Record release evidence consistently

Release evidence MUST identify source revision, version, toolchain, artifacts,
digests, provenance/SBOM state, checks, and publication status.

#### Scenario: Incomplete release

- **WHEN** a release lacks artifact or provenance evidence
- **THEN** the release is unverified and cannot satisfy a deployable gate
