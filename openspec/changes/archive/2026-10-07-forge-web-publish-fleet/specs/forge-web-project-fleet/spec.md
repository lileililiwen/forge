# forge-web-project-fleet (delta)

## ADDED Requirements

### Requirement: Local publish history is projected into the fleet

Forge SHALL project the most recent `publish` operation per `project_id`
from the local registry `operations` table into the authenticated fleet as
a bounded, read-only source, so projects the operator has published but not
locally registered appear in the web project list.

#### Scenario: Published-only project appears

- **WHEN** the local journal has publish operations for a project id that is
  not in the local `projects` table
- **THEN** the fleet contains one row for that id with `source` `published`,
  `management` `observed`, no mutating capability, and the most recent
  publish state and bounded publish evidence

#### Scenario: Registered and published project stays managed

- **WHEN** a project id is both locally registered and present in the publish
  journal
- **THEN** the fleet contains a single row for that id that remains
  `management` `managed`, keeps its `inspect` capability, is not marked as a
  conflict, and carries the publish projection

#### Scenario: Most recent publish wins

- **WHEN** the journal holds several publish operations for one project id
- **THEN** the row reflects only the newest operation, ordered by `op_id`

#### Scenario: Empty publish journal

- **WHEN** the registry has no publish operations
- **THEN** the `published` source is reported available with a zero count and
  contributes no rows, and the other sources are unchanged

#### Scenario: Publish history disabled

- **WHEN** `FORGE_PUBLISH_HISTORY` is set to `0`, `false` or `off`
- **THEN** the `published` source is reported unconfigured with a safe reason
  and contributes no rows, and the other sources are unchanged

#### Scenario: Publish evidence is path-safe

- **WHEN** a projected publish `detail` contains an absolute local filesystem
  path
- **THEN** the serialized response replaces that path with a fixed marker and
  no absolute path, credential or adapter binary reaches the browser
