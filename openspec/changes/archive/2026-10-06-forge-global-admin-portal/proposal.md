# Proposal: Forge-wide admin login and project dashboard

## Why

The current browser entry redirects to a project-id form and then expects each project to own OIDC configuration. That prevents the operator from reaching the fleet when projects lack identity settings and makes the portal look like an unfinished programmer tool. The required experience is one Forge login followed by management of every registered project.

## What Changes

- Add one Forge-owned administrator account and global browser session, independent of each project's `forge.yaml` identity configuration.
- Add standalone email/password sign-in and dashboard pages under `frontend/`, based visually on the named AllTools split-panel layout.
- Add authenticated JSON session and fleet endpoints; rework the frontend into a clear multi-project command center using only Forge registry and journal evidence.
- Keep project IDs available in project rows and detail URLs after authentication; never request one to sign in.
- Add a CLI setup path for the first administrator and store only a password hash.

## Package Boundary and Split Assessment

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `forge-global-admin-portal` | An operator signs in once to Forge and reaches the full registered-project dashboard. | Forge API / Rust 2021 and SQLite; browser / standalone HTML, CSS and JavaScript under `frontend/` | Forge-owned admin account/session plus JSON authentication and fleet endpoints | Existing registry and API transport; separately served static frontend | Credential setup and login/session contract, followed by authenticated fleet API and browser workflow smoke |

This is one package because login, global authorization and the dashboard are one operator journey; splitting them would leave an authenticated login without the requested project-management screen or a dashboard users cannot reach. They share the Forge API owner, registry lifecycle, browser session and one acceptance flow. The package does not change project-level OIDC used by project APIs. No second package is required for the requested outcome.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| AllTools login visual reference | `/home/paul/code/alltools-platform/rust/templates/frontend/login.html` | Split brand panel and email/password card establish the requested composition. | It extends AllTools templates and uses AllTools account/session storage and route helpers; those are not Forge contracts. | AllTools owns its runtime; Forge owns this portal and its operator sessions. | **keep local** |
| Forge project identity | `src/identity/mod.rs`, `openspec/specs/central-admin-identity/spec.md` | OIDC protocol verification and project-scoped admin sessions remain available to project API callers. | Per-project config/session ownership cannot authenticate the Forge fleet as one operator. | Forge identity module; existing project identity contract remains intact. | **keep local** |
| Forge portal | `src/api/ui/routes.rs`, `src/api/ui/render.rs`, `src/api/ui/data.rs` | Existing Rust-rendered HTML transport, fleet observations and project detail pages. | Existing entry asks for project ID; landing layout is a sparse table; UI markup is coupled to API Rust modules. | Forge API owns JSON/auth/data; `frontend/` owns browser HTML/CSS/JS and runs independently. | **split boundary** |

## BFS Impact Map

- **Actors and flow:** Forge operator initializes one admin, opens the separately served frontend, signs in with email/password, manages the fleet and signs out.
- **State and persistence:** one Forge-wide admin password hash and expiring revocable sessions stored in the Forge registry database; no per-project credential storage.
- **Contracts and callers:** CLI `forge identity setup`; browser calls JSON session endpoints and an authenticated fleet endpoint; existing `/v1` bearer authentication is unchanged. The frontend is served independently from the Rust API.
- **Integration/configuration:** existing SQLite registry, Argon2id PHC password hashes, opaque random session tokens, HttpOnly host cookie. The AllTools source is a visual reference only; no sibling import, DB or runtime dependency.
- **Failure boundaries:** no configured admin explains the setup command; malformed JSON, invalid credentials, expired/revoked sessions and missing state fail closed with safe API errors; login errors do not reveal account existence; password/token values are never logged or stored in plaintext.
- **UI:** split brand/login panel at sign-in, responsive Forge sidebar and dashboard cards/table, using actual registered projects and actual operation evidence. Keep all HTML, CSS and JavaScript in `frontend/`; project IDs appear only after login where project navigation requires them.
- **Compatibility/security:** project OIDC API flow, existing `/v1` routes, CLI identity operations, project detail and mutation permission checks remain intact; new `/v1/admin/*` endpoints require a valid Forge global-admin session.
- **Verification:** Rust module/API contract, portal contract, password/session tests, standalone frontend checks, format/build and local API plus frontend browser smoke.
- **Unaffected:** deployment providers, workspace registry adapters, generated apps, AllTools source, public APIs and account registration/recovery are unchanged.

## Capabilities

- `forge-admin-login`: initialize and authenticate the single Forge operator and manage a global browser session.
- `portal-web-ui`: provide a human-friendly Forge-wide fleet dashboard after authentication.

Source requirements: `requirement.md` sections 31, 35 and 36; archived `portal-login-entry-flow` and `portal-accessible-responsive-ui` requirements are superseded for browser entry and landing-page presentation by this change.

## Non-goals

- No project ID, project OIDC setup or project identity provider is required at Forge sign-in.
- No registration, social provider buttons, password recovery, multi-user roles, tenant model, billing or external identity-provider setup.
- No change to project-scoped OIDC tokens used by project API routes.
- No import of AllTools templates, auth code, database, runtime, or static assets.
- No fabricated metrics, placeholder links, or fake project actions in the dashboard. No HTML/CSS/JS embedded in Rust or mounted under API route handlers.
