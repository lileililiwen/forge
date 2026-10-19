# Proposal: Optional site-local user authentication starter

## Why

The platform already has identity lifecycle, authorization, and test contracts, but it intentionally leaves each application to create its own user store and login surface. New ASP.NET Core projects still repeat the login UI, cookie setup, initial-user bootstrap, role/permission wiring, and test setup.

## What Changes

- Extend the detached application template with an opt-in site-local user-auth feature.
- Generate ASP.NET Core Identity persistence and branded Razor login/logout/account pages into the application itself.
- Generate application-owned permission policies and a safe development/test setup; keep the feature disabled by default.

## Package Boundary and Split Assessment

One template capability creates a complete runnable slice: user persistence, authentication UI, authorization policies, and environment behavior must agree. The library does not own application users or migrations; generated code owns them. UI, API endpoints, and persistence split signals share one template option and one generated-consumer oracle.

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `platform-site-user-auth-starter` | Generate an opt-in site-local login and permission slice | dotnet-platform-libs, C#/.NET 10 | `platform-app` template option `EnableSiteUsers` | existing starter, Identity/EF Core and authorization packages | generated app build, route tests, auth boundary checks |
| Forge adoption | Offer the template capability from Forge's .NET web profile | Forge, Rust | existing deterministic profile feature descriptor | this package after publish | profile render/build fixture |

Forge adoption is a separate consumer change. This change may update the versioned template pack only; it must not edit any consumer repository.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| `Platform.Identity.*` | `docs/platform-identity-lifecycle.md`, `src/Platform.Identity.AspNetCore/` | lifecycle contracts, authorization integration, testing fakes | no user entity/store, login UI, or generated auth slice by design | dotnet-platform-libs packages | **extend shared owner** through an opt-in generated template, not a universal user store |
| application starter | `templates/platform-application-starter/`, `Platform.Application.Template` | detached generated app, `EnableIdentity`, persistence and test switches | `EnableIdentity` is lifecycle composition, not a complete login experience | dotnet-platform-libs template pack | **adopt** and add a distinct option |
| `platform-contracts` / Rust platform libs | identity-subject and permission-decision contracts; `platform-identity` | language-neutral subjects/permission decisions and Rust traits | neither owns HTTP, pages, or an application user DB | contract/platform releases | **adopt** for vocabulary only; no schema change |
| Hypora, OpenPanel, Hermora | their local auth implementations | evidence that manager admin identity is product-owned | manager logins are not generated-site end-user auth | each product release | **keep local**; do not couple admin accounts to this template |

## BFS Impact Map

- **Consumers:** new ASP.NET Core sites generated from the platform template; existing apps are untouched unless they opt in.
- **Generated state:** `EnableSiteUsers=false` by default; when true, application-owned `ApplicationUser`, EF Identity context/migration boundary, cookie auth, Razor pages, permission catalog/policies, and bootstrap tooling are rendered.
- **Identity boundary:** each site keeps its own users, sessions, roles, and password hashes. There is no cross-site account or SSO behavior.
- **Environments:** production requires the configured persistent store and secure cookie settings; test uses deterministic fake authentication; development uses a one-time generated local owner credential and no committed default password.
- **Failure/security:** fail startup for missing production secrets or development-only auth mode, deny unknown permissions by default, lock out repeated password failures, and never expose hashes/tokens in logs.
- **Compatibility:** `EnableIdentity` retains its current lifecycle meaning; the new option is additive and independent.

## Capabilities

- **Modified:** `platform-application-starter`.
- **New:** `platform-site-user-auth-starter`.

## Non-goals

- No central identity server or shared end-user account across projects.
- No migration of existing applications, OpenPanel/Hermora manager auth, OAuth/social login, billing, multi-tenant SaaS, or custom roles beyond generated app-owned policy definitions.
- No identity persistence or user schema owned by a `Platform.*` runtime package.
