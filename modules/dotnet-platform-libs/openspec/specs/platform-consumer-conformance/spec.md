# platform-consumer-conformance Specification

## Purpose

The platform packages are independently adoptable, but there is no reusable consumer conformance suite proving that registrations, replacement seams, health behavior, isolation, and provider failures work together. The starter kit's architecture, integration, migration-smoke, and template-smoke tests expose the verification gap.
## Requirements
### Requirement: Package-feed consumer fixture

The conformance suite SHALL restore platform packages from a locally produced package feed in a separate consumer fixture and SHALL not use cross-repository project references.

#### Scenario: Packed consumer restore
- **WHEN** the suite packs the platform and restores the fixture
- **THEN** the fixture compiles against package artifacts and can be tested independently from platform project references

### Requirement: Registration and replacement verification

The suite SHALL verify opt-in registration, consumer replacement of replaceable services, absence of unrelated provider dependencies, and deterministic options validation.

#### Scenario: Consumer override
- **WHEN** the fixture registers an application implementation after platform defaults
- **THEN** the application implementation is resolved and the platform default does not hijack the contract

### Requirement: Runtime safety verification

The suite SHALL cover health/readiness, safe failure classification, cancellation, tenant isolation where applicable, and bounded diagnostics for each supported adapter.

#### Scenario: Provider unavailable
- **WHEN** an adapter provider is unavailable in a deterministic fake
- **THEN** the fixture verifies the documented fail-open/fail-closed result and confirms that secrets are absent from diagnostics

### Requirement: Environment blocker reporting

The verification workflow SHALL distinguish source failures from unavailable Docker, databases, credentials, external providers, or package feeds and SHALL record the exact failed command and next action.

#### Scenario: Docker unavailable
- **WHEN** an integration gate cannot start its required container
- **THEN** the workflow reports an environment blocker without claiming the source implementation passed or failed
