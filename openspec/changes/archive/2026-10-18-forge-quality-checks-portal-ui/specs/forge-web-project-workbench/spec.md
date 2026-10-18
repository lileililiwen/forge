# forge-web-project-workbench Specification (delta: quality-checks-portal-ui)

## ADDED Requirements

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
