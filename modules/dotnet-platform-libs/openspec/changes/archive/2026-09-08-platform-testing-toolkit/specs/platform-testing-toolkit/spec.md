## ADDED Requirements

### Requirement: Test doubles SHALL be deterministic

The toolkit SHALL provide a controllable clock and SHALL not read system time during a test unless the test explicitly chooses a system-backed implementation.

#### Scenario: Advance time in a test

- **WHEN** a test advances the fake clock
- **THEN** subsequent platform consumers observe the advanced UTC time

### Requirement: Entitlement fakes SHALL be inspectable

The entitlement fake SHALL allow a test to configure snapshots and inspect lookup or invalidation behavior without a real database.

#### Scenario: Feature changes after invalidation

- **WHEN** a test changes a configured entitlement and invalidates a subject
- **THEN** the next lookup observes the new snapshot

### Requirement: Usage fakes SHALL support test assertions without a framework dependency

The usage fake SHALL record usage and return deterministic counts or decisions without embedding xUnit, NUnit, or a mocking framework.

#### Scenario: Record and count usage

- **WHEN** a test records two events for a metric
- **THEN** the fake returns the expected count for the requested subject and window

### Requirement: Test dependencies SHALL not leak into production packages

Production platform projects SHALL not reference the testing toolkit, and the testing toolkit SHALL be packable as a separate test-only package.

#### Scenario: Inspect package dependency graph

- **WHEN** a production package is packed
- **THEN** its dependency graph contains no testing toolkit or assertion package
