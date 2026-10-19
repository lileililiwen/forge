# Proposal: Add opt-in project-owned OIDC sign-in

## Why

A generated ASP.NET Core project currently repeats authentication wiring, while the shared identity layer deliberately avoids vendor and application-user ownership. A small optional OIDC host adapter can provide low-effort sign-in without making product accounts, permissions, or persistence global.

**Implementation authorized (2026-10-19):** the owner directed implementation of
this package. The authorized surface is the new
`Platform.Identity.OpenIdConnect.AspNetCore` package, its tests, and the module
records that describe it (solution, package manifest, architecture tests,
package docs). No template edit, feed publication, provider deployment, or
sibling-project change is authorized.

## What Changes

- Add an independently adoptable `Platform.Identity.OpenIdConnect.AspNetCore` adapter using standard OIDC authorization-code flow with PKCE.
- Keep application user records, sessions/cookies, role/permission assignment, and local account projection owned by each application.
- Keep local registration and provider-hosted account enrollment explicit opt-ins; do not claim OIDC standardizes user registration.
- Require a per-project provider population/issuer and reject implicit cross-project account or authorization sharing.

## Package Boundary and Split Assessment

This package owns reusable ASP.NET Core OIDC host integration only. Forge's local provider setup/opt-in UI and Keycloak/Supabase provider packs are a separate Forge-owned package; workspace policy is owned by `forge/modules/workspace-governance`. They have independent release cycles and acceptance oracles. No provider SDK, user database, frontend, or provider deployment belongs here.

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `provider-neutral-identity-policy` | Provider-neutral OIDC policy and identity population boundaries | `forge/modules/workspace-governance`; policy data | OIDC | None | Decision and audit |
| `project-oidc-auth-adoption` | Reusable ASP.NET Core OIDC sign-in adapter with app-owned users | `forge/modules/dotnet-platform-libs`; C#/.NET 10 | OIDC; `Platform.Identity.Contracts` | Governance policy | Consumer app authenticates with a standards-compatible issuer without shared user storage |
| `forge-auth-provider-extensions` | Simple opt-in provider setup and provider-specific quick-starts | `forge`; Rust + provider-pack boundary | OIDC and Forge provider extension | Governance and this package for ASP.NET apps | Local Keycloak and explicit Supabase setup remain replaceable and project-scoped |

Dependency order: governance policy → this library → Forge adoption for ASP.NET consumers. This package is the smallest independently adoptable code boundary; it does not provision the external identity provider.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| Existing identity contracts and host adapter | `docs/packages.md`, `Platform.Identity.Contracts`, `Platform.Identity.AspNetCore` | Provider-neutral current-user projection, authorization, identity interfaces | Existing adapter intentionally has no third-party auth client or OIDC flow | .NET module packages released independently | **extend shared owner** with a separate optional adapter package |
| Site-local auth starter | `openspec/specs/platform-site-user-auth-starter/spec.md` | App-owned users, roles, permissions, migrations, login/register flow | No reusable OIDC host setup | .NET module template lifecycle | **adopt** app-local ownership; make OIDC a separate explicit mode |
| Forge admin OIDC | `forge/openspec/specs/central-admin-identity/spec.md` | PKCE, strict issuer/audience/nonce checks, per-project sessions | Rust admin-only implementation; not a .NET library | Forge core release | **adapt through a generic adapter** by following OIDC security invariants, not sharing code |
| Platform contracts | `platform-contracts/openspec/specs/contract-foundation/spec.md` | Versioned cross-language payloads | OIDC itself is already the standard protocol; no custom shared payload is needed | Separate contract repository | **keep local**; no new platform-contracts dependency |
| Rust identity platform | Rust module docs and prior identity scope | Provider-neutral identity contracts | Existing policy excludes OIDC host concerns; no recurring Rust consumer proven | Rust module release | **keep local** until multiple Rust consumers exist |

## User Experience and Interface Impact

Actor: a developer adopting an ASP.NET Core app. Goal: add a login entry with issuer/client configuration and retain application-owned authorization. Entry point is package registration and the existing app login/register screen. The primary flow redirects to the configured issuer, returns through a validated callback, then maps the external subject to an application-local user. The alternate local-account flow remains available only when explicitly enabled. The adapter itself does not own HTML; the existing application starter owns labels, loading/error states, keyboard flow, and accessibility. A consumer-facing quickstart must show one-provider setup and explain that registration is provider/local-app policy, not an OIDC operation. Browser flow and accessibility checks belong to Forge's consumer adoption package, not this library-only package.

## BFS Impact Map

- **Capabilities:** optional OIDC host adapter; provider-neutral current user; application-owned identities and authorization.
- **Data:** stable external `(issuer, subject)` association is stored by the app's existing identity store; no platform-owned user table, migration, or session table.
- **Configuration:** explicit issuer/authority, client ID, callback, and required application secret reference; secrets are never logged or embedded in source. Per-app issuer/client/audience validation required.
- **Callers:** ASP.NET Core applications and later Forge-generated project adoption; existing contracts and local auth remain compatible.
- **Failure/security:** invalid issuer/audience/signature/state/nonce/time-bound token fails closed; provider outage does not create a user; no automatic cross-app permission transfer.
- **Unchanged:** provider provisioning, Forge CLI, Supabase/Keycloak SDKs, Rust, shared contract schemas, and deployment.

## Capabilities

- `project-oidc-auth-adoption` (new): optional ASP.NET Core OIDC integration that preserves project-owned users and authorization.

## Non-goals

- Selecting an identity vendor or provisioning realms/projects/users in an external provider.
- Standardizing OIDC registration, account recovery, email verification, password policy, or provider-hosted UI.
- Platform-owned user entities, database schema, migrations, roles, cookies shared across apps, or product permissions.
- Making OIDC default for every generated application or changing local-auth behavior.

## Authorized Files

| File | Change |
|---|---|
| `src/Platform.Identity.OpenIdConnect.AspNetCore/*` | New package: typed options, startup validation, scheme registration, replay guard, failure classification, identity-resolution seam. |
| `tests/Platform.Identity.OpenIdConnect.AspNetCore.Tests/*` | New test project: configuration, callback validation against a local signing-key issuer, ownership, and redaction tests. |
| `Platform.sln` | The new package and test project. |
| `Directory.Packages.props` | The pinned `Microsoft.AspNetCore.Authentication.OpenIdConnect` version. |
| `eng/package-manifest.json` | Regenerated from source. |
| `tests/Platform.Architecture.Tests/DependencyDirectionTests.cs` | The new production project and its dependency direction. |
| `docs/packages.md`, `README.md`, `CHANGELOG.md` | The package's documented public surface. |
