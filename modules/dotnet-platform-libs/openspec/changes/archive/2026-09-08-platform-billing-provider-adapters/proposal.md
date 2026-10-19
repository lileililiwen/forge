# Proposal: Stripe and Lemon Squeezy billing adapters

## Why

Applications need real provider integrations after the provider-neutral billing contracts
exist. Stripe and Lemon Squeezy differ in webhook shape, checkout, portal, subscription, and
merchant-of-record behavior.

## Scope

Add optional `Platform.Billing.Stripe` and `Platform.Billing.LemonSqueezy` adapters. Normalize
provider events, validate signatures, classify transient/permanent errors, expose provider
health, and integrate with billing idempotency.

## Non-goals

- no application plan catalog or price IDs;
- no tax, invoice, wallet, refund, or accounting policy;
- no provider SDK dependency in contracts or base packages;
- no production credentials in tests or repository files.

## API impact

Adds provider adapter packages and options. Existing provider-neutral contracts remain the
stable consumer surface.
