# Platform auditing

`Platform.Auditing` standardizes audit event capture across HTTP, security, exceptions, and
entity changes without owning a compliance store. Events are normalized, enriched, and masked
before they reach an application-owned sink. The audit storage schema, retention schedule,
product projections, and migrations remain authoritative in the consuming application.

## Packages and layout

- `Platform.Auditing.Contracts` — `Common`, `Contracts`, and `DependencyInjection`; framework-neutral
  event, sink, masking, enrichment, retention, dead-letter, and failure-policy contracts.
- `Platform.Auditing.AspNetCore` — optional request, exception, and security capture middleware.
- `Platform.Auditing.EfCore` — optional `SaveChangesInterceptor` capturing `IAuditedEntity` changes.

None of the packages reference ASP.NET Core (except the AspNetCore adapter, via
`FrameworkReference`) or EF Core (except the EfCore adapter) or Stripe, and none ship a durable
audit store. The architecture tests enforce these boundaries
(`tests/Platform.Architecture.Tests`).

## Registration

```csharp
// Minimal: in-memory sink, default masker/enricher, synchronous publish, fail-open.
services.AddPlatformAuditing();

// HTTP capture (registers the contracts pipeline too):
services.AddPlatformAuditingAspNetCore(o =>
{
    o.ExemptPathPrefixes = new[] { "/health", "/metrics" };
    o.SecurityStatusCodes = new[] { 401, 403 };
});
app.UsePlatformAuditing();

// EF Core change capture (registers the contracts pipeline too):
services.AddPlatformAuditingEfCore();
// Wire the interceptor into the application DbContext:
builder.Services.AddDbContext<AppDbContext>((sp, o) =>
    o.UseSqlServer(conn).AddInterceptors(sp.GetRequiredService<ISaveChangesInterceptor>()));
```

`AddPlatformAuditing` binds and validates `AuditOptions` and registers `IClock` (`SystemClock`)
when no clock is present, then `TryAdd`s the default masker, enricher, sink, dead-letter sink,
retention policy, provider status source, and recorder. Applications replace any of these before
or after the call — for example, register a durable `IAuditSink` and a `FailClosed` policy for
security events.

## Failure policy

`AuditOptions.FailurePolicy` controls sink failures:

- `FailOpen` (default) — log and drop the event (optionally routing it to `IAuditDeadLetterSink`);
  the business request is never failed because of an auditing problem.
- `FailClosed` — surface the failure so the caller can decide; selected security events may choose
  this to avoid silently losing security-relevant audits.

`PublishMode` chooses dispatch: `Synchronous` awaits the sink inline (deterministic for tests);
`BoundedAsync` enqueues onto a bounded channel (`BoundedCapacity`, default 1024, drop-on-full) and
publishes on a background reader so the request is never blocked.

The HTTP middleware and the EF interceptor are both fail-open: a throwing sink or a publishing
exception is logged and skipped, never blocking the response or the save.

## Sensitive-data handling

`IAuditMasker.Mask(key, value)` decides what to store. `DefaultAuditMasker` redacts values whose
key name matches a sensitive pattern (`password`, `token`, `secret`, `ssn`, `cvv`, `card`,
`privatekey`, `clientsecret`, `authorization`, `otp`, …; case-insensitive, separators ignored) and
returns `[REDACTED]`. Entity capture masks each changed scalar property by property name through
the same masker before publishing the `entity.change.{Property}` metadata. Replace `IAuditMasker`
to enforce application-specific rules; never rely on callers to pre-mask.

`AuditExceptionClassifier` maps exceptions to a safe `AuditExceptionClassification`
(`validation`, `not_found`, `conflict`, `dependency`, `timeout`, `server_error`, `unknown`) without
recording the message, stack, or inner details.

## Capture semantics

### HTTP

`AuditMiddleware` records an `http.request` event for every non-exempt path. On an unhandled
exception it records an `http.request` event with `AuditOutcome.Error` and `AuditSeverity.Error`
and still lets the host's exception handler produce the response. Configured security status codes
(401/403 by default) raise the event severity to a security/denied classification. The middleware
reads subject/tenant/correlation from configured headers via `IAuditSubjectResolver`
(`HeaderAuditSubjectResolver` default); replace the resolver to read claims or ambient context.

### EF Core

`AuditingSaveChangesInterceptor` captures `Added`, `Modified`, and `Deleted` entries for entities
implementing `IAuditedEntity`. Only scalar properties (strings and value types) are captured — not
navigation references — and each value is masked by property name. A `Modified` entry publishes
only changed properties as `old->new`; an `Added` entry publishes `set:`, and a `Deleted` entry
publishes `removed:`. Capture is opt-out via `AuditOptions.EnableEntityCapture`.

## Migration from a starter-kit auditing module

Convert the starter's audit primitives and core/persistence files into application-owned sink and
schema seams:

- Replace the starter's event type with `AuditEvent`; map its action/category/outcome/severity and
  metadata. Use `AuditEvent.Create` + `With*` helpers.
- Replace the starter's middleware with `UsePlatformAuditing`; map starter tenant/subject extraction
  into an `IAuditSubjectResolver` and starter header names into `AuditAspNetCoreOptions`.
- Replace the starter's save-changes hook with `AddPlatformAuditingEfCore` + the interceptor wired
  through `AddInterceptors`; mark the audited entities with `IAuditedEntity`.
- Keep the starter's database table, migration, retention job, and product projections in the
  application; the platform ships no persistence. The durable sink (an `IAuditSink`) is
  application-owned.

Run in shadow mode or on one category, compare emitted events, then remove the starter module.
Rollback removes `UsePlatformAuditing` and the `AddInterceptors` call; audit storage is unchanged.

## Ownership boundaries

Applications own: the audit store/schema, retention schedule and cleanup, product-specific event
projections, migrations, subject/tenant resolution beyond headers, and any durable buffer or queue.
The platform owns: the normalized `AuditEvent`, the default masker/enricher/sink/recorder, the
fail-open capture adapters, and the safe exception classifier.
