# Proposal: Adopt VisualFlow's idempotency building block into the platform

## Why

Every consumer in the portfolio that accepts payment provider callbacks, refund triggers, payout requests, or upload finalize endpoints must deduplicate replays. The pattern is identical across consumers: an `Idempotency-Key` header, a documented fingerprint, an `IdempotencyRecord` shape, an `IIdempotencyStore` contract, and a retention policy. VisualFlow has implemented exactly this in `src/BuildingBlocks/Idempotency/`, including the request fingerprint, the in-memory default store, the documented metrics surface, and the opt-in registration. The type surface is framework-neutral (no ASP.NET Core, no EF Core, no provider) and depends only on `Microsoft.Extensions.Options` and `Microsoft.Extensions.DependencyInjection.Abstractions`. Promoting it proves the second Phase 3 adoption hypothesis and gives the portfolio a shared, auditable idempotency contract.

## What Changes

- Add a new production package `Platform.Idempotency` that owns the idempotency record, fingerprint, store contract, in-memory default store, options, and the opt-in registration.
- The package depends on `Platform.Core` only; it does not depend on ASP.NET Core, EF Core, Redis, or a payment provider SDK.
- VisualFlow's `src/BuildingBlocks/Idempotency/` is removed in a follow-up VisualFlow change that adopts the new package; this change is the platform side only.

## Capabilities

### New Capabilities

- `platform-idempotency`: Documented idempotency record, request fingerprint, store contract, in-memory default store, and retention policy.

### Modified Capabilities

- (none)

## Impact

- Adds one new production NuGet package, `Platform.Idempotency`, versioned in `Directory.Packages.props`.
- Expands the architecture test to forbid `Platform.Idempotency` from referencing ASP.NET Core, EF Core, Redis, or a VisualFlow project.
- VisualFlow adopts the package in a separate change after this lands; no VisualFlow code is modified in this change.

## Context

The platform roadmap's Phase 3 adoption proof needs at least two real consumers worth of shared code. The VisualFlow idempotency block is the second cleanest candidate: it is small, framework-neutral, has a documented eviction policy, and the only `Microsoft.Extensions.*` references are `IOptions<>` and the DI abstractions.

## Goals / Non-Goals

**Goals:**

- Ship a self-contained, framework-neutral idempotency package.
- Preserve the documented `IdempotencyRecord` shape and the documented `RequestFingerprint` so consumers can swap in a Redis-backed store without changing the call sites.
- Provide a documented default retention window and `MaxKeyLength` so consumers do not reinvent them.

**Non-Goals:**

- Ship a Redis-backed or Postgres-backed store in this change; a follow-up change can adopt one if a second consumer needs it.
- Ship an ASP.NET Core endpoint filter; that is the consumer's responsibility.
- Provide automatic replay of the stored response; the consumer chooses how to honour a stored record.

## Decisions

- Reuse the existing `Platform.Core.IClock` for the documented `EvictExpiredAsync` sweep so the contract is deterministic and testable.
- Use the `IdempotencyOptions` shape verbatim from VisualFlow; the documented section name (`Idempotency`) is preserved.
- Keep the `IdempotencyRecord` and `RequestFingerprint` types as records to preserve value equality.

## Risks / Trade-offs

- The in-memory store uses `ConcurrentDictionary`; a multi-instance deployment needs a distributed store. This is documented as a non-goal and is the consumer's responsibility.
- `ResponseBody` is stored as `byte[]`; the size is bounded by the consumer's request body limits, not the platform. The architecture test will not enforce a body size cap.
