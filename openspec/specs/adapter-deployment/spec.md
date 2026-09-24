# adapter-deployment Specification

## Purpose
Simple deployment targets and observed runtime state. v0.1.0 of the contract supports Docker Compose and local-host adapters, refuses SSH and other planned kinds, and records timestamped health observations (running, failed, unknown) for every apply. Remote hosts are independent failure domains; disconnected means unknown, not offline proof. Recovery may require operator action.
## Requirements
### Requirement: Explicit adapter deployment

Forge SHALL implement deploy with project and target selection through versioned Docker Compose and local/SSH adapters using validated artifact and configuration inputs.

#### Scenario: Explicit adapter deployment success

- **WHEN** an authorized deployment selects a compatible target
- **THEN** the adapter applies the planned artifact and records the target receipt

#### Scenario: Explicit adapter deployment failure

- **WHEN** a target, credential reference or artifact is missing or incompatible
- **THEN** preflight fails before remote mutation

#### Scenario: Explicit adapter deployment boundary

- **WHEN** multiple targets exist and no default or explicit target is set
- **THEN** Forge requires selection rather than choosing an arbitrary server

### Requirement: Observed health and recovery

Forge SHALL distinguish pending, running, failed and unknown deployment states using timestamped health evidence and expose recovery information for failed rollouts.

#### Scenario: Observed health and recovery success

- **WHEN** a deployed artifact passes its health checks
- **THEN** the registry records running with artifact identity and observation time

#### Scenario: Observed health and recovery failure

- **WHEN** health checks fail after deployment
- **THEN** the operation reports failure and the supported recovery path without declaring success

#### Scenario: Observed health and recovery boundary

- **WHEN** the target becomes unreachable after a previous success
- **THEN** the last observation is retained as stale and current state is unknown

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

