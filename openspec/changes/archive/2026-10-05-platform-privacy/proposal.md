## Why

Every product must answer the same privacy questions — did this subject consent to this category, what data do we hold on them, delete it on request — and today each rebuilds the vocabulary from scratch. Consent categories, records, and re-consent rules are identical across products; only the data behind export/erasure differs. A shared package must therefore own the vocabulary and the enforcement mechanics while the data stays application-owned.

## What Changes

- Add `Platform.Privacy`: consent categories, records, versioned policy evaluation, an `IConsentStore` with an in-memory implementation, subject-rights provider contracts with a registry and result models, and anonymization helpers. No ASP.NET Core, no EF Core.
- Add `Platform.Privacy.AspNetCore`: consent-cookie service, request-scoped consent state, `RequireConsent` endpoint metadata with middleware enforcement, and DI wiring. No product routes, no banner markup, no plan or data knowledge.
- Tests in `Platform.Privacy.Tests` and `Platform.Privacy.AspNetCore.Tests`, plus an architecture guard keeping the core framework-neutral.

## Capabilities

### New Capabilities

- `platform-privacy`: consent vocabulary, policy evaluation, subject-rights contracts, anonymization.
- `platform-privacy-aspnetcore`: HTTP consent state and enforcement over `Platform.Privacy` contracts.

### Modified Capabilities

- None.

## Impact

Two new optional packages. Existing packages untouched. No public API change without this change; the baseline file updates on release.
