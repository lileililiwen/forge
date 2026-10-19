## Context

The starter's `QuotaEnforcementMiddleware` runs after authentication/rate limiting, skips probes and unresolved tenants, calls a quota service, and emits RFC 9457 429 responses with retry metadata.

Starter-kit references:

- `dotnet-starter-kit/src/BuildingBlocks/Quota/QuotaEnforcementMiddleware.cs`
- `dotnet-starter-kit/src/BuildingBlocks/Quota/QuotaPlanResolver.cs`
- `dotnet-starter-kit/src/BuildingBlocks/Quota/Extensions.cs`
- `dotnet-starter-kit/src/Tests/Framework.Tests/Quota/`

Copy the response and pipeline test patterns only. Replace `QuotaResource`, Finbuckle, and plan-specific services with platform resource/subject contracts and application callbacks.

## Goals / Non-Goals

**Goals:**

- Provide middleware/filter registration that uses `IQuotaStore` reservations/checks.
- Make resource selection, subject/tenant extraction, exemption, failure policy, and response formatting explicit.
- Support fail-closed quota exhaustion and configurable provider-unavailable behavior.

**Non-Goals:**

- Defining plans, prices, invoices, wallets, billing entitlements, or resource units.
- Replacing `Platform.RateLimiting` or implementing a distributed quota backend.

## Decisions

1. Create `Platform.Quota.AspNetCore` with middleware plus endpoint metadata helpers.
2. Require application-provided `IQuotaSubjectAccessor`/resource resolver or use the platform identity accessor when available.
3. Use RFC 9457 problem details with safe trace/correlation/resource/limit fields and `Retry-After` when a reset exists.
4. Make health paths and explicit endpoint metadata exemptions configurable, not hard-coded to a product.

Alternative rejected: putting HTTP middleware in `Platform.Quota` would violate its framework-neutral boundary.

## Risks / Trade-offs

- [Risk] Quota is charged before a handler fails → expose check/reserve/settle integration and document operation semantics.
- [Risk] Incorrect subject mapping creates shared quotas → require explicit subject/tenant tests and fail closed when required context is missing.
- [Risk] Provider outage blocks traffic → make unavailable policy explicit and observable.

## Migration Plan

Run the adapter in shadow mode or on one resource, compare starter responses and usage, then replace middleware. Rollback removes middleware registration; quota storage remains unchanged.

## Open Questions

- Whether endpoint metadata should support both one-unit checks and full reservation lifecycles in the first adapter.
