## ADDED Requirements

### Requirement: MVP profile discovery

Forge SHALL list and inspect versioned descriptors for all five MVP profiles with supported capabilities, packages, layout, conventions, build/test commands, deployment defaults and quality policies.

#### Scenario: MVP profile discovery success

- **WHEN** the profile catalog is queried
- **THEN** all five MVP IDs and their descriptor versions are visible

#### Scenario: MVP profile discovery failure

- **WHEN** a malformed descriptor omits required validation commands
- **THEN** registration fails with the missing field named

#### Scenario: MVP profile discovery boundary

- **WHEN** a valid profile has no database requirement
- **THEN** the descriptor does not force a database dependency

### Requirement: Stack compatibility contract

Forge SHALL resolve a profile only when its declared language, toolchain and capability constraints are compatible and SHALL explain unsupported combinations.

#### Scenario: Stack compatibility contract success

- **WHEN** a compatible profile version is selected
- **THEN** the resolver returns its exact version and adapter identity

#### Scenario: Stack compatibility contract failure

- **WHEN** a Flutter profile is asked to install server-side postgres
- **THEN** resolution fails before file changes and suggests a backend boundary

#### Scenario: Stack compatibility contract boundary

- **WHEN** a required toolchain is unavailable locally
- **THEN** preflight reports the missing prerequisite without claiming the profile was tested
