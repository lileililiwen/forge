# forge-portable-binary-distribution Specification

## MODIFIED Requirements

### Requirement: CI-gated versioned Forge release

Forge SHALL publish its existing versioned Linux x86_64 archive and SHA-256
sidecar as a GitHub Release only for an operator-pushed `v<semver>` tag that
exactly matches `Cargo.toml` and only after every required job in Forge's
existing CI workflow succeeds. Pull request and branch CI SHALL retain
read-only permissions and SHALL NOT publish, tag, push, delete or overwrite a
release asset. The release job SHALL use the archive produced and verified by
the existing artifact job and SHALL refuse an existing release identity
without replacing its assets.

#### Scenario: Matching version tag publishes verified assets

- **GIVEN** the tag is `v` followed by the version in `Cargo.toml`
- **AND** every required CI job, offline contract check, artifact checksum and
  installed-binary smoke check succeeds
- **WHEN** the tag push workflow completes
- **THEN** the GitHub Release at that tag contains exactly the existing
  `forge-<version>-x86_64-unknown-linux-gnu.tar.gz` archive and its `.sha256`
  sidecar
- **AND** the published files byte-match the verified workflow artifact

#### Scenario: Version tag does not match the package

- **WHEN** a pushed `v<semver>` tag differs from the version in `Cargo.toml`
- **THEN** tag validation fails before the release API is called
- **AND** no release or asset is created

#### Scenario: Required CI job fails

- **WHEN** formatting, build, tests, native scaffold, MSRV, dependency policy,
  governance, readiness, read surfaces, offline contract validation, Gate or
  artifact verification fails
- **THEN** the release publication job does not run
- **AND** no GitHub Release write occurs

#### Scenario: Existing release identity

- **GIVEN** a GitHub Release already exists for the matching tag
- **WHEN** the publication job attempts to publish again
- **THEN** publication fails without clobbering, deleting or replacing an
  existing asset

## REMOVED Requirements

### Requirement: Hosted contract parity uses the recorded source revision

**Reason:** Forge maintains the schemas it consumes. Its CI and release checks
validate the Forge-owned manifest, schemas, and fixtures offline and no longer
fetch or compare a sibling source.
