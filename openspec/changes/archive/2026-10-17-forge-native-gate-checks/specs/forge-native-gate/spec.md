# forge-native-gate Specification

## Purpose

Forge supplies a mandatory, versioned baseline gate and runs project-declared extensions through the same `forge gate` command without an external gate runtime.

## ADDED Requirements

### Requirement: Mandatory Forge baseline checks

Forge SHALL execute the versioned `forge-baseline/1.0.0` check pack for every valid project target, including when no project gate manifest exists. The initial universal baseline SHALL include the repository-aware `source-file-size` check with a default maximum of 1,000 physical lines per eligible source file. The baseline SHALL run exactly once per gate invocation, SHALL be required, SHALL NOT be removable, exempted, or weakened by project check configuration, and SHALL report incomplete enumeration as `REVIEW_REQUIRED` rather than `PASS`.

#### Scenario: Project has no gate manifest

- **WHEN** `forge gate <PATH>` targets a valid repository without `gate.toml` or `.ai-gate/gate.yaml`
- **THEN** Forge runs the baseline `source-file-size` check, records its result in the normal Forge evidence projection, and does not report "nothing to gate"

#### Scenario: Source file exceeds the default limit

- **WHEN** an eligible repository source file has more than 1,000 physical lines and no stricter baseline configuration applies
- **THEN** the baseline reports `FAIL` with the project-relative path, observed line count, and limit, and the gate exits nonzero

#### Scenario: Baseline enumeration is incomplete

- **WHEN** the scanner cannot enumerate or read an eligible file within its declared repository boundary
- **THEN** the baseline reports `REVIEW_REQUIRED` and the gate does not report a pass

#### Scenario: Project configuration attempts to suppress the baseline

- **WHEN** a project profile omits, exempts, disables, duplicates, or weakens `source-file-size`
- **THEN** Forge executes exactly one mandatory baseline row using at least the built-in 1,000-line constraint and the project declaration cannot turn it into a pass or exemption

### Requirement: Project checks extend the baseline

Forge SHALL accept explicitly declared project-specific checks through the existing `gate.toml` or `.ai-gate/gate.yaml` format. A project extension SHALL have a unique stable id and explicit command binding; it SHALL be scheduled in addition to the mandatory baseline and SHALL NOT replace or suppress any baseline row. Forge SHALL continue applying bounded execution, result normalization, credential redaction, timeout handling, and the configured blocking policy.

#### Scenario: Registered project check runs alongside baseline

- **WHEN** a valid gate manifest declares one project-specific check and binds its command
- **THEN** Forge's resolved plan contains the baseline and that extension exactly once, and the extension executes once during a real run

#### Scenario: Extension fails

- **WHEN** a registered required extension returns a failing or contradictory result
- **THEN** Forge records the extension failure, retains the baseline result, continues other scheduled checks, and exits nonzero according to the blocking policy

#### Scenario: Extension declaration is invalid

- **WHEN** a manifest contains duplicate ids, malformed command bindings, or an attempt to shadow the reserved baseline id
- **THEN** Forge rejects the plan before executing any check and leaves prior evidence and operation history unchanged

### Requirement: Forge is the sole gate runtime for Forge CLI

The public `forge gate` command SHALL execute the gate engine bundled in the Forge workspace and SHALL NOT resolve or require a separate DriftWatchdog executable, network service, or sibling checkout. New evidence SHALL identify Forge as the runtime while historical evidence remains readable without rewrite. Dry-run SHALL resolve the same baseline-plus-extension plan and SHALL NOT execute checks or persist a verdict.

#### Scenario: No external gate executable is installed

- **WHEN** `forge gate` runs with no DriftWatchdog executable on `PATH` and no external runtime override
- **THEN** Forge executes its bundled gate engine and reports the actual baseline and extension verdicts

#### Scenario: Legacy runtime declaration exists

- **WHEN** `.project.json` contains a legacy `verification.gate_runtime` value or `FORGE_GATE_BIN` is set
- **THEN** `forge gate` still executes the bundled Forge gate engine and does not probe or spawn the named external executable

#### Scenario: Dry-run contains no verdict

- **WHEN** `forge gate --dry-run` resolves a project with or without an extension manifest
- **THEN** the plan names the baseline and every extension, executes none of them, writes no evidence or journal row, and emits no passing or blocked verdict

#### Scenario: Historical evidence is read

- **WHEN** a Forge status, doctor, readiness, API, or workbench reader loads valid evidence recorded with runtime `driftwatchdog`
- **THEN** it returns the historical record unchanged and does not rewrite the evidence merely to rename its runtime
