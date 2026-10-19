# Billing provider adapters

## ADDED Requirements

### Requirement: Provider adapters SHALL verify webhook authenticity

Each adapter SHALL verify the provider-specific signature over the raw request body before
normalizing or projecting an event.

#### Scenario: Invalid webhook signature

- **WHEN** a webhook signature is invalid or absent
- **THEN** the adapter rejects the event
- **AND** no subscription or entitlement mutation occurs

### Requirement: Provider adapters SHALL normalize lifecycle events

Stripe and Lemon Squeezy events SHALL map into the same normalized subscription and provider
event contracts while preserving provider metadata for diagnostics.

#### Scenario: Subscription cancellation event

- **WHEN** either provider sends a cancellation event
- **THEN** the normalized event contains the subject, subscription, effective state, and event ID

### Requirement: Provider failures SHALL be classified safely

Transient, permanent, configuration, authentication, and malformed-response failures SHALL be
distinguishable without logging credentials or payment data.

#### Scenario: Provider timeout

- **WHEN** a provider request times out
- **THEN** the result is classified as transient
- **AND** logs contain correlation and provider operation metadata but no secret values
