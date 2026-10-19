# Platform.Jobs.Hangfire

Optional Hangfire adapter for the engine-neutral `Platform.Jobs` contracts.
The adapter implements `IJobDispatcher` and `IRecurringJobRegistry` on top of
Hangfire, restores application-owned tenant/subject context inside each job
scope, exposes an opt-in protected dashboard, and reports safe storage health.
The application owns storage configuration, job definitions, dashboard
authorization, and retry ownership.

## Registration

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

The registration is idempotent and uses `TryAdd` for the dispatcher,
registry, and health check, so application-owned implementations win.

## Context propagation

Register an application-owned `IJobExecutionContext` bridge to capture the
ambient tenant/subject when a job is created and restore it inside the job's
execution scope:

```csharp
services.AddSingleton<IJobExecutionContext, MyTenantContextBridge>();
```

Jobs created without an active context run context-free. The restoration is
disposed when the job scope ends, so context never leaks between jobs.

## Handling dispatched payloads

Register an application-owned `IJobPayloadHandler`; every dispatched payload
is routed to it inside the restored context. Argument values round-trip as
JSON values (strings, numbers, booleans; complex values arrive as
`JsonElement`).

## Recurring jobs

Decorate handlers with `[RecurringJob]` and register their descriptors on
startup through `IRecurringJobRegistry`; the adapter attaches the platform
recurring executor to the cron expression with the descriptor's time zone.

## Dashboard

The dashboard is disabled by default. Enable it with an authorization
callback — the platform never ships credentials or a default policy:

```csharp
app.UsePlatformHangfireDashboard();
```

## Health

`HangfireStorageHealthCheck` probes storage reachability and reports a
redacted diagnostic (exception type only — never messages, connection
strings, or storage responses). `HangfireJobsProviderStatus` exposes the
last probe outcome.

## Retry ownership

The adapter does not add retry policies. Hangfire's automatic retry
configuration remains the retry owner; `IJobTelemetry.JobFailed` records a
redacted failure and the exception is rethrown so Hangfire's retry model
stays in charge.
