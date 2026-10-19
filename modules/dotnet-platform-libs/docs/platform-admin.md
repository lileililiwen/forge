# Platform administration

`Platform.Admin.Contracts` contains provider-neutral projections and host-owned extension
points. Applications implement `IAdminStore`, `IAdminTenantScope`, and optionally
`IAdminAuditSink` and `IAdminImpersonationService`; the package does not own users, roles,
tenants, subscriptions, credentials, or persistence.

`Platform.Admin.AspNetCore` is opt-in:

```csharp
builder.Services.AddPlatformAdmin(options => options.MaximumPageSize = 100);
// Register IAdminStore in the host.
app.MapPlatformAdminEndpoints();
```

The mapped routes are under `/admin` by default. Every route uses an explicit platform
permission such as `admin.users.read` or `admin.sessions.manage`. Query endpoints reject
page sizes above `MaximumPageSize`, searches longer than 200 characters, and unknown sort
keys. Tenant scope is applied from `ICurrentUserAccessor`; applications that need cross-tenant
operations must replace `IAdminTenantScope` explicitly.

Mutation endpoints return `204` on success and a stable error object on failure. Successful
user state changes, session revocations, and enabled impersonation start/end operations are
sent to `IAdminAuditSink`; audit projections contain correlation, reason, and expiry metadata
but no secrets. Impersonation routes are not mapped unless `EnableImpersonation` is enabled,
and applications must provide `IAdminImpersonationService`.

`Platform.Admin.Testing` provides `InMemoryAdminStore` and `RecordingAdminAuditSink` for
deterministic tests. It is not referenced by production packages.
