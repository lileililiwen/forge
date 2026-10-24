# publish-dry-run-verdict-truth Specification

## Purpose
The publish report names what the run did: one rule judges every
publish, a plan is never recorded as a completed publish, and every
journal writer for a publish reads that one rule.
## Requirements
### Requirement: The publish report names what the run did

Forge SHALL judge every publish run with one rule, in the one place the
publish report is built: a run that attempted no write is a plan, a run
whose stages all completed is a delivery, and a run whose stage did not
complete is failed. The report SHALL carry that verdict, and SHALL report
itself as healthy only when the verdict is a delivery. A plan SHALL NOT be
recorded as a completed publish, and every registry journal row for a
publish SHALL follow the one rule rather than a private copy of it. A plan
is not a failure: the command SHALL exit 0 for a plan.

#### Scenario: A plan is not a completed publish

- **WHEN** a publish run is planned without attempting a write
- **THEN** the report's verdict is `planned`, the report is not healthy, and the journal records a planned publish

#### Scenario: A delivery is healthy

- **WHEN** every stage of a publish run completes
- **THEN** the report's verdict is `delivered`, the report is healthy, and the journal records a completed publish

#### Scenario: A failed publish is not a delivery

- **WHEN** a stage of a publish run does not complete
- **THEN** the report's verdict is `failed`, the report is not healthy, and the command exits with the publish failure

#### Scenario: The fleet lane follows the same rule

- **WHEN** a fleet publish run is planned, or a fleet project's publish fails
- **THEN** the per-project state names the run a plan or a failure, and a planned fleet publish is not counted as a failure

#### Scenario: One rule, every journal writer

- **WHEN** the publish pipeline, the fleet lane and the fleet exec lane record a publish
- **THEN** all three read the report's verdict
