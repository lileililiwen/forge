# .NET Platform Libraries

Small, privately maintained .NET libraries for the shared technical concerns
repeated across the local application portfolio. The platform is a
**library set**, not a runtime service: every package is small, opt-in, and
independently adoptable.

- **.NET baseline:** SDK `10.0.400`, `net10.0`.
- **Single version source:** `VersionPrefix` in `Directory.Build.props`.
- **License:** MIT (see [`LICENSE`](LICENSE)).
- **Package inventory:** [`eng/package-manifest.json`](eng/package-manifest.json)
  is the machine-readable source of truth; every row in the matrix below is
  checked against it.

> Need a one-line answer? Jump to the [Packages matrix](#packages), the
> [Versioning and consumer guide](#versioning-and-consumer-guide), or
> [Adoption](#adoption).

## Why adopt

- **Stable, narrow contracts** (`Platform.Core`, `Platform.Billing.Contracts`,
  `Platform.Identity.Contracts`, `Platform.Tenant.Lifecycle.Contracts`,
  `Platform.Ai.Contracts`, …) — zero ASP.NET Core / EF Core / Stripe / provider
  dependencies at the boundary.
- **Provider-neutral adapters** for the things every product needs
  (mailing, jobs, storage, caching, eventing, billing, AI, identity) that
  never assume a specific vendor.
- **Application-owned storage, credentials, and migrations** — the platform
  ships only the boundary and an in-memory default; products pick the
  durable adapter.
- **Architecture guardrails** (`tests/Platform.Architecture.Tests`) keep the
  dependency direction honest as the surface grows.

## Quickstart

```bash
dotnet restore Platform.sln
dotnet build  Platform.sln -c Release
dotnet test   Platform.sln -c Release --nologo
dotnet pack   Platform.sln -c Release --no-build --nologo
```

The solution restores, builds, tests, and packs under .NET 10. Packages are
written to each project's `bin/Release/` directory. See
[`docs/build-test-pack.md`](docs/build-test-pack.md) for the full set of
commands and [`docs/packages.md`](docs/packages.md) for the per-package
contract reference.

## Packages

Every production package under `src/` is listed below exactly once. The
matrix is the authoritative front door; the linked `docs/packages.md`
section is the surface reference (every public type, every registration
method, every option).

- **Kind** — `Production` (always-on, framework-neutral or ASP.NET Core
  adapter), `Optional adapter` (provider or framework-specific; consume one
  at a time), or `Test/support` (never referenced by production projects;
  enforced by `tests/Platform.Architecture.Tests`).
- **Depends on** — the platform packages it references (the
  `Microsoft.AspNetCore.App` framework reference is noted when present).
  Third-party package references live in the per-package surface.
- **Anchor** — the per-package section in
  [`docs/packages.md`](docs/packages.md).

### Core / domain / adoption

| Package | Kind | Depends on | Anchor | One-liner |
| --- | --- | --- | --- | --- |
| `Platform.Core` | Production | (none) | [§ Platform.Core](docs/packages.md#platformcore) | Framework-independent contracts: `IClock`, `Error`, `Result`, `Result<T>`, `CallerContext`, `IAuditable`. |
| `Platform.Domain` | Production | `Platform.Core` | [§ Platform.Domain](docs/packages.md#platformdomain) | `IEntity<TId>`, `IAggregateRoot<TId>`, `IDomainEvent`, `Entity<TId>`, `AggregateRoot<TId>`, `Money`, `ISoftDeletable`, `IHasTenant`, safe `DomainException` errors. |
| `Platform.Authorization` | Production | (none) | [§ Platform.Authorization](docs/packages.md#platformauthorization) | `PermissionDefinition`, `PermissionCatalog`, `PlatformPolicyNames`, `AuthorizationDecision`, and the decision auditor. |
| `Platform.Adoption` | Production | (none) | [§ Platform.Adoption](docs/packages.md#platformadoption) | `AdoptionAnalyzer`, `AdoptionReport`, `AdoptionStatus`, `AdoptionEvidence`, and the read-only `AdoptionEvidenceClassifier` (never modifies the target). |

### Identity

| Package | Kind | Depends on | Anchor | One-liner |
| --- | --- | --- | --- | --- |
| `Platform.Identity.Contracts` | Production | (none) | [§ Platform.Identity (contracts)](docs/packages.md#platformidentity-contracts) | `CurrentUser`, `ICurrentUserAccessor`, `ICredentialVerifier`, `IExternalIdentityProvider`, `ISessionStore`, `IIdentityAuditHook`, and the lifecycle contracts (refresh, recovery, two-factor, impersonation). |
| `Platform.Identity.AspNetCore` | Production | `Platform.Identity.Contracts`, `Platform.Authorization` + `Microsoft.AspNetCore.App` | [§ Platform.Identity.AspNetCore](docs/packages.md#platformidentityaspnetcore) | `AddPlatformIdentity`, `AddPlatformIdentityAuthentication`, claim projection, JWT options, permission policy, and the lifecycle minimal-API endpoints. |
| `Platform.Identity.EntityFrameworkCore` | Optional adapter | `Platform.Identity.Contracts`, `Platform.Persistence.EfCore` | [§ Platform.Identity.EntityFrameworkCore](docs/packages.md#platformidentityentityframeworkcore) | `IIdentityStore` and the `IdentityDbContextAdapter` base; the application owns the schema, the migration, and the user entity. |
| `Platform.Identity.Testing` | Test/support | `Platform.Identity.Contracts`, `Platform.Authorization` | [§ Platform.Identity.Testing](docs/packages.md#platformidentitytesting) | `FakeCredentialVerifier`, `InMemoryRefreshTokenStore`, `FakePasswordRecoveryService`, `FakeTwoFactorService`, `FakeImpersonationService`, `RecordingIdentityAuditHook`. |

### Administration

| Package | Kind | Depends on | Anchor | One-liner |
| --- | --- | --- | --- | --- |
| `Platform.Admin.Contracts` | Production | (none) | [§ Platform.Admin.Contracts](docs/packages.md#platformadmincontracts) | Provider-neutral admin records and extension points; no framework references. |
| `Platform.Admin.AspNetCore` | Production | `Platform.Admin.Contracts`, `Platform.Identity.Contracts`, `Platform.Identity.AspNetCore` + `Microsoft.AspNetCore.App` | [§ Platform.Admin.AspNetCore](docs/packages.md#platformadminaspnetcore) | Opt-in ASP.NET Core admin endpoints with the application-owned `IAdminStore` seam. |
| `Platform.Admin.Testing` | Test/support | `Platform.Admin.Contracts` | [§ Platform.Admin.Testing](docs/packages.md#platformadmintesting) | Deterministic `InMemoryAdminStore` and admin test doubles. |

### Billing

| Package | Kind | Depends on | Anchor | One-liner |
| --- | --- | --- | --- | --- |
| `Platform.Billing.Contracts` | Production | (none) | [§ Platform.Billing.Contracts](docs/packages.md#platformbillingcontracts) | Provider-neutral subscription, entitlement, feature-check, usage-meter, and processed-event contracts. |
| `Platform.Billing` | Production | `Platform.Billing.Contracts`, `Platform.Core` | [§ Platform.Billing](docs/packages.md#platformbilling) | Application-owned billing seam: `IBillingProvider`, checkout/portal mapping, webhooks, and status; consumers pick the provider. |
| `Platform.Billing.Stripe` | Optional adapter | `Platform.Billing` | [§ Platform.Billing provider adapters](docs/packages.md#platformbilling-provider-adapters) | Raw-HTTP Stripe checkout, portal, subscription lookup, webhook verification/normalization, and status. |
| `Platform.Billing.LemonSqueezy` | Optional adapter | `Platform.Billing` | [§ Platform.Billing provider adapters](docs/packages.md#platformbilling-provider-adapters) | Raw-HTTP Lemon Squeezy checkout, subscription lookup, webhook verification/normalization, and status. |
| `Platform.Billing.Testing` | Test/support | `Platform.Billing.Contracts`, `Platform.Core` | [§ Platform.Billing.Testing](docs/packages.md#platformbillingtesting) | `SubscriptionBuilder`, `EntitlementBuilder`, `FakeEntitlementStore`, and billing test doubles. |

### Jobs (scheduling)

| Package | Kind | Depends on | Anchor | One-liner |
| --- | --- | --- | --- | --- |
| `Platform.Jobs` | Production | `Platform.Core` | [§ Platform.Jobs](docs/packages.md#platformjobs) | Engine-neutral `IJobDispatcher`, `IRecurringJobHandler`, `IRecurringJobRegistry`, `IJobTelemetry`, and `AddPlatformJobs` registration. |
| `Platform.Jobs.Hangfire` | Optional adapter | `Platform.Jobs` + `Microsoft.AspNetCore.App` | [§ Platform.Jobs.Hangfire](docs/packages.md#platformjobshangfire) | Hangfire dispatcher, recurring registry, executor, context capture/restore, dashboard factory, and storage health check. |

### Mailing

| Package | Kind | Depends on | Anchor | One-liner |
| --- | --- | --- | --- | --- |
| `Platform.Mailing` | Production | `Platform.Core` | [§ Platform.Mailing](docs/packages.md#platformmailing) | Provider-neutral `IMailService`, `IMailTemplateRenderer<TModel>`, `MailMessage`, `MailSendResult`, and `AddPlatformMailing` registration. |
| `Platform.Mailing.Smtp` | Optional adapter | `Platform.Mailing` | [§ Platform.Mailing.Smtp](docs/packages.md#platformmailingsmtp) | MailKit-based SMTP adapter: `SmtpSecureMode`, `SmtpMailOptions`, `SmtpMailService`, and `AddPlatformSmtpMail` registration. |
| `Platform.Mailing.SendGrid` | Optional adapter | `Platform.Mailing` | [§ Platform.Mailing.SendGrid](docs/packages.md#platformmailingsendgrid) | SendGrid client adapter: `SendGridMailOptions`, `SendGridMailService`, and `AddPlatformSendGridMail` registration. |

### Eventing (messaging)

| Package | Kind | Depends on | Anchor | One-liner |
| --- | --- | --- | --- | --- |
| `Platform.Eventing` | Production | `Platform.Core` | [§ Platform.Eventing](docs/packages.md#platformeventing) | Transport-agnostic `IEventBus`, `IIntegrationEventHandler<TEvent>`, the `IntegrationEventEnvelope` serializer/deserializer, and the in-process `InProcessEventBus`. |
| `Platform.Eventing.Contracts` | Production | `Platform.Core` | [§ Platform.Eventing.Contracts and Platform.Eventing.EfCore](docs/packages.md#platformeventingcontracts-and-platformeventingefcore) | Framework-neutral durable outbox/inbox records, lease/retry states, store interfaces, and `IDurableEventPublisher`. |
| `Platform.Eventing.EfCore` | Optional adapter | `Platform.Eventing.Contracts`, `Platform.Core` | [§ Platform.Eventing.Contracts and Platform.Eventing.EfCore](docs/packages.md#platformeventingcontracts-and-platformeventingefcore) | Application-owned EF Core outbox/inbox mappings, stores, and the hosted outbox dispatcher. |
| `Platform.Eventing.RabbitMq` | Optional adapter | `Platform.Eventing.Contracts` | [§ Platform.Eventing.RabbitMq](docs/packages.md#platformeventingrabbitmq) | RabbitMQ durable publisher with confirmed publishes, application-owned topology, and safe failure classification. |

### Caching

| Package | Kind | Depends on | Anchor | One-liner |
| --- | --- | --- | --- | --- |
| `Platform.Caching` | Production | `Platform.Core` | [§ Platform.Caching](docs/packages.md#platformcaching) | Provider-neutral `ICacheStore`, `CacheKey`, `CacheEntryOptions`, `ICacheProviderStatus`, and the thread-safe `InMemoryCacheStore`. |
| `Platform.Caching.Hybrid` | Optional adapter | `Platform.Caching` | [§ Platform.Caching](docs/packages.md#platformcaching) | Microsoft HybridCache adapter: `HybridCacheStore` for local or single-host deployments. |
| `Platform.Caching.Redis` | Optional adapter | `Platform.Caching` | [§ Platform.Caching](docs/packages.md#platformcaching) | StackExchange.Redis adapter with application-owned serialization, bounded operations, tags, and provider health. |

### Storage (object/blob)

| Package | Kind | Depends on | Anchor | One-liner |
| --- | --- | --- | --- | --- |
| `Platform.Storage` | Production | `Platform.Core` | [§ Platform.Storage](docs/packages.md#platformstorage) | Provider-neutral `IObjectStorage`, `StorageObjectKey`, `StorageUploadRequest`, presigned operations, and `IStorageProviderStatus`. |
| `Platform.Storage.Local` | Optional adapter | `Platform.Storage` | [§ Platform.Storage](docs/packages.md#platformstorage) | Atomic local filesystem `LocalFileStorage` for development and single-host deployments. |
| `Platform.Storage.S3` | Optional adapter | `Platform.Storage` | [§ Platform.Storage](docs/packages.md#platformstorage) | AWS/S3-compatible `S3Storage` with an application-owned client, presigning, timeouts, and failure classification. |

### Quota

| Package | Kind | Depends on | Anchor | One-liner |
| --- | --- | --- | --- | --- |
| `Platform.Quota` | Production | `Platform.Core` | [§ Platform.Quota](docs/packages.md#platformquota) | Provider-neutral `IQuotaStore`, `QuotaDecision`, `QuotaReservation`, and the thread-safe `InMemoryQuotaStore`. |
| `Platform.Quota.AspNetCore` | Optional adapter | `Platform.Core`, `Platform.Quota` + `Microsoft.AspNetCore.App` | [§ Platform.Quota](docs/packages.md#platformquota) | ASP.NET Core enforcement middleware, subject/resource resolvers, exemptions, and RFC 9457 `429` responses. |
| `Platform.Quota.Testing` | Test/support | `Platform.Quota` | [§ Platform.Quota](docs/packages.md#platformquota) | `QuotaScenarioBuilder` for deterministic test setup. |

### Cross-cutting request guards

| Package | Kind | Depends on | Anchor | One-liner |
| --- | --- | --- | --- | --- |
| `Platform.Idempotency` | Production | `Platform.Core` | [§ Platform.Idempotency](docs/packages.md#platformidempotency) | `IIdempotencyStore`, `RequestFingerprint` (SHA-256 over method/route/body-hash), `IdempotencyOptions`, and the in-memory default. |
| `Platform.RateLimiting` | Production | `Platform.Core` | [§ Platform.RateLimiting](docs/packages.md#platformratelimiting) | `IRateLimiter`, `RateLimitPolicies` catalog, `IRateLimitBypassResolver`, `IRateLimiterBackendStatusProvider`, and the in-memory default. |

### Web / composition / observability

| Package | Kind | Depends on | Anchor | One-liner |
| --- | --- | --- | --- | --- |
| `Platform.AspNetCore` | Production | `Platform.Core` + `Microsoft.AspNetCore.App` | [§ Platform.AspNetCore](docs/packages.md#platformaspnetcore) | `AddPlatformAspNetCore`, sanitized `ProblemDetails` mapping, correlation middleware/accessor, and health-check helpers. |
| `Platform.Web` | Production | `Platform.AspNetCore` + `Microsoft.AspNetCore.App` | [§ Platform.Web](docs/packages.md#platformweb) | Composable web runtime foundations built on `Platform.AspNetCore`: `PlatformWebOptions`, runtime contracts, and middleware. |
| `Platform.Web.Composition` | Production | `Platform.Core` + `Microsoft.AspNetCore.App` | [§ Platform.Web.Composition](docs/packages.md#platformwebcomposition) | Explicit `IPlatformWebModule` composition: `AddPlatformWebModule`, `UsePlatformWebModules`, `MapPlatformWebModules`. No assembly scanning, no Mediator/FluentValidation coupling. |
| `Platform.Web.Telemetry` | Production | `Platform.Core` | [§ Platform.Web.Telemetry](docs/packages.md#platformwebtelemetry) | Framework-neutral redaction-safe web telemetry names, option-validation helpers, and the structured log sink. |
| `Platform.Web.Cors` | Optional adapter | `Platform.Core`, `Platform.Web.Telemetry` + `Microsoft.AspNetCore.App` | [§ Platform.Web.Cors](docs/packages.md#platformwebcors) | CORS configuration with production-time validation (wildcards + credentials rejected). |
| `Platform.Web.OpenApi` | Optional adapter | `Platform.Core`, `Platform.Web.Telemetry` + `Microsoft.AspNetCore.App` | [§ Platform.Web.OpenApi](docs/packages.md#platformwebopenapi) | OpenAPI document registry and explicit `IPlatformOpenApiDocumentProvider` mapping. |
| `Platform.Web.Resilience` | Optional adapter | `Platform.Core`, `Platform.Web.Telemetry` + `Microsoft.AspNetCore.App` + `Microsoft.Extensions.Http` | [§ Platform.Web.Resilience](docs/packages.md#platformwebresilience) | `HttpClient` retry, timeout, and circuit-breaker handler with bounded defaults. |
| `Platform.Web.Versioning` | Optional adapter | `Platform.Core` + `Microsoft.AspNetCore.App` | [§ Platform.Web.Versioning](docs/packages.md#platformwebversioning) | ASP.NET Core API versioning (`Asp.Versioning.Http`/`Asp.Versioning.Mvc.ApiExplorer`) and the API Explorer group mapper. |
| `Platform.FeatureManagement` | Optional adapter | `Platform.Core`, `Platform.Web.Telemetry` + `Microsoft.AspNetCore.App` | [§ Platform.FeatureManagement](docs/packages.md#platformfeaturemanagement) | ASP.NET Core feature-flag integration: tenant filter, `RequireFeature` endpoint filter, validated options. |
| `Platform.Http.Resilience` | Optional adapter | `Platform.Core`, `Platform.Web.Telemetry` + `Microsoft.Extensions.Http` | [§ Platform.Http.Resilience](docs/packages.md#platformhttpresilience) | `Microsoft.Extensions.Http.Resilience`-based pipeline: retry, total/per-attempt timeout, circuit breaker, and concurrency limiter. |
| `Platform.Observability` | Optional adapter | `Platform.Core`, `Platform.Web.Telemetry` + `Microsoft.AspNetCore.App` | [§ Platform.Observability](docs/packages.md#platformobservability) | Opt-in host observability: safe correlation, logging enrichment, tracing, metrics, and provider-status contracts (no exporter). |

### Realtime

| Package | Kind | Depends on | Anchor | One-liner |
| --- | --- | --- | --- | --- |
| `Platform.Realtime` | Production | `Platform.Core` | [§ Platform.Realtime (contracts)](docs/packages.md#platformrealtime-contracts) | Transport-neutral connection, authorization, tenant-routing, payload, and provider-status contracts. |
| `Platform.Realtime.AspNetCore` | Optional adapter | `Platform.Core`, `Platform.Realtime` + `Microsoft.AspNetCore.App` | [§ Platform.Realtime.AspNetCore (adapters)](docs/packages.md#platformrealtimeaspnetcore-adapters) | SSE and SignalR adapters, the connection limiter, and the status/resync endpoints. |

### Persistence / multitenancy

| Package | Kind | Depends on | Anchor | One-liner |
| --- | --- | --- | --- | --- |
| `Platform.Persistence.EfCore` | Optional adapter | `Platform.Core` | [§ Platform.Persistence.EfCore](docs/packages.md#platformpersistenceefcore) | Provider-neutral EF Core conventions: audit, soft-delete, tenant filter, paging/specification helpers, and the read-only migration readiness check. |
| `Platform.Persistence.EfCore.Migrator` | Optional adapter | `Platform.Persistence.EfCore` | [§ Platform.Persistence.EfCore.Migrator](docs/packages.md#platformpersistenceefcoremigrator) | Application-owned `IMigrationRunner` and `MigrationConsoleRunner` (no migrations, no connection strings, no provider selection). |
| `Platform.Persistence.Multitenancy` | Optional adapter | `Platform.Core`, `Platform.AspNetCore`, `Platform.Persistence.EfCore` + `Microsoft.AspNetCore.App` | [§ Platform.Persistence.Multitenancy](docs/packages.md#platformpersistencemultitenancy) | `ITenantResolver`, ambient scope, scoped connection routing, per-entity default filters, and the readiness check. |
| `Platform.Persistence.Postgres` | Optional adapter | `Platform.Persistence.EfCore` | [§ Platform.Persistence.Postgres](docs/packages.md#platformpersistencepostgres) | Npgsql `UsePlatformPostgres` options; no contexts, migrations, or domain behavior. |

### Tenant lifecycle

| Package | Kind | Depends on | Anchor | One-liner |
| --- | --- | --- | --- | --- |
| `Platform.Tenant.Lifecycle.Contracts` | Production | (none) | [§ Platform.Tenant.Lifecycle (contracts)](docs/packages.md#platformtenantlifecycle-contracts) | Provider-neutral `ITenantLifecycleStep`, `ITenantLifecycleWorkflow`, `ITenantLifecycleScopeCallback`, `ITenantLifecycleStore`, `ITenantLifecycleOrchestrator`. |
| `Platform.Tenant.Lifecycle` | Production | `Platform.Tenant.Lifecycle.Contracts`, `Platform.Core` | [§ Platform.Tenant.Lifecycle](docs/packages.md#platformtenantlifecycle) | Default `TenantLifecycleOrchestrator`, in-memory store, workflow registry, and `AddPlatformTenantLifecycle` registration. |
| `Platform.Tenant.Lifecycle.AspNetCore` | Production | `Platform.Tenant.Lifecycle`, `Platform.Tenant.Lifecycle.Contracts` + `Microsoft.AspNetCore.App` | [§ Platform.Tenant.Lifecycle.AspNetCore](docs/packages.md#platformtenantlifecycleaspnetcore) | Status/resume endpoints, `TenantLifecycleReadinessCheck`, and the provider-neutral readiness surface. |
| `Platform.Tenant.Lifecycle.Testing` | Test/support | `Platform.Tenant.Lifecycle.Contracts` | [§ Platform.Tenant.Lifecycle.Testing](docs/packages.md#platformtenantlifecycletesting) | `InMemoryTenantLifecycleStore`, `ScriptedLifecycleStep`, `DelegateLifecycleStep`, `StaticLifecycleWorkflow`, `RecordingLifecycleScopeCallback`. |

### AI

| Package | Kind | Depends on | Anchor | One-liner |
| --- | --- | --- | --- | --- |
| `Platform.Ai.Contracts` | Production | (none) | [§ Platform.Ai](docs/packages.md#platformai) | Provider-neutral generation, streaming, structured-output, embedding, usage, cost, capability, policy, and failure contracts. |
| `Platform.Ai` | Production | `Platform.Ai.Contracts`, `Platform.Core` | [§ Platform.Ai](docs/packages.md#platformai) | Policy-gated generation and single/feature-based routing with safe telemetry. |
| `Platform.Ai.Anthropic` | Optional adapter | `Platform.Ai.Contracts` | [§ Platform.Ai](docs/packages.md#platformai) | Raw-HTTP Anthropic adapter. |
| `Platform.Ai.Ollama` | Optional adapter | `Platform.Ai.Contracts` | [§ Platform.Ai](docs/packages.md#platformai) | Raw-HTTP Ollama adapter. |
| `Platform.Ai.OpenAiCompatible` | Optional adapter | `Platform.Ai.Contracts` | [§ Platform.Ai](docs/packages.md#platformai) | Raw-HTTP OpenAI-compatible adapter (supports the DeepSeek endpoint selection). |
| `Platform.Ai.Testing` | Test/support | `Platform.Ai.Contracts` | [§ Platform.Ai](docs/packages.md#platformai) | Deterministic AI fakes and usage recording. |

### Notifications

| Package | Kind | Depends on | Anchor | One-liner |
| --- | --- | --- | --- | --- |
| `Platform.Notifications` | Production | `Platform.Core`, `Platform.Idempotency`, `Platform.Jobs`, `Platform.Mailing` | [§ Platform.Notifications](docs/packages.md#platformnotifications) | Channel-neutral email/SMS intents, delivery outcomes, bounded retries, idempotency integration, and job scheduling helpers. |
| `Platform.Notifications.Testing` | Test/support | `Platform.Notifications` | [§ Platform.Notifications](docs/packages.md#platformnotifications) | Deterministic in-memory notification provider. |

### Auditing

| Package | Kind | Depends on | Anchor | One-liner |
| --- | --- | --- | --- | --- |
| `Platform.Auditing.Contracts` | Production | `Platform.Core` | [§ Platform.Auditing.Contracts](docs/packages.md#platformauditingcontracts) | `AuditEvent`, `IAuditRecorder`, `IAuditSink`, `IAuditMasker`, `IAuditEnricher`, retention, and dead-letter contracts. |
| `Platform.Auditing.AspNetCore` | Optional adapter | `Platform.Core`, `Platform.Auditing.Contracts` + `Microsoft.AspNetCore.App` | [§ Platform.Auditing.AspNetCore](docs/packages.md#platformauditingaspnetcore) | Request, exception, and security capture middleware with safe classification. |
| `Platform.Auditing.EfCore` | Optional adapter | `Platform.Core`, `Platform.Auditing.Contracts` | [§ Platform.Auditing.EfCore](docs/packages.md#platformauditingefcore) | `AuditingSaveChangesInterceptor` capturing `IAuditedEntity` changes (fail-open). |

### Webhooks

| Package | Kind | Depends on | Anchor | One-liner |
| --- | --- | --- | --- | --- |
| `Platform.Webhooks.Contracts` | Production | `Platform.Core` | [§ Platform.Webhooks](docs/packages.md#platformwebhooks) | Provider-neutral inbound verification, replay suppression, outbound subscriptions, and delivery attempts. |
| `Platform.Webhooks.AspNetCore` | Optional adapter | `Platform.Core`, `Platform.Webhooks.Contracts` + `Microsoft.AspNetCore.App` | [§ Platform.Webhooks](docs/packages.md#platformwebhooks) | `HttpRequest` reader and a default `HttpClient`-backed sender. |
| `Platform.Webhooks.EfCore` | Optional adapter | `Platform.Core`, `Platform.Webhooks.Contracts` | [§ Platform.Webhooks](docs/packages.md#platformwebhooks) | Inbox and delivery entity configurations for the application's `DbContext`. |

### UI (static assets) and composition

| Package | Kind | Depends on | Anchor | One-liner |
| --- | --- | --- | --- | --- |
| `Platform.UI.Razor` | Production | (none) | [§ Platform UI](docs/packages.md#platform-ui) | Token-backed Razor static web assets and conventions. |

The npm token layer this package's assets follow lives under [`ui/`](ui/README.md):
`@platform/design-tokens`, `@platform/react-ui` and `@platform/react-shell`, all at `0.1.0`.
It is resolvable from another repository by committing the packed bytes in `ui/feed/*.tgz`
into that repository's own tree and declaring a `file:` dependency specifier — no registry,
no sibling checkout, no absolute path and no environment variable. `ui/tokens/tokens.json`
is the single source of every token value, the generated CSS ships the canonical
`.visually-hidden` primitive, and `node ui/scripts/verify-ui.mjs` runs in
`scripts/quality-gate.sh`.
| `Platform.Starter` | Production | `Platform.Web`, `Platform.Identity.AspNetCore`, `Platform.Admin.AspNetCore`, `Platform.Billing`, `Platform.Mailing` | [§ Platform.Starter](docs/packages.md#platformstarter) | Opt-in application bootstrap that composes web, identity, admin, billing, and mailing (no application persistence or provider implementation). |

### Testing

| Package | Kind | Depends on | Anchor | One-liner |
| --- | --- | --- | --- | --- |
| `Platform.Testing` | Test/support | `Platform.Core`, `Platform.AspNetCore`, `Platform.Billing.Contracts`, `Platform.Eventing` | [§ Platform.Testing](docs/packages.md#platformtesting) | `ControllableClock`, `SubscriptionBuilder`, `EntitlementBuilder`, `FakeEntitlementStore`, `RecordingUsageMeter`, `RecordingEventBus`, `TransientFailureInjector`. Production projects must not reference this. |
| `Platform.Testing.AspNetCore` | Test/support | `Platform.Core` + `Microsoft.AspNetCore.App` | [§ Platform.Testing.AspNetCore](docs/packages.md#platformtestingaspnetcore) | `PlatformTestWebApplicationFactory<TEntryPoint>` boots the consumer's real application over `TestServer` (pulls `Microsoft.AspNetCore.Mvc.Testing` transitively); one host per instance. Production projects must not reference this. |

Product-specific EF Core entities, migrations, Stripe price IDs, invoice
rules, plan names, and business workflows remain in consuming applications.

## Versioning and consumer guide

The repository is a single-versioned library set. The single version source
is `VersionPrefix` in [`Directory.Build.props`](Directory.Build.props); every
production package under `src/` shares that version unless a project file
overrides `Version`. The manifest in
[`eng/package-manifest.json`](eng/package-manifest.json) records the
resolved version per package and is regenerated by
`./scripts/generate-package-manifest.sh` (with `--check` for CI).

- **Semantic Versioning.** Additive compatible changes are minor, fixes are
  patch, and reviewed breaking changes are major. Breaking changes require
  migration notes and a reviewed API-baseline update in the same change.
  See [`docs/release-governance.md`](docs/release-governance.md).
- **Baseline.** SDK `10.0.400` with `rollForward: latestPatch` and
  `allowPrerelease: false` (`global.json`). Every C# project targets
  `net10.0`. SDK 8 and the older 8.x target are no longer
  selectable. See
  [`docs/dotnet10-migration-contract.md`](docs/dotnet10-migration-contract.md).
- **Source vs package mode.** A consumer that lives under the workspace
  imports [`build/Platform.Consumer.props`](build/Platform.Consumer.props)
  and sets `PlatformConsumerBootstrap=true`. The
  `PlatformAsSource` property is auto-detected: a local platform checkout
  injects a `ProjectReference`, otherwise the bootstrap injects a
  `PackageReference` at `$(PlatformPackageVersion)` (default `0.1.0`).
  `PlatformConsumerOptOut=true` disables every default, reference, and
  diagnostic. See
  [`docs/workspace-consumer-bootstrap.md`](docs/workspace-consumer-bootstrap.md).
- **Central package management.** Production projects declare
  `<PackageReference Include="..." />` without a `Version` attribute; the
  version lives in
  [`Directory.Packages.props`](Directory.Packages.props). The same pattern
  applies to the consumer: with `ManagePackageVersionsCentrally=true`, the
  bootstrap injects the unversioned `Platform.Core` reference and the
  consumer's `Directory.Packages.props` adds
  `<PackageVersion Include="Platform.Core" Version="$(PlatformPackageVersion)" />`.
- **Upgrade and rollback.** Both source and package modes are exercised by
  `./scripts/conformance.sh`; `./scripts/consumer-upgrade-rollback.sh`
  verifies promotion and rollback against a previous and a candidate feed.
  Pin to an exact version (`Version="0.1.0"`); floating versions fail
  adoption. Application stores, migrations, credentials, and UI are
  untouched by both upgrade and rollback.
- **Public API baseline.** `./scripts/check-public-api.sh` diffs the public
  surface of `Platform.Core`, `Platform.Billing.Contracts`,
  `Platform.Identity.Contracts`, `Platform.Admin.Contracts`,
  `Platform.Eventing.Contracts` and `Platform.Testing.AspNetCore` against
  `eng/public-api-baseline.txt`; a baseline update is part
  of a reviewed release change (run with `--update`).

## Usage examples

Each example is the smallest snippet that registers the surface, with a link
to the sample, template, or `docs/packages.md` section that compiles. All
samples target `net10.0` and the platform's `VersionPrefix` (`0.1.0`).

### Core: `Platform.Core` ([sample](samples/Platform.MinimalWeb.Sample/Program.cs))

```csharp
using Microsoft.Extensions.DependencyInjection;
using Platform.Core.Time;

var services = new ServiceCollection();
services.AddSingleton<IClock>(_ => new SystemClock());
```

`Platform.Core` is the only package that ships `IClock`, `Result`,
`Error`, `CallerContext`, and `IAuditable`; `Platform.Domain` reuses them
for entities, aggregates, and domain events. See
[§ Platform.Core](docs/packages.md#platformcore).

### Web composition: `Platform.Web.Composition` ([sample](samples/Platform.MinimalWeb.Sample/Program.cs))

```csharp
using Platform.Starter.DependencyInjection;
using Platform.Web.Composition;

var builder = WebApplication.CreateBuilder(args);
builder.Services.AddPlatformWebModule<MyAppModule>();
var app = builder.Build();
app.UsePlatformWebModules();
app.MapPlatformWebModules();
app.Run();
```

Modules implement `IPlatformWebModule`; registration is explicit (no
assembly scanning, no Mediator/FluentValidation coupling). See
[§ Platform.Web.Composition](docs/packages.md#platformwebcomposition).

### Identity: `Platform.Identity.AspNetCore` ([sample](samples/Platform.Identity.Sample/Program.cs))

```csharp
using Platform.Identity.AspNetCore;
using Platform.Identity.Contracts;

var builder = WebApplication.CreateBuilder(args);
builder.Services.AddPlatformIdentity();
builder.Services.AddSingleton<ICredentialVerifier, MyAppVerifier>();
var app = builder.Build();
app.MapPlatformRefreshTokenRotation();
app.Run();
```

`Platform.Identity.Contracts` defines the contract; `Platform.Identity.AspNetCore`
adds the host integration and the lifecycle endpoints. The credential
verifier, the user store, and the session issuance remain
application-owned. See
[§ Platform.Identity.AspNetCore](docs/packages.md#platformidentityaspnetcore)
and [§ Platform.Identity (contracts)](docs/packages.md#platformidentity-contracts).

### Authorization: `Platform.Authorization` ([surface](docs/packages.md#platformauthorization))

```csharp
using Platform.Authorization;

catalog.Require("sample.read");
var decision = new AuthorizationDecision("sample.read", Allowed: true);
```

The catalog is module-owned; applications register permissions and policies.
See [§ Platform.Authorization](docs/packages.md#platformauthorization).

### Jobs: `Platform.Jobs` + `Platform.Jobs.Hangfire` ([surface](docs/packages.md#platformjobs))

```csharp
using Microsoft.Extensions.DependencyInjection;
using Platform.Jobs.Hangfire;
using Platform.Jobs.Hangfire.DependencyInjection;

var services = new ServiceCollection();
services.AddPlatformHangfireJobs(opts =>
{
    opts.Storage = HangfireStorageKind.InMemory;
    opts.DashboardEnabled = true;
    opts.DashboardAuthorization = ctx => true; // application policy
});
```

Recurring jobs declare `RecurringJobAttribute` on a handler; the
dispatcher and registry are platform-owned, storage is application-owned.
See [§ Platform.Jobs.Hangfire](docs/packages.md#platformjobshangfire).

### Mailing: `Platform.Mailing` + `Platform.Mailing.Smtp` ([surface](docs/packages.md#platformmailing))

```csharp
using Microsoft.Extensions.DependencyInjection;
using Platform.Mailing;
using Platform.Mailing.Smtp;

var services = new ServiceCollection();
services.AddPlatformMailing();
services.AddPlatformSmtpMail(opts =>
{
    opts.Host = "smtp.example.com";
    opts.Port = 587;
});
```

`AddPlatformSmtpMail` `TryAdd`s `IMailService`; an application-owned
registration always wins. SendGrid is the alternative
([§ Platform.Mailing.SendGrid](docs/packages.md#platformmailingsendgrid)).

### Eventing: `Platform.Eventing` ([sample](samples/Platform.MinimalWeb.Sample/Program.cs))

```csharp
using Microsoft.Extensions.DependencyInjection;
using Platform.Eventing;

var services = new ServiceCollection();
services.AddPlatformEventingInProcess();
```

Durable outbox/inbox live in `Platform.Eventing.Contracts` with the EF
Core adapter (`Platform.Eventing.EfCore`) and the RabbitMQ adapter
(`Platform.Eventing.RabbitMq`). See
[§ Platform.Eventing.Contracts and Platform.Eventing.EfCore](docs/packages.md#platformeventingcontracts-and-platformeventingefcore).

### Caching: `Platform.Caching` ([surface](docs/packages.md#platformcaching))

```csharp
using Microsoft.Extensions.DependencyInjection;
using Platform.Caching;
using Platform.Caching.Redis;
using StackExchange.Redis;

var services = new ServiceCollection();
services.AddPlatformCaching(application: "myapp");
var multiplexer = await ConnectionMultiplexer.ConnectAsync(connectionString);
services.AddPlatformCachingRedis(multiplexer, serializer: new MyAppCacheSerializer());
```

The provider-neutral `ICacheStore` ships with `InMemoryCacheStore`; the
`Hybrid` and `Redis` adapters are opt-in. `AddPlatformCaching` requires
the application key prefix that `CacheKeyBuilder` uses; the Redis
adapter takes a shared `IConnectionMultiplexer` and an
`ICacheValueSerializer` (the application owns the concrete
implementation; the package ships only the interface).

### Storage: `Platform.Storage` + `Platform.Storage.Local` ([sample](samples/Platform.ProviderStorage.Sample/Program.cs))

```csharp
using Microsoft.Extensions.DependencyInjection;
using Platform.Storage.Local;
using Platform.Storage.Contracts;

var services = new ServiceCollection();
services.AddSingleton<IObjectStorage>(new LocalFileStorage(root));
```

The application owns the root path, the bucket, the credentials, and the
tenant prefix. The S3 adapter is the production alternative
([§ Platform.Storage](docs/packages.md#platformstorage)).

### Quota: `Platform.Quota` + `Platform.Quota.AspNetCore` ([surface](docs/packages.md#platformquota))

```csharp
using Microsoft.Extensions.DependencyInjection;
using Platform.Quota;
using Platform.Quota.AspNetCore;

var services = new ServiceCollection();
services.AddPlatformQuota();
services.AddPlatformQuotaAspNetCore(opts =>
{
    opts.QuotaExceededStatusCode = StatusCodes.Status429TooManyRequests;
});
```

Quota is provider-neutral; the application owns the plan, the limit
resolution, and the durable store. `AddPlatformQuotaAspNetCore` configures
the enforcement middleware (`UsePlatformQuota`); the application must
register an `IQuotaResourceResolver` for the routes it wants to enforce.

### Identity lifecycle: `Platform.Identity.Contracts` + `Platform.Identity.AspNetCore` ([surface](docs/packages.md#platformidentity-contracts))

```csharp
using Platform.Identity.Contracts;
using Platform.Identity.AspNetCore.Endpoints;

var app = builder.Build();
app.MapPlatformRefreshTokenRotation();
app.MapPlatformPasswordRecoveryInitiation();
app.MapPlatformTwoFactorChallenge();
app.MapPlatformImpersonationStart();
```

The application owns the refresh-token store (`IRefreshTokenStore`),
the password-recovery service, the two-factor service, and the
impersonation policy; the platform never owns application users, roles,
or credentials. See
[§ Platform.Identity.AspNetCore](docs/packages.md#platformidentityaspnetcore).

### Tenant lifecycle: `Platform.Tenant.Lifecycle` + `Platform.Tenant.Lifecycle.AspNetCore` ([sample](samples/Platform.Tenancy.Sample/Program.cs))

```csharp
using Microsoft.Extensions.DependencyInjection;
using Platform.Tenant.Lifecycle;
using Platform.Tenant.Lifecycle.Contracts;

var services = new ServiceCollection();
services.AddSingleton<ITenantLifecycleStore, MyAppStore>();
services.AddPlatformTenantLifecycle();
await using var provider = services.BuildServiceProvider();
provider.GetRequiredService<TenantLifecycleWorkflowRegistry>().Register(workflow);
var status = await provider.GetRequiredService<ITenantLifecycleOrchestrator>()
    .StartAsync(workflow, tenantId: "tenant-1", metadata: new Dictionary<string, string>());
```

The orchestrator runs ordered steps, installs the tenant scope around
each `IsTenantScoped` step, and routes outcomes to the operation state.
The store, the workflow, the steps, and the scope callback are
application-owned. See
[§ Platform.Tenant.Lifecycle](docs/packages.md#platformtenantlifecycle).

### AI: `Platform.Ai.Contracts` + `Platform.Ai` ([surface](docs/packages.md#platformai))

```csharp
using Microsoft.Extensions.DependencyInjection;
using Platform.Ai;
using Platform.Ai.Contracts;
using Platform.Ai.Anthropic;

var services = new ServiceCollection();
services.AddHttpClient<AnthropicProvider>();
services.AddSingleton<IAiTextProvider>(sp => sp.GetRequiredService<AnthropicProvider>());
services.AddSingleton<IAiProviderRouter>(sp => new SingleAiProviderRouter(
    sp.GetRequiredService<IAiTextProvider>()));
services.AddSingleton<IAiFeaturePolicy, AllowAllAiFeaturePolicy>();
services.AddSingleton<AiClient>();
```

The provider selection (`Anthropic`/`Ollama`/`OpenAiCompatible`) and the
model parameters are application-owned; the platform owns the policy
gating, the structured output, the usage record, and the safe failure
shape. `Platform.Ai` does not ship a `AddPlatformAi` extension; the
application wires `IAiTextProvider`, `IAiProviderRouter`, and
`IAiFeaturePolicy` directly (the adapter package, e.g.
`Platform.Ai.Anthropic`, contributes the `IAiTextProvider`
implementation). See [`docs/platform-ai.md`](docs/platform-ai.md).

### Notifications: `Platform.Notifications` ([surface](docs/platform-notifications.md))

```csharp
using Microsoft.Extensions.DependencyInjection;
using Platform.Notifications;

var services = new ServiceCollection();
services.AddPlatformNotifications();
```

Channel selection (email/SMS) and template content remain
application-owned. See
[§ Platform.Notifications](docs/packages.md#platformnotifications).

### Persistence: `Platform.Persistence.EfCore` ([sample](samples/Platform.EfCore.Sample/Program.cs))

```csharp
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.DependencyInjection;
using Platform.Persistence.EfCore.DependencyInjection;

var services = new ServiceCollection();
services.AddPlatformPersistenceEfCore();
services.AddDbContext<MyAppDbContext>(options => options.UseSqlite(connectionString));
```

The application owns the context, the entity types, the migrations, and
the provider selection. The platform contributes the audit/soft-delete
interceptor, the tenant filter, and the paging/specification helpers.

### Testing: `Platform.Testing` + `Platform.Testing.AspNetCore` ([surface](docs/packages.md#platformtesting))

```csharp
using Microsoft.AspNetCore.Mvc.Testing;
using Platform.Testing;
using Platform.Testing.AspNetCore;
using Xunit;

// Program is the application's own entry point; a public marker type works too.
// The factory boots the real application, so every route below is the app's.
public class SampleTests : IClassFixture<PlatformTestWebApplicationFactory<Program>>
{
    private readonly PlatformTestWebApplicationFactory<Program> _factory;
    public SampleTests(PlatformTestWebApplicationFactory<Program> factory) => _factory = factory;

    [Fact]
    public async Task Liveness_returns_ok()
    {
        var client = _factory.CreateClient();
        var response = await client.GetAsync("/live");
        Assert.Equal(System.Net.HttpStatusCode.OK, response.StatusCode);
    }
}
```

`ControllableClock`, `SubscriptionBuilder`, `EntitlementBuilder`,
`FakeEntitlementStore`, `RecordingUsageMeter`, `RecordingEventBus`, and
`TransientFailureInjector` ship in `Platform.Testing`;
`PlatformTestWebApplicationFactory<TEntryPoint>` — which boots the consumer's real
application over `TestServer` — ships in `Platform.Testing.AspNetCore`. Production projects must not reference
either (enforced by `tests/Platform.Architecture.Tests`).

### Starter: `Platform.Starter` ([sample](samples/Platform.Starter.Sample/Program.cs), [template](templates/platform-application-starter/Program.cs))

```csharp
using Microsoft.AspNetCore.Builder;
using Microsoft.Extensions.DependencyInjection;
using Platform.Starter.DependencyInjection;

var builder = WebApplication.CreateBuilder(args);
builder.Services.AddPlatformApplication(options =>
{
    options.EnableIdentity = true;
    options.EnableAdmin = false;
    options.EnableNotifications = true;
    options.NotificationProviderName = "fake";
});
var app = builder.Build();
app.UsePlatformApplication();
app.MapPlatformApplicationEndpoints();
app.Run();
```

`Platform.Starter` composes the opt-in web, identity, admin, billing,
and mailing boundaries. The application owns the user store, the
identity store, the provider credentials, the migrations, and the UI.
See [`docs/platform-starter.md`](docs/platform-starter.md).

## Adoption

The single documented entry point for adopting the platform is the
workspace consumer bootstrap
([`docs/workspace-consumer-bootstrap.md`](docs/workspace-consumer-bootstrap.md)),
backed by [`build/Platform.Consumer.props`](build/Platform.Consumer.props).
For a staged, product-level plan, see
[`docs/platform-product-adoption.md`](docs/platform-product-adoption.md):
every product starts with `PlatformConsumerBootstrap=true`, runs the
read-only adoption diagnostics, pilots one package with an exact version
pin, verifies it with native evidence, and only then expands.

The diagnostics are surfaced by `tools/Platform.Adoption.Tool` (the
`platform-doctor` CLI):

```bash
dotnet run --project tools/Platform.Adoption.Tool -- doctor     --project-dir /absolute/path/to/product
dotnet run --project tools/Platform.Adoption.Tool -- inventory  --project-dir /absolute/path/to/product
dotnet run --project tools/Platform.Adoption.Tool -- conformance --project-dir /absolute/path/to/product
dotnet run --project tools/Platform.Adoption.Tool -- preview    --project-dir /absolute/path/to/product
```

`doctor` reports per-package evidence levels
(`Absent`/`Configured`/`Incompatible`/`Unverified`/`Verified`); a
package without native verification evidence stays `Unverified` at
most, it is never called production-ready. Exit codes:
`0` clean, `1` source failures, `2` environment-blocked only, `64`
usage error. The tool never modifies the target directory.

The verifiable end-to-end sample matrix is
[`samples/matrix.json`](samples/matrix.json); every entry maps a sample
to the packages it adopts, the verification command, and the rollback
plan. The sample matrix is asserted by
`tests/Platform.SampleMatrix.Tests`.

## License

The platform is released under the **MIT License**. The full text lives
in [`LICENSE`](LICENSE) at the repository root and matches the
`PackageLicenseExpression=MIT` metadata declared in
[`Directory.Build.props`](Directory.Build.props). See
[`docs/release-governance.md`](docs/release-governance.md) for the
release workflow and the public API baseline.

## Screenshots

`NOT_APPLICABLE`. The repository ships no user-facing UI; its consumer
surface is package APIs, a read-only CLI, and documentation. Visual
capture would be noise. Recorded against the `readiness` capability
spec; the no-UI justification is in
[`openspec/changes/post-mvp-readiness/specs/readiness/spec.md`](openspec/changes/post-mvp-readiness/specs/readiness/spec.md).

## Repository layout

```
src/
  Platform.Core/                  Framework-independent contracts
  Platform.Domain/                Framework-neutral domain primitives
  Platform.Authorization/         Permission catalog and decision contracts
  Platform.Adoption/              Provider-neutral adoption diagnostics core
  Platform.AspNetCore/            ASP.NET Core integration
  Platform.Web/                   Composable web runtime foundations
  Platform.Web.Composition/       Explicit ASP.NET Core module composition
  Platform.Web.Telemetry/         Framework-neutral web telemetry names + safe sink
  Platform.Web.{Cors,OpenApi,Resilience,Versioning}/
                                 Optional web edge packages
  Platform.FeatureManagement/     Optional feature-flag integration
  Platform.Http.Resilience/       Optional Microsoft.Extensions.Http resilience pipeline
  Platform.Observability/         Optional host observability (no exporter)
  Platform.Realtime/              Transport-neutral realtime contracts
  Platform.Realtime.AspNetCore/   SSE + SignalR adapters
  Platform.Persistence.EfCore/    Provider-neutral EF Core conventions
  Platform.Persistence.EfCore.Migrator/ Application-owned EF Core migration runner
  Platform.Persistence.Multitenancy/ Tenant scope, resolver, connection routing
  Platform.Persistence.Postgres/  Npgsql options configuration
  Platform.Identity.Contracts/    Provider-neutral identity + lifecycle contracts
  Platform.Identity.AspNetCore/   ASP.NET Core identity host integration
  Platform.Identity.EntityFrameworkCore/ Schema-independent EF Core identity store
  Platform.Identity.Testing/      Deterministic identity test providers
  Platform.Admin.Contracts/       Provider-neutral admin contracts
  Platform.Admin.AspNetCore/      Opt-in ASP.NET Core admin endpoints
  Platform.Admin.Testing/         Deterministic admin test doubles
  Platform.Billing.Contracts/     Subscription and entitlement contracts
  Platform.Billing/               Application-owned billing seam
  Platform.Billing.Stripe/        Raw-HTTP Stripe adapter
  Platform.Billing.LemonSqueezy/  Raw-HTTP Lemon Squeezy adapter
  Platform.Billing.Testing/       Deterministic billing test doubles
  Platform.Jobs/                  Engine-neutral scheduling contracts
  Platform.Jobs.Hangfire/         Hangfire adapter
  Platform.Mailing/               Provider-neutral mailing contracts
  Platform.Mailing.Smtp/          MailKit SMTP adapter
  Platform.Mailing.SendGrid/      SendGrid client adapter
  Platform.Eventing/              Transport-agnostic event-bus contract + in-process bus
  Platform.Eventing.Contracts/    Durable outbox/inbox contracts
  Platform.Eventing.EfCore/       Application-owned EF Core outbox/inbox mappings
  Platform.Eventing.RabbitMq/     RabbitMQ durable publisher
  Platform.Caching/               Provider-neutral cache contracts
  Platform.Caching.Hybrid/        Microsoft HybridCache adapter
  Platform.Caching.Redis/         StackExchange.Redis adapter
  Platform.Storage/               Provider-neutral object storage contracts
  Platform.Storage.Local/         Atomic local filesystem adapter
  Platform.Storage.S3/            AWS/S3-compatible adapter
  Platform.Quota/                 Provider-neutral quota contracts
  Platform.Quota.AspNetCore/      ASP.NET Core enforcement middleware
  Platform.Quota.Testing/         Quota test scenario builder
  Platform.Idempotency/           Idempotency contract + in-memory store
  Platform.RateLimiting/          Rate-limit contract + in-memory default
  Platform.Ai.Contracts/          Provider-neutral AI contracts
  Platform.Ai/                    Policy-gated AI generation and routing
  Platform.Ai.Anthropic/          Raw-HTTP Anthropic adapter
  Platform.Ai.Ollama/             Raw-HTTP Ollama adapter
  Platform.Ai.OpenAiCompatible/   Raw-HTTP OpenAI-compatible adapter (DeepSeek supported)
  Platform.Ai.Testing/            Deterministic AI fakes
  Platform.Notifications/         Channel-neutral email/SMS intents
  Platform.Notifications.Testing/ Deterministic in-memory notification provider
  Platform.Auditing.Contracts/    Provider-neutral audit event contracts
  Platform.Auditing.AspNetCore/   ASP.NET Core capture middleware
  Platform.Auditing.EfCore/       SaveChanges audit interceptor
  Platform.Webhooks.Contracts/    Provider-neutral webhook contracts
  Platform.Webhooks.AspNetCore/   HttpRequest reader + HttpClient sender
  Platform.Webhooks.EfCore/       Inbox/delivery EF Core entity configurations
  Platform.Tenant.Lifecycle.Contracts/ Provider-neutral tenant lifecycle contracts
  Platform.Tenant.Lifecycle/      Default orchestrator + in-memory store
  Platform.Tenant.Lifecycle.AspNetCore/ Status/resume endpoints + readiness
  Platform.Tenant.Lifecycle.Testing/    Deterministic lifecycle test doubles
  Platform.UI.Razor/              Token-backed Razor static web assets
  Platform.Starter/               Opt-in application bootstrap
  Platform.Testing/               Test-only helpers
  Platform.Testing.AspNetCore/    In-memory TestServer host builder
tests/
  Platform.<Package>.Tests/       Per-package unit + integration tests
  Platform.Architecture.Tests/    Dependency-direction and isolation guardrails
  Platform.SampleMatrix.Tests/    Per-stage sample wiring + matrix metadata
  Platform.Template.Tests/        Template pack/install/generate/build/test smoke
  Platform.ConsumerConformance/   Solution-excluded consumer-conformance fixture
samples/
  matrix.json                     Machine-readable sample matrix index
  Platform.MinimalWeb.Sample/     Stage 1: web runtime only (Platform.Starter)
  Platform.EfCore.Sample/         Stage 2: application-owned context + migration
  Platform.Identity.Sample/       Stage 3: application-owned credential store
  Platform.Tenancy.Sample/        Stage 4: application-owned workflow/steps/store
  Platform.ProviderStorage.Sample/ Stage 5: local storage adapter, no credentials
  Platform.Starter.Sample/        Platform.Starter end-to-end sample
templates/
  platform-application-starter/   Tracked template content tree (platform-app template)
  Platform.Application.Template/  Pack-only template project (PackageType=Template)
tools/
  Platform.Adoption.Tool/         Read-only adoption diagnostics CLI (doctor/inventory/conformance/preview)
docs/
  build-test-pack.md              Restore, build, test, pack, and validate commands
  packages.md                     Per-package contract reference
  release-governance.md           Versioning, baseline, API baseline, signing
  dotnet10-migration-contract.md  .NET 10 consumer migration order and evidence
  workspace-consumer-bootstrap.md PlatformConsumerBootstrap / PlatformAsSource / version pin
  platform-product-adoption.md    Staged product adoption plan
  platform-*.md                   Per-capability adoption and ownership guidance
openspec/
  changes/                        Active and archived OpenSpec proposals
  changes/archive/                Archived proposals
  specs/                          Generated capability specs
eng/
  package-manifest.json           Machine-readable platform package manifest
  public-api-baseline.txt         Public API surface baseline
scripts/
  quality-gate.sh                 Restore, build, test, and pack
  conformance.sh                  Pack + restore + run consumer conformance
  consumer-upgrade-rollback.sh    Upgrade/rollback smoke test
  generate-package-manifest.sh    Manifest generator (with --check)
  audit-packages.sh               NuGet vulnerability audit
  check-public-api.sh             Public API surface diff
  package-inventory.sh            List of source projects
  test-workspace-bootstrap.sh     Workspace consumer bootstrap smoke test
build/
  Platform.Consumer.props         Platform consumer bootstrap (imported by consumers)
Directory.Build.props             Shared MSBuild defaults and packaging metadata
Directory.Packages.props          Central package version management
HANDOFF.md                        Most recent change completion and next action
ROADMAP.md                        Phased delivery plan and current status
```

## Conventions

- Target frameworks: production libraries target `net10.0`; SDK `10.0.400`
  is the required baseline.
- Package versions are managed centrally in
  [`Directory.Packages.props`](Directory.Packages.props). Project files
  declare `<PackageReference Include="..." />` without a `Version`
  attribute.
- Nullable reference types, implicit usings, deterministic builds, and
  warnings-as-errors are enabled in
  [`Directory.Build.props`](Directory.Build.props) for production code.
- Test projects opt out of packaging via
  `tests/Directory.Build.props`.
- `Platform.Core`, `Platform.Billing.Contracts`,
  `Platform.Identity.Contracts`, and
  `Platform.Tenant.Lifecycle.Contracts` must not reference ASP.NET Core,
  EF Core, Stripe SDKs, or application projects. `Platform.AspNetCore`
  may reference only `Platform.Core`. `Platform.Testing` and the
  `*.Testing` packages are test-only; production projects must not
  reference them. `tests/Platform.Architecture.Tests` enforces these and
  the related dependency-direction rules.

## Delivery workflow

1. Select one active change with `openspec list`.
2. Implement only that change and its tests.
3. Update its `tasks.md` as tasks complete.
4. Verify with relevant tests and strict OpenSpec validation.
5. Archive the completed change.
6. Commit 1: implementation, tests, archive, and related generated specs
   only.
7. Update `HANDOFF.md` with completion evidence and the next change.
8. Commit 2: only the `HANDOFF.md` update.
9. Stop; do not start another change or push.

Incomplete or blocked work must not be reported as complete. The handoff
must record the exact failed command and next action.

## Current status

The active OpenSpec change at this writing is `post-mvp-readiness`; it
documents the front door (this README, the `LICENSE` file, and the
cross-links) without changing any public API, package, or version.
Before that, `platform-site-user-auth-starter` and
`platform-consumer-bootstrap` were implemented and archived. The
repository contains 75 source projects and 52 in-solution test projects
(plus the solution-excluded `Platform.ConsumerConformance` fixture),
with 1,391 passing tests in 48 in-solution suites and 350 guard tests
in `Platform.Architecture.Tests`. The supported baseline is SDK
`10.0.400` and `net10.0`. `Platform.Testing`,
`Platform.Testing.AspNetCore`, and the other testing-support packages
are not referenced by production projects; the architecture test
enforces the dependency direction. See
[`docs/packages.md`](docs/packages.md), `HANDOFF.md`, and `ROADMAP.md`
for package, completion, and planning details.
