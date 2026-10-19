# Platform web runtime foundation

## ADDED Requirements

### Requirement: Web runtime registration SHALL be explicit and composable

The package SHALL provide opt-in registration and pipeline methods whose options enable
individual capabilities without registering identity, persistence, or provider services.

#### Scenario: Minimal host

- **WHEN** a host calls the web registration method with default options
- **THEN** only documented web-runtime services are registered
- **AND** the host can replace a service before or after registration according to the documented policy

### Requirement: Runtime failures SHALL have safe stable responses

The package SHALL return stable live/readiness and safe error responses without exposing
stack traces, secrets, or connection strings.

#### Scenario: Readiness dependency fails

- **WHEN** an application readiness check fails
- **THEN** the readiness endpoint returns the documented failure status and machine-readable result
- **AND** liveness remains independent of the failed dependency

### Requirement: Requests SHALL have consistent diagnostics and limits

The package SHALL provide correlation, redaction, request-size, timeout, and security-header
contracts that applications can configure and observe.

#### Scenario: Untrusted correlation header

- **WHEN** a request contains an invalid or disallowed correlation value
- **THEN** the package generates a valid value according to host policy and exposes it downstream
