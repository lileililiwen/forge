# platform-capability-manifest Specification

## Purpose

The portfolio now has shared runtime libraries, a Gate executor, and repeated release scripts, but no common capability vocabulary showing which projects need identity, admin, jobs, billing, storage, or release evidence. Without that vocabulary, adoption becomes package copying rather than controlled reuse.
## Requirements
### Requirement: Declare reusable capabilities explicitly

Projects MAY declare identity, admin, tenancy, billing, jobs, storage, and
release capabilities, and the cross-cutting shared capabilities telemetry,
rate-limiting, idempotency, and feature-flags. Governance MUST treat those
declarations as intent and must not imply that the capabilities are implemented
or production-ready.

A cross-cutting shared capability is one that applies to a provider, a product,
a library, and a tool alike, and whose id is bound by the capability registry.
Every project profile that declares a `supported_capabilities` list MUST
include the cross-cutting shared capability ids, so a consumer can record a
lawful `consumes[]` adoption of a registered shared capability without a
spurious `CAPABILITY_UNSUPPORTED` warning. A capability id that the profile's
list does not contain MUST still be reported as WARNING
`CAPABILITY_UNSUPPORTED`, so the supported set stays intentional and a
role-shaped shared capability is admitted only by a profile that decides to.

#### Scenario: Declared capability

- **WHEN** a project declares a capability with an owner and evidence state
- **THEN** governance reports the declaration without fabricating verification

#### Scenario: Missing evidence

- **WHEN** a declared capability has no required native evidence
- **THEN** governance reports it as unverified or review-required

#### Scenario: A cross-cutting shared capability is declarable in every profile

- **WHEN** a project declares a cross-cutting shared capability such as
  `telemetry` under a profile whose `supported_capabilities` list contains it
- **THEN** governance reports no `CAPABILITY_UNSUPPORTED` warning for it

#### Scenario: A capability outside the profile's supported set still warns

- **WHEN** a project declares a capability id that its profile's
  `supported_capabilities` list does not contain
- **THEN** governance reports WARNING `CAPABILITY_UNSUPPORTED` naming the
  capability and the profile

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

### Requirement: Projects declare their interfaces

A project MAY declare the boundaries it provides and consumes in an optional
`interfaces` array in its `.project.json`. Every entry SHALL name a stable
kebab-case `id`, exactly one of `provides` or `consumes`, a `mechanism` from the
closed set `invoke`, `consume`, `observe`, `sibling`, and an `entry` expressed in
that mechanism's vocabulary. Every entry whose mechanism is not `sibling` SHALL
name a `contract` as `"<name>/<major>.<minor>.<patch>"` with both halves
present. Every `consume` and `observe` entry SHALL carry a `sha256:` `digest`.
A `consumes` entry SHALL name its `provider`. An absent `interfaces` array SHALL
be valid and SHALL produce no finding, so a project that declares nothing is
never charged for a boundary it did not claim.

`mechanism: sibling` — a repository-relative invocation of another project's
checkout — SHALL be lawful only when the consuming project also declares that
provider `required: true` and carries a digest for the edge; otherwise
governance SHALL report the edge as unpinned and SHALL name the two lawful
alternatives (declare `consume` with a digest, or publish an artifact).

#### Scenario: Interface declared with a contract

- **WHEN** a project declares an interface with mechanism `invoke` and contract
  `platform.gate-result/1.0.0`
- **THEN** governance accepts the declaration and reports no interface finding

#### Scenario: Mechanism outside the closed set

- **WHEN** a project declares a mechanism outside the four accepted values
- **THEN** governance reports ERROR `INTERFACE_MECHANISM_UNKNOWN` naming the
  interface id and the accepted set

#### Scenario: Contract without a version

- **WHEN** a non-sibling entry names a contract with no version
- **THEN** governance reports ERROR `INTERFACE_CONTRACT_UNVERSIONED`; the entry
  is not otherwise honoured

#### Scenario: Sibling edge without a digest

- **WHEN** a project declares a `sibling` edge without a digest or without
  declaring the provider required
- **THEN** governance reports `INTERFACE_SIBLING_PATH_UNPINNED` and names both
  lawful alternatives

#### Scenario: Nothing declared

- **WHEN** a project has no `interfaces` array
- **THEN** the declaration validates and governance reports no interface finding

### Requirement: A vendored copy is lawful only with provenance

A project that keeps a byte copy of another project's artifact SHALL declare it
as a `consume` interface naming the provider, the contract, the version and the
`sha256` digest of the retained bytes. A byte copy of a registered provider's
artifact that no declaration accounts for SHALL be reported as
`INTERFACE_VENDORED_WITHOUT_DIGEST`, so a copy is distinguishable from a
consumption edge and can be reviewed as one.

#### Scenario: Declared vendored copy

- **WHEN** a project keeps another project's checker bytes and declares the
  consumption edge with the contract, the version and the digest
- **THEN** governance reports no vendored-copy finding

#### Scenario: Undeclared copy

- **WHEN** a project holds a byte copy of a registered provider's artifact with
  no matching declaration
- **THEN** governance reports `INTERFACE_VENDORED_WITHOUT_DIGEST` naming the
  path and the provider
