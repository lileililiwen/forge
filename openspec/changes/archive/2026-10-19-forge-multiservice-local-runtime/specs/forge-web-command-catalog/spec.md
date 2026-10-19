## MODIFIED Requirements

### Requirement: Exhaustive CLI command coverage

Forge SHALL represent every user-facing top-level and nested CLI command in
an authenticated, versioned web catalog with a stable ID, user-facing purpose,
category, scope, risk, and web route or explicit CLI-only disposition.
Hidden internal dispatch commands SHALL NOT be represented as operator-facing
catalog rows. The `run` group and its `run.compose` and `run.local` routes
SHALL be present as CLI-only commands; their descriptions SHALL identify
Docker Compose and GitRunByKaru as the runtime owners.

#### Scenario: Local runtime routes are discoverable and honest

- **WHEN** an authenticated operator searches for local runtime commands
- **THEN** `run.compose` and `run.local` appear once as CLI-only entries, explain their required local runtime, and make no claim that the browser can launch or supervise local processes

#### Scenario: New CLI command added

- **WHEN** a user-facing top-level or nested Clap command is added without catalog metadata
- **THEN** the command coverage check fails and identifies the missing command path

#### Scenario: Catalog is complete

- **WHEN** an authenticated operator opens command search
- **THEN** every user-facing command path from the Rust Clap tree appears once with a valid category and disposition, and hidden internal commands are omitted

#### Scenario: Forge use facade is cataloged

- **WHEN** an authenticated operator searches the catalog for `forge use`
- **THEN** the command appears once with a truthful CLI-only disposition and its project-local execution risk
