## ADDED Requirements

### Requirement: The in-tree policy runtime is resolved from the source checkout

Forge SHALL resolve the policy runtime from the built Cargo workspace member
when it runs from a source checkout, after an explicit override and `PATH`.
Failure to find any runtime SHALL remain an honest unavailable outcome.

#### Scenario: A source checkout carries the built runtime

- **WHEN** `target/debug/driftwatchdog` (or the release profile) exists under
  the checkout that built Forge
- **THEN** the policy adapter runs that executable and the doctor reports the
  policy plane from its report rather than as unavailable

#### Scenario: An override is set

- **WHEN** `FORGE_DRIFTWATCH_BIN` names a path
- **THEN** that path is used verbatim and the checkout fallback is not consulted

#### Scenario: No runtime exists

- **WHEN** neither the override, `PATH` nor the checkout carries an executable
- **THEN** the outcome is unavailable and the reason names every attempted
  location

#### Scenario: An installed binary

- **WHEN** Forge runs from a release archive that does not ship the policy
  runtime and no runtime is installed
- **THEN** the policy plane remains unavailable; the checkout fallback does not
  manufacture one

### Requirement: The policy plane runs configured checkers, not the Gate

The policy adapter SHALL invoke the checker composition
(`check --dry-run --format json`) for every project. It SHALL NOT run the
project's Gate as a policy probe, because the Gate has its own evidence path
and its own verb.

#### Scenario: A gate-managed project

- **WHEN** the project carries `gate.toml` or `.ai-gate/gate.yaml`
- **THEN** the policy adapter still runs the checker composition, and the
  Gate's own evidence is read by the Gate finding

#### Scenario: The plane reports no configured checkers

- **WHEN** the checker composition succeeds with zero checkers
- **THEN** the policy finding is a warning that names the empty configuration,
  not an unavailable result and not a pass

#### Scenario: A configured rule does not apply

- **WHEN** at least one checker is configured and no rule applies to the profile
- **THEN** the result remains unavailable, because nothing was judged
