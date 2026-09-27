# governance-vocabulary-consumption Specification

## Purpose

Make Workspace Governance's vocabulary authoritative for Forge instead of
re-derived inside it, so that every kind, profile, state and word list Forge
emits or validates comes from a consumed, digest-pinned source; so Forge's own
declaration describes only claims this checkout can honour; and so vocabulary
divergence is visible without ever becoming a health claim.

## ADDED Requirements

### Requirement: Consumed vocabulary is the only source of declared values

Forge SHALL obtain governance kinds, profiles, states and word lists from a
consumed, digest-pinned copy of Workspace Governance's vocabulary, SHALL NOT
re-declare those values locally, and SHALL resolve them in the order explicit
path, environment, vendored copy.

#### Scenario: Descriptor validated against consumed profiles

- **WHEN** a profile descriptor declares a governance profile that appears in
  the consumed canonical profile set
- **THEN** generation emits that value verbatim and records the vocabulary source
  revision in the change evidence

#### Scenario: Runtime names come from one source

- **WHEN** the gate, policy and provider surfaces need the runtime candidate
  order
- **THEN** they read one shared definition and their refusal messages stay
  byte-compatible with the previously duplicated behaviour

#### Scenario: Vocabulary absent entirely

- **WHEN** the vendored vocabulary copy has been removed and a user runs
  generation, import, doctor, check, fleet or gate
- **THEN** each workflow behaves as it did before this capability and reports
  vocabulary-unavailable rather than failing or fetching a substitute

#### Scenario: Explicit vocabulary path is unparseable

- **WHEN** an explicit path or environment value points at a file that does not
  parse
- **THEN** the request is refused naming that file and no fallback to the
  vendored copy is used silently

### Requirement: Non-canonical declared values are refused at load

Forge SHALL refuse, by name, any governance value it is asked to declare that the
consumed vocabulary does not contain, and SHALL never coerce a foreign value into
a Forge value.

#### Scenario: Descriptor names an unknown profile

- **WHEN** a descriptor's governance profile field holds a value outside the
  consumed canonical set
- **THEN** descriptor load refuses naming the field and the offending value, and
  no project tree is staged

#### Scenario: Registry holds an unknown profile

- **WHEN** a workspace registry entry declares a profile Forge has never seen
- **THEN** fleet surfaces the value verbatim and does not map it onto a Forge
  profile

#### Scenario: Placeholder vocabulary term is dropped

- **WHEN** a word list is narrowed for a documented reason
- **THEN** the retained debt markers still fail closed and the dropped terms are
  recorded with their reason in the project's completion rules

### Requirement: Self-declaration states only honourable claims

Forge's own project declaration SHALL use canonical vocabulary values and SHALL
declare each capability with a state that this checkout can support, with a
resolving repository-relative evidence reference for any configured or verified
state and an explicit blocked state for non-goals.

#### Scenario: Capability evidence reference resolves

- **WHEN** a capability is declared configured or verified
- **THEN** its evidence reference names a file that exists in this repository and
  the governance audit reports no finding for it

#### Scenario: Capability is a non-goal

- **WHEN** Forge does not provide a capability by design
- **THEN** it is declared blocked with the non-goal recorded, rather than omitted
  silently or declared declared

#### Scenario: No evidence exists

- **WHEN** a capability is implemented locally but has no native run evidence
- **THEN** it is not promoted to verified, and the gap is recorded as the next
  action

#### Scenario: Declaration normalizes kind

- **WHEN** the consumed vocabulary remaps this project's declared kind
- **THEN** the declaration uses the canonical value and the governance audit
  reports neither KIND_UNKNOWN nor PROFILE_UNKNOWN for this project

### Requirement: Generated declarations gain capabilities only from descriptors

Forge SHALL add capability declarations to a generated project only when the
selected profile descriptor declares them, SHALL NOT copy the control plane's own
capability set into generated projects, and SHALL keep the declaration inert
metadata.

#### Scenario: Descriptor declares capabilities

- **WHEN** a supported descriptor declares a capability set
- **THEN** the staged declaration carries exactly those entries with their
  planned evidence state

#### Scenario: Descriptor declares none

- **WHEN** a descriptor declares no capabilities
- **THEN** the generated declaration carries no capabilities key at all rather
  than an empty or speculative block

#### Scenario: Declaration remains inert

- **WHEN** a generated project is built and tested with its native toolchain
  while the declaration is present and again after it is removed
- **THEN** both runs succeed identically

### Requirement: Vocabulary divergence is visible and never gates health

Forge SHALL report declaration-vocabulary divergence through a doctor finding and
the read-only surfaces, SHALL mark it not applicable outside its scope, and SHALL
not let it change a health verdict, maturity level, checker document or exit code
for a project that did not opt in.

#### Scenario: Project without a declaration

- **WHEN** the assessed project has no project declaration file
- **THEN** the finding is not applicable and the checker emits no alert for it

#### Scenario: Non-canonical value observed

- **WHEN** a declaration carries a value outside the consumed vocabulary
- **THEN** the finding warns naming field, value and vocabulary source while the
  doctor health verdict and maturity result stay unchanged

#### Scenario: Broken reference in a Forge-authored claim

- **WHEN** a capability reference Forge previously emitted no longer resolves
- **THEN** the finding fails for that entry with the path named, and redaction
  plus char bounds still apply to the reported string

### Requirement: Product-quality gate binding is auditable

Where Forge binds the portfolio's product-code quality check to its own gate, the
marker vocabulary decision SHALL be explicit and recorded, the debt threshold
SHALL stay fail-closed, and the measured before-and-after counts SHALL appear in
the change evidence.

#### Scenario: Quality check runs on Forge

- **WHEN** the gate executes the bound quality check against this repository
- **THEN** it passes under the declared marker set with a zero placeholder
  threshold, and the pre-change count and residual matches are recorded

#### Scenario: Debt introduced after binding

- **WHEN** a maintainer marker is added to product source
- **THEN** the bound check fails rather than tolerating a count

#### Scenario: Gate verdict is reported honestly

- **WHEN** the quality check or any other bound check blocks
- **THEN** the gate evidence records the blocked verdict and the recovery path,
  and no surface renders it as passing
