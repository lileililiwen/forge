# workspace-check Specification

## Purpose
Defines fail-closed checks for workspace configuration, including blocking new OpenSpec work when the live gate manifest contains unrecorded required-check debt.
## Requirements
### Requirement: New changes are blocked by unrecorded live gate debt

When a repository has an unarchived OpenSpec change, the checker SHALL compare the required concerns and commands in the current gate manifest with that repository's recorded `gate-debt.json` concerns. A live required concern with no command that is absent from the record SHALL block each non-grandfathered new change using `QUALITY_NEW_CHANGE_WITH_GATE_DEBT`, even if the snapshot reports the repository clean or omits it. If the current manifest cannot be read or its profile cannot be resolved, the checker SHALL fail closed for new work; a recorded per-change exception retains its explicit warning behavior. The checker SHALL NOT write the debt record or execute gate commands.

#### Scenario: A new unbound concern is added after a clean snapshot

- **WHEN** the debt record has no concern for a repository and its current manifest adds a required concern without a command while a new OpenSpec change is active
- **THEN** the checker emits an ERROR with `QUALITY_NEW_CHANGE_WITH_GATE_DEBT`
- **AND** the finding names the newly unbound concern and the active change

#### Scenario: A newly declared concern is bound

- **WHEN** a new required concern has a non-empty command in the current manifest
- **THEN** it is not counted as newly unbound debt
- **AND** it does not trigger this precondition on its own

#### Scenario: The current manifest cannot be resolved

- **WHEN** a new OpenSpec change is active and the gate manifest is missing, unreadable, malformed, or names an unsupported profile
- **THEN** the checker emits a blocking `QUALITY_NEW_CHANGE_WITH_GATE_DEBT` finding explaining that live gate state could not be established
- **AND** it does not treat the repository as debt-free

#### Scenario: Existing new-change exemptions remain effective

- **WHEN** an active change is a recorded baseline pair, an exception pair, or the designated debt-closing change
- **THEN** the live debt comparison preserves the existing grandfathering, warning, or exclusion behavior for that change

#### Scenario: The checker observes without mutating or executing

- **WHEN** the live debt comparison runs
- **THEN** it reads the current manifest and debt record only
- **AND** it does not write either file or execute any declared command

### Requirement: OpenSpec changes remain sequential

When auditing the `workspace-governance` project, the checker MUST allow at
most one active change directory under `openspec/changes/`. It MUST identify
active changes using the same rule as `unarchived_changes`: direct child
directories excluding `archive/` and dot-prefixed state directories. More
than one active change MUST emit ERROR `OPENSPEC_CHANGE_SERIALITY` naming all
active changes. With exactly one active change, `HANDOFF.md` MUST contain
exactly one line-anchored `current_spec:` pointer whose value matches that
change name. With no active changes, the pointer MAY be absent or blank or
`none`; a named or duplicate pointer MUST emit the same ERROR. The check MUST
emit the same ERROR if an existing `HANDOFF.md` cannot be read to establish
pointer state. It MUST NOT execute OpenSpec or any project command, write
files, or inspect sibling repositories.

#### Scenario: No active change has no stale pointer

- **WHEN** the governance repository has no active change and its pointer is
  absent or set to `none`
- **THEN** the checker emits no `OPENSPEC_CHANGE_SERIALITY` finding

#### Scenario: One active change matches the handoff pointer

- **WHEN** exactly one active change directory exists and the single
  `current_spec:` line names it
- **THEN** the checker emits no `OPENSPEC_CHANGE_SERIALITY` finding

#### Scenario: Multiple active changes block the Gate

- **WHEN** two or more active change directories exist
- **THEN** the checker emits ERROR `OPENSPEC_CHANGE_SERIALITY` naming each
  active change

#### Scenario: A single active change has a stale or ambiguous pointer

- **WHEN** exactly one active change exists and the handoff pointer is absent,
  duplicated, or names a different change
- **THEN** the checker emits ERROR `OPENSPEC_CHANGE_SERIALITY` with the
  expected change name and pointer problem

#### Scenario: A pointer to an archived change is stale

- **WHEN** no active change exists and `current_spec:` names an archived
  change
- **THEN** the checker emits ERROR `OPENSPEC_CHANGE_SERIALITY`

#### Scenario: Handoff pointer state cannot be read

- **WHEN** `HANDOFF.md` exists but cannot be decoded or read
- **THEN** the checker emits ERROR `OPENSPEC_CHANGE_SERIALITY` because pointer
  state cannot be established

#### Scenario: Other repositories are not constrained by this local rule

- **WHEN** the workspace audit evaluates a project whose id is not
  `workspace-governance`
- **THEN** this seriality rule does not inspect that project's OpenSpec state

#### Scenario: The check is read-only

- **WHEN** the seriality check evaluates active changes and the handoff pointer
- **THEN** it reads only those local paths, spawns no process, and changes no
  file
