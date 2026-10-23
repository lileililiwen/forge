# upgrade-fleet-verdict-truth Specification

## Purpose
One fleet verdict for an upgrade fleet run, derived from the per-project
outcomes and published by every surface that reports the fleet: a fleet is
healthy only when every selected project was judged, a project that could
not be judged is `unavailable` rather than healthy, and the human renderer,
the JSON document and the exit code all read that one verdict.
## Requirements
### Requirement: A fleet verdict names what the run established

Forge SHALL derive one fleet verdict from the per-project outcomes of an
upgrade fleet run, and SHALL publish that verdict in the human
renderer, in the JSON document and in the command's exit code. The
verdict SHALL be `healthy` only when every selected project was judged
and none failed or blocked. A project that could not be judged SHALL
make the verdict `unavailable`, never `healthy`. A project that failed
or blocked SHALL make the verdict `not healthy`.

#### Scenario: A fleet whose projects were all judged is healthy

- **WHEN** every selected project was upgraded, failed, blocked, or
  found already satisfied, and none failed or blocked
- **THEN** the verdict is `healthy` and the command exits 0

#### Scenario: A project that could not be judged is never healthy

- **WHEN** a selected project's registry record was unreadable, or its
  path was unavailable
- **THEN** that project's result is `unavailable`, the fleet's verdict
  is `unavailable`, and the command exits 1

#### Scenario: The verdict is published in the JSON document

- **WHEN** the fleet report is rendered as JSON
- **THEN** the document carries the verdict, so a reader can tell a
  judged fleet from an unjudged one without counting the outcomes

#### Scenario: A failure outranks an unknown

- **WHEN** one project failed and another could not be judged
- **THEN** the verdict is `not healthy`
