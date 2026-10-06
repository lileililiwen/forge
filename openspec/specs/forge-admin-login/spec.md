# forge-admin-login Specification

## Purpose
TBD - created by archiving change forge-global-admin-portal. Update Purpose after archive.
## Requirements
### Requirement: One Forge administrator credential

Forge SHALL provide a one-time interactive CLI setup for one Forge-wide administrator email and password. Forge SHALL persist only an Argon2id password hash, refuse weak or mismatched passwords and duplicate setup, and SHALL NOT use project manifests or project identity stores for Forge operator credentials.

#### Scenario: First administrator setup succeeds

- **WHEN** the operator runs `forge identity setup --email <address>` interactively and enters a valid matching password twice
- **THEN** Forge stores the normalized email and Argon2id hash in the Forge registry database and prints setup success without printing the password

#### Scenario: Setup is repeated or invalid

- **WHEN** setup is non-interactive, the email/password is invalid, confirmation differs, or an administrator already exists
- **THEN** Forge returns a safe CLI error and leaves any existing administrator and sessions unchanged

### Requirement: Forge-wide password login and session API

Forge SHALL authenticate frontend users against the single Forge administrator independent of registered project identity configuration. Successful login SHALL create an opaque cryptographically random session with bounded expiry, store only its token digest, and set a host-only HttpOnly SameSite=Lax cookie. Invalid credentials SHALL produce the same generic JSON response and SHALL NOT create a session.

#### Scenario: Valid Forge administrator signs in

- **WHEN** the standalone frontend posts the configured admin email and correct password to `POST /v1/admin/session`
- **THEN** Forge returns authenticated JSON state and sets a Forge-wide session cookie authorizing fleet API requests

#### Scenario: Invalid credentials

- **WHEN** the frontend posts an unknown email or incorrect password
- **THEN** Forge returns the same generic JSON authentication error, creates no session, and does not reveal whether the email exists

#### Scenario: Session expires or is revoked

- **WHEN** an expired, unknown or signed-out Forge session is presented to a protected admin API endpoint
- **THEN** Forge returns `401`, grants no access and returns no fleet data

#### Scenario: Project identity configuration is absent

- **WHEN** a registered project has no `identity:` configuration but the frontend has a valid Forge-wide session
- **THEN** the fleet API returns the project without asking for a project id during authentication

### Requirement: Safe Forge-wide logout and session isolation

Forge SHALL revoke only the current Forge-wide session on same-origin logout. Forge-wide cookies and session records SHALL remain distinct from project-scoped OIDC sessions and API bearer tokens.

#### Scenario: Same-origin logout

- **WHEN** the authenticated frontend sends `DELETE /v1/admin/session` with the exact configured frontend origin
- **THEN** Forge revokes the current Forge admin session and expires its cookie

#### Scenario: Logout origin mismatch

- **WHEN** logout lacks or mismatches the configured frontend origin
- **THEN** Forge refuses before changing session or cookie state

#### Scenario: Project OIDC session remains independent

- **WHEN** a Forge-wide browser session is created or revoked
- **THEN** existing project-scoped OIDC session validity is not implicitly changed

### Requirement: Authenticated fleet data API

Forge SHALL return all registered projects and real registry/journal summary evidence from `GET /v1/admin/projects` only to a valid Forge administrator session. Anonymous, expired, revoked or invalid sessions SHALL receive `401` and no project data.

#### Scenario: Authenticated administrator requests fleet

- **WHEN** a valid Forge administrator session requests `GET /v1/admin/projects`
- **THEN** Forge returns the complete registered-project fleet and available operation evidence, including projects with no project-scoped identity configuration

