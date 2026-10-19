# Platform application starter

`Platform.Starter` is a composition package, not a replacement application architecture.
`AddPlatformApplication` enables web by default and leaves identity, admin, billing, AI,
notifications, and SMS disabled until the host opts in.

```csharp
builder.Services.AddPlatformApplication(options =>
{
    options.EnableIdentity = true;
    options.EnableAdmin = true;
    options.EnableBilling = true;
    options.BillingProviderName = "stripe";
});

var app = builder.Build();
app.UsePlatformApplication();
app.MapPlatformApplicationEndpoints();
```

Registration uses `TryAdd` through the composed packages. Register application stores and
providers before or after the call as appropriate; consumer registrations must remain the
last registration for replaceable contracts. Admin requires identity. In production, every
enabled external capability requires an explicit provider name. The starter never creates
provider SDK clients, fake production success, entities, migrations, billing plan mappings,
or product permissions.

The sample host at `samples/Platform.Starter.Sample` is the conformance fixture. The scaffold
under `templates/platform-application-starter` is an equivalent `dotnet new` template layout;
copy it into a local template pack or use it as the starting point for a repository. The
`client` choice is intentionally a small Razor/React seam: the platform shares contracts and
states, not a forced UI implementation.

For incremental adoption, add only `Platform.Starter`, enable web, call `UsePlatformApplication`
after exception handling and before application endpoints, then adopt identity/admin or other
capabilities one at a time. Pin all Platform packages to the same minor version. Rollback is
the reverse: disable the capability, remove its endpoint mapping, restore the prior consumer
registration, and redeploy; no platform migration is required.

Durable eventing is intentionally not enabled by `Platform.Starter`. Applications that own an
EF Core context can opt into `Platform.Eventing.Contracts` and `Platform.Eventing.EfCore`, register
their own `IDurableEventPublisher`, and then add the hosted dispatcher. This keeps transport,
migrations, event schemas, and replay policy application-owned.

Caching is also opt-in. Use `Platform.Caching` for local defaults, or register the separate
`Platform.Caching.Hybrid` or `Platform.Caching.Redis` adapter when the host explicitly chooses a
provider. The starter does not choose Redis, serializers, cache authority, or tenant key policy.

Storage is likewise opt-in. Applications may register `Platform.Storage.Local` for development or
`Platform.Storage.S3` with an application-owned AWS/S3-compatible client. The starter does not own
bucket names, filesystem roots, authorization, product file metadata, retention, or migrations.

Quota is also opt-in. Applications define resource units and limits, then register `Platform.Quota`
or an application-owned persistent implementation. The platform does not infer billing plans or
settle invoices.
