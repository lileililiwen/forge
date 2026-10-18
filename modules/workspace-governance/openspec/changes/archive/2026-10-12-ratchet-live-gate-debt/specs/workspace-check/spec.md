# workspace-check Specification (delta: ratchet-live-gate-debt)

## ADDED Requirements

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
