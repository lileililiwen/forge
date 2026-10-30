## ADDED Requirements

### Requirement: The archive shape is checked against the documented layout

Forge's artifact job SHALL verify the produced archive against the documented
layout: the versioned root holds the binary, licence, readme and changelog, and
the only nested tree is `share/forge/**`. Any other nested path, and any
absolute path, SHALL block the job.

#### Scenario: Documented layout

- **WHEN** the archive contains root files and `share/forge/contracts`,
  `share/forge/runtime-templates` and `share/forge/governance`
- **THEN** the artifact job accepts it

#### Scenario: Undocumented path

- **WHEN** the archive contains a nested path outside `share/forge`
- **THEN** the job fails naming the archive and the unexpected path

#### Scenario: Absolute host path

- **WHEN** an entry starts with `/`
- **THEN** the job fails and does not upload the artifact
