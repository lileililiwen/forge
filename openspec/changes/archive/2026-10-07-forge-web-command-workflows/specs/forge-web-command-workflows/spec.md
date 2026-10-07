# forge-web-command-workflows (delta)

## ADDED Requirements

### Requirement: Task-ordered dashboard

Forge SHALL order the signed-in dashboard around operator work: fleet
summary, project workbench, project management (creation plus workspace
onboarding), portfolio, delivery, and the command catalog last as
reference. The sidebar navigation SHALL follow the same order. All existing
sections, controls, heading hierarchy and routes SHALL keep working
unchanged; only position and reference labelling change.

#### Scenario: Operator opens the dashboard

- **WHEN** a signed-in operator loads the dashboard
- **THEN** work sections (fleet, workbench, management) precede reference material (command catalog), with navigation in the same order

#### Scenario: Catalog remains complete reference

- **WHEN** the operator opens the command catalog section
- **THEN** every row, filter and state renders exactly as before, labelled as CLI reference

### Requirement: Live unonboarded signal with auto-discovery

Forge SHALL run workspace discovery automatically on dashboard load and
show an “N of M workspace directories are not yet onboarded” line under
the fleet, linking to the onboarding panel. Discovery failure or an
unconfigured root SHALL degrade to the manual Discover button and the
existing prerequisite notice with no invented count.

#### Scenario: Workspace has unmanaged siblings

- **WHEN** discovery succeeds with onboardable candidates
- **THEN** the fleet header shows the live count and the onboarding table is already populated

#### Scenario: Discovery cannot run

- **WHEN** discovery fails or the root is unconfigured
- **THEN** no count is shown and the manual path works exactly as before

### Requirement: One-confirm chunked bulk onboarding

Forge SHALL let one reviewed confirmation onboard a selection of any size
by previewing it in 25-item chunks, showing one combined plan with one
digest per chunk, and applying the chunks in order under the single
confirmation tick. Each chunk SHALL keep its own preview → digest →
confirm cycle against the unchanged route. A refused chunk SHALL stop the
run, keep completed chunk results visible, clear stored digests and prompt
re-preview — never inventing the remaining results.

#### Scenario: Large selection onboards in chunks

- **WHEN** the operator previews and confirms a 27-item selection
- **THEN** two chunk cycles run in order and the results total all 27 with per-item outcomes

#### Scenario: A later chunk is refused

- **WHEN** a chunk apply is refused after earlier chunks succeeded
- **THEN** the completed results stay visible and the operator is prompted to re-preview, with nothing invented
