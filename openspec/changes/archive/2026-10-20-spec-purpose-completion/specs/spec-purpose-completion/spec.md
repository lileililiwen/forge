## ADDED Requirements

### Requirement: Every capability states its purpose

Forge SHALL give every canonical capability a `## Purpose` paragraph that
says what the capability is for, in the product's own terms, drawn from that
capability's own requirements, and SHALL NOT leave the archive placeholder in
place. Completing a purpose SHALL NOT change the capability's requirements,
scenarios or wording.

#### Scenario: A capability states what it is for

- **WHEN** a canonical capability specification is read
- **THEN** its `## Purpose` section holds one paragraph that says what the capability is for, in the present tense, and names no placeholder

#### Scenario: Only the purpose changes

- **WHEN** a capability's purpose is completed
- **THEN** its requirement headers, its scenarios and its wording are exactly what they were before

### Requirement: The archive placeholder is refused

Forge SHALL refuse a canonical capability whose purpose is the archive
placeholder, whose purpose section is missing, or whose purpose is too short to
be a paragraph, and the refusal SHALL name the capability.

#### Scenario: A placeholder purpose is refused

- **WHEN** a canonical capability's purpose is the archive placeholder
- **THEN** the repository's contract check fails and names that capability

#### Scenario: A missing purpose is refused

- **WHEN** a canonical capability has no `## Purpose` section
- **THEN** the repository's contract check fails and names that capability

#### Scenario: A label is not a purpose

- **WHEN** a canonical capability's purpose is shorter than a paragraph
- **THEN** the repository's contract check fails and names that capability

#### Scenario: A written purpose passes

- **WHEN** every canonical capability states a real purpose
- **THEN** the repository's contract check passes
