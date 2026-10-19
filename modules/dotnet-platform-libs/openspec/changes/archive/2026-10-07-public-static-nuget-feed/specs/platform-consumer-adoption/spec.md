## ADDED Requirements

### Requirement: New and legacy consumers SHALL share one portable adoption shape

The platform SHALL provide a canonical consumer configuration and an
idempotent legacy migration that retain exact package versions, allow an
execution-time version override, and contain no committed local feed path.

#### Scenario: Legacy migration applies safely

- **WHEN** a legacy consumer has one exact version across its `Platform.*` references
- **THEN** apply mode writes the canonical feed mapping and single defaulted version property, then its check mode passes

#### Scenario: Legacy migration detects ambiguity

- **WHEN** a legacy consumer carries multiple, floating or unresolved `Platform.*` versions
- **THEN** migration exits non-zero and writes nothing

#### Scenario: Already conforming consumer

- **WHEN** check or apply mode examines an already conforming consumer
- **THEN** it succeeds without changing bytes

### Requirement: Agent guidance SHALL keep local sources user-scoped

The platform adoption skill SHALL direct agents to configure local feeds only
in the current user's NuGet configuration and to validate clean consumers
without that local configuration.

#### Scenario: Agent tests unpublished packages

- **WHEN** an agent needs an unpublished platform build
- **THEN** it packs a unique prerelease, uses the user-level local source and passes the version as an MSBuild property without editing tracked consumer configuration

#### Scenario: Agent claims portability

- **WHEN** an agent reports a consumer as portable
- **THEN** it has verified restore/build with an isolated package cache and without the user-level local source
