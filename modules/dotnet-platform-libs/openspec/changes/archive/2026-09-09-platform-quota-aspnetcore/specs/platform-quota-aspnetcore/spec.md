## ADDED Requirements

### Requirement: HTTP quota enforcement

The adapter SHALL enforce application-selected quota resources through ASP.NET Core middleware or endpoint metadata without defining application plans or resource limits.

#### Scenario: Allowed request
- **WHEN** the application resource resolver selects a resource and the quota store allows it
- **THEN** the request proceeds and the adapter records the configured quota operation

### Requirement: Standard quota rejection

The adapter SHALL return HTTP 429 RFC 9457 problem details with safe resource, limit, usage, correlation, and trace metadata and SHALL include `Retry-After` when a reset time is available.

#### Scenario: Limit exceeded
- **WHEN** the quota store denies the operation with a reset time
- **THEN** the response is 429 with problem details and a correctly calculated `Retry-After` value

### Requirement: Missing-context policy

The adapter SHALL require an explicit application policy for unresolved subject or tenant context and SHALL not silently combine unrelated subjects into a shared quota.

#### Scenario: Missing subject
- **WHEN** a protected quota operation has no resolved subject
- **THEN** the configured fail-closed or explicit anonymous policy is applied and the decision is observable

### Requirement: Configurable exemptions

The adapter SHALL support explicit health, endpoint, method, and application-defined exemptions without hard-coding product routes.

#### Scenario: Health probe
- **WHEN** a configured liveness endpoint is requested
- **THEN** the quota adapter bypasses enforcement and does not consume quota
