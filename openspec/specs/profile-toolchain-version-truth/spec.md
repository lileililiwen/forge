# profile-toolchain-version-truth Specification

## Purpose
A profile's declared toolchain version is a requirement, not decoration: the preflight and the readiness matrix compare it with the host's runtime version through one shared rule, a mismatch is its own error and never a missing toolchain, the Node ecosystem's runtime is `node` so both Node profiles judge the same version, and the catalogue's declarations match the versions the repository's own qualification provisions.
## Requirements
### Requirement: A declared toolchain version is a checked requirement

Forge SHALL treat a profile's declared toolchain version as a requirement
on the version of the runtime that executes the profile's commands, and
SHALL compare it with the host's version wherever Forge judges the
toolchain. The comparison SHALL be component-wise on the declared
version's component count, and SHALL read the version from the runtime's
own version banner. A declaration that names a moving channel rather than
a version SHALL state no version requirement. A requirement that cannot be
compared because the version could not be read SHALL be recorded as
unknown and SHALL NOT be read as a mismatch.

#### Scenario: A host that meets the declaration

- **WHEN** the host's runtime version satisfies the profile's declared
  toolchain version
- **THEN** the preflight succeeds and names the version it checked

#### Scenario: A host that does not meet the declaration

- **WHEN** the host's runtime version does not satisfy the profile's
  declared toolchain version
- **THEN** the preflight is refused with a version-mismatch error naming
  the profile, the declared version and the observed version, and the
  toolchain is not reported as missing

#### Scenario: The comparison is component-wise

- **WHEN** a profile declares `3.12` and the host reports `3.12.4`
- **THEN** the requirement is satisfied, and a host reporting `3.14.4` or
  `3.1` does not satisfy it

#### Scenario: A profile that states no version requirement

- **WHEN** a profile declares no toolchain version, or declares a moving
  channel rather than a version
- **THEN** no version requirement is judged, and the toolchain's presence
  is the only condition

### Requirement: The runtime is the ecosystem's binary

Forge SHALL judge a profile's toolchain version against the version of the
runtime that executes the profile's commands, and SHALL resolve that
runtime from the ecosystem the toolchain belongs to. The Node ecosystem's
runtime SHALL be `node`, so the profiles that build with `npm run build`
declare and check the same node version.

#### Scenario: The npm ecosystem's runtime

- **WHEN** a profile whose toolchain belongs to the Node ecosystem is
  judged
- **THEN** the version compared is the node runtime's version, and the
  profile's declared version is the node version

#### Scenario: A toolchain with no registered ecosystem

- **WHEN** a toolchain belongs to no registered ecosystem
- **THEN** the toolchain is its own runtime and its own version

### Requirement: The declared versions match the repository's qualification

Forge SHALL declare, for every profile that declares one, a toolchain
version that the profile's runtime can have and that the repository's own
qualification provisions. A declaration that contradicts the repository's
qualification evidence SHALL NOT ship.

#### Scenario: A declared version the runtime can have

- **WHEN** the catalogue is read
- **THEN** every declared toolchain version is a version the profile's
  runtime can have, and the repository's contract check fails naming the
  profile when it does not

#### Scenario: A declared version the qualification contradicts

- **WHEN** a profile's declared toolchain version disagrees with the
  version the repository's own CI qualification provisions
- **THEN** the repository's contract check fails naming the profile and
  both versions

### Requirement: The toolchain version claims are pinned

Forge SHALL pin the toolchain version rule, the catalogue's declarations,
the preflight's verdict and the readiness matrix's verdict in the
repository's contract check, so that a declaration cannot drift and a
check cannot stop comparing.

#### Scenario: The contract check reads the rule and the callers

- **WHEN** the repository's contract check reads the version rule, the
  catalogue, the preflight and the readiness matrix
- **THEN** it fails naming the profile or the rule when a declared
  version cannot be checked, when a mismatch is not refused, or when a
  matrix row on a mismatched host is reported as passing
