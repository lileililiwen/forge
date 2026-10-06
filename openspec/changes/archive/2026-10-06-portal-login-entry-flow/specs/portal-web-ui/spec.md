## MODIFIED Requirements

### Requirement: Browser OIDC sign-in without URL credentials

Forge SHALL provide a browser sign-in flow for the server-rendered portal using a registered project's identity configuration and standard OIDC authorization-code flow with PKCE. Browser routes SHALL never accept or emit a session credential in a query string or HTML form field. An unauthenticated HTML request to `/ui` SHALL redirect to the local sign-in entry page. Opening `/ui/sign-in` without a project parameter SHALL render an accessible form that asks for a project id and starts the existing project-scoped OIDC flow when submitted. The entry page SHALL NOT disclose the registered project inventory.

#### Scenario: Unauthenticated browser reaches sign-in

- **WHEN** a browser requests `/ui` without a session
- **THEN** Forge redirects locally to `/ui/sign-in` and does not render `api-unauthorized`

#### Scenario: Sign-in entry asks for a project

- **WHEN** an unauthenticated browser opens `/ui/sign-in` without a project parameter
- **THEN** Forge renders an accessible required project-id form that submits to the existing sign-in route

#### Scenario: Sign-in starts for a configured project

- **WHEN** an unauthenticated browser opens `/ui/sign-in` with a registered project and a safe local return path
- **THEN** Forge creates a bounded, persisted, project-scoped state/nonce/PKCE challenge and redirects to the configured provider with the configured client, redirect URI, and scopes

#### Scenario: No configured identity is available

- **WHEN** the selected project is unknown, has no identity configuration, or has invalid identity configuration
- **THEN** Forge refuses sign-in with a safe HTML error, performs no provider redirect, and creates no session

#### Scenario: Unsafe return path

- **WHEN** sign-in receives an absolute URL, protocol-relative URL, malformed path, or a path outside `/ui`
- **THEN** Forge uses `/ui` as the post-login destination and never redirects off-origin

#### Scenario: JSON and authenticated callers remain compatible

- **WHEN** an authenticated browser or JSON API client uses existing routes
- **THEN** authenticated portal rendering and `/v1` JSON response contracts remain unchanged
