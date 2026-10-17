## ADDED Requirements

### Requirement: Clean-clone quickstart is runnable

The repository SHALL provide a single documented quickstart that installs and runs the plugin from a
clean clone using only committed files. The quickstart MUST include the local install command and the
demonstration command.

#### Scenario: A new user follows the quickstart

- **WHEN** a user clones the repository and runs the documented install and demo commands in order
- **THEN** the plugin installs and `cargo matrix-boost --demo` completes without requiring the user to edit source files

#### Scenario: Quickstart command is missing or wrong

- **WHEN** the documented command fails because it references a flag, path, or tool that the shipped version does not provide
- **THEN** the readiness claim is FAIL and the document MUST be corrected before completion

### Requirement: Visual evidence is present and reproducible

The repository SHALL contain TUI screenshot(s), HTML dashboard screenshot(s), and a demo walkthrough
GIF, each generated from the shipped `--demo` mode and accompanied by the command that produced it.

#### Scenario: A reader evaluates the product

- **WHEN** a reader opens the README before installing
- **THEN** they can see the TUI grid and the HTML dashboard as images, and the repository names the exact command that regenerates them

#### Scenario: Evidence cannot be generated

- **WHEN** the environment lacks a TTY or a recorder, or the demo command fails
- **THEN** the missing asset MUST be recorded as BLOCKED with the failed command and output, and no placeholder or fabricated image may be committed

### Requirement: Toolchain and feature matrix is documented

The repository SHALL document which toolchains and feature combinations the matrix exercises, how
feature sets are resolved, and how to read the resulting grid. The document MUST describe only
behavior present in the shipped version.

#### Scenario: A user plans a matrix run

- **WHEN** a user reads the matrix document to choose toolchains and features
- **THEN** the document states the supported toolchains, the powerset/default/exclusion behavior, and the meaning of each grid state

#### Scenario: Documentation claims unsupported behavior

- **WHEN** the matrix document describes a toolchain or option that the resolver or runner does not implement
- **THEN** the claim is FAIL and MUST be removed or moved to a separate proposed change

### Requirement: Headless CI integration recipe is documented

The repository SHALL provide a copy-paste CI recipe that runs the matrix headlessly using only the
shipped flags (`--no-open` and `--output-dir`) and that retains the generated report artifacts.

#### Scenario: A maintainer integrates the plugin into CI

- **WHEN** a maintainer copies the documented recipe into a CI workflow
- **THEN** the matrix runs without opening a browser and emits `report.json` and `report.html` into the configured output directory

#### Scenario: Recipe depends on an unshipped flag

- **WHEN** the recipe uses a flag or output contract that the shipped CLI does not support
- **THEN** the recipe is FAIL and MUST be rewritten using only shipped behavior

### Requirement: Error-case catalog is documented

The repository SHALL document observed failure modes for the matrix workflow, including at least a
missing rustup toolchain, no resolvable features, `cargo metadata` failure, and browser-open failure,
each with the observed output and the next action.

#### Scenario: A user hits a failure

- **WHEN** a user encounters one of the documented failure modes
- **THEN** the catalog shows the observed output and a concrete next action

#### Scenario: A failure mode is invented

- **WHEN** the catalog describes a failure that was not observed and reproduced
- **THEN** the entry MUST be removed or marked as unverified rather than presented as cataloged behavior

### Requirement: License and package metadata remain consistent

The repository SHALL retain its MIT license and SHALL keep package metadata, the README license
statement, and any linked documentation consistent with each other.

#### Scenario: License metadata is checked

- **WHEN** `LICENSE`, `Cargo.toml`, and the README license section are compared
- **THEN** they agree that the project is MIT-licensed

#### Scenario: Documentation introduces a conflicting license claim

- **WHEN** new documentation states a license different from the repository `LICENSE`
- **THEN** the change is FAIL and the conflicting statement MUST be removed