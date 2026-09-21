# central-admin-identity Specification

## Purpose
Federated admin sign-in over standard OIDC: PKCE challenge, callback and claims validation, and project-scoped admin sessions gated on an explicit admin-claim allow list, so a valid provider login never silently grants admin.
## Requirements
### Requirement: Federated admin sign-in

Forge SHALL support compatible auth/admin features using standard OIDC so an existing provider login can authenticate separate project admin applications.

#### Scenario: Federated admin sign-in success

- **WHEN** a user authenticated at the configured provider opens a second authorized project admin
- **THEN** OIDC establishes that project own session without a second provider login when provider policy allows

#### Scenario: Federated admin sign-in failure

- **WHEN** issuer, audience, redirect, state or nonce validation fails
- **THEN** the project rejects authentication and records a redacted failure

#### Scenario: Federated admin sign-in boundary

- **WHEN** the user authenticates but lacks the project admin permission
- **THEN** the application denies admin access

### Requirement: Session and project isolation

Forge SHALL retain per-project sessions and avoid requiring shared cookies across unrelated applications.

#### Scenario: Session and project isolation success

- **WHEN** two projects use one identity provider
- **THEN** each stores and expires its own session

#### Scenario: Session and project isolation failure

- **WHEN** a cookie or token intended for one project is presented to another
- **THEN** the other project rejects it

#### Scenario: Session and project isolation boundary

- **WHEN** one project session is terminated
- **THEN** other project sessions follow their configured policies and are not implicitly treated as revoked or valid without evidence

