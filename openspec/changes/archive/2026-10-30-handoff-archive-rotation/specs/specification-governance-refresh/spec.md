## ADDED Requirements

### Requirement: The handoff stays bounded and history is retained

Forge SHALL keep `HANDOFF.md` within a declared line budget and SHALL move
history that no longer describes the current tree into a retained archive
rather than deleting it. The governance check SHALL fail when the budget is
exceeded or when an existing archive is not linked from the handoff.

#### Scenario: The handoff exceeds its budget

- **WHEN** `HANDOFF.md` grows past 8,000 lines
- **THEN** the governance check fails naming the line count and the budget, and
  the next rotation moves a historical span

#### Scenario: History is retained

- **WHEN** a historical span is rotated
- **THEN** it moves byte-for-byte into `HANDOFF-ARCHIVE.md` at the repository
  root, where its relative links still resolve, and the handoff links to it

#### Scenario: The archive is not linked

- **WHEN** `HANDOFF-ARCHIVE.md` exists and `HANDOFF.md` has no link to it
- **THEN** the governance check fails naming the missing link

#### Scenario: The active pointer stays current

- **WHEN** an active change exists
- **THEN** exactly one `current_spec` pointer remains in `HANDOFF.md`, never in
  the archive
