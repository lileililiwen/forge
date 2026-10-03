# platform-testing-toolkit Specification

## Purpose

The platform-testing-toolkit specification covers test doubles SHALL be deterministic and the related scenarios that govern this capability across the platform.
## Requirements
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

### Requirement: Testing helpers SHALL remain production-independent

Testing-support packages SHALL be referenced only by test projects and SHALL not be dependencies of production platform packages.

#### Scenario: Architecture dependency check
- **WHEN** the platform architecture tests inspect production project references
- **THEN** any reference to a testing-support package fails the check

### Requirement: Deterministic fixtures SHALL control time and context

The toolkit SHALL provide deterministic clock and tenant/subject context fixtures that can be reset and restored between tests.

#### Scenario: Context cleanup
- **WHEN** a test executes with a tenant and subject context and completes or fails
- **THEN** fixture cleanup restores the prior context and does not leak it to the next test

### Requirement: Scenario fakes SHALL record observable behavior

Fake eventing, caching, storage, quota, and identity lifecycle components SHALL expose recorded calls and configurable outcomes through public contracts.

#### Scenario: Inject provider failure
- **WHEN** a test configures a fake provider to return a transient failure
- **THEN** the consumer can assert the safe outcome, retry behavior, and recorded call without a live provider

### Requirement: Integration prerequisites SHALL be explicit

Optional web and Testcontainers fixtures SHALL identify required infrastructure and SHALL report unavailable infrastructure as skipped or unverified rather than passing.

#### Scenario: Docker unavailable
- **WHEN** a provider integration test requires Docker and Docker is unavailable
- **THEN** the test result identifies the prerequisite and does not claim provider behavior was verified
