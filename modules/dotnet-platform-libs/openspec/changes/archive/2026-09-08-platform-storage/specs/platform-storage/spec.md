## ADDED Requirements

### Requirement: Storage contracts are provider-neutral
The platform SHALL provide object upload, download, metadata, delete, and presigned-operation contracts without requiring a filesystem, cloud SDK, or database.

#### Scenario: Application uses a non-S3 provider
- **WHEN** an application implements the storage contract for another object store
- **THEN** it can do so without changing application-facing storage calls

### Requirement: Object keys are safe
The platform SHALL validate object keys and SHALL reject empty keys, control characters, path traversal segments, and keys exceeding the configured maximum length.

#### Scenario: Traversal key is supplied
- **WHEN** an upload request contains `../private/secret`
- **THEN** the request is rejected before provider access

### Requirement: Upload constraints are enforced
The platform SHALL allow an application or adapter to enforce maximum content length and expected content type for an upload operation.

#### Scenario: Upload exceeds its declared limit
- **WHEN** the content length is greater than the configured limit
- **THEN** the operation fails with a safe size-limit result and no object is written

### Requirement: Presigned operations are bounded
The platform SHALL return presigned operations with an explicit HTTP method, object key, expiry, and optional content constraints.

#### Scenario: Presign lifetime exceeds the maximum
- **WHEN** a caller requests a lifetime longer than the configured maximum
- **THEN** the adapter clamps or rejects the request according to its documented policy and never issues an unbounded URL

### Requirement: Provider failures are safe and observable
The platform SHALL classify storage failures without exposing secrets or provider response bodies and SHALL expose provider availability for readiness integration.

#### Scenario: Object store is unavailable
- **WHEN** a transient provider error occurs
- **THEN** the caller receives a classified transient failure and provider status reports unavailable
