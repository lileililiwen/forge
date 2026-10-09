# portal-web-ui (delta)

## ADDED Requirements

### Requirement: Idea entry in workbench rail step zero

The workbench lifecycle card SHALL render an idea entry block inside step 0 (Idea) naming the graduation preview/import CLI and a studio spec entry link, preserving `?project=` and `?step=` deep links, with no new endpoint or framework.

#### Scenario: Idea entry names graduation and studio

- **WHEN** the workbench loads a managed project
- **THEN** `#wb-idea-entry` names `forge graduation preview`, `forge graduation import --confirm`, and links to the studio spec entry (`forge studio spec` + `/workbench?project=<id>&step=spec`)

#### Scenario: Idea links preserve project and step

- **WHEN** the operator follows an idea entry link
- **THEN** the URL carries both `?project=<id>` and `&step=` and an unknown `&step=bogus` is still ignored without moving `aria-current`

### Requirement: Publish to maintain loop in delivery

A successful delivery publish SHALL render a Next-idea prompt with a workbench `&step=idea` link and a maintain refresh shortcut that refreshes the workbench maintain view, reusing the existing maintain refresh control.

#### Scenario: Publish success writes the Next idea prompt

- **WHEN** `Publish approved preview` succeeds
- **THEN** `#delivery-next-idea` names the Next idea prompt, links to `/workbench?project=<id>&step=idea`, and offers a maintain refresh shortcut firing `#wb-maintain-refresh`

#### Scenario: Publish failure writes no loop

- **WHEN** the publish is refused or fails
- **THEN** no Next-idea block renders and the existing error summary receives focus

### Requirement: Capability group filter and rail badge

The projects view SHALL offer a capability group filter and the workbench rail SHALL carry a capability badge, both as text (never color-only) with no new frontend dependency.

#### Scenario: Projects filter by cap group

- **WHEN** the operator picks a group in `#cap-filter`
- **THEN** the fleet table shows only projects in that `cap_group` and the count line names the active group

#### Scenario: Rail carries a cap badge

- **WHEN** the workbench loads a managed project
- **THEN** `#cap-badge` names the project's capability group in words with `role="status"`

### Requirement: Canonical flywheel demo script

The repository SHALL ship `docs/flywheel-demo.md` walking the `hookit` candidate through idea→scaffold→gate→publish→maintain with exact CLI strings and `?project=&step=` web URLs.

#### Scenario: Demo covers five steps with URLs

- **WHEN** the operator opens `docs/flywheel-demo.md`
- **THEN** five steps (idea, scaffold, gate, publish, maintain) each name the exact `forge …` command and a `/workbench?project=hookit&step=<key>` URL
