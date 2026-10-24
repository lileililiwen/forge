# mirror-plan-verdict-truth Specification

## Purpose
The mirror report names what the run did: one rule judges every mirror
run, a plan is never reported as a delivery, and the registry journal
records a plan as a planned run.
## Requirements
### Requirement: The mirror report names what the run did

Forge SHALL judge every mirror run with one rule, in the one place the
report is built: a run that attempted no write is a plan, a run that
attempted a write and reached every enabled remote is a delivery, and a
run that attempted a write and did not reach every enabled remote is
partial. The report SHALL carry that verdict, and SHALL report itself as
healthy only when the verdict is a delivery. A plan SHALL NOT be reported
as a delivery, and the registry journal SHALL record a plan as a planned
run rather than a completed one. A plan is not a failure: the command
SHALL exit 0 for a plan.

#### Scenario: A plan is not a delivery

- **WHEN** a mirror run is planned without attempting a write
- **THEN** the report's verdict is `planned`, the report is not healthy, and the journal records a planned run

#### Scenario: A delivery is healthy

- **WHEN** a mirror run reaches the requested ref on every enabled remote
- **THEN** the report's verdict is `delivered`, the report is healthy, and the journal records a completed run

#### Scenario: A partial delivery is not a delivery

- **WHEN** a mirror run attempts a write and an enabled remote does not reach the requested ref
- **THEN** the report's verdict is `partial`, the report is not healthy, and the command exits with the distribution failure

#### Scenario: The plan and the apply say the same thing about the provider

- **WHEN** a mirror is declared on a provider that is planned but not supported, and the run is planned and then applied
- **THEN** both reports record the provider as unavailable, and neither report claims a delivery

#### Scenario: One verdict, every surface

- **WHEN** the mirror report is rendered for a human, rendered as JSON, or used as the journal detail of the run
- **THEN** all three read the same verdict
