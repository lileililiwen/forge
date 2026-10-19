## 1. Fixture and package feed

- [x] 1.1 Create a test-only consumer fixture and local package-feed build script with no production project references to testing packages.
- [x] 1.2 Add deterministic fake providers/stores for identity, eventing, jobs, mailing, auditing, quota, realtime, and observability seams as applicable.
- [x] 1.3 Adapt starter architecture and template-smoke structure from the paths in `design.md` while keeping the fixture minimal.

## 2. Conformance gates

- [x] 2.1 Add registration/replacement and package-dependency boundary tests.
- [x] 2.2 Add health/readiness, cancellation, failure classification, redaction, and tenant isolation tests.
- [x] 2.3 Add optional Docker/provider integration gates with explicit prerequisites and no fake production success.
- [x] 2.4 Add package version alignment and compatibility checks for .NET 8 consumers and the supported .NET 10 path.

## 3. CI and documentation

- [x] 3.1 Add serial restore/build/test commands and artifact logs suitable for environment-blocked diagnosis.
- [x] 3.2 Document local feed usage, package-consumer boundaries, optional integration prerequisites, and rollback.
- [x] 3.3 Add a verification matrix covering each platform adapter and its required environment.
- [x] 3.4 Run the conformance suite, `git diff --check`, and strict OpenSpec validation.
