# Platform starter sample

This sample intentionally does not enable durable eventing by default. A real
application that owns an EF Core context can opt in separately:

```csharp
builder.Services.AddPlatformEventingEfCore<ApplicationDbContext>();
builder.Services.AddSingleton<IDurableEventPublisher, ApplicationEventPublisher>();
builder.Services.AddPlatformDurableOutboxDispatcher("sample-api-1");
```

The application must call `ConfigurePlatformEventing` from its own
`DbContext.OnModelCreating`, own the migrations for the outbox/inbox tables,
and implement the transport publisher. No product event types, transport
credentials, or migrations belong in this sample.

For cache adoption, register `Platform.Caching` with an application namespace. The in-memory
store is the default and is not authoritative:

```csharp
builder.Services.AddPlatformCaching("sample");
```

Choose `Platform.Caching.Hybrid` or `Platform.Caching.Redis` explicitly when the host owns the
provider, serializer, and operational policy. Include tenant and schema-version segments in keys.

Storage follows the same boundary: the sample does not select a bucket or filesystem root. A host
can register `Platform.Storage.Local` or `Platform.Storage.S3` explicitly after deciding its
authorization, retention, and metadata ownership.

Quota resources and limits remain application-owned; the sample does not enable quota enforcement,
define plan names, or create persistence models.
