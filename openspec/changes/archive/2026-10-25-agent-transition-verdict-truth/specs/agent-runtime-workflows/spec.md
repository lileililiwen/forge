## MODIFIED Requirements

### Requirement: Truthful transition mapping

Forge SHALL expose only transitions the backing runtime genuinely supports;
unsupported requests SHALL report the operator's real control path (attach
guidance or the runtime's own pause primitive) and SHALL never claim a
state the runtime did not report. The journal row for a transition SHALL
record a refused or lost transition as the state the runtime reported and
SHALL NOT record it as a completed action.

#### Scenario: Unknown session handle

- **WHEN** the runtime reports no session for the stored handle
- **THEN** Forge surfaces disconnected/lost state, preserves the session
  file and transitions, and does not fabricate an active session

#### Scenario: Malformed runtime outcome

- **WHEN** a delegated run finishes but its outcome document cannot be
  parsed
- **THEN** the result is unverified, never done, and the raw failure is
  bounded and redacted

#### Scenario: A refused transition is journalled as refused

- **WHEN** a transition ends in a state the runtime reported as
  unsupported or disconnected
- **THEN** the journal row records that state, never a completed action

### Requirement: Independent verification for spec execution

When spec execution is delegated to an iteration supervisor, Forge SHALL
base its run-spec verdict on the supervisor's independent verification
outcome, not on agent completion claims. A run-spec with no supervisor
verdict SHALL be journalled as the state the runtime reported and SHALL
NOT carry a `done` verdict.

#### Scenario: Verified loop

- **WHEN** the supervisor exits clean with verification passed
- **THEN** the journaled run-spec verdict is done with the supervisor named
  as the evidence source

#### Scenario: Retryable failure

- **WHEN** the supervisor reports a retryable or blocked iteration
- **THEN** the verdict is partial with the supervisor's named reason and the
  spec binding intact

#### Scenario: A run-spec with no reported verdict

- **WHEN** a run-spec completes without a supervisor reporting a verdict
- **THEN** the journal row records the state the runtime reported and the
  detail names that no verdict was reported
