## ADDED Requirements

### Requirement: Host observability registration

The package SHALL provide explicit opt-in registration for correlation, service identity, logging enrichment, tracing, metrics, and runtime diagnostics without requiring an application to adopt a specific exporter.

#### Scenario: Minimal registration
- **WHEN** an application calls the platform observability registration with defaults
- **THEN** it receives correlation and platform instrumentation without a provider client or exporter being constructed

### Requirement: Safe diagnostic enrichment

The package SHALL exclude secrets, authorization values, request bodies, cache values, and raw provider responses from default logs, traces, and metrics.

#### Scenario: Sensitive request
- **WHEN** an authenticated request contains an authorization header and a tenant identifier
- **THEN** diagnostics contain only configured safe correlation and subject metadata and never the authorization value

### Requirement: Stable platform telemetry

The package SHALL expose documented platform activity source and meter names with bounded labels that do not include raw identifiers or unbounded request data.

#### Scenario: Platform operation
- **WHEN** a platform adapter records an operation
- **THEN** the emitted activity and metric use stable names and bounded outcome/provider labels

### Requirement: Replaceable exporter and redactor seams

The package SHALL allow consumers to provide their own exporter, logger integration, redactor, and service name without modifying platform code.

#### Scenario: Consumer redactor
- **WHEN** a consumer registers a custom redactor
- **THEN** platform diagnostics use that redactor for configured sensitive values
