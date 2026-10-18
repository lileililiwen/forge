# forge-web-project-workbench Specification

## Purpose
Provide project-scoped browser workflows that plan before every write, execute only typed Core contracts, and show partial failures honestly.
## Requirements
### Requirement: Project-scoped typed workflows

Forge SHALL provide browser workflows for each project setup, inspection, planning and quality command mapped to this package, using the same Core validation and results as CLI/MCP.

#### Scenario: Inspect managed project

- **WHEN** an authenticated operator opens a managed project
- **THEN** the browser displays current manifest, profile, health, features, standards and journal-backed evidence with timestamps

#### Scenario: Project not manageable

- **WHEN** a project is observed-only or lacks the required capability
- **THEN** Forge displays available evidence and disables mutation controls with the precise capability reason

### Requirement: Plan before project writes

Forge SHALL return a side-effect-free plan or diff before every browser-triggered project file mutation and SHALL bind confirmation to the displayed plan.

#### Scenario: Plan reviewed and confirmed

- **WHEN** an operator confirms the unchanged plan for the selected project and operation
- **THEN** Forge applies the corresponding Core operation and returns a journaled operation identity

#### Scenario: Plan changed or confirmation missing

- **WHEN** project state changes after planning or the confirmation is absent/invalid
- **THEN** Forge refuses the write, records no successful mutation and returns a refreshed plan or safe error

### Requirement: Isolated and safe project execution

Forge SHALL authorize each typed workflow against the global session and resolved managed project and SHALL never execute arbitrary shell text received from the browser.

#### Scenario: Cross-project path attempt

- **WHEN** a request attempts to address files outside the selected registered project root
- **THEN** Forge refuses the request and leaves project files and operation journal unchanged

#### Scenario: CLI-only operation

- **WHEN** a command requires interactive terminal input or lacks a safe typed Core contract
- **THEN** the browser explains that it is CLI-only and does not offer a nonfunctional control

### Requirement: Workflow outcome and partial failure visibility

Forge SHALL show validation, provider, stale-state and partial-operation outcomes without presenting failed or unavailable work as successful.

#### Scenario: Partial operation failure

- **WHEN** an accepted operation completes some stages and fails another
- **THEN** the browser shows operation identity, each stage result, recovery guidance and links to preserved evidence

#### Scenario: Stale evidence

- **WHEN** evidence is stale or project state no longer matches a plan
- **THEN** Forge labels its observation time and requires a new plan before mutation

### Requirement: Dedicated project Quality checks view

The authenticated project workbench SHALL present persisted Gate evidence in
a dedicated **Quality checks** card, separate from project Health. It SHALL
show the Gate aggregate, freshness, revision, observation time, summary and
every recorded check state/note, using the existing status API and existing
explicit rerun action. The status and rerun response wrappers SHALL normalize
to the same evidence projection. The card SHALL state one accurate next step
for absent, fresh, stale, blocked, failed, unknown and unavailable evidence.
It SHALL never infer `passed` from missing, stale, malformed or unavailable
evidence, add a Gate control, execute Gate on page load, or display a provider
path/runtime implementation detail as part of normal project quality.

#### Scenario: Recorded checks and current passing evidence

- **WHEN** the operator opens a managed project with a `passed` Gate result
  bound to the current revision
- **THEN** the Quality checks card shows `Passed`, `Fresh`, the observation
  time and revision, every check and note, and says the result applies to the
  current revision

#### Scenario: Gate status API response is wrapped

- **WHEN** `GET /v1/admin/projects/{id}/gate/status` returns the existing
  `{summary, envelope}` response
- **THEN** the workbench renders the inner Gate evidence and does not replace
  it with an unverified empty state

#### Scenario: Gate run response uses the same renderer

- **WHEN** the operator activates the existing Re-run gate action and the
  route returns a Gate envelope
- **THEN** the Quality card renders the same aggregate, freshness and checks
  model as a read, and the action outcome is announced

#### Scenario: No Gate evidence exists

- **WHEN** a managed project has no persisted Gate record
- **THEN** the card says `Not run` or `Unverified`, explains that this is not
  a pass, and directs the operator to the existing Re-run gate action

#### Scenario: Evidence is stale

- **WHEN** persisted Gate evidence belongs to an earlier revision
- **THEN** the card labels it `Stale`, retains its recorded aggregate and
  checks, and directs the operator to rerun before relying on current evidence

#### Scenario: A check blocks or fails

- **WHEN** Gate evidence reports `blocked` or `failed`
- **THEN** the card preserves that result and its check rows and directs the
  operator to review the failures, fix the project, and rerun Gate

#### Scenario: Gate result is unknown or unavailable

- **WHEN** the evidence cannot be classified or the status route cannot be
  read
- **THEN** the card shows `Unknown` or `Unavailable` with safe recovery text
  and never presents a pass

#### Scenario: No project is selected

- **WHEN** the workbench has no selected managed project
- **THEN** the card explains that a project must be opened before Gate status
  can be read and offers no project-specific result

### Requirement: Select and identify Gate profiles in the Quality checks card

The authenticated project workbench SHALL display the profile recorded by local
Gate evidence and hosted CI evidence whenever that profile is present. It SHALL
provide an explicitly labelled `fast` or `full` selector for the existing Gate
rerun action, default it to `full`, and explain the scope of each profile. The
browser SHALL send the selected profile as a typed value to the existing Gate run
route. The API SHALL validate the request before execution, SHALL preserve the
existing `full` behavior when a legacy caller omits `profile`, and SHALL reject
malformed or unsupported profile values without executing Gate or changing
evidence. Unrecognized profile values SHALL be labelled unavailable. Historical
local evidence that omits the profile field SHALL retain the existing evidence
contract's `full` deserialization default. Existing status, authorization,
freshness, result, error and hosted evidence rules SHALL remain in force.

#### Scenario: Run the selected fast profile

- **WHEN** an authenticated operator selects `fast` and activates the existing
  Gate rerun action
- **THEN** the browser sends `profile: fast`, the API invokes Forge Gate with the
  fast profile, and the resulting persisted evidence and workbench show `fast`
  with the actual aggregate and check rows

#### Scenario: Run the selected full profile

- **WHEN** an authenticated operator selects `full` and activates the existing
  Gate rerun action
- **THEN** the browser sends `profile: full`, the API invokes Forge Gate with the
  full profile, and the resulting persisted evidence and workbench show `full`
  with the actual aggregate and check rows

#### Scenario: Legacy run caller omits profile

- **WHEN** an authorized client posts a valid legacy empty JSON object to the
  existing Gate run route
- **THEN** Forge runs the full profile and returns evidence identifying `full`

#### Scenario: Invalid profile request

- **WHEN** a client posts invalid JSON, an invalid request shape, or a profile
  other than `fast` or `full`
- **THEN** the API returns a typed client error before running Gate, and the
  prior persisted evidence and journal remain unchanged

#### Scenario: Render local and hosted profile facts

- **WHEN** local status or a hosted CI observation contains a Gate profile
- **THEN** the Quality checks card labels that evidence with its profile; an
  unrecognized value is shown as `Profile unavailable`

#### Scenario: Render legacy local evidence

- **WHEN** historical local evidence omits the profile field
- **THEN** the existing evidence deserializer treats it as `full`, and the
  workbench displays `full` consistently with the legacy runtime behavior

#### Scenario: Rerun pending or fails

- **WHEN** the operator submits a selected profile and the run is pending or
  returns an error
- **THEN** the control reports progress or the safe error, and on failure keeps
  the previous evidence and its recorded profile visible

