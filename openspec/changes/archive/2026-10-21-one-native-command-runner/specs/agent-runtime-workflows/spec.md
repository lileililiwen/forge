## MODIFIED Requirements

### Requirement: Verified test and Git operations

Forge SHALL expose test, commit and push as separate scoped operations, requiring explicit execution intent for remote writes and preserving unrelated work. The test operation SHALL run the profile's declared test command through the shared native command runner, under a wall-clock bound that kills the command's whole process group when it elapses, and SHALL report a command that overran the bound as a timeout rather than as a non-zero exit.

#### Scenario: Verified test and Git operations success

- **WHEN** verified related changes are selected for commit
- **THEN** only their reviewed paths are committed with the originating operation recorded

#### Scenario: Verified test and Git operations failure

- **WHEN** tests fail or requested paths include unrelated edits
- **THEN** completion or commit preparation reports the blocker without silently including those edits

#### Scenario: Verified test and Git operations boundary

- **WHEN** tests pass but push was not requested
- **THEN** Forge records verification without updating any remote

#### Scenario: A test command that never returns

- **WHEN** a project's declared test command is still running when the bound elapses
- **THEN** the command's process group is killed and reaped, and the operation reports a timeout that names the command and the bound
