## MODIFIED Requirements

### Requirement: Declared gate runtime execution

Forge SHALL execute the gate engine bundled in the Forge workspace through the public `forge gate` command. The bundled engine SHALL always run the mandatory Forge baseline plus any valid project-declared extensions. Forge SHALL preserve each structured check state, source, severity, bounded diagnostic, top-level aggregate, and exit behavior in its revision-bound evidence. Forge SHALL NOT resolve or spawn an external gate binary; legacy runtime declarations and `FORGE_GATE_BIN` SHALL NOT change the Forge-owned execution path.

#### Scenario: Passing gate

- **WHEN** the Forge baseline and every blocking project extension pass for the captured revision
- **THEN** Forge persists fresh evidence with runtime `forge`, records the journal outcome as done, and exits zero

#### Scenario: Blocked gate

- **WHEN** the baseline or a required project extension reports a blocking failure
- **THEN** Forge persists the per-check blocked evidence, records the journal outcome as blocked, and exits nonzero

#### Scenario: No runtime resolvable

- **WHEN** no DriftWatchdog executable or sibling checkout exists
- **THEN** `forge gate` runs the bundled engine and does not classify the project as unavailable solely because an external runtime is missing

#### Scenario: Invalid project gate manifest

- **WHEN** the selected project extension manifest is malformed or violates the supported schema
- **THEN** Forge refuses before executing checks, leaves prior evidence and journal history unchanged, and reports a typed configuration error
