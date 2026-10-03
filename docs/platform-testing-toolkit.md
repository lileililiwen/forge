# Platform Testing Toolkit

Reusable, deterministic test fixtures that build on public platform
contracts. The toolkit is split across two packages so consumers can pull
exactly what they need without inheriting unwanted dependencies.

| Package | What it adds | Production references allowed |
| --- | --- | --- |
| `Platform.Testing` | Clocks, scenario builders, entitlement/usage fakes, `RecordingEventBus`, `TransientFailureInjector`. | No. |
| `Platform.Testing.AspNetCore` | `PlatformTestWebApplicationFactory<TEntryPoint>` boots the consumer's real application over `TestServer`, with in-memory configuration, environment selection, service replacement/decoration, deterministic clock injection, teardown hooks and a temporary directory. One host per instance. | No. |

Both packages are enforced as test-only by the architecture suite: any
`src/Platform.*.csproj` that is not itself a `*.Testing.csproj` must
not reference them. The `Platform.Architecture.Tests` project asserts
the rule across the production project set.

## Adoption

```bash
dotnet add package Platform.Testing --version 0.1.0
dotnet add package Platform.Testing.AspNetCore --version 0.1.0
```

`Platform.Testing.AspNetCore` pulls in `Microsoft.AspNetCore.Mvc.Testing`
transitively; if a consumer already pins the package, the
`Directory.Packages.props` central version wins.

## Scenarios

### Deterministic clock

`ControllableClock` is an `IClock` that never reads the system clock.
`Set(DateTimeOffset)` and `Advance(TimeSpan)` are the only mutation
APIs; both reject non-UTC values. Pair it with billing, quota, and
idempotency code paths that need a stable timeline.

### Recording event bus

```csharp
var bus = new RecordingEventBus();
await bus.PublishAsync(envelope);
var checkoutEvents = bus.EnvelopesOfType("Checkout.Completed");
```

The fake never throws. Tests that need a failure to bubble up should
wrap the publish call in a `TransientFailureInjector`.

### Failure injection

```csharp
var injector = new TransientFailureInjector()
    .WithTransient("checkout.publish", new TimeoutException("transient"))
    .WithPermanent("billing.charge", new InvalidOperationException("permanent"));

// First call throws, second call passes through.
Assert.Throws<TimeoutException>(() => injector.Run("checkout.publish", () => { }));
injector.Run("checkout.publish", () => { /* ... */ });

// Permanent injections persist until Reset.
Assert.Throws<InvalidOperationException>(() => injector.Run("billing.charge", () => { }));
injector.Reset();
```

`History` records every emission with a label, kind, and sequence so
tests can assert on call counts without parsing exception text.

### Test host

`PlatformTestWebApplicationFactory<TEntryPoint>` boots the **consumer's own**
application over `TestServer` from the named entry point — it never builds an
application of its own. Configure it in place: every member below mutates the same
factory, so the suite builds exactly one host per instance.

```csharp
await using var factory = new PlatformTestWebApplicationFactory<Program>()
    .WithEnvironment("Testing")
    .WithConfiguration("Platform:Test", "ok")
    .WithClock(new ControllableClock(DateTimeOffset.UnixEpoch))
    .ConfigureTestServices((services, configuration) =>
    {
        services.AddSingleton<IEventBus>(new RecordingEventBus());
    });

using var client = factory.CreateClient();
var response = await client.GetAsync("/health");
response.EnsureSuccessStatusCode();
```

The factory builds a `WebApplication` with `TestServer`, applies the
testing environment name, and exposes in-memory configuration. The
fluent `ConfigureTestServices` callback runs after the host's default
service registration so callers can swap platform services for fakes.

## What is not in the toolkit

The toolkit intentionally omits:

- **Testcontainers** fixtures. Docker is not available in every CI
  environment, and platform tests must run without privileged
  infrastructure. Tests that need an integration target (Postgres,
  RabbitMQ, Redis, Keycloak) gate themselves with an explicit
  Docker-availability check and report unavailability as
  unverified, not passed.
- **Mocking-framework abstractions**. The toolkit prefers recorded
  fakes built on public contracts over hand-rolled mocks. Tests that
  need to swap a service for a recording fake use `ConfigureTestServices`
  on the factory or `AddSingleton` in the consumer's test project.
- **Provider credentials, connection strings, or live-service
  assumptions.** The toolkit builds only on the platform's public
  contracts; consumers wire their own adapters.

## Environment-blocked semantics

The `scripts/audit-packages.sh` and `scripts/conformance.sh` scripts
already distinguish environment blockers from real failures by
recording the failed command and the next action. Tests that
exercise Docker-gated paths should follow the same pattern: assert
availability up front, and on `false` (Docker missing, daemon not
running) record the test as `Unverified` rather than `Passed`.