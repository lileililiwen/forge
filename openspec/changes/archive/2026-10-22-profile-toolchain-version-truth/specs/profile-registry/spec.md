## MODIFIED Requirements

### Requirement: Stack compatibility contract

Forge SHALL resolve a profile only when its declared language, toolchain and capability constraints are compatible and SHALL explain unsupported combinations. Forge SHALL judge a profile's declared toolchain version against the host's runtime version, and SHALL refuse a host whose version does not satisfy the declaration without reporting the toolchain as missing.

#### Scenario: Stack compatibility contract success

- **WHEN** a compatible profile version is selected
- **THEN** the resolver returns its exact version and adapter identity

#### Scenario: Stack compatibility contract failure

- **WHEN** a Flutter profile is asked to install server-side postgres
- **THEN** resolution fails before file changes and suggests a backend boundary

#### Scenario: Stack compatibility contract boundary

- **WHEN** a required toolchain is unavailable locally
- **THEN** preflight reports the missing prerequisite without claiming the profile was tested

#### Scenario: A toolchain of the wrong version

- **WHEN** the required toolchain is present but its version does not
  satisfy the profile's declared toolchain version
- **THEN** preflight refuses with a version-mismatch error naming the
  profile, the declared version and the observed version, and does not
  report the toolchain as missing
