# deploy-target-ssh Specification

## Purpose
`requirement.md` §30 names an SSH server as an initial deployment target and
says deployment should be adapter-based. The `ssh` deploy target kind was
known but deliberately not wired, so the promised target did not exist, a
deploy target kind could not say which fields it needed, and one adapter
binary had to serve every kind. This capability serves the SSH host target
behind the shared extension contract: a kind declares the target fields it
requires and may name its own adapter binary, and a reference SSH adapter
ships the artifact to the named host with a staged copy and an atomic rename
and observes the host's health.
## ADDED Requirements

### Requirement: The SSH deploy target is a served kind

Forge SHALL serve the `ssh` deploy target kind as a wired kind of the shared
deploy target registry, so a manifest that declares an `ssh` target with the
fields that kind requires is accepted and the deploy reaches an SSH adapter
under the unchanged `forge-deploy-executor/0.1.0` contract. A kind that is
known but not wired SHALL be refused in one wording that every entry point
shares.

#### Scenario: An SSH target is served

- **WHEN** a manifest declares a target of kind `ssh` that states the fields
  the kind requires
- **THEN** the target is accepted, the deploy reaches the SSH adapter, and the
  stage evidence attributes the outcome to the adapter that served it

#### Scenario: A kind that is not wired is refused

- **WHEN** a manifest declares a target of a kind that the registry knows but
  does not wire
- **THEN** the target is refused, and the refusal names the kind and the kinds
  that are wired, in the same words in the manifest, the configuration and the
  plan

### Requirement: A deploy target kind declares the target fields it needs

Forge SHALL keep the target fields a deploy target kind requires in the shared
kind registry, next to the kind, and SHALL refuse a target that does not
state them, naming the kind and the fields it is missing. The requirement
SHALL be read from the registry and SHALL NOT be hard-coded in a validator.

#### Scenario: A target that states what its kind needs

- **WHEN** a manifest declares a target of a kind that requires a host and a
  remote path, and the target states both
- **THEN** the target is accepted and no field of it is required again

#### Scenario: A target that does not state what its kind needs

- **WHEN** a manifest declares a target of a kind that requires a host and a
  remote path, and the target omits one of them
- **THEN** the target is refused before any adapter runs, and the refusal
  names the kind and the fields it is missing

#### Scenario: The legacy target form cannot state a host

- **WHEN** a manifest declares a legacy `deployment.type` of `ssh`
- **THEN** the target is refused for the same missing fields, because the
  legacy form cannot state a host

### Requirement: A deploy target kind may name its own adapter

Forge SHALL resolve the adapter binary per deploy target kind, in one order:
the kind's own adapter variable when it is set, then the shared adapter
variable when it is set, then the kind's default adapter binary. The resolved
binary SHALL be the one the lane spawns, the one a spawn failure names and
the one the evidence attributes to when the adapter does not name itself. A
kind that names no adapter of its own SHALL resolve to the shared binary.

#### Scenario: A kind names its own adapter

- **WHEN** the kind's own adapter variable is set
- **THEN** the kind is served by that binary, and the other kinds keep the
  binary they resolve to

#### Scenario: One adapter serves every kind

- **WHEN** only the shared adapter variable is set
- **THEN** every kind, the SSH kind included, is served by that binary

#### Scenario: Nothing is set

- **WHEN** no adapter variable is set
- **THEN** each kind is served by its own default adapter binary

### Requirement: The reference SSH adapter ships the artifact and observes the host

Forge SHALL ship a reference SSH adapter that maps the executor contract onto
an SSH transfer. The adapter SHALL copy the declared artifact to the named
host under a staged name and rename it into place, so a half-copied artifact
is never visible on the host. The adapter SHALL run the declared health check
on the host, SHALL report a host that cannot be reached as `unknown` rather
than `failed`, SHALL use argument arrays and bounded waits, SHALL never place
a credential or an absolute host path in its evidence, and SHALL contact no
host on a dry run.

#### Scenario: The artifact is transferred

- **WHEN** the adapter applies an artifact to a reachable host
- **THEN** the artifact is copied under a staged name and renamed into place,
  and the stage evidence names the copy and the rename without naming the
  remote path

#### Scenario: The host reports its health

- **WHEN** the adapter observes a target whose health check succeeds
- **THEN** the observation is `running`, and a check that fails is `failed`

#### Scenario: The host cannot be reached

- **WHEN** the adapter cannot reach the host
- **THEN** the observation is `unknown`, and the last good `running`
  observation is retained

#### Scenario: A dry run contacts no host

- **WHEN** the adapter is asked for a dry run
- **THEN** no host is contacted, no file is copied, and the stage is reported
  as a rehearsal

#### Scenario: The adapter refuses what it cannot serve safely

- **WHEN** the adapter is invoked for a kind it does not serve, or with a
  declared host, user or remote path that holds a character it will not put
  on a command line
- **THEN** the adapter refuses before anything runs, names the reason, and
  contacts no host
