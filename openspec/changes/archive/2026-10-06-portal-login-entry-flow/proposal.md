# Proposal: Make browser sign-in reachable from the portal

## Why

Unauthenticated `GET /ui` currently returns `api-unauthorized`. The page gives no usable sign-in route. `/ui/sign-in` without a `project` parameter returns another error, while the existing project-specific OIDC route does work. This leaves the delivered browser login unreachable to a user opening the portal normally.

## What Changes

- Redirect unauthenticated browser navigation from `/ui` to a useful sign-in entry page.
- Let `/ui/sign-in` without a project parameter render a small project-id form that submits to the existing OIDC start route.
- Preserve project-scoped OIDC, state/nonce/PKCE, callback validation, cookie scope, and return-path checks.
- Preserve bearer-authenticated UI automation and all `/v1` JSON behavior.

## Package Boundary and Split Assessment

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `portal-login-entry-flow` | An unauthenticated browser user can reach configured OIDC sign-in from `/ui`. | Forge / Rust 2021, minimum Rust 1.87. | Existing `GET /ui` and `/ui/sign-in` HTML routes and project-scoped identity configuration. | Archived `portal-browser-sign-in`; no active dependencies. | Route contract verifies `/ui` redirects, `/ui/sign-in` renders a project form, and submitting a valid project reaches the existing provider redirect. |

The form and redirect are one coherent entry flow: the chooser alone cannot authenticate, and the redirect alone is not discoverable from the portal. They share the existing OIDC route and session lifecycle, so one package is the smallest independently verifiable outcome.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| Forge portal and identity | `src/api/ui/routes.rs::{handle_fleet,handle_sign_in}`; `src/api/ui/render.rs::sign_in_page`; `src/identity/mod.rs::IdentityConfig` | Existing server-rendered HTML, registered project lookup, and OIDC route. | `/ui` currently returns a terminal 401 and `/ui/sign-in` requires a manually supplied project ID. | Forge owns its Rust portal and project-scoped operator sessions. | **keep local**; this is a single Forge route-flow gap with no reusable cross-project contract. |
| Platform Contracts / sibling products | `platform-contracts` ownership notes and Forge sibling products | No shared browser portal-entry contract established. | Other products own different sessions and applications; sharing a login form would couple unrelated identity lifecycles. | Each product owns its portal/auth lifecycle. | **keep local**; no sibling source, database, or runtime dependency is justified. |

## BFS Impact Map

- **Actors and flow:** browser opens `/ui`; missing session navigates to `/ui/sign-in`; user supplies a registered project id; existing OIDC starts and returns to the local portal.
- **Contracts and callers:** `/ui` may return a redirect for an unauthenticated HTML request; `/ui/sign-in` accepts an omitted project only to render the entry form. Existing project-specific requests keep their provider redirect behavior.
- **Modules:** `src/api/ui/routes.rs`, `src/api/ui/render.rs`, and `tests/portal_ui_contract.rs`.
- **Persistence:** no changes; existing OIDC challenge/session files are reused.
- **Failures and boundaries:** unknown/unconfigured project remains a typed safe error; no credential is accepted from a URL; return paths remain same-origin `/ui` paths; form values are HTML-escaped.
- **Compatibility/security:** authenticated bearer requests and `/v1` JSON remain unchanged. Login still grants only the existing per-project session after OIDC verification and configured admin-claim checks.
- **Tests:** unauthenticated `/ui` redirect, missing-project form, escaping, existing OIDC start/callback route contracts, and JSON/API regression.
- **Unaffected:** provider verification, session persistence, project permissions, publish behavior, generated projects, and sibling repositories.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `portal-web-ui`: make unauthenticated browser navigation reach the existing project-scoped sign-in flow.

## Non-goals

- Adding a new identity provider, session model, or cross-project shared session.
- Automatically selecting a project or disclosing the registered project inventory before authentication.
- Changing OIDC verification, authorization policy, or API bearer behavior.
