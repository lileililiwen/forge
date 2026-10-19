# platform-mailing Specification

## Purpose

Three Rust consumers already implement the same email-delivery seam on
lettre 0.11; `platform-mailing` carries the shared, provider-neutral
send contract (message types, transport trait, error taxonomy), one
opt-in SMTP adapter, and one deterministic mock, so consumers stop
duplicating lettre plumbing without the platform owning retries,
queues, templates, or provider credentials.

## Requirements

### Requirement: A provider-neutral mail send contract SHALL exist (MAIL-001)

The `platform-mailing` crate MUST expose `EmailAddress`, `EmailMessage`,
`MailTransport`, `SendContext`, `DeliveryReceipt`, and `MailingError`, with
message validation performed before any transport call and every error
variant mapped to an existing `platform-core` `ErrorCategory` without
leaking credentials, message bodies, or recipient local-parts.

#### Scenario: Valid message accepted by injected transport

- **WHEN** a consumer builds a valid `EmailMessage` and sends it through any
  `MailTransport` implementation that accepts
- **THEN** the call returns a `DeliveryReceipt` whose timestamp comes from
  the `SendContext` clock

#### Scenario: Invalid message never reaches the transport

- **WHEN** the message has a malformed address, an empty `to` list, more
  than 50 recipients, CR/LF in the subject, or a declared body over 5 MiB
- **THEN** send returns `MailingError::Validation` and the transport is not
  invoked

#### Scenario: Error text is safe

- **WHEN** any `MailingError` is rendered with `Display` after a failure
  containing credentials or recipient addresses
- **THEN** the output omits secrets, message bodies, and recipient
  local-parts

### Requirement: The SMTP adapter SHALL be an opt-in feature (MAIL-002)

The lettre-based `LettreSmtpTransport` and `SmtpTransportConfig` MUST exist
only under the `smtp-lettre` cargo feature, MUST use rustls (no native-tls),
and MUST map every lettre SMTP transport error kind to a `MailingError`
variant.

#### Scenario: Feature enabled

- **WHEN** the crate is built with `--features smtp-lettre`
- **THEN** the SMTP types compile and satisfy the `MailTransport` contract

#### Scenario: Feature disabled

- **WHEN** the crate is built with default features
- **THEN** neither `lettre` nor `tokio` appears in its dependency graph and
  the SMTP symbols do not exist

#### Scenario: Insecure transport refused off-loopback

- **WHEN** `SmtpTransportConfig` sets `Security::None` for a non-loopback
  host
- **THEN** construction fails with `MailingError::Configuration`

### Requirement: A deterministic mock transport SHALL support consumer tests (MAIL-003)

The `mock` cargo feature MUST provide a scriptable `MailTransport` that
returns configured success, failure, and timeout results without network
access, living inside `platform-mailing` so production crates may use it
without referencing `platform-testing`.

#### Scenario: Scripted outcomes

- **WHEN** a test scripts the mock to accept then to fail with `Timeout`
- **THEN** the two sends return a receipt and a `MailingError::Timeout`
  respectively, using only the injected clock

#### Scenario: Mock without feature

- **WHEN** the crate is built without the `mock` feature
- **THEN** the mock types are unavailable

### Requirement: Satellite dependencies SHALL stay quarantined by the dependency-direction oracle (MAIL-004)

`platform-arch-tests` MUST enforce a named-satellite allowlist: only
`platform-mailing` may reach `lettre`/`tokio`; the four foundation crates and
the `platform-testing` prohibition MUST keep failing on any violation.

#### Scenario: Compliant tree passes

- **WHEN** the workspace dependency graph is walked with `platform-mailing`
  as the only lettre consumer
- **THEN** the architecture tests pass

#### Scenario: Foundation violation fails

- **WHEN** the rule is evaluated over a graph that attaches `lettre` to any
  foundation crate
- **THEN** the rule function reports a violation naming the crate and the
  forbidden dependency

#### Scenario: Unknown satellite denied

- **WHEN** a crate not listed in the allowlist reaches `lettre`
- **THEN** the architecture tests fail until an explicit policy change names
  it
