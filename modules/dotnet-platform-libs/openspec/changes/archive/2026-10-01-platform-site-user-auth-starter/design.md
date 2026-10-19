# Design: Optional site-local user authentication starter

## 1. Implementation boundary

Repository `/home/paul/code/dotnet-platform-libs`; C# on pinned .NET SDK `10.0.400`, `net10.0`. Extend `templates/platform-application-starter/.template.config/template.json`, `Program.cs`, generated data/pages/config, the pack project and `tests/Platform.Template.Tests/`. Keep all generated user entities, EF mappings, migrations, pages, and permission registrations in the detached consumer output. `Platform.Identity.Contracts`, `Platform.Authorization`, and `Platform.Identity.Testing` remain reused packages; no platform package may add a platform-owned user entity or migration.

## 2. Language and runtime

Generated host is ASP.NET Core 10 with ASP.NET Core Identity, EF Core 10, and the template-selected EF provider. The template pack itself targets `net10.0`. Verification: `dotnet pack`, `dotnet new install`/template render, generated-app restore/build/test, plus repository `./scripts/quality-gate.sh` and strict OpenSpec validation.

## 3. Ownership and shared code

The template pack owns rendering only. Generated application owns `ApplicationUser`, `ApplicationIdentityDbContext`, migration history, role/permission seed data, and authentication cookie configuration. The application references the existing identity lifecycle and authorization packages when selected. Consumers of other stacks receive no implicit dependency; Forge can opt into this feature only on a compatible ASP.NET Core profile in a later change.

## 4. Behavioral model

Add template option `EnableSiteUsers` (default `false`), independent of existing `EnableIdentity` (lifecycle endpoints) and `EnableAdmin` (manager-style admin capability). When false, generated tree contains no Identity EF/UI references, routes, tables, or cookies. When true, render:

- application-owned `ApplicationUser : IdentityUser` and `ApplicationIdentityDbContext`;
- cookie authentication and ASP.NET Core Identity sign-in/sign-out/forgot-password Razor UI with the starter layout;
- self-registration disabled by default; app owner enables it explicitly in configuration;
- app-owned `PermissionCatalog` registration and policy mapping; generated sample `site.profile.read` and `site.profile.update`, with deny-by-default for unknown permissions;
- a `bootstrap-owner` command that generates a random password, prompts for the owner email, hashes through ASP.NET Identity, and prints the generated password once only in Development; it refuses to run in Production;
- test-only authentication through `Platform.Identity.Testing`, never referenced by production projects.

The app uses its selected connection string/provider; the template does not create a database server. Each site stores users locally in its own database. The feature introduces an application-owned initial EF migration so fresh generated apps are runnable.

## 5. Contract and compatibility

Template parameter is `EnableSiteUsers` (boolean, default false). Generated routes are `/Identity/Account/Login`, `/Identity/Account/Logout` (POST), and the standard Identity recovery route; exact paths are emitted in the generated README and route tests. Configuration keys: `SiteUsers:Enabled`, `SiteUsers:SelfRegistrationEnabled`, `SiteUsers:Cookie:SecurePolicy`; production validation rejects weak/empty cookie configuration and `DevelopmentOnly` auth. Permission names use stable `{resource}.{action}` keys and `PlatformPolicyNames.ForPermission`. Existing template options and package identities remain unchanged.

Errors: template incompatibility fails generation with a named option diagnostic; missing production persistence or secure cookie settings fails startup/readiness; unknown permission denies access; invalid sign-in returns a generic message with no account-enumeration signal.

## 6. Failure and boundary policy

Feature disabled → no auth behavior or schema. Missing/malformed connection string → startup/readiness classified configuration failure. Password mismatch/unknown email → identical public response. Lockout → generic auth failure and recorded safe audit event. Missing owner → sign-in remains unavailable until explicit Development-only bootstrap command. Public registration is disabled until the application owner enables it. In Production, development bootstrap/fake authentication causes startup failure. Removing the feature from an already-used site is a migration/data-retention decision and is not automated by this template change.

## 7. Verification oracle

Template tests verify disabled output excludes identity dependencies/routes, enabled output contains exact files and dependencies, and install/render is deterministic. A generated fixture uses an isolated SQLite provider for route tests: login success/failure, generic unknown-user response, sign-out POST/CSRF, lockout, permission allow/deny, disabled self-registration, and production rejection of development bootstrap. Build and test the detached output without a template-pack project reference. Architecture tests prove production platform packages do not own user entities/migrations and test fakes are test-only.

## 8. Decision ledger

Resolved: site-local identities, no cross-site SSO; ASP.NET Core Identity is the first-stack implementation; generated app owns persistence and migrations; self-registration is off by default; Development bootstrap uses a random one-time password; permission evaluation denies by default. Deferred: non-.NET stack auth templates, social/OIDC login for site users, multi-tenant identity, passkeys/MFA UI, and migration of existing products. Blockers: none for the .NET template slice.
