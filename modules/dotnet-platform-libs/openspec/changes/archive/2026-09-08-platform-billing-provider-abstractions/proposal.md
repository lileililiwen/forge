# Proposal: Billing provider abstractions

## Why

The portfolio needs subscriptions, checkout, customer portals, webhook processing,
entitlements, usage limits, and provider-independent admin visibility. Stripe and Lemon
Squeezy have different models and must not leak into application business code.

## Scope

Extend the existing billing contracts into a provider-neutral billing orchestration package.
Normalize customers, plans, subscriptions, subscription items, provider events, checkout,
portal, cancellation, entitlement projection, usage, and provider status.

## Non-goals

- no Stripe or Lemon Squeezy SDK dependency;
- no provider price IDs, invoice rules, wallet, tax, or merchant-of-record policy;
- no automatic entitlement meaning beyond application-registered feature keys;
- no separately deployed billing service.

## API impact

Adds public billing provider, checkout, webhook, entitlement, usage, and orchestration
contracts while preserving existing `Platform.Billing.Contracts` identifiers and decisions.
