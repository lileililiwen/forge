# EF Core persistence conventions

## ADDED Requirements

### Requirement: Persistence conventions SHALL be explicit

The package SHALL let a consumer opt into each EF Core convention and SHALL not require a
particular entity base class, tenant implementation, or repository abstraction.

#### Scenario: Application registers one convention

- **WHEN** an application enables audit interception only
- **THEN** soft-delete, tenancy, and migration behavior remain disabled unless separately configured

### Requirement: Database readiness SHALL be distinct from liveness

The package SHALL expose migration and connection status through composable checks without
applying production migrations implicitly.

#### Scenario: Pending migrations exist

- **WHEN** the readiness check detects pending migrations
- **THEN** readiness reports the documented non-ready state
- **AND** the package does not mutate the database

### Requirement: Query helpers SHALL preserve application ownership

Pagination and specification helpers SHALL operate on consumer queryables without owning
domain entities or business filters.

#### Scenario: Consumer defines a business filter

- **WHEN** a consumer composes a specification with a product predicate
- **THEN** the platform applies paging/order rules without changing the predicate semantics
