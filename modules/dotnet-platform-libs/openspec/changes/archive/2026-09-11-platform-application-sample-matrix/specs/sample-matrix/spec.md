## ADDED Requirements

### Requirement: Focused adoption samples
The repository MUST provide independently buildable samples for minimal web, EF Core persistence, identity, tenant scope, and one provider adapter adoption stages.

#### Scenario: Build one stage
- **WHEN** a consumer builds a selected sample project
- **THEN** it restores only that sample's declared platform capabilities and compiles without unrelated sample projects

### Requirement: Application ownership is visible
Each sample MUST identify its application-owned stores, entities, migrations, provider configuration, and rollback boundary.

#### Scenario: Inspect persistence sample
- **WHEN** the EF Core sample is inspected
- **THEN** its context and migration fixture are in the sample and no platform package owns its schema

### Requirement: Deterministic verification
Each sample MUST have at least one automated test proving its intended platform wiring without requiring live provider credentials.

#### Scenario: Provider adapter sample
- **WHEN** the provider sample test runs without credentials
- **THEN** it uses a deterministic fake/local adapter and verifies the public contract

### Requirement: Adoption matrix metadata
The repository MUST publish machine-readable or structured metadata for each sample covering package references, target framework, external prerequisites, verification command, and rollback action.

#### Scenario: Consumer chooses a pilot
- **WHEN** a consumer reads the matrix
- **THEN** it can select the smallest matching sample and understand its prerequisites and rollback
