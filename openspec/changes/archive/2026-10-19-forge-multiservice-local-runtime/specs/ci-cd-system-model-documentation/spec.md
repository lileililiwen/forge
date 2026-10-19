## MODIFIED Requirements

### Requirement: Current CI/CD flows are traceable

Forge SHALL provide a maintainer guide that distinguishes generated-project
local hooks and hosted CI, Forge's own verification and binary release
workflow, project release checks, provider-backed staged delivery, and local
development runtime entry points. The guide SHALL show command order,
execution owner, inputs, outputs, evidence persistence, revision binding,
and failure transitions using current implementation references. It SHALL
state that Docker Compose owns the declared multi-service graph and lifecycle,
while GitRunByKaru owns a single local app process session; neither is a Forge
CI executor or delivery provider.

#### Scenario: Maintainer traces local runtime startup

- **WHEN** a maintainer follows `forge run compose` or `forge run local`
- **THEN** the guide identifies Forge as an argv-preserving entry-point facade, names the delegated runtime owner, and states the limits of any session/evidence Forge stores

#### Scenario: Maintainer traces a Gate result

- **WHEN** a maintainer follows a local or generated CI Gate run
- **THEN** the guide identifies the command, baseline and extension inputs, revision captured, evidence path, journal owner, readers, and conditions under which evidence is absent or stale

#### Scenario: Maintainer traces release and deployment

- **WHEN** a maintainer follows release preparation or staged delivery
- **THEN** the guide shows their separate preconditions and evidence records, identifies provider-owned operations and explicit promotion confirmation, and does not imply that a Gate or CI pass automatically causes deployment
