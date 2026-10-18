# forge-native-gate Specification

## MODIFIED Requirements

### Requirement: Project checks extend the baseline

Forge SHALL accept explicitly declared project-specific checks through the existing `gate.toml` or `.ai-gate/gate.yaml` format. A project extension SHALL have a unique stable id and explicit command binding and SHALL never replace or suppress the mandatory baseline. The `full` Gate profile SHALL schedule all valid enabled project extensions alongside the baseline. The `fast` profile SHALL omit project extensions explicitly while retaining the mandatory in-process baseline. Forge SHALL continue applying bounded execution, result normalization, credential redaction, timeout handling, and the configured blocking policy to scheduled extension checks.

#### Scenario: Registered project check runs alongside baseline

- **WHEN** a valid Gate manifest declares one project-specific check and binds its command, and Forge runs the full profile
- **THEN** the resolved plan contains the baseline and that extension exactly once, and the extension executes once during a real run

#### Scenario: Fast profile omits extensions without passing them

- **WHEN** a valid Gate manifest declares project-specific checks and Forge runs the fast profile
- **THEN** the plan executes the baseline only and marks no omitted extension as passed

#### Scenario: Extension fails

- **WHEN** a registered required extension returns a failing or contradictory result under the full profile
- **THEN** Forge records the extension failure, retains the baseline result, continues other scheduled checks, and exits nonzero according to the blocking policy

#### Scenario: Extension declaration is invalid

- **WHEN** a manifest contains duplicate ids, malformed command bindings, or an attempt to shadow the reserved baseline id
- **THEN** Forge rejects the plan before executing any check and leaves prior evidence and operation history unchanged under either profile
