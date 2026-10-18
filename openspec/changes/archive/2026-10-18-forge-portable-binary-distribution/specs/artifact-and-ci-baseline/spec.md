## MODIFIED Requirements

### Requirement: Honest labelling and explicit non-publication

No workflow step, script label or document SHALL name a check it does not
run. Forge's CI SHALL produce local artifacts without publishing, tagging,
deploying or promoting them on branches or pull requests. Only a user-pushed
tag exactly matching `v< Cargo.toml version >` MAY trigger release publication,
and only after every required Forge CI job succeeds. The release job SHALL
reuse the artifact job's verified archive and checksum and SHALL refuse to
overwrite an existing release or asset.

#### Scenario: Label matches command

- **WHEN** a step is labelled with a check name
- **THEN** the commands it runs include that check, or the label is removed

#### Scenario: Artifact is not promoted

- **WHEN** the artifact job completes for a branch push or pull request
- **THEN** the archive and digest exist as CI artifacts only, and no registry,
  package host, remote tag or deployment target is contacted

#### Scenario: Explicit verified tag publishes the archive

- **GIVEN** the pushed tag matches the Cargo package version and every
  required CI job succeeds
- **WHEN** the tagged workflow completes
- **THEN** it publishes the exact verified archive and digest as a GitHub
  Release, without changing the source tag or deploying elsewhere

#### Scenario: Failed verification blocks release publication

- **WHEN** a required CI job or artifact verification fails
- **THEN** the release job does not run and no GitHub Release write occurs

#### Scenario: Existing release is immutable

- **WHEN** a release already exists for the requested tag
- **THEN** publication fails without clobbering, deleting or replacing its
  assets

#### Scenario: Gate block is reported, not laundered

- **WHEN** the real gate returns blocked or an unresolved review requirement
- **THEN** the evidence records that verdict and the recovery path, and no
  configuration is adjusted to obtain a passing job
