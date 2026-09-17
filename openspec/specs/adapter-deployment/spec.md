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

