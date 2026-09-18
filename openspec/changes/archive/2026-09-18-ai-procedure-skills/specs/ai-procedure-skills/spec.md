## ADDED Requirements

### Requirement: Discoverable operation procedures

Forge SHALL expose versioned SOPs for the named lifecycle workflows using stable CLI/MCP operations and explicit prerequisites and verification.

#### Scenario: Discoverable operation procedures success

- **WHEN** a create-project procedure is loaded
- **THEN** it directs discovery, compatible component resolution, deterministic creation, doctor, tests, quality checks and gap reporting

#### Scenario: Discoverable operation procedures failure

- **WHEN** a procedure references an unavailable or unstable operation
- **THEN** validation marks it unusable for that installed Forge version

#### Scenario: Discoverable operation procedures boundary

- **WHEN** the agent provider changes
- **THEN** the procedure retains its platform-neutral workflow and Core contracts

### Requirement: Procedures do not bypass Core

Forge SHALL keep implementation and mutation validation in Core and preserve operation-specific execution authority when procedures run.

#### Scenario: Procedures do not bypass Core success

- **WHEN** an upgrade SOP reaches a semantic conflict
- **THEN** it requests a bounded spec through the existing workflow

#### Scenario: Procedures do not bypass Core failure

- **WHEN** a procedure or model asks to bypass a failed release check
- **THEN** Core rejects the operation

#### Scenario: Procedures do not bypass Core boundary

- **WHEN** the workflow ends with unresolved findings
- **THEN** its result reports the gaps rather than asserting completion
