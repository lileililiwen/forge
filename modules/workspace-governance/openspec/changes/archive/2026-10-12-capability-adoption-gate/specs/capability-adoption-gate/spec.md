## ADDED Requirements

### Requirement: Derive capability adoption from consumer evidence

Workspace Governance SHALL measure whether a registered project consumes a
registered shared capability by joining three sources: the capability
registry entry, a dependency edge in the consumer's own manifests, and an
additive `consumes` record in the consumer's `.project.json`. A hand-written
declaration of adoption without a resolvable manifest edge SHALL NOT satisfy
the measurement. The measurement SHALL be read-only: it SHALL NOT execute a
project command, invoke a package manager, resolve a lockfile or transitive
graph, or make a network request.

#### Scenario: Adoption proven by all three sources

- **WHEN** a capability's registry entry binds it to one provider, the
  consumer's manifest contains a pinned dependency on the named package, and
  the consumer's `.project.json` records the pairing in `capabilities
  <name>.consumes[]` with an `evidence_ref` that resolves inside the
  consumer root
- **THEN** the audit records the capability as adopted for that project and
  emits no adoption finding

#### Scenario: Manifest proves consumption but the record is missing

- **WHEN** the consumer's manifest contains a pinned dependency on the
  named package but the consumer's `.project.json` has no matching
  `consumes[]` entry, or the entry has no `evidence_ref`
- **THEN** the audit emits `CAPABILITY_ADOPTION_UNEVIDENCED` as a warning
  naming the provider and package, and does not emit
  `CAPABILITY_ADOPTION_MISSING`, because the build already restores the
  edge

#### Scenario: Declaration without a manifest edge

- **WHEN** the consumer's `.project.json` records `consumes[]` for a
  capability but no bounded manifest in the consumer root contains a pinned
  dependency on the named package
- **THEN** the audit emits `CAPABILITY_ADOPTION_MISSING` naming the
  provider, the package, and the declared `consumes[]` entry, and does not
  report the capability as adopted

#### Scenario: Evidence reference escapes the consumer root

- **WHEN** a `consumes[].evidence_ref` is absolute, contains a `..`
  segment, or resolves outside the consumer project root
- **THEN** the audit refuses to read the path and emits
  `CAPABILITY_ADOPTION_EVIDENCE_STALE` naming the offending reference; the
  value is never normalized into a portable form

#### Scenario: Evidence reference stops resolving

- **WHEN** a `consumes[].evidence_ref` that previously resolved no longer
  exists in the consumer root
- **THEN** the audit emits `CAPABILITY_ADOPTION_EVIDENCE_STALE` and treats
  the capability as undocumented, never as adopted

### Requirement: Report a local duplicate implementation

The audit SHALL emit `CAPABILITY_ADOPTION_MISSING` when a project declares a
local implementation of a capability it is expected to consume and no
consumption edge is proven. Detection SHALL be bounded to
declaration-supplied paths and SHALL NOT attempt to recognise an
unregistered hand-rolled implementation.

#### Scenario: Declared local implementation and no consumption

- **WHEN** a project records a local implementation path for a capability in
  `expected` or `required` posture and no manifest contains the named
  package
- **THEN** the audit emits `CAPABILITY_ADOPTION_MISSING` naming the
  provider, the package, and the local implementation path, at WARNING level
  in `expected` posture and ERROR level in `required` posture

#### Scenario: Unregistered local implementation is not inferred

- **WHEN** a project contains code that resembles a capability
  implementation but declares no local implementation path
- **THEN** the audit emits no duplicate-implementation finding and does not
  guess, because an inferred duplicate cannot be distinguished from a
  legitimate boundary decision

### Requirement: Escalate on a recorded WARN to FAIL ramp

The audit SHALL assign every registry capability one posture of `exempt`,
`expected`, or `required`, and SHALL report findings at WARNING for
`expected` and ERROR for `required`. A capability SHALL be admitted as
`expected` and SHALL NOT be admitted directly as `required`. Promotion to
`required` SHALL require all three of: a provider whose capability is
`verified` with a resolving `evidence_ref`; a recorded number of consecutive
audit runs in `expected` posture; and an appended graduation entry recording
the transition. The audit SHALL NOT promote a capability to `required` on
elapsed wall-clock time alone.

#### Scenario: New capability is admitted as a warning

- **WHEN** a capability is first added to the capability registry
- **THEN** its posture is `expected` and every adoption finding for it is a
  warning, so admitting a capability cannot introduce a fleet error

#### Scenario: Graduation requires provider evidence

- **WHEN** the consecutive-audit count for a capability has been reached but
  the provider's capability is not `verified` or its `evidence_ref` does not
  resolve
- **THEN** the audit does not promote the capability, keeps it `expected`,
  and emits no graduation entry

#### Scenario: Graduation is a recorded entry

- **WHEN** a capability has a verified provider, a recorded consecutive
  audit count, and a matching appended graduation entry
- **THEN** the capability is reported in `required` posture and its
  adoption findings are errors that fail the unattended gate

#### Scenario: Count cannot be forged

- **WHEN** a graduation entry is appended without a verified provider or
  without a recorded consecutive audit count
- **THEN** the audit rejects the entry, keeps the capability in `expected`
  posture, and emits no error for the entry itself

### Requirement: Accept an evidence-backed decline

The audit SHALL allow a project to decline a capability without adopting it,
and a decline SHALL exempt only that project and only that capability. A
decline SHALL cite a rationale from a closed vocabulary and SHALL cite an
`evidence_ref` that resolves inside the project root. A decline without
resolvable evidence SHALL be reported as
`CAPABILITY_ADOPTION_DECLINE_UNRECORDED`.

#### Scenario: Documented decline

- **WHEN** a project records a decline for a capability with a rationale
  from the closed vocabulary and an `evidence_ref` that resolves in the
  project root
- **THEN** the audit emits no adoption finding for that capability in that
  project

#### Scenario: Prose decline without a record

- **WHEN** a project's documentation states a non-adoption boundary but the
  project records no `declines[]` entry
- **THEN** the audit emits `CAPABILITY_ADOPTION_DECLINE_UNRECORDED` naming
  the capability and the documented rationale, and does not treat the prose
  as either adoption or a decline

#### Scenario: Rationale outside the closed vocabulary

- **WHEN** a `declines[]` entry cites a rationale that is not one of
  `ownership`, `privacy`, `lifecycle`, `dependency-direction`,
  `naming-symmetry`, `not-applicable`, or `provider-unavailable`
- **THEN** the audit emits `CAPABILITY_ADOPTION_DECLINE_UNRECORDED` naming
  the unknown rationale and does not exempt the capability

#### Scenario: Decline is repo-local and capability-scoped

- **WHEN** one project records a decline for a capability
- **THEN** the audit emits no adoption finding for that capability in that
  project, still emits the finding in every other project that has not
  declined it, and still emits findings in that project for every other
  capability

#### Scenario: Provider is exempt from its own capability

- **WHEN** a registry entry's `provider` equals the audited project's id
- **THEN** the audit resolves the capability to `exempt` for that project
  and emits no self-adoption finding

### Requirement: Keep unresolvable capability identities non-blocking

The audit SHALL emit `CAPABILITY_ADOPTION_UNRESOLVED` as a review-required
finding when a capability id does not bind to exactly one provider. That
finding SHALL NOT escalate to an error at any posture, and SHALL NOT be
attributed to the project under audit.

#### Scenario: Ambiguous capability id

- **WHEN** a capability id is bound to more than one provider, or to none,
  in the capability registry
- **THEN** the audit emits `CAPABILITY_ADOPTION_UNRESOLVED` as
  review-required, emits no `CAPABILITY_ADOPTION_MISSING` for it, and never
  escalates it to an error regardless of posture

#### Scenario: Missing or unparseable capability registry

- **WHEN** the capability registry file is absent, unparseable, or carries
  an empty capability set
- **THEN** the audit emits a governance error and does not fall back to an
  empty registry, so the check can never degrade into enforcing nothing

### Requirement: Declare stable finding codes

The audit SHALL report capability adoption under the stable identifiers
`CAPABILITY_ADOPTION_MISSING`, `CAPABILITY_ADOPTION_UNEVIDENCED`,
`CAPABILITY_ADOPTION_EVIDENCE_STALE`, `CAPABILITY_ADOPTION_DECLINE_UNRECORDED`,
and `CAPABILITY_ADOPTION_UNRESOLVED`. These codes SHALL be additive: no
existing finding code, severity vocabulary, exit code, report key, or Forge
adapter observation key SHALL be renamed, removed, or re-leveled.

#### Scenario: Existing findings are unchanged

- **WHEN** a project is audited after the adoption check ships
- **THEN** the finding set contains every finding it contained before, plus
  only findings under the five new `CAPABILITY_ADOPTION_*` codes, and the
  JSON report and Forge adapter observation keys are structurally identical
  to the previous version
