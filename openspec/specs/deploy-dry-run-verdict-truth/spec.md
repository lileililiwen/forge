# deploy-dry-run-verdict-truth Specification

## Purpose
The deploy report names what the run did: one rule in the one place the report is built turns a rehearsal into a plan, so a dry run is never reported or journalled as a completed deployment, the report's health is true only for a delivery, and the note, the journal row and the exit code all follow the one verdict.
## Requirements
### Requirement: The deploy report names what the run did

Forge SHALL judge every deploy apply with one rule, in the one place
the report is built: a run that deployed nothing planned, a run that
deployed and completed every stage delivered, and a run whose stage
did not complete failed. The report SHALL carry that verdict, SHALL
report itself as healthy only when the verdict is a delivery, and the
note it writes SHALL say what the run did rather than that a deploy
happened. Every registry journal row for a deploy SHALL follow the one
rule, and the command's exit code SHALL follow the verdict, so a
rehearsal exits 0.

#### Scenario: A rehearsal is a plan

- **WHEN** a deploy apply runs as a dry run
- **THEN** the report's verdict is `planned`, the report is not healthy, the note says the stages were rehearsed and that nothing was deployed, and the journal records a planned deploy

#### Scenario: A delivery is healthy

- **WHEN** a deploy apply runs and every stage completes
- **THEN** the report's verdict is `delivered`, the report is healthy, and the journal records a completed deploy

#### Scenario: A failed stage is a failure

- **WHEN** a deploy apply runs and a stage does not complete
- **THEN** the report's verdict is `failed`, the report is not healthy, the journal records a failed deploy, and the command exits 1 with the report on stdout

#### Scenario: A rehearsal is not a failed command

- **WHEN** a deploy apply runs as a dry run
- **THEN** the command exits 0

#### Scenario: One rule, every writer

- **WHEN** the CLI and the API report a deploy apply
- **THEN** both read the report's verdict, and the journal state, the journal detail and the note follow it
