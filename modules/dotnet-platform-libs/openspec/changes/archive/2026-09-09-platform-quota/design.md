## Context

Current billing usage contracts are intentionally replaceable, while sibling applications implement monthly minutes, reservations, settlements, and releases inside their own domains. A reusable quota package should standardize the lifecycle and concurrency expectations without standardizing commercial plans.

## Goals / Non-Goals

**Goals:**

- Represent a quota subject, resource, window, amount, and stable operation key.
- Make reservation lifecycle idempotent and concurrency-safe at the store boundary.
- Provide useful decisions for API and worker callers.

**Non-Goals:**

- No plan catalog, price, invoice, wallet, subscription, or provider-specific rule.
- No mandatory database, Redis, billing package, or background worker.

## Decisions

- **Use resource/dimension keys.** Applications define resources such as `transcription_seconds` or `ai_tokens`; the platform does not define business names.
- **Model reservations explicitly.** `Check`, `Reserve`, `Settle`, and `Release` are separate operations with stable operation identifiers.
- **Require atomic store semantics.** The in-memory implementation is thread-safe; persistent implementations must make reserve decisions atomically for the same subject/resource/window.
- **Use snapshots for explanations.** Decisions return limit, consumed, reserved, requested, and remaining values without exposing plan internals.
- **Package layout.** Base package uses `Contracts`, `Stores`, `Evaluation`, and `DependencyInjection`; test helpers are a separate package.

## Risks / Trade-offs

- [Risk] Different products count units differently → [Mitigation] keep units opaque and make conversion application-owned.
- [Risk] Reservation leaks consume quota indefinitely → [Mitigation] support expiry/reconciliation and explicit release; durable cleanup remains application-owned.
- [Risk] Quota gets confused with billing entitlements → [Mitigation] integrate only through subject/resource/limit values, never product plan entities.

## Migration Plan

Pilot one usage-heavy sibling by wrapping its existing quota service. Run shadow checks before enforcing decisions, then migrate reservation calls. Rollback is removing the wrapper; existing ledger data remains application-owned.

## Open Questions

- Whether windows need calendar-period support in the first version or only explicit start/end timestamps.
