## ADDED Requirements

### Requirement: Platform packages SHALL have an anonymous static NuGet v3 distribution

The platform SHALL publish its unchanged `Platform.*` package IDs to a static
NuGet v3 endpoint that supports anonymous restore and does not require a
consumer credential.

#### Scenario: Clean remote consumer

- **WHEN** a consumer restores on GitHub Actions, in Docker, or on another computer
- **THEN** it resolves `Platform.*` from the public platform feed without a token, sibling checkout, Forge runtime or machine-specific path

#### Scenario: Package identity

- **WHEN** a platform project is packed for the public feed
- **THEN** its package ID, assembly name and C# namespace remain in the existing `Platform.*` family

### Requirement: Publication SHALL preserve immutable feed history

The release workflow SHALL preserve earlier package versions, refuse different
bytes for an existing ID/version, validate the generated feed, and publish only
after the platform quality and compatibility checks pass.

#### Scenario: New release

- **WHEN** an authorized workflow dispatch supplies a new SemVer version
- **THEN** all packable artifacts are added to the retained feed tree and the validated tree is deployed to GitHub Pages

#### Scenario: Conflicting version

- **WHEN** the feed already contains the requested package ID and version with different bytes
- **THEN** publication fails before the retained feed or deployed Pages artifact is changed

### Requirement: Consumer source mapping SHALL prevent package substitution

The canonical consumer configuration SHALL map `Platform.*` to trusted
platform sources and all other packages to nuget.org.

#### Scenario: Unrelated public collision

- **WHEN** nuget.org contains an unrelated package such as `Platform.Core`
- **THEN** a conforming consumer does not consider nuget.org for that package ID

#### Scenario: User-local development package

- **WHEN** the current user has configured `platform-local` and selects a unique local prerelease version
- **THEN** NuGet can restore it without a committed local path or consumer credential
