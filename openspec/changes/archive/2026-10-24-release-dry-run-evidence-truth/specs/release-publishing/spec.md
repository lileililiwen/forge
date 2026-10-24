## MODIFIED Requirements

### Requirement: Resumable multi-destination publication

Forge SHALL execute authorized commit, tag, primary push, mirror, package, container and release-note stages with per-stage records and safe retry; forge publish SHALL use the same verified release identity and publication stages without bypassing release checks or the Gate evidence. A stage SHALL write only when the release is confirmed and the run is not a dry run, and a stage that may not write SHALL record a status that is not a delivery together with a note that names the reason it did not write.

#### Scenario: Resumable multi-destination publication success

- **WHEN** every selected release stage completes
- **THEN** the release record links immutable versions and provider receipts

#### Scenario: Resumable multi-destination publication failure

- **WHEN** a package publishes but the container push fails
- **THEN** the release is partial and retry does not republish or overwrite the package

#### Scenario: Resumable multi-destination publication boundary

- **WHEN** a tag or immutable version already exists with different content
- **THEN** the operation refuses replacement and reports the identity conflict

#### Scenario: A dry run applies no stage

- **WHEN** a release is applied as a dry run
- **THEN** no stage writes to the repository, the host or a provider, and every stage record names why it did not write
