## ADDED Requirements

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
- **AND** every required CI job, artifact checksum and installed-binary smoke
  check succeeds
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
  governance, readiness, read surfaces, contract parity, Gate or artifact
  verification fails
- **THEN** the release publication job does not run
- **AND** no GitHub Release write occurs

#### Scenario: Existing release identity

- **GIVEN** a GitHub Release already exists for the matching tag
- **WHEN** the publication job attempts to publish again
- **THEN** publication fails without clobbering, deleting or replacing an
  existing asset

### Requirement: Hosted contract parity uses the recorded source revision

Forge's hosted CI contract-parity job SHALL obtain the public
`platform-contracts` source at the immutable commit recorded in
`contracts/manifest.json`, pass that checkout to the existing parity script,
and fail if the revision is malformed, unavailable or differs after checkout.
The local parity script SHALL remain offline and SHALL continue to fail when
no source directory is supplied or resolvable.

#### Scenario: Hosted parity uses pinned commit

- **GIVEN** `contracts/manifest.json` records a valid source revision
- **WHEN** Forge's CI contract-parity job runs
- **THEN** it checks out that exact revision without sibling checkout or
  write credentials
- **AND** it passes the source directory to `scripts/contract-parity.sh`

#### Scenario: Pinned source cannot be resolved

- **WHEN** the revision is malformed, unavailable or checkout resolves to a
  different commit
- **THEN** contract parity fails with the requested and observed revisions
- **AND** release publication remains blocked

#### Scenario: Local parity remains offline

- **WHEN** a local caller runs `scripts/contract-parity.sh` without an
  explicit source or sibling checkout
- **THEN** the script reports `parity-source-unresolvable` and exits nonzero
- **AND** it does not fetch source or turn missing input into a pass

### Requirement: Generated CI installs an exact checksum-verified Forge

Forge's current service and client standard packs SHALL provide new immutable
versions whose GitHub Actions workflows download an explicitly pinned Forge
release for `x86_64-unknown-linux-gnu`, validate the checksum sidecar's exact
archive basename and digest before extraction, install the expected executable
below the runner temporary directory, and then run the existing verifier and
`forge gate .`. The consumer workflow SHALL never compile Forge from source,
resolve a latest release or use release write permissions.

#### Scenario: New project runs Gate with its pinned release

- **GIVEN** a generated project uses a current supported service or client
  profile and the pinned Forge release exists
- **WHEN** its GitHub Actions job runs on Ubuntu x86_64
- **THEN** it downloads the exact versioned archive and sidecar, verifies the
  checksum, installs Forge on `PATH`, runs its existing project verifier and
  runs `forge gate .`
- **AND** it performs no Forge source checkout or Cargo install

#### Scenario: Release is unavailable

- **WHEN** the pinned release or either named asset is absent or unreachable
- **THEN** the installer fails with the exact version/asset and the workflow
  does not run Forge Gate

#### Scenario: Checksum sidecar is malformed or names another archive

- **WHEN** the sidecar has an invalid digest, additional record, unexpected
  filename or digest mismatch
- **THEN** the installer fails before extraction, installation or `GITHUB_PATH`
  modification

#### Scenario: Archive omits or redirects the expected executable

- **WHEN** the archive is invalid or the expected
  `forge-<version>-x86_64-unknown-linux-gnu/forge` regular file is absent,
  non-regular or a symlink
- **THEN** the installer fails before exposing a binary on `PATH`

#### Scenario: Existing project snapshot is not overwritten

- **GIVEN** a project already has a receipt for an older supported standard
  pack or a user-edited owned file
- **WHEN** a new Forge version is installed or the new standard pack is
  proposed through `standard upgrade`
- **THEN** installing Forge alone changes no project file and a conflicting
  standard file remains a conflict under the existing ownership rules

### Requirement: New-project defaults select the release-backed pack versions

Forge SHALL default service profiles to
`baseline-service@1.4.0` and client profiles to `baseline-client@1.1.0` only
after both new versions are supported, verified and compatible with every
profile currently in those pack families. Existing pack descriptors and
receipts SHALL retain their original rendered bytes and explicit pins.

#### Scenario: All currently supported profiles use the new defaults

- **WHEN** Forge generates any of `aspnet-web`, `nextjs-web`,
  `python-service`, `rust-web`, `react-web` or `flutter-app` without an
  explicit pack selector
- **THEN** it selects the corresponding new service or client pack and emits
  its version in the snapshot and receipt

#### Scenario: Explicit older supported pack remains selectable

- **WHEN** a project explicitly selects an older compatible supported pack
- **THEN** Forge renders that pack's original bytes and version unchanged
- **AND** does not rewrite the selector to the current default

#### Scenario: New pack installation conflicts with user-owned content

- **WHEN** a new pack path already exists with different bytes and is not
  receipt-owned by that pack
- **THEN** standard installation refuses the whole write before changing any
  project file
