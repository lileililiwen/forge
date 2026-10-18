## MODIFIED Requirements

### Requirement: Canonical kinds are enforced

A declaration whose `kind` is neither a member of the legacy `kinds`
vocabulary nor the value derived from its own axes MUST be WARNING
`KIND_UNKNOWN` naming the value, and `kind` MUST NOT drive profile
requirements.

`kind` is a derived, deprecated composite whose authority is the pair `role`
and `stack`, and `kinds` is a legacy vocabulary rather than the authority on a
derived value. A `kind` therefore counts as canonical when it is a member of
`kinds`, or when it equals `derive_kind(role, stack)` with both axes present
and both canonical. The derived form MUST NOT be required to appear in
`kinds`: the derivation is the product of two enumerated axis sets and is not
enumerable as a maintained list. A value whose axes are absent, only partly
declared, or non-canonical MUST still be reported, because in that case
nothing has been derived and the value is not measurable. The level MUST
remain WARNING and the code MUST NOT be repurposed or suppressed.

#### Scenario: Non-canonical kind

- **WHEN** a declaration uses a one-off kind such as `quality-gate`
- **THEN** the audit reports WARNING `KIND_UNKNOWN` and the project's required
  files are unchanged

#### Scenario: Composite with the axes present

- **WHEN** a declaration carries `kind: rust-product` together with
  `role: platform` and `stack: rust`
- **THEN** the audit reports WARNING `TAXONOMY_LEGACY_VALUE` and does not
  report `KIND_UNKNOWN`, because `rust-product` is inside the legacy kind set

#### Scenario: A derived value that the legacy vocabulary does not list

- **WHEN** a declaration carries `role: platform`, `stack: rust`, and
  `kind: rust-platform`, and `rust-platform` is not a member of `kinds`
- **THEN** the audit reports no `KIND_UNKNOWN` for that project, because the
  value is the canonical form its own axes derive

#### Scenario: Every derived value is known without being listed

- **WHEN** a declaration's `kind` equals `derive_kind(role, stack)` for its
  own canonical axes, whether or not `kinds` contains that string
- **THEN** the audit reports no `KIND_UNKNOWN` for that project, and the
  legacy `kinds` vocabulary is left unchanged

#### Scenario: The kind set stays a projection

- **WHEN** the two axis sets are enumerated
- **THEN** every canonical kind value is either a role value or a
  `<stack>-<role>` pair from those sets, and no value needs to be added to
  the legacy kind set to express a new combination

#### Scenario: A hand-authored value that disagrees with its own derivation

- **WHEN** a declaration carries `role: governance`, `stack: python`, and
  `kind: rust-governance`
- **THEN** the audit reports WARNING `KIND_UNKNOWN` and WARNING
  `TAXONOMY_LEGACY_VALUE`, because the value is in neither the legacy
  vocabulary nor the derived form

#### Scenario: The axes are absent, so nothing is derived

- **WHEN** a declaration carries `kind: rust-platform` and neither `role` nor
  `stack`
- **THEN** the audit reports WARNING `KIND_UNKNOWN` naming the value, because
  no derivation exists to measure it against

#### Scenario: The axes are non-canonical, so no derivation is authoritative

- **WHEN** a declaration carries `role: library`, `stack: rust`, and
  `kind: rust-platform`, and `library` is not a canonical role
- **THEN** the audit reports WARNING `KIND_UNKNOWN` together with WARNING
  `TAXONOMY_AXIS_UNKNOWN`, and does not treat the value as a known
  derivation
