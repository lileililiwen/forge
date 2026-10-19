## ADDED Requirements

### Requirement: Core SHALL provide testable time access

The core package SHALL expose a small abstraction for obtaining the current UTC time, with a production implementation that delegates to the platform clock and a deterministic test implementation.

#### Scenario: Test a time-dependent operation

- **WHEN** a consumer supplies a fixed clock value
- **THEN** the operation observes that value without reading system time directly

### Requirement: Core SHALL provide framework-neutral operation results

The core package SHALL represent successful and failed outcomes without requiring HTTP, EF Core, or provider-specific types.

#### Scenario: Return a validation failure

- **WHEN** a consumer constructs a failed result with a stable error code
- **THEN** the result preserves the code and safe message for a higher layer to translate

### Requirement: Core SHALL expose optional caller context

The caller context SHALL represent optional subject and tenant identifiers and SHALL not resolve, persist, or authorize them.

#### Scenario: Anonymous caller

- **WHEN** no subject is available
- **THEN** the context represents an anonymous caller without throwing

### Requirement: Core SHALL avoid universal persistence inheritance

The core package SHALL use small interfaces for shared metadata and SHALL not require consumers to inherit from a platform entity base class.

#### Scenario: Product-specific entity model

- **WHEN** an application uses its own aggregate base or record types
- **THEN** it can implement the relevant platform interface without changing its inheritance hierarchy
