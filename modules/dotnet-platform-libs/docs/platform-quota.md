# Platform quota

`Platform.Quota` standardizes atomic capacity checks and reservation lifecycle without defining
commercial plans. Subjects, resources, units, limits, and windows are application-owned. The
database, billing system, or domain service remains authoritative for how those values are derived.

## Packages and layout

- `Platform.Quota` — `Contracts`, `Stores`, `Evaluation`, and `DependencyInjection`; includes the
  thread-safe `InMemoryQuotaStore`.
- `Platform.Quota.Testing` — deterministic `QuotaScenarioBuilder` and inspection-oriented helpers.
- `Platform.Quota.AspNetCore` — optional HTTP enforcement adapter (see below).

The package does not own plans, prices, invoices, wallets, ledgers, persistence schemas, Redis, or
background cleanup.

## ASP.NET Core enforcement

`Platform.Quota.AspNetCore` enforces application-selected quota resources through middleware without
defining plans or resource limits. Register it with `AddPlatformQuotaAspNetCore` (after
`AddPlatformQuota` or another `IQuotaStore` registration) and insert `UsePlatformQuota` after
authentication and rate limiting.

The application owns two seams:

- `IQuotaSubjectResolver` resolves the opaque `QuotaSubject` (and any tenant context) per request. The
  default `HeaderQuotaSubjectResolver` reads a configured header and reports missing context so the
  missing-context policy applies.
- `IQuotaResourceResolver` selects the `QuotaRequest` operations a request consumes (resource, limit,
  amount, window, and whether to reserve). An unconfigured host fails closed with a clear response
  until the application registers this resolver.

`QuotaEnforcementOptions` controls behavior: `Enabled`, `MissingContextPolicy` (fail-closed by default
or allow with a configured anonymous subject), `ProviderUnavailablePolicy` (fail-closed by default or
allow), `ExemptPathPrefixes`, `ExemptMethods`, the HTTP status codes, and the RFC 9457 type/title.
Endpoints may also opt out with `[QuotaExempt]`.

A denied request returns HTTP 429 RFC 9457 problem details with safe `resource`, `limit`, `usage`,
`requested`, `traceId`, `correlationId`, and `resetAtUtc` extensions, plus `Retry-After` when the
window reset is known. Limits, plans, units, entitlements, and persistence remain application-owned.

### Migration from a starter-kit middleware

Replace the starter `QuotaEnforcementMiddleware` with `UsePlatformQuota`. Map the starter's tenant
extraction into an `IQuotaSubjectResolver`, its resource/plan selection into an
`IQuotaResourceResolver`, and its plan limits into each `QuotaRequest.Limit`. Keep Finbuckle, plan
services, and Redis wiring in the application; the adapter only enforces `IQuotaStore` outcomes. Run
in shadow mode or on one resource, compare responses, then remove the starter middleware. Rollback
removes `UsePlatformQuota`; quota storage is unchanged.

## Lifecycle and concurrency

Use a stable operation key for `Reserve`, `Settle`, and `Release`. Repeated lifecycle calls are
idempotent. Accepted reservations consume reserved capacity until settlement, release, or expiry;
the in-memory implementation performs the capacity decision and reservation mutation under one
lock. Persistent adapters must provide equivalent atomicity for the same subject/resource/window.

Applications should reconcile expired or leaked reservations and decide whether settlement uses the
reserved amount or an application-converted amount. Unit conversion, cleanup, and durable storage
remain application responsibilities.

## Entitlements and migration

`IQuotaLimitResolver` is an optional seam for resolving an opaque limit from an application-owned
entitlement. It does not import billing plans or invoice entities. A migration can shadow-check the
platform store beside an existing quota service, then switch reserve calls after decisions match.
Rollback removes the wrapper and leaves existing application ledger data unchanged.
