## ADDED Requirements

### Requirement: Explicit target selection
The tool MUST require or clearly resolve an explicit absolute project/repository directory and MUST report the resolved target before running checks.

#### Scenario: Sibling project audit
- **WHEN** the user supplies `--project-dir` for a sibling repository
- **THEN** every check runs against that directory and the output identifies it

### Requirement: Read-only diagnostics
Default and preview modes MUST NOT modify project files, package references, lock files, git state, or generated artifacts.

#### Scenario: Preview package alignment
- **WHEN** the tool detects an outdated or floating platform reference
- **THEN** it reports a proposed change without writing the project

### Requirement: Adoption checks
The tool MUST inspect SDK compatibility, package pinning, test-only package boundaries, project/solution discoverability, and nullable/warnings-as-errors signals, with evidence for each result.

#### Scenario: Test package boundary violation
- **WHEN** a production project references a platform testing package
- **THEN** the tool reports a failed boundary check with the project and reference evidence

### Requirement: Environment-blocked classification
Unavailable feeds, Docker, databases, or external services MUST be classified separately from source failures and MUST include a bounded rerun instruction.

#### Scenario: Docker unavailable
- **WHEN** an optional integration check cannot reach Docker
- **THEN** the result is environment-blocked and does not claim that the consumer source is invalid

### Requirement: Stable automation output
The tool MUST provide stable exit codes and machine-readable JSON containing target, check identifier, status, evidence summary, and remediation or rerun guidance without secrets.

#### Scenario: CI consumption
- **WHEN** CI invokes the tool in JSON mode
- **THEN** it can fail on source failures while separately counting environment-blocked checks
