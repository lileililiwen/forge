# Platform Jobs — Hangfire adapter

`Platform.Jobs` stays engine-neutral: it defines `IJobDispatcher`,
`IRecurringJobRegistry`, `IRecurringJobHandler`, and `IJobTelemetry` without
referencing any scheduler. `Platform.Jobs.Hangfire` is the opt-in adapter that
implements those contracts on Hangfire, propagates tenant/subject context into
jobs, exposes an opt-in protected dashboard, and reports safe storage health.

## Packages

| Package | Purpose |
| --- | --- |
| `Platform.Jobs` | Engine-neutral scheduling contracts (unchanged; no Hangfire reference). |
| `Platform.Jobs.Hangfire` | Hangfire dispatcher, recurring registry, scoped context restoration, opt-in dashboard, storage health. |

The adapter depends on `Platform.Jobs`, Hangfire (`Hangfire.Core`,
`Hangfire.AspNetCore`, `Hangfire.InMemory`, `Hangfire.PostgreSql`), and the
platform options/DI/health-check abstractions. It declares the
`Microsoft.AspNetCore.App` framework reference only for the opt-in dashboard
mapping.

## Adoption

```csharp
services.AddPlatformJobs();
services.AddPlatformHangfireJobs(options =>
{
    options.Storage = HangfireStorageKind.PostgreSql;
    options.PostgreSqlConnectionString = configuration.GetConnectionString("Jobs");
    options.WorkerCount = 5;
    options.Queues = new[] { "default", "email" };
});
```

- Registration is idempotent; calling it twice is a no-op.
- `IJobDispatcher`, `IRecurringJobRegistry`, and `IHealthCheck` are added with
  `TryAdd`, so application-owned implementations registered before the adapter
  always win.
- Storage is application-selected: `HangfireStorageKind.InMemory` (default,
  non-durable, useful for development) or `HangfireStorageKind.PostgreSql`
  with an application-supplied connection string. The platform never owns
  connection strings, credentials, or migrations; the Hangfire schema is
  created by Hangfire itself on first use.

## Context propagation

Register an application-owned `IJobExecutionContext` bridge. The adapter
captures a `JobContextSnapshot` (opaque tenant/subject identifiers only) when
a job is created and restores it inside the job's execution scope before any
handler is resolved:

```csharp
services.AddSingleton<IJobExecutionContext, MyTenantContextBridge>();
```

- Jobs created without an active context run context-free.
- A job that carries a captured context without a registered bridge fails
  closed with a clear `InvalidOperationException`.
- The restoration and the DI scope are disposed when the job ends, so ambient
  context never leaks between jobs.

## Handling dispatched payloads

Register an application-owned `IJobPayloadHandler`; every dispatched payload
is routed to it inside the restored context. Argument values round-trip as
JSON values (strings, numbers, booleans; complex values arrive as
`JsonElement`).

## Recurring jobs

Decorate handlers with `[RecurringJob]` and register their descriptors on
startup through `IRecurringJobRegistry`:

```csharp
[RecurringJob("0 2 * * *", Name = "nightly-cleanup")]
public sealed class NightlyCleanupHandler : IRecurringJobHandler { ... }

registry.Register(RecurringJobAttribute.GetDescriptor(typeof(NightlyCleanupHandler)));
```

The adapter attaches the platform recurring executor to the cron expression
with the descriptor's time zone. Registering the same name again is a no-op;
the earlier descriptor is kept.

## Dashboard

The dashboard is disabled by default. Enable it with an authorization
callback — the platform never ships credentials or a default policy:

```csharp
options.DashboardEnabled = true;
options.DashboardAuthorization = context => IsAdmin(context.Request);
...
app.UsePlatformHangfireDashboard();
```

Mapping without the callback fails fast. The dashboard never displays the
storage connection string.

## Health

`HangfireStorageHealthCheck` probes storage reachability and reports a
redacted diagnostic (exception type only — never messages, connection strings,
or storage responses). `HangfireJobsProviderStatus` exposes the last probe
outcome for the `hangfire` provider.

## Retry ownership

The adapter does not add retry policies. Hangfire's automatic retry
configuration remains the retry owner; `IJobTelemetry.JobFailed` records a
redacted failure (stable `jobs.execution_failed` code, fixed safe message,
exception type name only) and the exception is rethrown so Hangfire's retry
model stays in charge. Caller cancellation is rethrown and never recorded as
a failure.

## Migration from the starter kit

1. Register the adapter alongside the starter job service
   (`AddHeroJobs`); both can coexist while you migrate.
2. Replace `FshJobFilter`/`FshJobActivator` tenant/user capture with the
   `IJobExecutionContext` bridge over the platform tenant scope and current
   user seams — no Finbuckle or starter identity types are referenced.
3. Migrate one recurring job: register its descriptor through
   `IRecurringJobRegistry` and compare execution and tenant context.
4. Move dispatched work to `IJobDispatcher` payloads handled by
   `IJobPayloadHandler`.
5. Remove the duplicate starter registration. Rollback is disabling the
   adapter registration; Hangfire storage remains application-owned.

## Test-host storage ownership

When a test fixture or application needs explicit ownership of the
`JobStorage` lifetime (for example, to share storage across hosts, to
verify that the storage is disposed with the host, or to control the
in-memory dispatcher's lifecycle deterministically), pass the storage
instance to the adapter:

```csharp
var storage = new InMemoryStorage();
services.AddPlatformHangfireJobs(options => options.Queues = new[] { "default" }, storage);
```

The supplied `JobStorage` is registered as a DI singleton so the
service provider owns its lifetime. The Hangfire `BackgroundJobServer`,
the `IBackgroundJobClient`, the `IRecurringJobManager`, and the platform
`HangfireJobDispatcher` all resolve the same instance. When the host is
disposed (via `host.StopAsync()` followed by `host.DisposeAsync()`),
the service provider disposes the storage, which joins the in-memory
dispatcher's worker thread.

The reliability test suite ships a `HangfireEndToEndHost` helper that
exercises this ownership path: each test creates, starts, and disposes
an isolated host and waits for the background server to register itself
with the storage before enqueuing. The host's `DisposeAsync` runs the
shutdown with a bounded timeout so the test suite never hangs on a
misconfigured host.
