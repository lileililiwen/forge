# Notifications and SMS

## ADDED Requirements

### Requirement: Notification delivery SHALL be channel-neutral

Applications SHALL express notification intent and select email or SMS through normalized
contracts without depending on a concrete provider.

#### Scenario: Email provider changes

- **WHEN** the host replaces its email provider
- **THEN** notification intent and delivery results remain unchanged

### Requirement: Delivery failures SHALL be explicit

Providers SHALL classify invalid address/number, configuration, authentication, transient, and
permanent failures without reporting unconfigured delivery as successful.

#### Scenario: Production provider missing

- **WHEN** production sends a notification without a configured provider
- **THEN** the operation returns a configuration failure
- **AND** no success result is emitted

### Requirement: Repeated notification delivery SHALL be controllable

The orchestration layer SHALL support stable idempotency keys and job/event integration.

#### Scenario: Duplicate reminder event

- **WHEN** the same reminder event is delivered twice
- **THEN** the configured idempotency policy prevents duplicate delivery
