## MODIFIED Requirements

### Requirement: Observed health and recovery

Forge SHALL distinguish pending, running, failed and unknown deployment states using timestamped health evidence and expose recovery information for failed rollouts. The deploy report SHALL carry a verdict that names what the run did, so a run that deployed nothing is reported and journalled as a plan and never as a completed deployment, and the report's health SHALL be true only for a delivery.

#### Scenario: Observed health and recovery success

- **WHEN** a deployed artifact passes its health checks
- **THEN** the registry records running with artifact identity and observation time

#### Scenario: Observed health and recovery failure

- **WHEN** health checks fail after deployment
- **THEN** the operation reports failure and the supported recovery path without declaring success

#### Scenario: Observed health and recovery boundary

- **WHEN** the target becomes unreachable after a previous success
- **THEN** the last observation is retained as stale and current state is unknown

#### Scenario: A rehearsal is never a completed deployment

- **WHEN** a deploy apply runs as a dry run and no deploy is attempted
- **THEN** the report's verdict names the run a plan, the report is not healthy, the note says nothing was deployed, and the journal records a planned deploy
