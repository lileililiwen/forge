## ADDED Requirements

### Requirement: Web-edge integrations are opt-in
The platform SHALL expose CORS, resilience, OpenAPI, and observability integrations independently so a consumer can adopt one without enabling the others.

#### Scenario: Consumer adopts resilience only
- **WHEN** an application registers the resilience package
- **THEN** it does not receive an OpenAPI endpoint or CORS policy implicitly

### Requirement: CORS production configuration is validated
The platform SHALL reject production configurations that allow credentials with unrestricted wildcard origins and SHALL require explicit allowed origins when unrestricted mode is disabled.

#### Scenario: Credentials are combined with wildcard origin
- **WHEN** production CORS options enable credentials and wildcard origin
- **THEN** startup validation fails with a safe configuration error

### Requirement: HTTP resilience is bounded and explicit
The platform SHALL provide configurable retry, timeout, and circuit-breaker conventions with bounded defaults and SHALL avoid retrying non-idempotent operations by default.

#### Scenario: Transient GET fails
- **WHEN** a configured GET receives a retryable transient failure
- **THEN** the client applies the bounded retry policy and stops at the configured attempt limit

### Requirement: OpenAPI exposure is explicit
The platform SHALL allow applications to register named API documents and map their JSON endpoints explicitly, with authorization metadata preserved.

#### Scenario: Application maps one API document
- **WHEN** the application registers version `v1` and maps its document endpoint
- **THEN** only the explicitly mapped document is exposed

### Requirement: Telemetry is redaction-safe
The platform SHALL define stable request and provider instrumentation names and SHALL exclude authorization headers, cookies, raw request bodies, secrets, and unrestricted query values by default.

#### Scenario: Provider call fails
- **WHEN** an instrumented provider call fails
- **THEN** telemetry includes a safe operation/category and omits credentials and response bodies

### Requirement: Package organization remains independently adoptable
The platform SHALL keep independently packable dependency groups in separate top-level package roots and SHALL use shallow concern folders within each package.

#### Scenario: Consumer installs only CORS support
- **WHEN** a consumer references the CORS package
- **THEN** it does not acquire resilience, OpenAPI UI, or unrelated provider dependencies
