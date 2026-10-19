# Platform eventing RabbitMQ

Optional RabbitMQ adapter over the durable eventing contracts
(`Platform.Eventing.Contracts`). The contracts package stays transport-neutral;
this adapter is an independent opt-in package that references only
`Platform.Eventing.Contracts` and `RabbitMQ.Client`.

## Package

| Package | Transport dependency | Purpose |
| --- | --- | --- |
| `Platform.Eventing.RabbitMq` | RabbitMQ.Client | Confirmed publishing of `DurableEventEnvelope` through an application-owned topology with safe failure classification. |

The adapter implements `IDurableEventPublisher`, so the durable outbox
dispatcher (`Platform.Eventing.EfCore`) keeps driving claiming, retries, and
dead-lettering. The adapter never owns event schemas, queues, migrations,
credentials, or retry policy.

## Adoption

```csharp
services.AddPlatformRabbitMqEventing(options =>
{
    options.Exchange = "platform.events";   // required; application-owned
    options.HostName = "rabbitmq.internal";
    options.UserName = "…";
    options.Password = "…";                 // never echoed in diagnostics
    // options.DeclareExchange = true;      // opt-in; default is no topology creation
});

services.AddSingleton<IRabbitMqEventTopology>(new RabbitMqEventTopology()
    .Map("order.created", new RabbitMqEventBinding("order.created.v1"))
    .Map("order.paid", new RabbitMqEventBinding("order.paid.v1", "billing.events")));
```

`AddPlatformRabbitMqEventing` validates the options at registration and
`TryAdd`s `IRabbitMqEventTopology` (fail-closed `UnconfiguredRabbitMqEventTopology`),
`IRabbitMqChannelFactory`, and `IDurableEventPublisher` →
`RabbitMqDurableEventPublisher`. An application-owned `IDurableEventPublisher`
registration always wins, and the topology and channel factory seams can be
replaced before the call (for shared connections or an in-process test
transport).

## Topology ownership

- Every publish resolves the envelope's `PayloadType` through
  `IRabbitMqEventTopology`. An unregistered type fails with
  `eventing.rabbitmq.unregistered_type` before any message is sent; a missing
  topology registration fails with `eventing.rabbitmq.topology_unconfigured`.
- Routing keys are deterministic: they come from the registered
  `RabbitMqEventBinding`, never from payload contents. A binding may override
  the configured exchange.
- The adapter declares no queues. It declares the configured exchange only
  when `DeclareExchange` is enabled (durable by default, `ExchangeType`
  configurable); the default is to create no topology at all.

## Failure classification

The publisher surfaces every failure as a `RabbitMqPublishException` carrying
a `RabbitMqPublishFailure` with a stable code, a fixed safe message, and a
`Permanent` flag. The exception message never carries broker response text,
connection details, or credentials, and no inner exception is attached.

| Code | Classification | Meaning |
| --- | --- | --- |
| `eventing.rabbitmq.topology_unconfigured` | Permanent | No `IRabbitMqEventTopology` is registered. |
| `eventing.rabbitmq.unregistered_type` | Permanent | The payload type has no registered binding. |
| `eventing.rabbitmq.invalid_binding` | Permanent | The registered binding is invalid. |
| `eventing.rabbitmq.connect_failed` | Transient | The broker could not be reached. |
| `eventing.rabbitmq.connect_timeout` | Transient | The bounded connect wait elapsed. |
| `eventing.rabbitmq.confirm_timeout` | Transient | The bounded publisher-confirmation wait elapsed. |
| `eventing.rabbitmq.publish_failed` | Transient | The broker did not accept or confirm the event. |

Transient failures are the retry signal: the durable outbox dispatcher catches
them and leaves the message eligible for retry under the application's
`DurableEventingOptions` retry and dead-letter policy. The adapter never
retries internally. Caller cancellation is preserved —
`OperationCanceledException` is rethrown, never converted into a provider
failure.

## Publishing semantics

- Channels are created in publisher-confirm mode with confirmation tracking;
  `PublishAsync` completes only after the broker confirms the event.
- Connection and channel lifecycle is managed by the adapter: an open channel
  is reused, a closed or failed channel is disposed and re-created on the next
  publish, and every connect is bounded by `ConnectTimeout`.
- Messages are published persistent with `application/json` content type, the
  durable `MessageId`, the correlation identifier, the occurrence timestamp,
  and `payload-type`/`tenant-id` headers.
- The adapter exposes a `Status` snapshot (`RabbitMqEventingProviderStatus`)
  with the provider name (`rabbitmq`), health state, and last stable error
  code.

## Migration from the starter kit

The starter `RabbitMqEventBus` publishes typed integration events with an
in-adapter retry loop and declares its exchange on connect. To migrate:

1. Map each starter event type to a `DurableEventEnvelope` payload type and
   register its binding in `IRabbitMqEventTopology`; keep the starter exchange
   and routing-key names so existing consumers keep working.
2. Register the adapter as the durable publisher behind the EF Core outbox
   dispatcher; the starter's in-adapter retry loop is replaced by the outbox
   lease/retry/dead-letter state machine.
3. Keep consumer handling idempotent: RabbitMQ and the outbox provide
   at-least-once delivery, and the platform inbox contracts suppress
   duplicates.

Rollback: remove the `AddPlatformRabbitMqEventing` registration and restore the
starter publisher; durable outbox rows remain application-owned and no broker
topology is deleted by the platform.

## Verification

- `tests/Platform.Eventing.RabbitMq.Tests` covers options defaults and
  validation, topology mapping, confirmed publishing, unregistered and
  unconfigured topology failures, connect/confirm timeouts, cancellation
  preservation, reconnection, exchange declaration opt-in, DI registration and
  consumer overrides, an unreachable-broker transient classification, and a
  Docker-gated integration test against a real RabbitMQ container (silently
  skipped without Docker).
- `Platform.Architecture.Tests` guards the package boundary: the adapter
  references only `Platform.Eventing.Contracts` and no ASP.NET Core, EF Core,
  Redis, Stripe, MailKit, SendGrid, Hangfire, Quartz, or MassTransit
  packages.
