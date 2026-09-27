# artifact-and-ci-baseline Specification

## Purpose
TBD - created by archiving change artifact-and-ci-baseline. Update Purpose after archive.
## Requirements
### Requirement: Truthful verification entry point

Forge SHALL declare and enforce one verification entry point whose recorded
command describes the package graph that actually exists, so that the
project declaration, the local script and CI name the same real checks.

#### Scenario: Declared command runs

- **WHEN** the verification command recorded in `.project.json` is executed from
  a clean checkout
- **THEN** it resolves against the real package graph, runs the declared test
  suite and exits non-zero on any failure

#### Scenario: Declaration names a nonexistent structure

- **WHEN** a declaration references a package structure the manifest does not
  define
- **THEN** the consistency test fails naming the declaration and the manifest
  rather than leaving the claim unverifiable

#### Scenario: Local and CI are one contract

- **WHEN** the local verification script and the CI job set are compared
- **THEN** every check either runs in both or is documented as CI-only with its
  reason

### Requirement: Licence and package metadata consistency

Forge SHALL ship the licence text its manifest declares, SHALL carry release
metadata that resolves against real files, and SHALL keep the binary's reported
version, the manifest version and the changelog's newest entry in agreement.

#### Scenario: Licence file matches the declaration

- **WHEN** the package manifest declares a licence identifier
- **THEN** the matching licence file exists at the repository root and is
  included in the distributed archive

#### Scenario: Version disagreement

- **WHEN** the reported CLI version, the manifest version and the newest
  changelog entry differ
- **THEN** a contract test fails quoting all three values

#### Scenario: Changelog path resolves

- **WHEN** the release plane looks for the project changelog at its default
  path
- **THEN** a real file exists there rather than a constant pointing at absent
  documentation

### Requirement: Declared toolchain floor enforcement

Forge SHALL declare a minimum supported Rust version derived from the binding
dependency floor, SHALL record the derivation and the raise procedure in an
architecture decision record, and SHALL verify the floor in CI.

#### Scenario: Floor builds

- **WHEN** the workspace is checked at the declared floor toolchain
- **THEN** every target compiles and the job passes

#### Scenario: Floor is aspirational

- **WHEN** the declared floor is lower than what a dependency actually requires
- **THEN** the floor job fails rather than the claim surviving in documentation

#### Scenario: Floor is raised

- **WHEN** a dependency floor bump raises the minimum supported version
- **THEN** the decision record names the previous floor and the binding
  dependency

### Requirement: Dependency and licence policy enforcement

Forge SHALL enforce an explicit advisory, licence, duplicate and ban policy over
its dependency graph, SHALL record every exception with its reason, and SHALL
resolve shared dependency floors in one place instead of restating them per
section.

#### Scenario: Advisory present

- **WHEN** the dependency graph contains a denied advisory
- **THEN** the policy job fails naming the crate and the advisory identifier

#### Scenario: Bundled native dependency

- **WHEN** a dependency ships a vendored or bundled native component with a
  licence outside the default allow-list
- **THEN** it is allowed by an explicit entry stating the reason, not by a
  wildcard

#### Scenario: Floors restated

- **WHEN** the development section re-declares a dependency the runtime section
  already declares
- **THEN** the inconsistency is resolved by inheritance so one floor governs

### Requirement: Reproducible installable artifact with digest

Forge SHALL produce a release artifact with a published checksum from a
documented script, SHALL install only from that artifact after digest
verification, and SHALL smoke-test the installed binary's core read surfaces.

#### Scenario: Package then install then smoke

- **WHEN** packaging, checksum verification, installation to a scratch prefix
  and the smoke script run in sequence
- **THEN** the installed binary reports the packaged version and its read
  surfaces return parseable documents

#### Scenario: Digest mismatch

- **WHEN** an artifact does not match its digest file
- **THEN** installation refuses before writing anything and names both digests

#### Scenario: Missing artifact

- **WHEN** the install script is invoked without an artifact or with a prefix
  that does not exist
- **THEN** it refuses with the exact expectation and performs no download

#### Scenario: Archive carries no host paths

- **WHEN** a produced archive is inspected
- **THEN** every entry is repository-relative and no absolute build-host path is
  present

### Requirement: CI executes Forge's own verification surfaces

Forge's continuous integration SHALL run the read-only surfaces, the gate
runtime, the contract parity walk and the native-build evidence in named jobs,
with explicit permissions and timeouts, and SHALL fail loudly when a required
tool is unavailable instead of skipping to green.

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

#### Scenario: Parity walk drifts

- **WHEN** the vendored contract set no longer matches the pinned upstream
  revision
- **THEN** the parity job fails naming the diverging files

#### Scenario: Long native evidence is not dropped

- **WHEN** the job graph is split
- **THEN** the native scaffold build evidence still runs in its own named job
  instead of being skipped with the fast suite

### Requirement: Honest labelling and explicit non-publication

No workflow step, script label or document SHALL name a check it does not run,
and Forge's baseline SHALL produce artifacts and digests without publishing,
tagging, deploying or promoting anything automatically.

#### Scenario: Label matches command

- **WHEN** a step is labelled with a check name
- **THEN** the commands it runs include that check, or the label is removed

#### Scenario: Artifact is not promoted

- **WHEN** the artifact job completes
- **THEN** the archive and digest exist as CI artifacts only, and no registry,
  package host, remote tag or deployment target was contacted

#### Scenario: Gate block is reported, not laundered

- **WHEN** the real gate returns blocked or an unresolved review requirement
- **THEN** the evidence records that verdict and the recovery path, and no
  configuration is adjusted to obtain a passing job

