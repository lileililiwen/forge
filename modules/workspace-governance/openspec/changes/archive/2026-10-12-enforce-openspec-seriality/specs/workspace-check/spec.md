## ADDED Requirements

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
