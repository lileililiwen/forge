# doctor-maturity-assessment Specification

## ADDED Requirements

### Requirement: DriftWatch workspace marker detection

Doctor SHALL recognize the DriftWatch configuration surface that sibling
projects actually use — `driftwatch.toml`, `gate.toml`, `.ai-gate/gate.yaml`
and the `.driftwatch/` state directory — alongside legacy names, and SHALL
report presence as evidence only, never as maturity by itself.

#### Scenario: Checker-configured project detected

- **WHEN** a project contains only `driftwatch.toml` with configured
  checkers
- **THEN** doctor reports DriftWatch as configured and names the file

#### Scenario: Gate-manifest project detected

- **WHEN** a project contains only `.ai-gate/gate.yaml`
- **THEN** doctor reports DriftWatch as configured and names the file

#### Scenario: No markers

- **WHEN** a project contains none of the recognized names
- **THEN** doctor reports no DriftWatch configuration as a distinct state
  from a configured-but-failing run
