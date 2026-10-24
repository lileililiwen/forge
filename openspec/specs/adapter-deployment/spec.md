# adapter-deployment Specification

## Purpose
Simple deployment targets and observed runtime state. v0.1.0 of the contract supports Docker Compose, local-host, Jenkins, runtime and SSH adapters, and records timestamped health observations (running, failed, unknown) for every apply. Remote hosts are independent failure domains; disconnected means unknown, not offline proof. Recovery may require operator action. The deploy report carries a verdict that names what the run did, and the registry journal names a plan a plan and never a completed deployment.
## Requirements
### Requirement: Explicit adapter deployment

Forge SHALL implement deploy with project and target selection through versioned Docker Compose, local, Jenkins, runtime and SSH adapters using validated artifact and configuration inputs. A deploy target kind SHALL declare the target fields a target of that kind must state, and the configuration validator SHALL refuse a target that does not state them, naming the kind and the fields. A deploy target kind MAY name its own adapter binary, and the adapter for a target SHALL be resolved from the kind's own binary, then the shared binary, then the kind's default.

#### Scenario: Explicit adapter deployment success

- **WHEN** an authorized deployment selects a compatible target
- **THEN** the adapter applies the planned artifact and records the target receipt

#### Scenario: Explicit adapter deployment failure

- **WHEN** a target, credential reference or artifact is missing or incompatible
- **THEN** preflight fails before remote mutation

#### Scenario: Explicit adapter deployment boundary

- **WHEN** multiple targets exist and no default or explicit target is set
- **THEN** Forge requires selection rather than choosing an arbitrary server

#### Scenario: A target does not state what its kind needs

- **WHEN** a selected target omits a field that its deploy target kind
  declares as required
- **THEN** preflight fails before remote mutation, and the refusal names the
  kind and the missing fields

### Requirement: Observed health and recovery

Forge SHALL distinguish pending, running, failed and unknown deployment states using timestamped health evidence and expose recovery information for failed rollouts. The deploy report SHALL carry a verdict that names what the run did, so a run that deployed nothing is reported and journalled as a plan and never as a completed deployment, and the report's health SHALL be true only for a delivery. A deploy plan SHALL be journalled by the same rule: a plan the shared delivery rule allows is a planned run, a plan it blocks is a blocked run, and neither is a completed deployment.

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

#### Scenario: A plan is never a completed deployment

- **WHEN** a deploy plan is computed and no deploy is attempted
- **THEN** the journal records a planned deploy when the shared delivery rule allows the revision and a blocked deploy when it does not, and never a completed deployment

### Requirement: Versioned deploy executor contract

Forge SHALL define its deploy executor boundary as a versioned
`forge-deploy-executor/0.1.0` contract — fixed argv operations, a single
JSON envelope on stdout, and classification rules where a parseable
non-zero result is a failed stage while an unparseable or timed-out result
is unavailable with prior state preserved.

#### Scenario: Contract-conformant adapter

- **WHEN** an installed adapter emits a valid envelope for `apply`
- **THEN** Forge records the stage outcome exactly as the envelope names it
  and attributes the evidence to the adapter and its revision

#### Scenario: Contract-violating output

- **WHEN** an adapter emits non-JSON or an unknown contract version
- **THEN** the run is unavailable, the last good DeployState stands, and
  the refusal names the contract mismatch

### Requirement: Reference Jenkins adapter

Forge SHALL ship a tested reference adapter mapping its executor contract
onto the workspace's existing Jenkins deployment scripts (dry-run preview,
deploy, status) with an explicit health vocabulary where unrecognized
status reports `unknown` and never `healthy`.

#### Scenario: Dry-run rehearsal

- **WHEN** `forge provider run deploy --fixture` exercises the reference
  adapter with `--dry-run`
- **THEN** the row reports the sandbox fixture result without triggering
  any real deployment side effect

#### Scenario: Unrecognized status

- **WHEN** the underlying deployment reports a state outside the mapping
  table
- **THEN** observe returns `unknown` while the previous good observation
  remains the current_state history entry

#### Scenario: Production promotion is external

- **WHEN** the adapter is used against a real Jenkins host
- **THEN** success or failure is attributed to the provider run evidence,
  and no fixture-only claim is represented as production support
