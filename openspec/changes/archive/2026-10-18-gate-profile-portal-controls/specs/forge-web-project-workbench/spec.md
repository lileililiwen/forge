# forge-web-project-workbench Specification (delta: gate-profile-portal-controls)

## ADDED Requirements

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
