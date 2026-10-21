## MODIFIED Requirements

### Requirement: MVP profile discovery

Forge SHALL list and inspect versioned descriptors for all five MVP profiles with supported capabilities, packages, layout, conventions, build/test commands, deployment defaults and quality policies. A descriptor whose declared build command only analyses the project SHALL NOT be inspectable.

#### Scenario: MVP profile discovery success

- **WHEN** the profile catalog is queried
- **THEN** all five MVP IDs and their descriptor versions are visible

#### Scenario: MVP profile discovery failure

- **WHEN** a malformed descriptor omits required validation commands
- **THEN** registration fails with the missing field named

#### Scenario: MVP profile discovery boundary

- **WHEN** a valid profile has no database requirement
- **THEN** the descriptor does not force a database dependency

#### Scenario: MVP profile discovery build command truth

- **WHEN** a profile in the catalog declares a build command whose verb is
  a named analysis verb
- **THEN** inspecting that profile is refused with `invalid-profile`,
  naming the profile, the command and the verb
