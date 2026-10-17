# readiness Specification

## Purpose

Define what post-MVP readiness means for the `cargo-sonar-boost` quality
dashboard: a runnable clean-clone quickstart, reproducible visual evidence, a
tool-availability matrix, a headless CI recipe, an error-case catalog, and
consistent MIT metadata.

## ADDED Requirements

### Requirement: Clean-clone quickstart is runnable

The repository SHALL provide a single documented quickstart that installs and
runs the dashboard from a clean clone using only committed files. The quickstart
MUST include the local install command and the demonstration command.

#### Scenario: A new user follows the quickstart

- **WHEN** a user clones the repository and runs the documented install and demo commands in order
- **THEN** the tool installs and `cargo sonar-boost --demo` completes without requiring the user to edit source files

#### Scenario: Quickstart command is missing or wrong

- **WHEN** the documented command fails because it references a flag, path, or tool that the shipped version does not provide
- **THEN** the readiness claim is FAIL and the document MUST be corrected before completion

### Requirement: Visual evidence is present and reproducible

The repository SHALL contain HTML dashboard screenshot(s) and a demo walkthrough
GIF, each generated from the shipped `--demo` mode and accompanied by the command
that produced it.

#### Scenario: A reader evaluates the product

- **WHEN** a reader opens the README before installing
- **THEN** they can see the HTML dashboard as images and the repository names the exact command that regenerates them

#### Scenario: Evidence cannot be generated

- **WHEN** the environment lacks a browser or a recorder, or the demo command fails
- **THEN** the missing asset MUST be recorded as BLOCKED with the failed command and output, and no placeholder or fabricated image may be committed

### Requirement: Tool availability is documented

The repository SHALL document which underlying tools are required versus
optional, the probe used to detect each, the exact behavior when a tool is
absent, and which dashboard dimension each absence leaves empty. The document
MUST describe only behavior present in the shipped version.

#### Scenario: A user runs without all tools installed

- **WHEN** a user runs the dashboard and an optional tool is not installed
- **THEN** the document states the skip message and which dimension is left empty, matching the observed run

#### Scenario: Documentation claims an unsupported probe or message

- **WHEN** the tool-availability document describes a probe or skip message the analyzer does not implement
- **THEN** the claim is FAIL and MUST be removed or moved to a separate proposed change

### Requirement: Headless CI integration recipe is documented

The repository SHALL provide a copy-paste CI recipe that runs the dashboard
headlessly using only the shipped flags (`--no-open` and `--output-dir`) and that
retains the generated report artifacts.

#### Scenario: A maintainer integrates the dashboard into CI

- **WHEN** a maintainer copies the documented recipe into a CI workflow
- **THEN** the dashboard is generated without opening a browser and emits `report.json` and `report.html` into the configured output directory

#### Scenario: Recipe depends on an unshipped flag

- **WHEN** the recipe uses a flag or output contract that the shipped CLI does not support
- **THEN** the recipe is FAIL and MUST be rewritten using only shipped behavior

### Requirement: Error-case catalog is documented

The repository SHALL document observed failure modes for the quality workflow,
including at least a missing `Cargo.toml`, a clippy failure, each optional tool
being absent, a browser-open failure, and an unwritable output directory, each
with the observed output and the next action.

#### Scenario: A user hits a failure

- **WHEN** a user encounters one of the documented failure modes
- **THEN** the catalog shows the observed output and a concrete next action

#### Scenario: A failure mode is invented

- **WHEN** the catalog describes a failure that was not observed and reproduced
- **THEN** the entry MUST be removed or marked as unverified rather than presented as cataloged behavior

### Requirement: MIT license and package metadata remain consistent

The repository SHALL retain its MIT license and SHALL keep `LICENSE`, the README
license statement, and the `Cargo.toml` license metadata consistent.

#### Scenario: License metadata is checked

- **WHEN** `LICENSE`, `Cargo.toml` (`license`, `readme`, `repository`), and the README license section are compared
- **THEN** they agree that the project is MIT-licensed

#### Scenario: Documentation introduces a conflicting license claim

- **WHEN** new documentation states a license different from the repository `LICENSE`
- **THEN** the change is FAIL and the conflicting statement MUST be removed
