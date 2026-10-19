# Durable eventing

`Platform.Eventing.Contracts` defines the durable boundary without ASP.NET Core,
EF Core, or a transport dependency. It contains `DurableEventEnvelope`, outbox
and inbox records, lease/attempt state, `IOutboxStore`, `IInboxStore`, and
`IDurableEventPublisher`. `InMemoryOutboxStore` and `InMemoryInboxStore` are
thread-safe defaults for development and deterministic tests.

`Platform.Eventing.EfCore` is optional. It maps `PlatformOutboxMessage` and
`PlatformInboxMessage` into an application-owned `DbContext` through
`ConfigurePlatformEventing`. The application owns the context, migrations,
table deployment, event types, and transport publisher.

```csharp
builder.Services.AddPlatformEventingEfCore<ApplicationDbContext>(options =>
{
    options.BatchSize = 100;
    options.MaxAttempts = 5;
});
builder.Services.AddPlatformDurableOutboxDispatcher("api-1");
```

The application must register `IDurableEventPublisher`. The optional
`Platform.Eventing.RabbitMq` adapter provides a confirmed RabbitMQ
implementation with application-owned topology registration and safe
transient/permanent failure classification (see
[`platform-eventing-rabbitmq.md`](platform-eventing-rabbitmq.md)); applications
can also register their own publisher or wrap an existing transport. The
dispatcher claims messages with a lease, publishes them at least once, marks
successful messages, and records safe retry or dead-letter state after
failures. Handler and publisher side effects must be idempotent because
at-least-once delivery is intentional.

The initial EF adapter evaluates time eligibility after a state-filtered query
to remain compatible with SQLite and relational providers that cannot order
nullable `DateTimeOffset` values portably. A provider-specific optimization can
be added later without changing the contracts.

## Ownership and rollback

The platform does not create migrations, infer tenant scope, or resolve
secrets. The durable contracts stay transport-neutral; the optional
`Platform.Eventing.RabbitMq` adapter publishes through an application-owned
exchange, routing keys, and credentials, and never creates queues or event
schemas. A pilot should first wrap its existing publisher (or adopt the
RabbitMQ adapter) and run the in-process bus in development. Rollback is
disabling the hosted dispatcher and returning to the existing publisher; the
application-owned tables remain available for inspection or later replay.

New package roots remain independently packable under `src/`, with shallow
folders for `Persistence`, `Dispatch`, and `DependencyInjection`.
