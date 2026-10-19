## Why

Multiple applications implement subscription status, feature access, quotas, and usage limits, but their persistence and payment workflows differ. A shared contract can remove repeated integration assumptions without forcing a shared billing database.

## What Changes

- Add normalized plan, subscription, entitlement, and usage contracts.
- Add feature-check results suitable for web or non-web consumers.
- Add provider-neutral webhook idempotency contracts.

## Capabilities

### New Capabilities

- `platform-entitlement-contracts`: Provider-neutral subscription and feature-entitlement contracts.

### Modified Capabilities

## Impact

Adds public billing contract APIs. Consuming applications retain their own plan catalog, persistence, payment provider, and domain rules.

## Context

The existing projects include Stripe-backed user subscriptions, tenant billing, and simple feature-limit systems. These cannot share one entity model safely.

## Goals / Non-Goals

**Goals:**

- Normalize only the information needed by feature-gating code.
- Support free, active, suspended, past-due, canceled, and unknown states.
- Make webhook processing idempotency explicit.

**Non-Goals:**

- Implement Stripe SDK calls.
- Define invoices, wallets, taxes, or payment collection.
- Define product-specific plan names or price IDs.

## Decisions

- Use opaque plan and feature identifiers.
- Return immutable entitlement snapshots.
- Make usage metering an interface; do not prescribe storage or counting windows.

## Risks / Trade-offs

- A too-generic contract may not capture complex billing; advanced workflows remain local.
- Subscription status mapping must be documented so adapters do not silently grant access.
