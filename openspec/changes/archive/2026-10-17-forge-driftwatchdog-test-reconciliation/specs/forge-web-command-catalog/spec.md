## MODIFIED Requirements

### Requirement: Exhaustive CLI command coverage

Forge SHALL represent every user-facing top-level and nested CLI command in
an authenticated, versioned web catalog with a stable ID, user-facing purpose,
category, scope, risk, and web route or explicit CLI-only disposition.
Hidden internal dispatch commands SHALL NOT be represented as operator-facing
catalog rows.

#### Scenario: New CLI command added

- **WHEN** a user-facing top-level or nested Clap command is added without
  catalog metadata
- **THEN** the command coverage check fails and identifies the missing command
  path

#### Scenario: Catalog is complete

- **WHEN** an authenticated operator opens command search
- **THEN** every user-facing command path from the Rust Clap tree appears once
  with a valid category and disposition, and hidden internal commands are
  omitted

#### Scenario: Forge use facade is cataloged

- **WHEN** an authenticated operator searches the catalog for `forge use`
- **THEN** the command appears once with a truthful CLI-only disposition and
  its project-local execution risk
