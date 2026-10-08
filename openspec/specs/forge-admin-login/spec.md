# forge-admin-login Specification

## Purpose
Authenticate the single Forge-wide administrator with an Argon2id-hashed credential, bounded session cookie, and CLI-managed password lifecycle.
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

### Requirement: Self-service administrator password change

Forge SHALL provide a non-destructive `forge identity change-password` CLI verb that replaces the single Forge-wide administrator password. It SHALL read the new password twice without terminal echo, apply the same strength and confirmation policy as initial setup, persist only a new Argon2id hash for the existing normalized email, and revoke every outstanding administrator session so no previously issued cookie survives the change. Forge SHALL NOT change the stored email, SHALL refuse the operation when no administrator is configured, and SHALL NOT print the password.

#### Scenario: Operator changes their own password

- **WHEN** the operator runs `forge identity change-password` and enters a valid matching new password twice
- **THEN** Forge replaces only the stored Argon2id hash, keeps the same normalized email, and prints a success message without echoing the password

#### Scenario: Password change invalidates existing sessions

- **WHEN** a password change succeeds while administrator sessions are outstanding
- **THEN** Forge revokes those sessions so a subsequent protected admin API request with a previously issued cookie returns `401`

#### Scenario: Weak or mismatched new password

- **WHEN** the new password is shorter than the minimum length, exceeds the maximum, or the two entries differ
- **THEN** Forge returns a safe CLI error and leaves the stored hash and all sessions unchanged

#### Scenario: Change before any administrator exists

- **WHEN** `forge identity change-password` runs against a registry with no configured administrator
- **THEN** Forge returns a safe error directing the operator to run setup first and writes no administrator row

### Requirement: Strong password generation

Forge SHALL provide `forge identity generate-password` that prints exactly one cryptographically strong random password drawn from operating-system entropy, with a bounded configurable length defaulting to a value that satisfies the administrator password policy. The generator SHALL NOT read or write the registry, SHALL NOT require a terminal, and SHALL NOT persist or log the generated value; it SHALL emit the password to standard output once so an operator can paste it into the change-password prompt.

#### Scenario: Generate a default-strength password

- **WHEN** the operator runs `forge identity generate-password` with no arguments
- **THEN** Forge prints a single random password whose length meets the administrator minimum and does not contact the registry

#### Scenario: Requested length is out of bounds

- **WHEN** the operator requests a length below the administrator minimum or above the supported maximum
- **THEN** Forge returns a safe CLI error and prints no password

