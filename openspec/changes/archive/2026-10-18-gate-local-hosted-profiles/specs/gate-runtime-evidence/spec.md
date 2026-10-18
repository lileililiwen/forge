# gate-runtime-evidence Specification

## MODIFIED Requirements

### Requirement: Declared gate runtime execution

Forge SHALL execute the gate engine bundled in the Forge workspace through the public `forge gate` command. The engine SHALL support `fast` and `full` profiles. Both profiles SHALL always run the mandatory Forge baseline; `fast` SHALL omit project-declared extensions and `full` SHALL run all valid enabled project-declared extensions. An omitted profile SHALL mean `full`. New evidence SHALL use contract `0.2.0` and preserve the selected profile and each structured check state, source, severity, bounded diagnostic, top-level aggregate, and exit behavior. Valid legacy `0.1.0` evidence SHALL deserialize as `full`, because legacy runs always selected the configured full plan. Forge SHALL NOT resolve or spawn an external gate binary; legacy runtime declarations and `FORGE_GATE_BIN` SHALL NOT change the Forge-owned execution path.

#### Scenario: Passing gate

- **WHEN** the Forge baseline and every blocking project extension pass for the captured revision under `full` or the default profile
- **THEN** Forge persists fresh full-profile evidence with runtime `forge`, records the journal outcome as done, and exits zero

#### Scenario: Fast gate preserves baseline

- **WHEN** a project runs `forge gate --profile fast`
- **THEN** Forge runs the mandatory baseline, omits project extensions, records fast-profile evidence, and exits zero only when the baseline passes

#### Scenario: Blocked gate

- **WHEN** the baseline fails, or a required project extension fails under `full`
- **THEN** Forge persists the per-check blocked evidence, records the journal outcome as blocked, and exits nonzero

#### Scenario: No runtime resolvable

- **WHEN** no DriftWatchdog executable or sibling checkout exists
- **THEN** `forge gate` runs the bundled engine and does not classify the project as unavailable solely because an external runtime is missing

#### Scenario: Invalid project gate manifest

- **WHEN** the selected project extension manifest is malformed or violates the supported schema, including when the caller selects `fast`
- **THEN** Forge refuses before executing checks, leaves prior evidence and journal history unchanged, and reports a typed configuration error

### Requirement: Revision-bound evidence lifecycle

Forge SHALL bind each gate evidence record to the captured source revision and SHALL treat evidence as stale, never as current verification, once the working tree revision has moved. Evidence written before profile metadata existed SHALL deserialize as `full`, because the legacy runtime always ran the configured full plan.

#### Scenario: Stale evidence

- **WHEN** persisted evidence names a revision different from the current HEAD
- **THEN** doctor's gate finding warns as stale and readiness claims cannot cite it

#### Scenario: Never-run gate

- **WHEN** no evidence record exists
- **THEN** Forge reports the gate as unverified rather than passing it

#### Scenario: Legacy evidence profile

- **WHEN** valid historical evidence has no profile field
- **THEN** Forge reads it as full-profile evidence without rewriting the persisted file
