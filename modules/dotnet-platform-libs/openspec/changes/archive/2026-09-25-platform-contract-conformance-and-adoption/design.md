# Design: .NET contract conformance and adoption

## Ownership

Platform packages provide contracts, safe defaults, adapters, and test tools.
Applications provide their user store, role/permission policy, tenant scope,
EF migrations, provider credentials, and UI. Conformance tests consume local
canonical fixtures but do not make the contract repository a runtime package
dependency.

## Conformance

Add a test-only fixture reader and package-level tests for stable envelopes.
Framework-neutral packages remain free of ASP.NET Core, EF Core, provider SDK,
or application references. ASP.NET Core adapters may map the contracts to
HTTP, while applications choose routes and authentication schemes.

## Adoption diagnostics

The diagnostics surface reports package presence, compatible version, missing
registration, forbidden production references to `*.Testing`, and unresolved
application-owned adapters. It is informational until a project explicitly
promotes a finding into its own Gate.

## Verification

Run the full solution quality gate, architecture tests, package API check,
consumer conformance fixtures, strict OpenSpec validation, and package audit.
No consumer is described as migrated without native project evidence.
