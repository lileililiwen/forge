## ADDED Requirements

### Requirement: Browser sessions require cryptographically verified OIDC

Forge SHALL mint a browser-usable administrator session only after a standard OIDC provider response is cryptographically verified against the project's configured issuer, client audience, redirect URI, PKCE challenge, nonce, token time bounds, and explicit admin-claim allowlist. CLI/test helpers that accept claim fields directly SHALL NOT be treated as browser authentication.

#### Scenario: Verified provider response

- **WHEN** OIDC discovery and callback verification succeed and the validated claims include an allowed admin value
- **THEN** Forge mints and persists one project-scoped session with `admin:access`

#### Scenario: Claims supplied outside a verified provider response

- **WHEN** a caller supplies issuer, audience, nonce, scope, or admin claims without a verified provider-signed ID token bound to a one-use challenge
- **THEN** the browser authentication path refuses to mint or accept an administrator session

#### Scenario: Project isolation

- **WHEN** a valid browser session minted for project A is presented to project B
- **THEN** Forge refuses the request with the existing cross-project identity error and does not alter either session
