# workspace-decision-register Specification

## ADDED Requirements

### Requirement: A fail-closed decision register SHALL exist (REG-001)

Workspace Governance SHALL maintain `decisions.json` (version 1) and a
stdlib loader that raises a `DecisionError` naming the file, entry index,
and field on any malformed, duplicate, or unknown-top-level-key register,
and SHALL never substitute an empty decision list for a broken one.

#### Scenario: Valid or empty register loads

- **WHEN** the loader reads a conforming register, including one with an
  empty `decisions` array
- **THEN** it returns the parsed decisions without error

#### Scenario: Malformed register refuses consumers

- **WHEN** the register contains a duplicate id, an invalid status, a bad
  date, or an unknown top-level key
- **THEN** loading raises, and `workspace_check.py --decisions` exits
  non-zero naming the offender

#### Scenario: Missing register in governance self-check

- **WHEN** `workspace_check.py --self-check` runs while `decisions.json` is
  absent
- **THEN** it reports the register-missing error code and fails

### Requirement: Entries SHALL cross-reference the project registry (REG-002)

Every `owner_project` and every `downstream[].repo` value MUST exist as an
id in `projects.json`, and every `downstream[].change` MUST be kebab-case;
violations are loader errors listing all offenders.

#### Scenario: Known references pass

- **WHEN** a seeded accepted decision names registered repos and kebab-case
  change names
- **THEN** validation against the registry returns no offenders

#### Scenario: Unknown repository flagged

- **WHEN** an entry references a repo id absent from the registry
- **THEN** validation raises naming that id

### Requirement: Decision lifecycle SHALL forbid id reuse and require supersede entries (REG-003)

Status values MUST be one of `proposed`, `accepted`, `rejected`,
`superseded`; changing an accepted decision requires a NEW entry whose
predecessor is marked `superseded`; ids are permanent. The register's first
content SHALL be the four accepted toolchain decisions (D1–D4), and the D2
entry SHALL restate that the public GitHub Pages catalog remains excluded
from shared login.

#### Scenario: Supersede transition is representable

- **WHEN** a new entry with a fresh id supersedes an accepted one and the
  predecessor's status becomes `superseded` with its text retained
- **THEN** the register still loads with unique ids and no mutation of the
  old id

#### Scenario: Public-site exclusion is machine-checkable

- **WHEN** the seeded D2 entry is loaded
- **THEN** its `policy_boundary` field names the public-catalog exclusion
  (asserted by test string)

### Requirement: Checks and human view SHALL stay in sync (REG-004)

`workspace_check.py` SHALL expose a `--decisions` summary, the portfolio
audit SHALL emit exactly one absent-register warning (never per-project
errors), and `docs/toolchain-decisions.md` SHALL present the same four
decisions; `docs/capabilities.md` SHALL record that capability entry schemas
are owned by platform-contracts and registered instances by this
repository.

#### Scenario: Summary surfaces every entry

- **WHEN** `workspace_check.py --decisions --json` runs
- **THEN** output lists each decision's id, title, status, and date

#### Scenario: Portfolio audit tolerates adoption

- **WHEN** `workspace_check.py --root .. --audit` runs on the whole
  workspace with the register present
- **THEN** no per-project finding attributable to the register is raised
