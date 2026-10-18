# portfolio-vocabulary Specification

## Purpose

Two vocabularies drifted without a source of truth. **Kind and profile.** The 77 declarations carry **22 distinct `kind` values and 17 distinct `profile` values**, several of them one-off prose (`cross-border-care-platform`, `privacy-sensitive-web-saas`, `web-platform`, `mobile-platform`, `open-source-knowledge-platform`, `runtime-operator`, `quality-gate`, `control-plane`). The checker admits profiles through an ad-hoc allow-list plus suffix rules (`workspace_check.validate_registry`), so a descriptive value silently passes or fails depending on spelling, and `profiles/*.json` describes only 6 of the 17 in use. **Shared word lists.** The placeholder markers and secret-field substrings that decide whether a check fires are declared in more than one place: `scripts/product_code_quality.py:DEFAULT_PLACEHOLDER_MARKERS`, `scripts/workspace_check.py:SECRET_WORDS`, `platform-contracts/schemas/registry.json:secret_field_substrings`, and `rust-platform-libs/crates/platform-testing/src/reasons.rs`. Driftwatchdog documents the same placeholder vocabulary for its `placeholder-threshold` concern without owning a copy. A word added in one place does not reach the others. Both are governance-owned vocabulary, so this is the same owner, lifecycle and oracle as the two shipped packages.
## Requirements
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

### Requirement: Canonical profiles are enforced

A registry entry whose `profile` is outside the canonical set MUST be ERROR
`PROFILE_UNKNOWN` naming the value, and the ad-hoc allow-list MUST be removed. A
`profiles/<name>.json` document naming a non-canonical profile MUST be ERROR
`PROFILE_DOC_UNKNOWN`.

#### Scenario: Canonical profile

- **WHEN** a registry entry uses a canonical profile
- **THEN** no profile finding is reported

#### Scenario: Non-canonical profile

- **WHEN** a registry entry declares a one-off profile such as `web-saas`
- **THEN** the audit reports ERROR `PROFILE_UNKNOWN` naming the value

#### Scenario: Profile document disagrees

- **WHEN** `profiles/` contains a document for a profile outside the set
- **THEN** the audit reports ERROR `PROFILE_DOC_UNKNOWN`

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

### Requirement: Shared word lists have one source

The secret scan MUST use `secret_field_substrings` from `vocabulary.json`, and
the product-code quality checker MUST default its placeholder markers from
`placeholder_markers`. A per-project `placeholder_markers` override MUST still
be honoured. The secret list MUST remain a superset of the previous terms.

**The marker vocabulary is restricted to unambiguous code-debt tokens.** A word
that is ordinary English MUST NOT be in `placeholder_markers`, because
`product_code_quality.py` matches case-insensitively on word boundaries across
every product line: prose that *describes* the word, a doc comment naming it,
and the gate's own `placeholder-threshold` concern identifier all match it. A
check that fires on the English language produces findings that are not debt,
and a check that cries wolf is ignored — which is strictly worse than a check
that does not run.

`placeholder` and `stub` were removed on 2026-10-05 for exactly this reason.
Measured over `driftwatchdog/src/`: the word `placeholder` matched **26** times
and **zero** of the 26 was unfinished work. Every one was either prose
describing the word (`//! ... extends this placeholder`, `/// Ordered,
comma-separated placeholder names`) or the concern identifier
`placeholder-threshold` in `src/gate/adapters.rs`, against 1 `TODO` and 1
`FIXME` on the same tree. The retained set is `todo`, `fixme`, `xxx`, `hack`,
`unimplemented!`, `todo!`.

**This reduces the finding count, and that is stated rather than hidden.**
Across the 39 repositories the derived `quality` block was written to, the
marker findings fall from **468 to 38**, and the repositories reporting
`FAIL` fall from **28 to 13**. 430 of the 468 were the two ordinary English
words. The number is reported because a vocabulary edit that only ever lowers a
count is indistinguishable from a filter; the difference is that this edit
corrects a mis-specified vocabulary in the one place that owns it, changes no
threshold, adds no `exclude`, skips no test, and leaves the finding machinery
exactly as it was.

**This is a correction, not a filter, and the distinction is load-bearing.** An
`exclude` names files the checker must not look at; a threshold buys a
tolerance; a `skip` hides a test. None of those is what happened. The
vocabulary answers a narrower question — *which words mean unfinished work* —
and it now answers it correctly. Every `TODO` and `FIXME` in the fleet is
still reported, and `product_code_quality.py` is unchanged.

A hand-written per-project `placeholder_markers` list MUST be left exactly as
its author wrote it. A project that wants a stricter list than the portfolio
default has said so on purpose, and rewriting a deliberate local decision to
match a portfolio default is a bigger error than the noise it removes. Only
`forge` carries such a list in this fleet, and it already names the six
retained tokens, so in practice the override and the default agree there.

#### Scenario: Marker default from vocabulary

- **WHEN** a project declares no `placeholder_markers`
- **THEN** the checker uses the vocabulary's list

#### Scenario: Override still honoured

- **WHEN** a project declares its own `placeholder_markers`
- **THEN** the checker uses those instead, unchanged by any portfolio edit

#### Scenario: An ordinary English word is not a finding

- **WHEN** a product line contains the word `placeholder` or `stub`, in prose,
  in a doc comment, or inside an identifier such as `placeholder-threshold`
- **THEN** the checker reports no `placeholder-marker` finding for it

#### Scenario: A real debt token is still a finding

- **WHEN** a product line contains `TODO` or `FIXME`
- **THEN** the checker reports a `placeholder-marker` finding for it, at any
  threshold, and the count still drives the `placeholder-threshold` verdict

#### Scenario: Secret scan still fires

- **WHEN** a registry entry contains a value under a secret-shaped key named in
  the vocabulary
- **THEN** the audit reports ERROR `SECRET_VALUE`

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
