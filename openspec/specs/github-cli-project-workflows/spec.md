# github-cli-project-workflows Specification

## Purpose

Provide bounded local GitHub repository workflows through the user's
installed `gh` CLI without requiring a second Forge GitHub login,
with explicit confirmation on every remote write and a credential-
safe execution boundary.

## Requirements
### Requirement: Reuse existing GitHub CLI authentication

Forge SHALL use the installed `gh` authentication context for supported local GitHub repository operations without requiring a second Forge GitHub login.

#### Scenario: Authenticated account

- **WHEN** `gh auth status --hostname github.com` succeeds
- **THEN** Forge reports the authenticated host/account state without reading or displaying the credential

#### Scenario: Missing authentication

- **WHEN** `gh` is unavailable or has no authenticated account
- **THEN** Forge reports a typed unavailable/auth-required result and does not attempt a remote write

### Requirement: Confirmed repository operations

Forge SHALL require explicit confirmation for repository creation, source push, and pull-request creation, with private visibility as the default.

#### Scenario: Private repository creation

- **WHEN** an authorized project owner confirms repository creation without a visibility override
- **THEN** Forge creates the remote as private and records the operation outcome

#### Scenario: Public repository boundary

- **WHEN** public visibility is requested without explicit public confirmation
- **THEN** Forge refuses before invoking `gh`

#### Scenario: Canceled write

- **WHEN** the owner declines a write confirmation
- **THEN** Forge performs no `gh` or `git` mutation

### Requirement: Bounded credential-safe process execution

Forge SHALL invoke GitHub CLI operations through an allowlisted argument array with bounded duration/output and shall not extract, persist, or log the CLI credential.

#### Scenario: Process failure

- **WHEN** `gh` exits nonzero, times out, or returns an unsupported outcome
- **THEN** Forge returns a bounded typed result and keeps credential-shaped data out of output and its journal

