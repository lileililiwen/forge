# forge-web-project-workbench Specification

## Purpose

Let a Forge operator onboard, inspect, plan and maintain managed projects through typed browser workflows backed by the existing Forge Core.

## ADDED Requirements

### Requirement: Project-scoped typed workflows

Forge SHALL provide browser workflows for each project setup, inspection, planning and quality command mapped to this package, using the same Core validation and results as CLI/MCP.

#### Scenario: Inspect managed project

- **WHEN** an authenticated operator opens a managed project
- **THEN** the browser displays current manifest, profile, health, features, standards and journal-backed evidence with timestamps

#### Scenario: Project not manageable

- **WHEN** a project is observed-only or lacks the required capability
- **THEN** Forge displays available evidence and disables mutation controls with the precise capability reason

### Requirement: Plan before project writes

Forge SHALL return a side-effect-free plan or diff before every browser-triggered project file mutation and SHALL bind confirmation to the displayed plan.

#### Scenario: Plan reviewed and confirmed

- **WHEN** an operator confirms the unchanged plan for the selected project and operation
- **THEN** Forge applies the corresponding Core operation and returns a journaled operation identity

#### Scenario: Plan changed or confirmation missing

- **WHEN** project state changes after planning or the confirmation is absent/invalid
- **THEN** Forge refuses the write, records no successful mutation and returns a refreshed plan or safe error

### Requirement: Isolated and safe project execution

Forge SHALL authorize each typed workflow against the global session and resolved managed project and SHALL never execute arbitrary shell text received from the browser.

#### Scenario: Cross-project path attempt

- **WHEN** a request attempts to address files outside the selected registered project root
- **THEN** Forge refuses the request and leaves project files and operation journal unchanged

#### Scenario: CLI-only operation

- **WHEN** a command requires interactive terminal input or lacks a safe typed Core contract
- **THEN** the browser explains that it is CLI-only and does not offer a nonfunctional control

### Requirement: Workflow outcome and partial failure visibility

Forge SHALL show validation, provider, stale-state and partial-operation outcomes without presenting failed or unavailable work as successful.

#### Scenario: Partial operation failure

- **WHEN** an accepted operation completes some stages and fails another
- **THEN** the browser shows operation identity, each stage result, recovery guidance and links to preserved evidence

#### Scenario: Stale evidence

- **WHEN** evidence is stale or project state no longer matches a plan
- **THEN** Forge labels its observation time and requires a new plan before mutation
