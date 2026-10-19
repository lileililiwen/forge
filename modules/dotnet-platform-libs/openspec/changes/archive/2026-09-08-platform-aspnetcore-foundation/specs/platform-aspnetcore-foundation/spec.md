## ADDED Requirements

### Requirement: ASP.NET integration SHALL map known core failures safely

The package SHALL map known platform failures to standard ProblemDetails responses with stable status/category information and without exposing sensitive internal details.

#### Scenario: Known validation failure

- **WHEN** an endpoint surfaces a known validation failure
- **THEN** the response contains a client-safe ProblemDetails payload with the stable error code

### Requirement: ASP.NET integration SHALL preserve unknown-exception policy

The package SHALL not expose stack traces or exception details in production responses and SHALL allow the host application to configure its broader exception policy.

#### Scenario: Unexpected exception

- **WHEN** an unrecognized exception reaches the error boundary
- **THEN** the response is generic and the exception remains available to configured server-side logging

### Requirement: Request correlation SHALL be available to consumers

The package SHALL expose the current correlation identifier through an injectable accessor and SHALL preserve a trusted incoming identifier only according to an explicit host policy.

#### Scenario: Request without correlation header

- **WHEN** a request arrives without a correlation identifier
- **THEN** the middleware creates one and makes it available to downstream code

### Requirement: Registration SHALL be explicit

The package SHALL use documented opt-in registration methods and SHALL not silently register unrelated authentication, persistence, or provider services.

#### Scenario: Minimal host adoption

- **WHEN** a host registers the platform web services
- **THEN** only the documented platform services and middleware behavior are added
