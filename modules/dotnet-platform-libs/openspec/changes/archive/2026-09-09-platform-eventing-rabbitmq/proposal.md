## Why

The platform now has durable outbox/inbox contracts and an EF Core adapter, but a starter-kit consumer still needs to write its own RabbitMQ publisher and delivery integration. The missing adapter prevents a low-risk migration from the starter's working RabbitMQ event bus.

## What Changes

- Add an optional RabbitMQ adapter over `Platform.Eventing.Contracts` and durable dispatch contracts.
- Normalize publish, connection, retry, confirmation, and failure behavior.
- Keep event type registration, migrations, topology ownership, and replay policy application-owned.
- Add an in-process/test transport seam and failure classification tests.

## Capabilities

### New Capabilities

- `platform-eventing-rabbitmq`: RabbitMQ publisher/consumer adapter for the platform durable event boundary.

### Modified Capabilities

- None.

## Impact

Adds an optional RabbitMQ client package. Existing transport-neutral eventing contracts remain unchanged. Consumers must supply event schemas, topology policy, credentials, and application-owned handlers.
