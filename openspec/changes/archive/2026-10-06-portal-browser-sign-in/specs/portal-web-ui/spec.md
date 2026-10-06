## ADDED Requirements

### Requirement: Browser OIDC sign-in without URL credentials

Forge SHALL provide a browser sign-in flow for the server-rendered portal using a registered project's identity configuration and standard OIDC authorization-code flow with PKCE. Browser routes SHALL never accept or emit a session credential in a query string or HTML form field.

#### Scenario: Sign-in starts for a configured project

- **WHEN** an unauthenticated browser opens `/ui/sign-in` with a registered project and a safe local return path
- **THEN** Forge creates a bounded, persisted, project-scoped state/nonce/PKCE challenge and redirects to the configured provider with the configured client, redirect URI, and scopes

#### Scenario: No configured identity is available

- **WHEN** the selected project is unknown, has no identity configuration, or has invalid identity configuration
- **THEN** Forge refuses sign-in with a safe HTML error, performs no provider redirect, and creates no session

#### Scenario: Unsafe return path

- **WHEN** sign-in receives an absolute URL, protocol-relative URL, malformed path, or a path outside `/ui`
- **THEN** Forge uses `/ui` as the post-login destination and never redirects off-origin

### Requirement: Verified callback and one-use challenge

Forge SHALL consume callback state once and SHALL mint a browser session only after PKCE code exchange and cryptographic OIDC validation of issuer, audience, signature, expiry, nonce, and configured admin claim.

#### Scenario: Valid administrator callback

- **WHEN** the callback matches an unexpired challenge and the provider returns a valid signed ID token with the configured admin claim
- **THEN** Forge persists one active session for that project's identity and redirects to the validated local return path

#### Scenario: Invalid callback or non-admin user

- **WHEN** callback state is missing, mismatched, expired, or replayed, code exchange fails, any ID-token validation fails, or the admin claim is absent
- **THEN** Forge refuses authentication, creates no session, consumes any callback-reached challenge, and returns only a redacted error

#### Scenario: Provider unavailable

- **WHEN** discovery, JWKS retrieval, or code exchange times out or is unavailable
- **THEN** Forge fails closed within the configured request bound and does not mint a session

### Requirement: Project-scoped browser session cookie

Forge SHALL transport the existing opaque project session ID through a host-only HttpOnly SameSite cookie scoped to `/ui`, validate it against current persisted session state on every protected UI request, and preserve bearer-only authentication for `/v1` routes.

#### Scenario: Active session accesses portal

- **WHEN** a browser presents a valid active cookie on a protected `/ui` route
- **THEN** Forge resolves and authorizes that project session using the same owner, expiry, revocation, and permission checks used by the API

#### Scenario: Invalid, expired, revoked, or cross-project cookie

- **WHEN** a cookie is malformed, unknown, expired, revoked, or owned by a different project than the requested project operation
- **THEN** Forge denies access, creates no new session, and clears an expired/invalid browser cookie

#### Scenario: Cookie does not authorize JSON API

- **WHEN** a request to `/v1` supplies only the browser cookie and no Authorization bearer header
- **THEN** the API returns its existing unauthorized response without changing its JSON contract

#### Scenario: Credential supplied in URL

- **WHEN** a browser UI request supplies the legacy `?token=` parameter
- **THEN** Forge refuses or ignores it without reflecting or logging its value; it never authenticates from the query parameter

### Requirement: Browser sign-out and request integrity

Forge SHALL provide same-origin sign-out that revokes/removes only the current project's session, clears the browser cookie, and requires exact same-origin checks for browser state-changing requests.

#### Scenario: Sign out

- **WHEN** an authenticated browser posts sign-out from the portal's exact origin
- **THEN** Forge invalidates only the owning project's current session, expires the cookie, and redirects to the sign-in page

#### Scenario: Cross-origin state-changing request

- **WHEN** sign-out or another state-changing portal request has a missing or mismatched Origin
- **THEN** Forge refuses it before changing session or project state

#### Scenario: Sign-out isolation

- **WHEN** one project session is signed out
- **THEN** sessions belonging to other projects remain governed by their own persisted state and are not implicitly revoked
