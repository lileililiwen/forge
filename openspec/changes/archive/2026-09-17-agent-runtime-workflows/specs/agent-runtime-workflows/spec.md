## ADDED Requirements

### Requirement: Managed agent sessions

Forge SHALL expose start, pause, takeover, resume, restart and new-session operations for supported OpenCode/Codex adapters, and route spec run to the selected project and active spec.

#### Scenario: Managed agent sessions success

- **WHEN** a supported spec execution starts and resumes
- **THEN** the session identity, provider and project association remain observable

#### Scenario: Managed agent sessions failure

- **WHEN** the manager disappears or the requested transition is unsupported
- **THEN** Forge reports failed, disconnected or unsupported with recovery guidance

#### Scenario: Managed agent sessions boundary

- **WHEN** a session ends without verification evidence
- **THEN** agent completion remains distinct from verified spec completion

### Requirement: Verified test and Git operations

Forge SHALL expose test, commit and push as separate scoped operations, requiring explicit execution intent for remote writes and preserving unrelated work.

#### Scenario: Verified test and Git operations success

- **WHEN** verified related changes are selected for commit
- **THEN** only their reviewed paths are committed with the originating operation recorded

#### Scenario: Verified test and Git operations failure

- **WHEN** tests fail or requested paths include unrelated edits
- **THEN** completion or commit preparation reports the blocker without silently including those edits

#### Scenario: Verified test and Git operations boundary

- **WHEN** tests pass but push was not requested
- **THEN** Forge records verification without updating any remote
