## Context

The core package is the lowest dependency layer. It must be usable by .NET 8 and .NET 10 applications and should remain usable by non-web application code.

## API Shape

Add `IAuditable`, `IClock` or an equivalent time abstraction, caller context contracts, and a generic operation-result model with success and typed failure states. Use immutable records where appropriate. Avoid exposing mutable collections or application-specific enums.

The caller context should represent optional subject and tenant identifiers without deciding how they are resolved. Implementations belong to consuming applications or higher platform layers.

## Error Semantics

Core failures carry a stable code, safe message, and optional metadata. They must not depend on HTTP status codes. ASP.NET integration will map them later.

## Verification

Unit tests cover success/failure construction, UTC clock behavior, empty identity handling, and serialization-friendly public shapes. A compile-time dependency check confirms no web, data, or provider packages are referenced.
