# project-oidc-auth-adoption Specification

## Purpose

Provide optional standards-based OIDC sign-in to ASP.NET Core applications while preserving application ownership of end-user records, enrollment, sessions, and authorization.

## ADDED Requirements

### Requirement: OIDC host integration is optional and provider-neutral

The platform SHALL expose an independently adoptable ASP.NET Core OIDC adapter using standard OIDC authorization-code flow with PKCE. Applications SHALL enable it explicitly; existing applications and local authentication SHALL remain unchanged when it is absent.

#### Scenario: Application opts in

- **WHEN** an application registers the adapter with valid OIDC options
- **THEN** it can initiate and complete an OIDC sign-in without a vendor-specific SDK

#### Scenario: Application does not opt in

- **WHEN** an application does not register the OIDC adapter
- **THEN** no OIDC routes, schemes, provider calls, or configuration requirements are added

#### Scenario: Provider-specific API is proposed

- **WHEN** an application uses the generic OIDC adapter with a compatible provider
- **THEN** authentication uses the OIDC protocol and does not require provider-specific client code

### Requirement: OIDC responses are validated before sign-in

The adapter SHALL validate issuer, audience, signature, redirect URI, state, nonce, token time bounds, and PKCE binding before returning an authenticated principal. State SHALL be one-use. Any validation failure SHALL fail closed without creating a session or user.

#### Scenario: Valid callback

- **WHEN** a callback has a valid state, nonce, PKCE binding, signature, issuer, audience, redirect URI, and time-valid token
- **THEN** the adapter returns the validated external subject to the application's identity flow

#### Scenario: Invalid callback

- **WHEN** any required callback or token check fails
- **THEN** the adapter rejects sign-in, creates no session or local user, and emits a redacted failure classification

#### Scenario: Callback state is replayed

- **WHEN** a callback repeats a state value already consumed
- **THEN** the adapter rejects the replay and does not create another session

### Requirement: Product user records and permissions remain application-owned

The platform SHALL NOT create a shared user database, platform-owned user schema, or cross-project account. The consuming application SHALL resolve the validated issuer/subject through its own identity store and SHALL map application permissions explicitly; provider authentication alone SHALL grant no local role or permission.

#### Scenario: Existing local identity is linked

- **WHEN** a validated external issuer/subject matches an identity link in the application store
- **THEN** the application authenticates only that application's local user and applies its local authorization policy

#### Scenario: Enrollment is disabled

- **WHEN** a valid external subject has no local identity and application enrollment is disabled
- **THEN** sign-in is denied without creating or linking a user

#### Scenario: Two applications authenticate the same human

- **WHEN** two applications independently enable OIDC
- **THEN** each resolves identities and permissions from its own store and neither inherits the other's users, sessions, roles, or grants

#### Scenario: Provider roles are unmapped

- **WHEN** an external token contains a role or permission with no explicit application mapping
- **THEN** that claim grants no local permission

### Requirement: OIDC configuration and failures are safe

The adapter SHALL validate required configuration at startup, require an explicit issuer/client/callback policy, and SHALL NOT log tokens, codes, secrets, or raw claim values. Provider unavailability SHALL NOT create or mutate a local identity.

#### Scenario: Invalid configuration

- **WHEN** a required issuer, client, callback, or credential reference is missing or malformed
- **THEN** startup validation identifies the option and refuses to enable the OIDC scheme

#### Scenario: Provider unavailable

- **WHEN** discovery or key retrieval is unavailable
- **THEN** the application receives a provider-unavailable outcome and no identity mutation occurs

#### Scenario: Duplicate external identity link

- **WHEN** an issuer/subject link conflicts with an existing application identity
- **THEN** the application rejects the link without merging accounts or changing permissions
