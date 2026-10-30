# artifact-and-ci-baseline Specification

## Purpose

Forge's own CI executes every surface, the gate runtime and the contract parity walk in named jobs with explicit permissions and timeouts, produces a reproducible installable artifact with a published digest, and enforces a declared minimum supported Rust version derived from the binding dependency floor.
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

### Requirement: Honest labelling and explicit non-publication

No workflow step, script label or document SHALL name a check it does not
run. Forge's CI SHALL produce local artifacts without publishing, tagging,
deploying or promoting them on branches or pull requests. Only a user-pushed
tag exactly matching `v< Cargo.toml version >` MAY trigger release publication,
and only after every required Forge CI job succeeds. The release job SHALL
reuse the artifact job's verified archive and checksum and SHALL refuse to
overwrite an existing release or asset.

#### Scenario: Label matches command

- **WHEN** a step is labelled with a check name
- **THEN** the commands it runs include that check, or the label is removed

#### Scenario: Artifact is not promoted

- **WHEN** the artifact job completes for a branch push or pull request
- **THEN** the archive and digest exist as CI artifacts only, and no registry,
  package host, remote tag or deployment target is contacted

#### Scenario: Explicit verified tag publishes the archive

- **GIVEN** the pushed tag matches the Cargo package version and every
  required CI job succeeds
- **WHEN** the tagged workflow completes
- **THEN** it publishes the exact verified archive and digest as a GitHub
  Release, without changing the source tag or deploying elsewhere

#### Scenario: Failed verification blocks release publication

- **WHEN** a required CI job or artifact verification fails
- **THEN** the release job does not run and no GitHub Release write occurs

#### Scenario: Existing release is immutable

- **WHEN** a release already exists for the requested tag
- **THEN** publication fails without clobbering, deleting or replacing its
  assets

#### Scenario: Gate block is reported, not laundered

- **WHEN** the real gate returns blocked or an unresolved review requirement
- **THEN** the evidence records that verdict and the recovery path, and no
  configuration is adjusted to obtain a passing job

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

### Requirement: CI installs every tool it invokes

Every Forge CI job SHALL install the external tools its steps invoke, at a
pinned version, or derive them from the workspace build. A missing tool SHALL
fail the job with an error naming it; no job SHALL skip a check because a tool
is absent.

#### Scenario: Validator is absent on a fresh runner

- **WHEN** the governance job starts without the OpenSpec CLI on `PATH`
- **THEN** it installs `@fission-ai/openspec@1.6.0` and runs strict validation,
  and a failed install fails the job rather than skipping the step

#### Scenario: Gate checker is absent on a fresh runner

- **WHEN** the gate job runs a plan whose `security` check invokes
  `cargo audit`
- **THEN** the job installs `cargo-audit` at a pinned version before running
  the Gate, and a failed install fails the job

### Requirement: The Gate job is bounded to its resolved plan

The CI gate job SHALL pass a time bound that covers the resolved plan's
per-check budgets. A wrapper timeout shorter than the plan SHALL NOT be used.

#### Scenario: Plan budgets exceed the default wrapper

- **WHEN** the plan grants `tests` and `declared-verification` 600 seconds each
- **THEN** the job runs `forge gate` with a bound of at least their sum plus
  the remaining checks, and the job mirrors the runtime's verdict

#### Scenario: Gate is blocked

- **WHEN** a bound check returns a blocking verdict
- **THEN** the job fails with that verdict and the recovery path, and no audit
  ignore or check exemption is added to make the job pass

### Requirement: The checker surface runs against a registered project

The surfaces job SHALL exercise `forge check` against a registered project. It
SHALL NOT assume that a fresh checkout has a registry row, because the checker
contract requires a registered target.

#### Scenario: Fresh checkout

- **WHEN** the surfaces job starts on a clean runner
- **THEN** it generates a scratch project under the runner's temporary
  directory, registers it and obtains an `alerts` document from `forge check`

#### Scenario: Registration or check fails

- **WHEN** generation, registration or the checker exits non-zero
- **THEN** the job fails and no partial document is treated as a pass

### Requirement: Rust sources are formatted and warning-free

The workspace SHALL pass `cargo fmt --all -- --check` and
`cargo clippy --workspace --all-targets --all-features -- -D warnings` at the
toolchain CI uses. A structural lint exception SHALL be a documented
`#[allow]` at the item, never a workflow-level suppression.

#### Scenario: Formatting drift

- **WHEN** any workspace source, including a vendored tooling crate, differs
  from rustfmt output
- **THEN** the formatting job fails and names the file

#### Scenario: Lint warning

- **WHEN** clippy emits a warning in any workspace target
- **THEN** the lint job fails, unless the item carries an explicit allow with
  its reason

### Requirement: Advisory exceptions are reachable, reasoned and agreed

Forge SHALL upgrade a dependency advisory that has a compatible fixed release.
When no fixed release exists, Forge SHALL record an exception that names the
reachable code path, why the affected operation cannot be exercised, and the
upstream trigger that ends the exception. Every vulnerability-class exception
SHALL be present in both dependency-policy files.

#### Scenario: A fixed release exists

- **WHEN** an advisory names a compatible fixed version
- **THEN** the dependency is upgraded and no exception is recorded for it

#### Scenario: No fixed release, unreachable path

- **WHEN** the only affected operation needs a private key or a signing
  operation that Forge never performs
- **THEN** the exception names that boundary and the transitive path, and both
  `deny.toml` and `.cargo/audit.toml` carry it

#### Scenario: One policy file is edited

- **WHEN** a vulnerability-class exception exists in one policy file but not
  the other
- **THEN** the policy contract test fails naming the advisory and both files

#### Scenario: The exception ends

- **WHEN** an upstream release removes the affected code path
- **THEN** the exception is removed and the upgrade is recorded, rather than
  the reason being left to rot

### Requirement: The archive shape is checked against the documented layout

Forge's artifact job SHALL verify the produced archive against the documented
layout: the versioned root holds the binary, licence, readme and changelog, and
the only nested tree is `share/forge/**`. Any other nested path, and any
absolute path, SHALL block the job.

#### Scenario: Documented layout

- **WHEN** the archive contains root files and `share/forge/contracts`,
  `share/forge/runtime-templates` and `share/forge/governance`
- **THEN** the artifact job accepts it

#### Scenario: Undocumented path

- **WHEN** the archive contains a nested path outside `share/forge`
- **THEN** the job fails naming the archive and the unexpected path

#### Scenario: Absolute host path

- **WHEN** an entry starts with `/`
- **THEN** the job fails and does not upload the artifact

