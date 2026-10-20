# forge-governance-consumer-migration Specification

## Purpose
Generic concern checks lived in a separate Workspace Governance checkout, so a
product Gate could not run them without a sibling repository and its
configuration carried another project's implementation paths. The migration into
Forge gives products a `forge concern check` command that resolves the checker
from Forge's source or installed resources and forwards its result unchanged,
keeps governance checkout paths out of product Gate configuration, and installs
the checker resources a release needs.
## Requirements
### Requirement: Forge concern-check facade

Forge SHALL expose a `forge concern check <id> [project-root]` command for the
supported concern checker IDs (`accessibility`, `fixtures`, `placeholder-threshold`,
`product-code-boundary`, `privacy`, `provenance`, and `schemas`), SHALL resolve the implementation from Forge's
source or installed resource bundle, SHALL execute it without a shell, and
SHALL preserve the helper's stdout, stderr and exit code. The default project
root SHALL be the current working directory. The facade SHALL NOT require a
Workspace Governance checkout in the project or its parent directories.

#### Scenario: Concern check passes or is not applicable

- **WHEN** a supported concern is run against a readable project with no
  findings or a proven absence of the applicable surface
- **THEN** Forge forwards the checker's existing JSON result and exit code 0

#### Scenario: Concern check finds an issue

- **WHEN** a supported concern reports one or more findings
- **THEN** Forge forwards the checker's JSON and exit code 1 unchanged

#### Scenario: Concern check cannot complete

- **WHEN** the concern ID is unsupported, the project root is invalid, the
  resource or Python runtime is unavailable, or the checker reports incomplete
  coverage
- **THEN** Forge emits actionable failure evidence and never reports PASS

#### Scenario: Installed Forge resolves its bundled checker

- **WHEN** the Forge release binary runs outside the source checkout
- **THEN** it resolves the checker under its installed `share/forge` resource
  tree without searching sibling repositories or the network

### Requirement: Product Gate configuration hides governance implementation

Forge-owned consolidated product projects SHALL use the Forge baseline
identity and Forge concern command for generic checks. Their live Gate
configuration and active checker wrappers SHALL NOT contain Workspace
Governance checkout paths, `workspace-governance@1`, or commands that invoke
the workspace portfolio audit. Product Gate SHALL retain native build, test,
repository, security and project-specific checks. Workspace registry and
portfolio audit SHALL remain owned by Forge and run through Forge's own
governance workflow.

#### Scenario: Product check runs without governance sibling checkout

- **WHEN** a consolidated product runs its Gate with Forge installed and
  Workspace Governance absent as a sibling checkout
- **THEN** all remaining required product checks resolve and generic concern
  checks run through Forge
- **AND** the product's native build/test/security checks remain configured

#### Scenario: Product configuration exposes no governance implementation

- **WHEN** the live manifests and executable checker wrappers for consolidated
  Forge products are inspected
- **THEN** they contain no sibling Workspace Governance path or provider
  identity
- **AND** historical OpenSpec/HANDOFF records may retain provenance references

#### Scenario: Workspace audit remains available to Forge

- **WHEN** Forge's own governance module CI/self-check runs
- **THEN** Workspace Governance validation still executes from its in-tree
  owner and no product-local Gate is required to run the workspace registry
  audit

### Requirement: Forge release preserves concern-check resource compatibility

Forge release packaging and installation SHALL include the canonical generic
and product-quality concern checker resources with their vocabulary dependencies
under `share/forge` and SHALL refuse to claim those concern commands are
available if a required resource is missing.

#### Scenario: Resource survives package installation

- **WHEN** a Forge release archive is packaged and installed to a prefix
- **THEN** all required concern checker and vocabulary resources are installed
  under the Forge prefix
- **AND** the installed Forge binary can run a supported concern check
