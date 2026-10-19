## ADDED Requirements

### Requirement: End-to-end Hangfire tests SHALL own their lifecycle

Each end-to-end test SHALL create, start, and dispose an isolated host, service provider, Hangfire storage, and worker lifecycle, and SHALL enqueue jobs only after the worker is ready.

#### Scenario: Dispatched job executes with no ambient context
- **WHEN** an isolated test host enqueues a context-free payload after worker startup
- **THEN** the payload executes successfully and the test can dispose the host without a disposed-dispatcher failure

#### Scenario: Tests run sequentially
- **WHEN** the end-to-end suite runs multiple tests in one process
- **THEN** no test observes storage, dispatcher, scope, or hosted-server state from another test

### Requirement: Job lifecycle failures SHALL be surfaced

The adapter tests SHALL fail when enqueue or execution occurs after disposal, rather than converting the failure into a timeout or skipping the assertion.

#### Scenario: Enqueue after disposal
- **WHEN** a dispatcher is used after its owned storage lifecycle has ended
- **THEN** the test SHALL observe a deterministic failure identifying the invalid lifecycle

### Requirement: Job reliability tests SHALL use bounded synchronization

End-to-end tests SHALL use explicit readiness and completion signals with bounded cancellation instead of unbounded waits or arbitrary sleep-only synchronization.

#### Scenario: Worker does not start
- **WHEN** the worker fails to become ready within the configured test timeout
- **THEN** the test fails with the startup error and does not hang