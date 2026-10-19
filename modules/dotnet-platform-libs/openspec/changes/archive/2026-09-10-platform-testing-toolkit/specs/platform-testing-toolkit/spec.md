## ADDED Requirements

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
