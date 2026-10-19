## ADDED Requirements

### Requirement: SMTP adapter

The SMTP adapter SHALL implement `IMailService`, construct messages from `MailMessage`, honor configured TLS/authentication settings, and return normalized results.

#### Scenario: SMTP accepted
- **WHEN** a valid message is accepted by the SMTP server
- **THEN** the adapter returns `Sent` with no secret-bearing diagnostic

### Requirement: SendGrid adapter

The SendGrid adapter SHALL implement `IMailService`, map recipients and attachments, and classify provider responses into normalized outcomes without exposing response bodies by default.

#### Scenario: SendGrid rate limit
- **WHEN** SendGrid returns a rate-limit response
- **THEN** the adapter returns a transient failure with a stable error code suitable for application retry

### Requirement: Input and configuration validation

Each adapter SHALL reject invalid sender, recipient, subject, body, attachment, and provider configuration before attempting delivery.

#### Scenario: Missing sender
- **WHEN** a message has no sender and no configured default sender
- **THEN** the adapter returns a configuration failure and does not call the provider

### Requirement: Cancellation preservation

Each adapter SHALL honor cancellation tokens and SHALL not convert caller cancellation into a permanent provider failure.

#### Scenario: Cancelled send
- **WHEN** the caller cancels during connection or provider I/O
- **THEN** the adapter stops the operation and preserves cancellation semantics
