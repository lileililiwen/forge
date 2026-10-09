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

### Requirement: Create registers a missing project when asked

`forge project github create` SHALL accept `--register-if-missing`, SHALL validate the local `forge.yaml` manifest and register the canonical project path before creating the remote when the project is not yet registered, and SHALL report the registration outcome without changing the default path.

#### Scenario: Create with register-if-missing on an unregistered checkout

- **WHEN** `forge project github create <dir> --repo <owner/name> --confirm [--push-source --confirm] --register-if-missing` runs for an existing directory with a valid `forge.yaml` that is not in the registry
- **THEN** Forge validates the manifest, registers the project, creates the remote via `gh repo create`, optionally pushes the source, and reports `registered=true`

#### Scenario: Invalid manifest is refused before any remote write

- **WHEN** `--register-if-missing` is set but the directory has no valid `forge.yaml`
- **THEN** Forge refuses with a typed invalid error naming the manifest reason and performs no `gh` mutation

#### Scenario: Default create path is unchanged

- **WHEN** `create` runs without `--register-if-missing`
- **THEN** registry behavior is exactly as before and the envelope reports `register_if_missing=false registered=false`

