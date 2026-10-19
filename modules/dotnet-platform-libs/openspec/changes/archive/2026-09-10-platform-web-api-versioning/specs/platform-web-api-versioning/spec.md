## ADDED Requirements

### Requirement: API versioning SHALL be opt-in

The platform SHALL provide explicit registration for API versioning and SHALL not alter route matching or response headers in applications that do not register it.

#### Scenario: Consumer does not opt in
- **WHEN** an application uses `Platform.Web` without versioning registration
- **THEN** existing endpoint routing behavior remains unchanged

#### Scenario: Consumer opts in
- **WHEN** an application calls the versioning registration extension
- **THEN** versioning services and API Explorer conventions are registered once

### Requirement: Version policy SHALL be configurable

Consumers SHALL be able to configure the default API version, whether unspecified requests assume that version, version readers, and API Explorer group formatting.

#### Scenario: URL-segment default
- **WHEN** the consumer selects URL-segment reading and default version `1.0`
- **THEN** a versioned `v1` route is discoverable and an unspecified request follows the configured assumption

#### Scenario: Header reader
- **WHEN** the consumer selects a header reader
- **THEN** the configured header determines the requested version without the package forcing URL segments

### Requirement: Versioning SHALL integrate with API description

The adapter SHALL expose versioned API descriptions compatible with the platform OpenAPI package and SHALL not own application endpoint metadata.

#### Scenario: Versioned OpenAPI groups
- **WHEN** an application exposes two API versions
- **THEN** API Explorer produces distinct configured groups for those versions
