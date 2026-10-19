## ADDED Requirements

### Requirement: RabbitMQ durable publisher

The adapter SHALL implement the platform durable publisher contract and publish envelopes using an application-configured exchange, routing key, serializer, and connection.

#### Scenario: Confirmed publish
- **WHEN** a valid envelope is published and RabbitMQ confirms it
- **THEN** the adapter returns success and does not expose broker response bodies or credentials

### Requirement: Safe transient failure

The adapter SHALL classify connection, channel, timeout, and broker-unavailable failures as safe transient failures suitable for durable retry.

#### Scenario: Broker unavailable
- **WHEN** a publish cannot reach the configured broker
- **THEN** the adapter returns a transient failure and leaves the durable message eligible for retry

### Requirement: Explicit topology ownership

The adapter SHALL require application-provided topology and event-type registration and SHALL not create product queues, migrations, or event schemas implicitly.

#### Scenario: Unregistered event
- **WHEN** an envelope payload type has no registered topology mapping
- **THEN** publishing fails with a configuration error before an ambiguous message is sent

### Requirement: Cancellation and bounded waits

The adapter SHALL honor cancellation tokens and configured confirm/connect timeouts without hanging a dispatcher indefinitely.

#### Scenario: Cancelled publish
- **WHEN** the caller cancels during connection or confirmation
- **THEN** the adapter stops waiting and returns a cancellation or transient result without swallowing cancellation
