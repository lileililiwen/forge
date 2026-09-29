# Capability: project-semantic-description-review

## ADDED Requirements

### Requirement: Traceable semantic proposals

Forge SHALL produce, contract `forge-semantic-proposal/0.1.0`, a reviewable
proposal for each non-deterministic metadata change — description, domain,
portfolio tags, profile or lifecycle — carrying its current value, suggested
value, evidence sources, confidence, generator identity and revision.

#### Scenario: A suggestion carries its provenance

- **WHEN** Forge suggests a description or classification
- **THEN** the proposal records the evidence sources, their revisions, a
  confidence label and the generating provider, and the suggestion is a
  proposal rather than an applied value

#### Scenario: A generated value is not observed fact

- **WHEN** a suggestion is created
- **THEN** it is stored in `Suggested` state and is never presented as the
  project's declared or observed value until an operator approves it

### Requirement: Human approval before any write

Forge SHALL require an explicit operator decision to move a proposal to
`Approved` or `Rejected`, and SHALL NOT apply a suggestion to a project file, a
provider, or any other surface without that approval.

#### Scenario: Approval is required

- **WHEN** a proposal is `Suggested`
- **THEN** no local or external write occurs until an operator approves it, and
  a rejection preserves the current value

#### Scenario: A stale proposal cannot be approved

- **WHEN** a newer evidence revision supersedes a proposal
- **THEN** the proposal is `Superseded` and an approval attempt against the
  earlier revision is refused

### Requirement: Conflicting or unavailable interpretation is explicit

Forge SHALL represent conflicting evidence and unavailable providers as their
own states, and SHALL route an unresolved or conflicting interpretation to
bounded review rather than guessing.

#### Scenario: Conflicting evidence is not resolved silently

- **WHEN** two evidence sources imply different values
- **THEN** the proposal is `Conflicted` with each source retained, and no
  approval is offered on the conflict

#### Scenario: Unavailable provider is reported, not faked

- **WHEN** the optional generation provider is unavailable
- **THEN** Forge reports the typed unavailable state and leaves prior proposals
  untouched, rather than inventing a suggestion
