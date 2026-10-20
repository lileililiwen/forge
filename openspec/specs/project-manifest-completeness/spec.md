# project-manifest-completeness Specification

## Purpose
Every project Forge creates carries a complete manifest. `forge new` and `forge import --accept` write a `release` block and a `deployment` block alongside the feature, kit, runtime and project blocks, so the path from "generated" to "releasable and deployable" needs no manual edit of `forge.yaml`. The defaults are derived from the project itself: the release block declares checks the project can satisfy, and the deployment block declares an artifact only when the project ships that file. A manifest the owner wrote is never rewritten.
## Requirements
### Requirement: A generated or imported project carries a complete release block

Forge SHALL write a `release` block into every manifest it creates, for a generated project and for an adopted one. The block SHALL declare a supported versioning scheme and SHALL declare at least one check kind. Forge SHALL NOT rewrite a `release` block that a project already carries.

#### Scenario: A generated project carries a releasable configuration

- **WHEN** an owner generates a project
- **THEN** the manifest carries a `release` block, the release configuration resolves, and the declared checks are `doctor` and `test`

#### Scenario: A project without a changelog is refused for that reason

- **WHEN** an owner runs `forge release prepare` on a generated project that has no changelog file yet
- **THEN** the refusal names the missing changelog, and the manifest is not reported as incomplete

#### Scenario: An adopted project carries a releasable configuration

- **WHEN** an owner accepts an import proposal
- **THEN** the adopted manifest carries the same `release` block as a generated one

#### Scenario: The declared checks are ones the project can satisfy

- **WHEN** Forge writes the `release` block
- **THEN** the block declares the `doctor` and `test` checks, and does not declare a check that needs evidence the owner has not produced

#### Scenario: The owner's own release block is kept

- **WHEN** a project already declares a `release` block
- **THEN** Forge leaves that block exactly as the owner wrote it

### Requirement: A generated or imported project carries a complete deployment block

Forge SHALL write a `deployment` block into every manifest it creates. The block SHALL declare at least one target, and every declared target SHALL name a supported adapter kind and a valid target name. Forge SHALL NOT rewrite a `deployment` block that a project already carries.

#### Scenario: A generated project is deployable

- **WHEN** an owner generates a project and then runs `forge deploy plan`
- **THEN** the manifest carries a `deployment` block, the deployment configuration resolves, and the plan names the target

#### Scenario: An adopted project is deployable

- **WHEN** an owner accepts an import proposal
- **THEN** the adopted manifest carries a `deployment` block with at least one target

#### Scenario: The block never mixes the two target forms

- **WHEN** Forge writes a `deployment` block
- **THEN** the block uses the typed `targets[]` form and does not also declare the legacy `type`/`target` fields

#### Scenario: The owner's own deployment block is kept

- **WHEN** a project already declares a `deployment` block
- **THEN** Forge leaves that block exactly as the owner wrote it

### Requirement: The defaults are derived from what the project has

Forge SHALL derive the default blocks from the project itself and SHALL NOT declare anything the project does not have. The deployment artifact SHALL be declared only when the project contains that file, and the target kind SHALL follow the artifact. Forge SHALL NOT declare a health check for a service the project does not run.

#### Scenario: A new project with a container file

- **WHEN** Forge generates a project whose profile ships a container file
- **THEN** the `deployment` block declares that file as the artifact, and the artifact is a file the generated tree contains

#### Scenario: A new project without a container file

- **WHEN** Forge generates a project whose profile ships no container file
- **THEN** the `deployment` block declares its target and no artifact

#### Scenario: An adopted repository with a Compose file

- **WHEN** an owner accepts an import proposal for a repository that ships a Compose file
- **THEN** the `deployment` block declares a `docker-compose` target and that file as the artifact

#### Scenario: An adopted repository with no container file

- **WHEN** an owner accepts an import proposal for a repository that ships neither a Compose file nor a container file
- **THEN** the `deployment` block declares its target and no artifact

#### Scenario: No fabricated health check

- **WHEN** Forge writes a `deployment` block
- **THEN** the block declares no health check, because the project runs no service yet

### Requirement: The registry observation reflects the completed manifest

Forge SHALL record the deployment target of a project from the manifest it registers, whether the manifest declares the legacy `type`/`target` fields or the typed `targets[]` list. A project whose manifest declares no deployment SHALL record no deployment target.

#### Scenario: A typed target is observed

- **WHEN** a project whose manifest declares typed `deployment.targets[]` entries is registered
- **THEN** `forge inspect` names a deployment target for it

#### Scenario: The declared default wins

- **WHEN** a project declares several typed targets and names one as the default
- **THEN** the observation names the declared default

#### Scenario: No deployment block

- **WHEN** a project whose manifest declares no `deployment` block is registered
- **THEN** the observation records no deployment target

### Requirement: A manifest Forge did not write is never rewritten

Forge SHALL change only the manifest it writes in the operation it is performing. Adding a release block or a deployment block SHALL NOT touch a manifest that already exists, and SHALL NOT remove, reorder or rewrite any other block of a manifest Forge writes.

#### Scenario: An existing project is adopted again

- **WHEN** an owner runs an operation against a project that already has a `forge.yaml`
- **THEN** that manifest is unchanged

#### Scenario: The other blocks survive

- **WHEN** Forge writes a manifest for a project that pins a kit and requests features
- **THEN** the `features` block and the `kit` block are written exactly as before, next to the two new blocks
