# artifact-and-ci-baseline Specification

## REMOVED Requirements

### Requirement: CI executes Forge's own verification surfaces

**Reason:** The CI check graph now validates Forge-owned contract assets
offline. The former parity-walk scenario is replaced by direct manifest,
family-catalog, schema, and fixture validation.

## ADDED Requirements

### Requirement: CI executes Forge verification and offline contract validation

Forge's continuous integration SHALL run the read-only surfaces, the Gate
runtime, offline contract manifest and schema validation, and native-build
evidence in named jobs, with explicit permissions and timeouts. Required tools
and validators SHALL fail loudly when unavailable rather than skip to green.

#### Scenario: Gate runtime executes in CI

- **WHEN** the gate job installs the declared runtime and initializes its store
  in the runner workspace
- **THEN** `forge gate` runs for real and the job mirrors the runtime's verdict,
  including a blocked outcome

#### Scenario: Gate runtime unavailable

- **WHEN** the runtime cannot be resolved or its store cannot be initialized
- **THEN** the job fails naming the resolution attempts and records the
  unavailable verdict rather than reporting success

#### Scenario: Read surfaces gain execution

- **WHEN** the surfaces job runs doctor, the checker document, the fleet listing
  and the provider matrix
- **THEN** each returns a parseable document and a regression fails the job

#### Scenario: Offline contract validation is required

- **WHEN** CI validates Forge-maintained contract assets
- **THEN** the manifest, supported-family schemas, fixtures and support catalog
  are checked without an upstream checkout, and a missing validator fails

#### Scenario: Long native evidence is not dropped

- **WHEN** the job graph is split
- **THEN** the native scaffold build evidence still runs in its own named job
  instead of being skipped with the fast suite
