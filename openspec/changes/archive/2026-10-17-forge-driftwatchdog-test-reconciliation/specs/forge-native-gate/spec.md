## ADDED Requirements

### Requirement: Gate profiles keep project-owned governance checks explicit

Forge SHALL include the mandatory `source-file-size` baseline exactly once
for every resolved Gate profile. A profile name, an extension command binding,
or an unrelated declared concern SHALL NOT implicitly select
`governance-quality`; a project SHALL select that concern explicitly. A
project that explicitly selects it SHALL retain it alongside the baseline.

#### Scenario: Default profile resolves only its declared defaults and baseline

- **WHEN** Forge resolves a built-in or project-defined profile with no
  explicit `governance-quality` check
- **THEN** the plan includes one required in-process `source-file-size` row,
  contains no implicit `governance-quality` row, and does not classify the
  baseline as unbound

#### Scenario: Project explicitly selects governance quality

- **WHEN** a project explicitly selects and binds `governance-quality`
- **THEN** the plan contains the governance extension alongside exactly one
  required `source-file-size` baseline row

#### Scenario: Required extension is unbound

- **WHEN** a project explicitly selects a required extension without a command
  or exemption
- **THEN** that extension appears in `unbound_required_checks` and can force a
  block prediction, while the in-process baseline does not appear there

#### Scenario: Unscheduled concerns are reported accurately

- **WHEN** a dry-run reports concerns not selected by the profile or project
- **THEN** it does not list the mandatory `source-file-size` baseline as
  unscheduled

#### Scenario: Source-size policy cannot narrow the baseline

- **WHEN** a project supplies source-size include or exclude patterns
- **THEN** Forge retains the default baseline scan and may use the project
  policy only to add source coverage or apply a stricter limit
