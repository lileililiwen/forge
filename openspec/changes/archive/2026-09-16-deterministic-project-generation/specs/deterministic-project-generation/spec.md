## ADDED Requirements

### Requirement: Equivalent deterministic creation modes

Forge SHALL implement forge new with explicit profile selection and interactive input, producing equivalent ordinary source from identical inputs and pinned assets.

#### Scenario: Equivalent deterministic creation modes success

- **WHEN** the same creation request is supplied by flags and interactive answers
- **THEN** both outputs have equivalent file contents except documented identity fields

#### Scenario: Equivalent deterministic creation modes failure

- **WHEN** creation targets a nonempty directory or a template escapes its destination
- **THEN** creation fails before overwriting existing files

#### Scenario: Equivalent deterministic creation modes boundary

- **WHEN** creation is cancelled during interactive input
- **THEN** no project or registry entry is left behind

### Requirement: Portable generated projects

Forge SHALL generate the five MVP profile projects with manifests and documented native build/test commands, without a Forge runtime dependency.

#### Scenario: Portable generated projects success

- **WHEN** a generated fixture is built using its stack toolchain with Forge unavailable
- **THEN** the native build and applicable tests succeed

#### Scenario: Portable generated projects failure

- **WHEN** generation or validation fails before promotion
- **THEN** the destination is not registered as a completed project and staged output is recoverable or cleaned

#### Scenario: Portable generated projects boundary

- **WHEN** a host lacks a target stack toolchain
- **THEN** Forge reports that native validation is unverified and does not claim a successful build
