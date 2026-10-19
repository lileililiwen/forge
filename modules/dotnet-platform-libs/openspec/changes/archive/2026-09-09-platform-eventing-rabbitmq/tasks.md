## 1. Adapter setup

- [x] 1.1 Create `Platform.Eventing.RabbitMq` with only the durable eventing contracts and RabbitMQ client dependency.
- [x] 1.2 Define validated options for connection, topology, serializer, confirms, and bounded operation waits.
- [x] 1.3 Study and adapt `dotnet-starter-kit/src/BuildingBlocks/Eventing/RabbitMq/` and `Outbox/OutboxDispatcher.cs`; do not copy module event types or persistence ownership.

## 2. Publisher implementation

- [x] 2.1 Implement connection/channel lifecycle with cancellation, reconnect, and disposal semantics.
- [x] 2.2 Implement application-owned topology/type registration and deterministic routing-key resolution.
- [x] 2.3 Implement publisher confirms and safe transient/permanent failure normalization.
- [x] 2.4 Add optional DI registration without replacing consumer-owned `IDurableEventPublisher` unexpectedly.

## 3. Verification and documentation

- [x] 3.1 Add fake-client/unit tests for success, timeout, cancellation, unregistered types, and broker failures.
- [x] 3.2 Add optional broker integration tests with explicit Docker gating.
- [x] 3.3 Add migration guidance for wrapping the starter RabbitMQ bus and preserving at-least-once/idempotent handling.
- [x] 3.4 Run architecture tests, serial package tests, `git diff --check`, and strict OpenSpec validation.
