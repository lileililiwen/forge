## ADDED Requirements

### Requirement: Durable event contracts are framework-neutral
The platform SHALL provide outbox and inbox contracts that do not require ASP.NET Core, EF Core, a message broker, or an application project.

#### Scenario: Contract package is consumed by a non-EF application
- **WHEN** an application references the durable event contracts package
- **THEN** it can implement event storage without referencing EF Core or a transport SDK

### Requirement: Outbox records preserve publication metadata
The platform SHALL represent an outbox message with a stable message identifier, serialized envelope, created time, attempt state, next-attempt time, and optional tenant and correlation identifiers.

#### Scenario: Application stores an event for later publication
- **WHEN** an application writes an outbox message inside its business transaction
- **THEN** a dispatcher can recover the message and publish the original envelope with its metadata intact

### Requirement: Dispatch claims are recoverable
The platform SHALL support claiming pending messages with a lease and SHALL make expired leases eligible for another worker.

#### Scenario: Worker crashes during dispatch
- **WHEN** a worker claims a message and its lease expires without completion
- **THEN** a later worker can claim the message again

### Requirement: Retry and dead-letter decisions are explicit
The platform SHALL classify dispatch outcomes as succeeded, retryable failure, or permanent failure and SHALL calculate a bounded next-attempt time for retryable failures.

#### Scenario: Retry limit is exceeded
- **WHEN** a message has exhausted its configured attempts
- **THEN** the message enters a dead-letter state and is not automatically retried

### Requirement: Inbox processing suppresses duplicates
The platform SHALL provide an inbox operation that atomically identifies whether a message identifier is new, already completed, or currently claimed.

#### Scenario: Duplicate delivery arrives after success
- **WHEN** an inbox receives a message identifier already marked completed
- **THEN** it returns a duplicate decision without invoking the application handler
