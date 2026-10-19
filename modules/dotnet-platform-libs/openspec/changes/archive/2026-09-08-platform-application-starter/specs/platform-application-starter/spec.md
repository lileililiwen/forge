# Platform application starter

## ADDED Requirements

### Requirement: Starter composition SHALL preserve independent capability boundaries

The starter SHALL compose platform packages through explicit options and SHALL allow each
capability to be disabled or replaced.

#### Scenario: Web-only application

- **WHEN** a host enables only web runtime and health capabilities
- **THEN** identity, billing, AI, mail, SMS, and provider SDK services are not registered

### Requirement: Generated applications SHALL be runnable in development

The template SHALL generate a host, tests, safe development configuration, deterministic fake
providers, and documented commands that boot without production credentials.

#### Scenario: Fresh template checkout

- **WHEN** a developer restores and runs the generated solution
- **THEN** the application boots, health endpoints respond, tests run, and fake provider flows are available

### Requirement: Production provider configuration SHALL fail clearly

The starter SHALL validate enabled provider configuration at startup or readiness and SHALL not
silently use demo credentials or fake success in production.

#### Scenario: Billing enabled without credentials

- **WHEN** production billing is enabled without provider credentials
- **THEN** startup/readiness reports a classified configuration failure
- **AND** checkout cannot return a false success

### Requirement: Adoption SHALL support existing applications

The starter documentation SHALL describe incremental package adoption, service replacement,
middleware ordering, versioning, and rollback without requiring a repository rewrite.

#### Scenario: Existing Razor application adopts runtime defaults

- **WHEN** an existing application adds only the web starter package
- **THEN** it can retain its own domain, persistence, identity, and UI implementations
- **AND** it receives only the explicitly enabled platform runtime behavior
