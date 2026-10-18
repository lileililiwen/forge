# portfolio-vocabulary Specification (delta: identity-and-taxonomy-axis-split)

## MODIFIED Requirements

### Requirement: One machine-readable vocabulary source

`vocabulary.json` MUST define the canonical `roles`, `stacks`, `profiles`,
`kinds`, `placeholder_markers` and `secret_field_substrings`. A missing,
unparseable, duplicate-bearing or empty-list vocabulary MUST make a consumer
exit non-zero with the reason and MUST NOT fall back to an empty list. The
`roles` and `stacks` arrays MUST each be non-empty and free of duplicates,
because an empty axis set would mark every repository in the workspace
unknown.

#### Scenario: Vocabulary loaded

- **WHEN** a consumer starts and `vocabulary.json` is valid
- **THEN** it uses the canonical sets without a local duplicate list

#### Scenario: Vocabulary unusable

- **WHEN** `vocabulary.json` is missing, unparseable, or lists an empty
  `secret_field_substrings` or an empty `roles`
- **THEN** the consumer exits non-zero and names the file and the problem

#### Scenario: Existing consumers see no diff

- **WHEN** `roles` and `stacks` are added to `vocabulary.json`
- **THEN** `profiles`, `kinds`, `placeholder_markers`,
  `secret_field_substrings`, `standalone_statuses` and
  `integration_required_states` are unchanged and every existing consumer
  resolves them exactly as before
### Requirement: Canonical kinds are enforced

A declaration whose `kind` is outside the canonical set MUST be WARNING
`KIND_UNKNOWN` naming the value, and `kind` MUST NOT drive profile
requirements. `kind` SHALL additionally be a derived composite: a
stack-qualified value present alongside two declared axes MUST be WARNING
`TAXONOMY_LEGACY_VALUE`, and a `kind` that disagrees with the value derived
from its own axes MUST be the same finding. The canonical kind set SHALL
remain the projection of the two axis sets, so no repository becomes
`KIND_UNKNOWN` merely because the axes were introduced.

#### Scenario: Non-canonical kind

- **WHEN** a declaration uses a one-off kind such as `quality-gate`
- **THEN** the audit reports WARNING `KIND_UNKNOWN` and the project's required
  files are unchanged

#### Scenario: Composite with the axes present

- **WHEN** a declaration carries `kind: rust-product` together with
  `role: platform` and `stack: rust`
- **THEN** the audit reports WARNING `TAXONOMY_LEGACY_VALUE` and does not
  report `KIND_UNKNOWN`, because `rust-product` is inside the canonical kind
  set

#### Scenario: The kind set stays a projection

- **WHEN** the two axis sets are enumerated
- **THEN** every canonical kind value is either a role value or a
  `<stack>-<role>` pair from those sets, and no value needs to be added to
  the kind set to express a new combination

### Requirement: The portfolio is normalized

Every registry entry and declaration MUST use a canonical `profile` and
`kind`, and MUST additionally use a canonical `role` and `stack`. The
portfolio MUST be free of a registry/declaration disagreement on either
axis, and each resolution made during the migration MUST be recorded with
the source it was derived from.

#### Scenario: Scan is clean

- **WHEN** the canonical sets are compared against every project
- **THEN** no non-canonical `kind` or `profile` value remains, no
  `TAXONOMY_MISMATCH` remains, and no unresolved row remains in the
  migration table

#### Scenario: Every resolution is traceable

- **WHEN** a row of the migration table is inspected
- **THEN** it names the repository, the resolved `role` and `stack`, and the
  rule or evidence the value was derived from
