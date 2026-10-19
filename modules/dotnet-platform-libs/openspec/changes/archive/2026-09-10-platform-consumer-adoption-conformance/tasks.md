## 1. Adoption fixture

- [x] 1.1 Audit the existing `tests/Platform.ConsumerConformance` fixture and `scripts/conformance.sh`.
- [x] 1.2 Add exact-version local-feed restore and package manifest generation.
- [x] 1.3 Add a minimal pilot consumer project with no platform source references.

## 2. Conformance behavior

- [x] 2.1 Cover registration, consumer replacement, health/status, safe failures, and opt-in behavior.
- [x] 2.2 Add previous/current package upgrade matrix checks.
- [x] 2.3 Add rollback verification after candidate failure.
- [x] 2.4 Add production/test dependency and application-ownership architecture checks.

## 3. Documentation

- [x] 3.1 Document private/local feed setup, exact pinning, pilot selection, and rollback.
- [x] 3.2 Document package compatibility, vulnerability review, and no-ProjectReference policy.
- [x] 3.3 Add an adoption checklist suitable for each of the 26+ consumer repositories.

## 4. Verification

- [x] 4.1 Run conformance against packed artifacts and both package versions.
- [x] 4.2 Run strict OpenSpec validation and `git diff --check`.
- [x] 4.3 Record environment-blocked feed/signature checks as unverified, not passed.