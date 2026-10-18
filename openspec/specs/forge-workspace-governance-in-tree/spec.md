# forge-workspace-governance-in-tree Specification

## Purpose
TBD - created by archiving change forge-workspace-governance-in-tree. Update Purpose after archive.
## Requirements
### Requirement: Complete Workspace Governance in-tree project

Forge SHALL maintain the complete Workspace Governance project as ordinary
source under `modules/workspace-governance/`, preserving its independently
runnable Python 3.11 project layout, all files tracked at the imported source
revision except for the explicitly declared Forge monorepo registry and
fleet-test adaptations, and every source commit present on the local source
branch at import time. Forge SHALL NOT represent the project as a Git
submodule.

#### Scenario: Imported project content and history are complete

- **WHEN** the import is verified against the recorded source revision
- **THEN** every source tracked path remains under
  `modules/workspace-governance/`
- **AND** every unadapted source path retains the source mode and blob
- **AND** only the recorded monorepo registry/test adaptations differ
- **AND** every source-only commit recorded at import is reachable from the
  Forge commit that contains the project
- **AND** no `.gitmodules` entry or nested Git repository is introduced

#### Scenario: Imported project remains independently checkable

- **WHEN** the documented Workspace Governance self-check and tests run with
  the working directory set to `modules/workspace-governance/`
- **THEN** they resolve project-relative configuration from that module root
- **AND** fleet-level checks use the derived or explicitly configured
  portfolio root

### Requirement: Forge maintains the imported project after origin archival

Forge documentation SHALL state that the original Workspace Governance
repository is archived and that `modules/workspace-governance/` is the
maintained copy. Any needed source changes SHALL be made in the Forge copy.
The module's source-size conformance and sibling consumer portability SHALL
remain tracked as Forge work, independent of the completed origin archival.

#### Scenario: A needed source change is found after origin archival

- **WHEN** maintainers need to change Workspace Governance behavior or source
- **THEN** they make and verify the change under
  `modules/workspace-governance/` in Forge
- **AND** Forge does not direct maintenance to the archived source repository
- **AND** source-size conformance and sibling portability remain separate from
  the origin's archive status

### Requirement: Workspace Governance resolves consolidated Forge projects

The Workspace Governance project registry SHALL preserve the IDs of the
consolidated `driftwatchdog`, `cargo-audit-boost`, `cargo-matrix-boost`, and
`cargo-sonar` projects while resolving each path to its actual declaration
under `forge/crates/`. The path migration SHALL be recorded and tested without
rewriting the historical identity-and-taxonomy migration record.

#### Scenario: Consolidated crate declarations are found

- **WHEN** Workspace Governance audits the portfolio root after import
- **THEN** the four tooling registry entries resolve to
  `forge/crates/<project-id>/.project.json`
- **AND** no `PROJECT_MISSING` finding is emitted for their former sibling
  paths
- **AND** each project ID and declared role/stack remain unchanged

#### Scenario: Nested monorepo members remain in axis migration checks

- **WHEN** the axis migration checks a workspace containing registered nested
  project paths
- **THEN** it includes each path that resolves inside the workspace and has a
  `.project.json` declaration in its plan
- **AND** it refuses paths that escape the configured workspace root
