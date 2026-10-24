# analytics-inspect-verdict-truth Specification

## Purpose
The analytics inspection names what the inspection did: one rule in the one place the report is built reads the observations the inspection actually made and turns them into a verdict (`delivered`, `planned` or `failed`), so a dry run is never reported or journalled as a completed health check and the report's own availability agrees with its evidence.
## Requirements
### Requirement: The inspection report names what the inspection did

Forge SHALL judge every analytics inspection with one rule, in the one
place the report is built: an inspection that invoked no adapter
planned, an inspection whose providers were all healthy delivered, and
an inspection whose provider failed is failed. The report SHALL carry
that verdict, and SHALL report itself as healthy only when the verdict
is a delivery. A planned inspection SHALL NOT report a provider as
available, the report's availability SHALL be read from its own
observations, and every registry journal row for an inspection SHALL
follow the one rule rather than a private copy of it. A planned
inspection is not a failure: the command SHALL exit 0 for a plan.

#### Scenario: A plan is not a completed inspection

- **WHEN** an analytics inspection is run as a dry run
- **THEN** each observation's status is `planned`, the report's verdict is `planned`, the report is not healthy, and the journal records a planned inspection

#### Scenario: A delivery is healthy

- **WHEN** an analytics inspection invokes the adapter and the provider reports a healthy state
- **THEN** the observation's status is `available`, the report's verdict is `delivered`, the report is healthy, and the journal records a completed inspection

#### Scenario: A failed provider is not a delivery

- **WHEN** an analytics inspection invokes the adapter and the provider cannot be reached, fails, or is not supported
- **THEN** the observation's status is `unavailable`, the report's verdict is `failed`, the report is not healthy, and the journal records a failed inspection

#### Scenario: The report's availability agrees with its observations

- **WHEN** an analytics inspection reports its availability
- **THEN** the availability is true only when an observation reports an available provider or every provider is disabled, and it never contradicts the observations it carries

#### Scenario: One rule, every writer

- **WHEN** the analytics inspection writes its journal row
- **THEN** the journal state and the journal detail are read from the report's verdict
