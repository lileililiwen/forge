## ADDED Requirements

### Requirement: Framework-neutral domain contracts
The domain package MUST target the supported application framework without referencing ASP.NET Core, EF Core, Mediator, validation libraries, provider SDKs, or application projects.

#### Scenario: Dependency inspection
- **WHEN** the package project and compiled references are inspected
- **THEN** only the allowed platform core dependency and framework references are present

### Requirement: Entity and aggregate primitives
The package MUST provide contracts and optional base implementations for typed entity identity, aggregate roots, and transient domain-event collection, including a way to inspect and clear recorded events.

#### Scenario: Aggregate records and clears an event
- **WHEN** an application aggregate raises a domain event through the provided contract or base implementation
- **THEN** the event is observable in insertion order and can be cleared without dispatching it

### Requirement: Money value object
The package MUST provide a validated money value object with normalized currency, addition, subtraction, multiplication, and explicit rejection of arithmetic across different currencies.

#### Scenario: Cross-currency arithmetic fails
- **WHEN** two money values with different currencies are added or subtracted
- **THEN** the operation throws a deterministic argument or invalid-operation exception and produces no result

### Requirement: Optional domain markers
The package MUST provide opt-in markers for soft deletion and tenant ownership without applying persistence filters or requiring inheritance.

#### Scenario: Application-owned persistence
- **WHEN** an application implements a marker on its entity
- **THEN** the platform exposes the marker data but does not alter queries or database mappings

### Requirement: Safe domain errors
Domain exceptions MUST carry a stable `Platform.Core.Error` and MUST NOT require an HTTP or provider-specific status code.

#### Scenario: Web adapter maps a domain error
- **WHEN** an application catches or translates a domain exception at its web boundary
- **THEN** it can obtain the stable error code and safe message without inspecting exception internals
