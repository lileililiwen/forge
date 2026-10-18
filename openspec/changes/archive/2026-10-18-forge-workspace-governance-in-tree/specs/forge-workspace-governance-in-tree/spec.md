# forge-workspace-governance-in-tree Specification

## ADDED Requirements

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

### Requirement: Consolidation and retirement status remain honest

Forge documentation SHALL distinguish the in-tree source import from
consumer migration and origin retirement. It SHALL identify Workspace
Governance in the final retirement gate, retain the source-only commits, and
state that sibling consumers remain dependent until migrated and verified.

#### Scenario: Import is complete but consumers still rely on origin layout

- **WHEN** the source project is imported but sibling consumers have not
  completed their portability migration
- **THEN** Forge reports the origin as not yet retired
- **AND** the roadmap lists consumer migration as a prerequisite to archival

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
