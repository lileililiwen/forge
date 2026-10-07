# forge-web-human-dashboard (delta)

## ADDED Requirements

### Requirement: Human-readable project views

Forge SHALL present projects by display name with one plain status line
per row, and SHALL NOT render identifier grammars, hashes, digests,
operation ids or full revisions in primary dashboard views. Technical
identifiers SHALL remain available one step away (detail/journal) and
SHALL remain in JS memory and wire bodies wherever confirmation binding
requires them.

#### Scenario: Operator reads the fleet

- **WHEN** a signed-in operator opens the dashboard
- **THEN** every row leads with a human name and one plain status or next-step line, with no kebab lecture, hash, or code column

#### Scenario: Operator previews a mutation

- **WHEN** a preview renders for any confirm-gated action
- **THEN** it reads as sentences naming the action and its target, with no digest, hash or operation id on screen

#### Scenario: Full identifiers stay reachable

- **WHEN** the operator opens a project detail or the journal evidence
- **THEN** exact ids, revisions and operation references are shown there

### Requirement: Coherent visual system

Forge SHALL style the dashboard as one dark command-center system with
token-driven surfaces, hierarchy and states, meeting WCAG 2.2 AA
contrast on all text. All existing selectors, landmarks, controls and
responsive breakpoints SHALL keep working unchanged.

#### Scenario: Dashboard renders in the new system

- **WHEN** any shipped panel renders in Chromium
- **THEN** surfaces, hierarchy and states follow the token system and sampled text contrast meets 4.5:1

### Requirement: Plain-language failures with in-place recovery

Forge SHALL phrase refusals and failures as plain instructions with a
next step, keeping typed codes in the JSON body only. After a successful
onboarding run the fleet SHALL refresh in place with results still
visible, falling back to manual reload when the refresh fails.

#### Scenario: Action is refused

- **WHEN** a preview, confirmation or apply is refused
- **THEN** the operator reads what happened and what to do next, with no grammar lecture or hash

#### Scenario: Onboarding succeeds

- **WHEN** a confirmed batch completes
- **THEN** per-item outcomes stay visible and the fleet shows the new projects without a full-page reload
