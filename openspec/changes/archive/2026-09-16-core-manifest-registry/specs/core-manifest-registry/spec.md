## ADDED Requirements

### Requirement: Versioned manifest contract

Forge SHALL validate schema-versioned forge.yaml with project identity, profile, current and target maturity, runtime, versioned features, quality, AI, deployment, distribution and documentation metadata; unsupported versions and ambiguous manifest sources SHALL fail without mutation.

#### Scenario: Versioned manifest contract success

- **WHEN** valid metadata is loaded
- **THEN** inspect returns the normalized metadata and schema version

#### Scenario: Versioned manifest contract failure

- **WHEN** an unsupported schema or two conflicting manifest sources are found
- **THEN** a diagnostic names the conflict and leaves both files unchanged

#### Scenario: Versioned manifest contract boundary

- **WHEN** optional future integration sections are absent
- **THEN** the manifest remains valid and the project requires no enabled integrations

### Requirement: Registry and CLI identity

Forge SHALL expose list and inspect through shared Core contracts and persist project ID, name, path, Git repository, primary and mirror remotes, stack, profile, maturity, schema/platform versions, features, deployment target, runtime, last commit, quality, agent and documentation observations.

#### Scenario: Registry and CLI identity success

- **WHEN** a registered project is inspected after restarting Forge
- **THEN** its persisted identity and timestamped observations are returned

#### Scenario: Registry and CLI identity failure

- **WHEN** a different project attempts to reuse an existing ID or canonical path
- **THEN** the registry rejects the collision without changing the original record

#### Scenario: Registry and CLI identity boundary

- **WHEN** a project path is unavailable or has no runtime observation
- **THEN** list and inspect report unavailable or unknown rather than healthy, and an unknown ID returns a nonzero diagnostic

### Requirement: Independent command contract

Forge SHALL provide CLI help, version reporting and structured errors independently of GUI, AI or network services, with nonzero exit codes for failed operations.

#### Scenario: Independent command contract success

- **WHEN** help is invoked without network or an AI configuration
- **THEN** the supported command surface is displayed

#### Scenario: Independent command contract failure

- **WHEN** an unknown subcommand is supplied
- **THEN** the CLI returns a nonzero usage error without executing another operation

#### Scenario: Independent command contract boundary

- **WHEN** the registry is empty
- **THEN** list succeeds with an empty collection
