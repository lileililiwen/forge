# semantic-ui-patterns Specification

## Purpose
TBD - created by archiving change semantic-ui-patterns. Update Purpose after archive.
## Requirements
### Requirement: Semantic UI catalog and states

Forge SHALL describe reusable UI patterns through data, interaction, typography, spacing, responsive, loading, error, success, form, accessibility and navigation contracts.

#### Scenario: Semantic UI catalog and states success

- **WHEN** a CRUD table pattern is inspected
- **THEN** its sorting/filtering/pagination and state/accessibility contracts are visible with supported adapters

#### Scenario: Semantic UI catalog and states failure

- **WHEN** a pattern lacks required error or keyboard/focus behavior
- **THEN** verification blocks its supported status

#### Scenario: Semantic UI catalog and states boundary

- **WHEN** a pattern has a web implementation but no Flutter mapping
- **THEN** the catalog marks Flutter unsupported without substituting copied web markup

### Requirement: Deterministic UI installation

Forge SHALL install pinned compatible pattern implementations that remain editable ordinary source and preserve the host project design conventions.

#### Scenario: Deterministic UI installation success

- **WHEN** a compatible form pattern is installed
- **THEN** its validation, pending, failure and success behavior pass adapter tests

#### Scenario: Deterministic UI installation failure

- **WHEN** an installation would overwrite a customized component
- **THEN** the operation stops with an ownership conflict

#### Scenario: Deterministic UI installation boundary

- **WHEN** Forge is removed after installation
- **THEN** the installed pattern continues to build and operate through the project native toolchain

