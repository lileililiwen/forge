# governance-front-door Specification

## Purpose

Give the workspace one auto-triggering entry that recognises any governance
situation and routes to the owning script or spec, so agents stop guessing which
of the many audits and rules applies.

## ADDED Requirements

### Requirement: One auto-triggering entry

The repository MUST ship exactly one governance skill whose description covers
governance, project declarations, profiles and vocabulary, gates and evidence,
the rules baseline, platform adoption and shared contracts, so it activates
without being named.

#### Scenario: Governance situation triggers the door

- **WHEN** a task concerns the workspace registry, a declaration, a gate, or
  shared rules
- **THEN** the skill is the entry point and presents the routing table

### Requirement: Routes resolve to references

Each situation in the routing table MUST route to a reference file, and every
reference MUST link to the owning script or canonical spec rather than restate
policy.

#### Scenario: Every route resolves

- **WHEN** the routing table is walked
- **THEN** each row names an existing reference and each reference links an
  existing script or spec

#### Scenario: No restated policy

- **WHEN** a reference describes a rule
- **THEN** it links the canonical spec instead of copying its requirements

### Requirement: References stay valid

A reference that cites a path or `scripts/*.py` command that does not exist MUST
fail the reference check.

#### Scenario: Stale reference detected

- **WHEN** a reference cites a script that does not exist
- **THEN** the check exits non-zero naming the reference and the missing path

### Requirement: Unsafe and unknown cases are handled

The skill MUST NOT instruct pushing, deploying, or promoting, and MUST state a
safe default for a situation the routing table does not cover.

#### Scenario: Unknown situation

- **WHEN** no route matches the situation
- **THEN** the skill directs to `workspace_check.py --root .. --audit` and
  `openspec list` as the safe default

#### Scenario: Destructive action requested

- **WHEN** the situation involves a push, deploy, or promotion
- **THEN** the skill defers to the existing explicit-approval boundary rather
  than instructing the action

### Requirement: Installation is documented

`README.md` MUST document how to make the skill discoverable by an agent.

#### Scenario: Install documented

- **WHEN** a reader wants to use the skill
- **THEN** the README states the install path and method
