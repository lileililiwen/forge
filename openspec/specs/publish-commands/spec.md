# publish-commands Specification

## Purpose
Linux-controlled runtime deployment over an artifact-only Mac boundary, where builds happen locally and Mac-local secrets never leave the target.
## Requirements
### Requirement: Linux-controlled runtime deployment

Forge MUST consume the `forge-deploy-executor/0.1.0` contract for a
`deployment.targets[].kind: mac-runtime` target and MUST pass the project root
as the executor working directory.

#### Scenario: Dry-run does not contact Mac

- **WHEN** `forge deploy apply --dry-run` targets `mac-runtime`
- **THEN** Forge reports the release and adapter plan
- **AND** no SSH, Docker, Jenkins, or Mac-side script is invoked.

#### Scenario: Apply records runtime evidence

- **WHEN** the adapter applies a valid immutable runtime release
- **THEN** Forge records source revision, artifact hash, target, adapter,
  apply outcome, and observation outcome.

### Requirement: Artifact-only Mac boundary

Forge and its Mac adapter MUST deploy an immutable release artifact and MUST NOT
sync a product source tree or invoke a path under the Mac deployment-script
directory.

#### Scenario: Source sync is absent

- **WHEN** a release is deployed
- **THEN** no rsync operation is planned and no product checkout is required on
  the Mac.

### Requirement: Local build ownership

Forge MUST run the configured container build/publish stage on Linux before a
runtime deployment may reference the image digest.

#### Scenario: Build is local to Forge

- **WHEN** a release declares a container destination
- **THEN** Forge invokes the configured local container adapter
- **AND** GitHub Actions is not a prerequisite.

### Requirement: Mac-local secret boundary

Runtime configuration MUST reference a Mac-local env-file path without reading
or persisting its contents on Linux.

#### Scenario: Secret path is passed only as a reference

- **WHEN** the release contains `env_file`
- **THEN** the runtime command passes the path to the Mac Docker runtime
- **AND** Forge output contains no secret value.

