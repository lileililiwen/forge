## MODIFIED Requirements

### Requirement: Deterministic UI installation

Forge SHALL install pinned compatible pattern implementations that remain editable ordinary source and preserve the host project design conventions. A resolution SHALL report the plan it produced and SHALL NOT record a completed installation, because a resolution installs nothing.

#### Scenario: Deterministic UI installation success

- **WHEN** a compatible form pattern is installed
- **THEN** its validation, pending, failure and success behavior pass adapter tests

#### Scenario: Deterministic UI installation failure

- **WHEN** an installation would overwrite a customized component
- **THEN** the operation stops with an ownership conflict

#### Scenario: Deterministic UI installation boundary

- **WHEN** Forge is removed after installation
- **THEN** the installed pattern continues to build and operate through the project native toolchain

#### Scenario: A resolved UI pattern plan is not an installation

- **WHEN** a resolution produces a plan for a profile
- **THEN** the journal records a planned resolution, the detail names
  the run, and no pattern is recorded as installed
