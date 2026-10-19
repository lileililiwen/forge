## 1. Repository and package metadata

- [x] 1.1 Inventory packable production projects and define explicit package inclusion/exclusion rules.
- [x] 1.2 Add centralized repository/package metadata, repository URL, license, README, symbols, and Source Link settings.
- [x] 1.3 Document versioning, changelog, supported frameworks, breaking changes, signing, and SBOM policy.

## 2. Quality automation

- [x] 2.1 Add serial restore/build/test/pack and strict OpenSpec validation scripts using the repository SDK baseline.
- [x] 2.2 Add path-scoped pull-request CI and separate release workflow with explicit permissions.
- [x] 2.3 Add `git diff --check`, vulnerability audit, and generated artifact reporting.

## 3. Compatibility and consumer verification

- [x] 3.1 Add a checked-in public API baseline and compatibility check for selected contract packages.
- [x] 3.2 Extend the packed NuGet consumer conformance fixture to consume candidate packages.
- [x] 3.3 Add an upgrade/rollback smoke test and record exact commands in `docs/build-test-pack.md`.

## 4. Tests and verification

- [x] 4.1 Test metadata and package inclusion rules.
- [x] 4.2 Test that unavailable audit evidence is reported as unverified.
- [x] 4.3 Run Release build/test/pack, strict specs validation, API compatibility, vulnerability audit, and `git diff --check`.
