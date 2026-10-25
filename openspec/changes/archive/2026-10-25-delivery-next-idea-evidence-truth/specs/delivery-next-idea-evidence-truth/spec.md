## ADDED Requirements

### Requirement: The next-idea row states only what the server recorded

Forge SHALL record the next-idea transition in the journal and SHALL
state in that row only what the server itself recorded. Forge SHALL NOT
claim in the row that a publish landed when the server observed no
publish. A note supplied by the caller SHALL be recorded as the
caller's report and SHALL NOT be stated as the server's own finding.

#### Scenario: A next-idea row claims no publish

- **WHEN** a confirmed next-idea request is journalled without a
  caller note
- **THEN** the row names the transition the server recorded and claims
  no publish

#### Scenario: A caller's note is the caller's report

- **WHEN** a confirmed next-idea request carries a note
- **THEN** the row states the transition the server recorded and keeps
  the note marked as the caller's report

#### Scenario: The refusals are unchanged

- **WHEN** a next-idea request is unconfirmed, over-long or carries a
  credential-shaped note
- **THEN** the route refuses it and journals nothing
