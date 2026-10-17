# readiness Specification

## Purpose

Define what post-MVP readiness means for the `cargo-audit-boost` cargo build
profiler: a runnable clean-clone quickstart, reproducible visual evidence, a
documented interpretation of the profiler's output, a headless CI recipe, an
error-case catalog, and consistent MIT metadata.

## ADDED Requirements

### Requirement: Clean-clone quickstart is runnable

The repository SHALL provide a single documented quickstart that installs and
runs the profiler from a clean clone using only committed files. The quickstart
MUST include the local install command and the demonstration command.

#### Scenario: A new user follows the quickstart

- **WHEN** a user clones the repository and runs the documented install and demo commands in order
- **THEN** the profiler installs and `cargo-audit-boost --demo` completes without requiring the user to edit source files

#### Scenario: Quickstart command is missing or wrong

- **WHEN** the documented command fails because it references a flag, path, or tool that the shipped version does not provide
- **THEN** the readiness claim is FAIL and the document MUST be corrected before completion

### Requirement: Visual evidence is present and reproducible

The repository SHALL contain TUI screenshot(s), HTML report screenshot(s), and a
demo walkthrough GIF, each generated from the shipped `--demo` mode and
accompanied by the command that produced it.

#### Scenario: A reader evaluates the product

- **WHEN** a reader opens the README before installing
- **THEN** they can see the TUI and the HTML report as images, and the repository names the exact command that regenerates them

#### Scenario: Evidence cannot be generated

- **WHEN** the environment lacks a PTY or a recorder, or the demo command fails
- **THEN** the missing asset MUST be recorded as BLOCKED with the failed command and output, and no placeholder or fabricated image may be committed

### Requirement: Profile output is documented

The repository SHALL document the exported `report.json` schema and how to read
the HTML report's Gantt, cached, bottleneck, and concurrency views. The document
MUST describe only fields the shipped exporter emits.

#### Scenario: A user interprets a profile

- **WHEN** a user has a generated `report.json` or `report.html`
- **THEN** the document explains each field and view and what a bottleneck or cached crate means

#### Scenario: Documentation invents a field

- **WHEN** the profile document describes a field or metric the exporter does not emit
- **THEN** the claim is FAIL and MUST be removed or moved to a separate proposed change

### Requirement: Headless CI integration recipe is documented

The repository SHALL provide a copy-paste CI recipe that profiles a build
without an interactive terminal and retains the generated report artifacts.

#### Scenario: A maintainer integrates the profiler into CI

- **WHEN** a maintainer copies the documented recipe into a CI workflow
- **THEN** a profile run completes headlessly and emits `target/build-scan/report.json` and `report.html` for artifact retention

#### Scenario: Recipe depends on unshipped behavior

- **WHEN** the recipe uses a flag or output contract that the shipped CLI does not support
- **THEN** the recipe is FAIL and MUST be rewritten using only shipped behavior, or the PTY requirement MUST be documented explicitly

### Requirement: Error-case catalog is documented

The repository SHALL document observed failure modes for the profiling workflow,
including at least a missing Cargo project, invalid cargo arguments, a build
failure, a non-TTY TUI invocation, and an unwritable output directory, each with
the observed output and the next action.

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

- **WHEN** `LICENSE`, `Cargo.toml`, and the README license section are compared
- **THEN** they agree that the project is MIT-licensed

#### Scenario: Documentation introduces a conflicting license claim

- **WHEN** new documentation states a license different from the repository `LICENSE`
- **THEN** the change is FAIL and the conflicting statement MUST be removed
