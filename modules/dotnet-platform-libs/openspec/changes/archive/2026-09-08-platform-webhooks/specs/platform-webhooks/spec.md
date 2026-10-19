## ADDED Requirements

### Requirement: Inbound requests are verified before processing
The platform SHALL expose an inbound verification contract that receives raw request bytes, headers, provider identity, and a secret resolver before an application handler processes the payload.

#### Scenario: Signature is invalid
- **WHEN** an inbound request fails the configured verifier
- **THEN** the request is rejected and the application handler is not invoked

### Requirement: Inbound replay is suppressed
The platform SHALL persist or delegate persistence of a provider-scoped event identifier before processing and SHALL return a duplicate decision for a previously accepted identifier.

#### Scenario: Provider retries an accepted webhook
- **WHEN** the same provider and event identifier arrive again
- **THEN** the platform reports a duplicate without executing the handler twice

### Requirement: Outbound targets are SSRF-safe
The platform SHALL validate outbound webhook targets and SHALL reject non-absolute, non-HTTPS, loopback, private, link-local, and otherwise disallowed destinations.

#### Scenario: Subscription targets a private address
- **WHEN** an outbound subscription resolves to a private network address
- **THEN** delivery is rejected before an HTTP request is sent

### Requirement: Outbound delivery is retryable and observable
The platform SHALL record delivery attempts, response classification, next-attempt time, and terminal status without storing response bodies or secrets by default.

#### Scenario: Receiver returns a transient failure
- **WHEN** an outbound request receives a retryable status or transport failure
- **THEN** the delivery is scheduled for a bounded retry and exposes a safe failure category

### Requirement: Webhook secrets are redacted
The platform SHALL prevent webhook secrets and raw signed payloads from appearing in default logs, status projections, or failure metadata.

#### Scenario: Verification fails
- **WHEN** a verifier returns a failure
- **THEN** diagnostics contain a safe category/code and not the secret or complete payload
