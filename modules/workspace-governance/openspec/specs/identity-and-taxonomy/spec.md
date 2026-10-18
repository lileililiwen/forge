# identity-and-taxonomy Specification

## Purpose
Defines the independent project role and implementation stack axes, their derived legacy kind, and the requirement that registry entries and declarations agree without changing profile-driven governance requirements.
## Requirements
### Requirement: A project is classified by two independent axes

Workspace Governance SHALL describe every project with two independent
enumerated values: a `role` drawn from `product`, `platform`, `tooling`,
`governance`, `deployment`, `inventory`, and a `stack` drawn from `dotnet`,
`rust`, `typescript`, `python`, `flutter`, `none`. Both sets SHALL be
declared once in `vocabulary.json` and enumerated in the project schema. A
value outside its set, or an absent value, SHALL be reported as WARNING
`TAXONOMY_AXIS_UNKNOWN` naming the offending value and both canonical sets.
A missing, unparseable, duplicate-bearing, or empty `roles` or `stacks`
vocabulary SHALL make a consumer exit non-zero with the reason and SHALL NOT
fall back to an empty set.

#### Scenario: Both axes are canonical

- **WHEN** a registry entry and a declaration both carry
  `role: platform` and `stack: rust`
- **THEN** the audit reports no axis finding for that project

#### Scenario: A value outside its set

- **WHEN** a declaration carries `role: library` and `library` is not in the
  canonical role set
- **THEN** the audit reports WARNING `TAXONOMY_AXIS_UNKNOWN` naming the
  value and both canonical sets, and the declaration remains schema-valid

#### Scenario: An axis is absent

- **WHEN** a declaration carries `role` but no `stack`
- **THEN** the audit reports WARNING `TAXONOMY_AXIS_UNKNOWN` for the missing
  `stack` and no other identity finding for that project

#### Scenario: Every role and stack combination is legal

- **WHEN** a project declares `role: platform` with `stack: rust`
- **THEN** the audit reports no finding, because no combination of the two
  sets is invalid

#### Scenario: The vocabulary cannot be empty

- **WHEN** `vocabulary.json` declares an empty `roles` array
- **THEN** the consumer exits non-zero naming the file and the empty array,
  and does not evaluate any project against an empty set

### Requirement: The legacy kind is a derived composite

A project's `kind` SHALL be treated as a deprecated composite derived from
its two axes, never as an independently authored value: the derived value
SHALL be `<stack>-<role>` when `stack` is not `none` and `<role>` when it
is. A stack-qualified composite still present while both axes are declared
SHALL be reported as WARNING `TAXONOMY_LEGACY_VALUE`, as SHALL a `kind` that
disagrees with the value derived from its own axes. The derivation SHALL be
lossless: `role: product` with `stack: dotnet` SHALL derive
`dotnet-product`, and `dotnet-product` SHALL decompose back to the same pair.

#### Scenario: Derived value matches the declaration

- **WHEN** a declaration carries `role: product`, `stack: dotnet`, and
  `kind: dotnet-product`
- **THEN** the audit reports no legacy finding for that project

#### Scenario: Composite still present without the axes

- **WHEN** a declaration carries only `kind: dotnet-product` and neither
  axis
- **THEN** the audit reports WARNING `TAXONOMY_AXIS_UNKNOWN` for both absent
  axes and does not report `TAXONOMY_LEGACY_VALUE`, because nothing has been
  derived yet

#### Scenario: Hand-authored kind drifts from its own axes

- **WHEN** a declaration carries `role: platform`, `stack: rust`, and
  `kind: rust-product`
- **THEN** the audit reports WARNING `TAXONOMY_LEGACY_VALUE` naming the
  derived value `rust-platform`

#### Scenario: Stack none derives the bare role

- **WHEN** a declaration carries `role: governance` and `stack: none`
- **THEN** the derived `kind` is `governance` and no legacy finding is
  reported

### Requirement: Registry and declaration MUST agree on both axes

A registry entry and the declaration it governs SHALL agree on `role` and on
`stack`. A disagreement on either axis SHALL be reported as ERROR
`TAXONOMY_MISMATCH` naming both values and both project keys. An axis
present in only one of the two surfaces SHALL NOT be reported as a
mismatch; it SHALL be reported as `TAXONOMY_AXIS_UNKNOWN` on the surface
that omits it. The axes SHALL NOT drive required files, CI, verification
commands, or any promotion, deployment, or capability evidence state.

#### Scenario: Both surfaces agree

- **WHEN** a registry entry and its declaration both carry
  `role: tooling` and `stack: rust`
- **THEN** the audit reports no taxonomy finding for that project

#### Scenario: The surfaces disagree

- **WHEN** a registry entry carries `profile: rust-product` and its
  declaration resolves to `role: tooling`, `stack: rust`
- **THEN** the audit reports ERROR `TAXONOMY_MISMATCH` naming both values and
  the project id

#### Scenario: The axes are declared on one side only

- **WHEN** a registry entry carries `role` and `stack` and its declaration
  carries neither
- **THEN** the audit reports WARNING `TAXONOMY_AXIS_UNKNOWN` for the
  declaration and does not report `TAXONOMY_MISMATCH`

#### Scenario: Classification is not a requirements driver

- **WHEN** a project's `role` and `stack` are both canonical and agree with
  its registry entry
- **THEN** the project's required files, CI requirement, OpenSpec
  requirement, and verification-command requirement are identical to the
  values produced before the axes existed

### Requirement: The portfolio classification is resolved

Every declaration and every registry entry in the workspace SHALL carry a
canonical `role` and `stack`. The disagreement that exists at authoring time
— 30 of 77 comparable entries where the registry `profile` differs from the
declaration `kind` — SHALL be resolved by a recorded per-repository
decision. An active declaration with no registry entry SHALL be registered,
and a registry entry whose `path` does not resolve SHALL be removed or
archived, using the existing findings `DISCOVERED_UNREGISTERED` and
`PROJECT_MISSING` rather than new codes. No audit-time inference from the
file tree SHALL produce or correct an axis.

#### Scenario: The disagreement set is empty

- **WHEN** every registry entry is compared against its declaration
- **THEN** no repository reports `TAXONOMY_MISMATCH` and no repository
  remains in the resolution table

#### Scenario: An unregistered active repository is registered

- **WHEN** a directory named `devloom` holds a valid declaration, a real
  source tree, and no registry entry
- **THEN** the audit first reports ERROR `DISCOVERED_UNREGISTERED`, and after
  a registry entry carrying `role: product` and `stack: dotnet` is added it
  reports no registry finding for that project

#### Scenario: A ghost entry is removed

- **WHEN** a registry entry carries `lifecycle: active`,
  `adoption: adopted`, and a `path` that does not resolve on disk
- **THEN** the audit reports ERROR `PROJECT_MISSING` using the existing code,
  and the remediation removes or archives the entry rather than adding a
  second code for the same condition

#### Scenario: The tree never decides the axes

- **WHEN** a repository's declared `stack` differs from a bounded census of
  its source extensions
- **THEN** the audit reports no finding, because a census is a migration aid
  and the declared value is the authority

#### Scenario: A decided row amended after the fleet moved

- **WHEN** the declaration and the registry both carry a corrected `stack`, and
  the decided row still records the superseded value
- **THEN** `axis_migration.py --check` refuses and names the row's rule, because
  the decided row is the authority the fleet is checked against
- **AND** the check is clean only once the decided row records the corrected
  `stack`, and the committed plan row agrees with it

#### Scenario: A stack of none accepts the bare derived role

- **WHEN** a decided row carries `stack: none` and the declaration carries the
  derived bare `role` that `stack: none` produces
- **THEN** the drift guard accepts it, and re-running the check does not report
  the tool's own output as drift
- **AND** a `kind` that is neither the decided value nor any value the row's own
  `role` and `stack` derive is still refused
