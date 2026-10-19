# StarterApp

Generated from the `platform-app` template (`Platform.Application.Template`).
This source is application-owned and detached: it has no project reference
to the template pack and needs no template package at runtime.

## Run

```bash
dotnet restore
dotnet run
```

The host exposes `/` (this application), `/live` (liveness), and `/ready`
(readiness) with safe development defaults and no production provider
credentials. Enable capabilities explicitly in `Program.cs` and register
application-owned stores and providers before deployment.

//#if (EnableSiteUsers)
## Site user authentication

The template renders an opt-in site-local user authentication slice when
the `EnableSiteUsers` template option is true. The feature is
application-owned: the user record (`ApplicationUser`), EF Core context
(`ApplicationIdentityDbContext`), initial migration, cookie configuration,
permission catalog, and Razor pages live entirely in this generated
source. The platform never owns application users or migrations.

### Generated routes

| Route | Method | Purpose |
|---|---|---|
| `/Identity/Account/Login` | GET/POST | Sign in. Returns an identical generic message for unknown email and wrong password; never reveals which condition occurred. |
| `/Identity/Account/Logout` | POST | Sign out. Requires an antiforgery token. GET requests show a confirmation form. |
| `/Identity/Account/ForgotPassword` | GET/POST | Begin a recovery flow. Returns the same response whether or not the email matches an account. |
| `/Identity/Account/AccessDenied` | GET | Shown when authorization fails. |

### Configuration

The feature reads from the `SiteUsers` section in `appsettings.json`:

| Key | Default | Description |
|---|---|---|
| `SiteUsers:Enabled` | `true` | Master switch for the feature at runtime. |
| `SiteUsers:SelfRegistrationEnabled` | `false` | Opt-in public registration. Off by default to avoid anonymous sign-ups. |
| `SiteUsers:Cookie:SecurePolicy` | `SameAsRequest` | `Always`, `SameAsRequest`, or `None`. Production requires `Always`. |
| `SiteUsers:Cookie:SameSite` | `Lax` | `Strict`, `Lax`, or `None`. |
| `SiteUsers:Cookie:LifetimeMinutes` | `60` | Cookie expiration. |
| `SiteUsers:AuthenticationScheme` | `SiteUsers` | Application-owned scheme name. |
| `SiteUsers:DevelopmentOnly` | `false` | Refuse in Production when true. |

Production startup fails when `SiteUsers:Cookie:SecurePolicy` is not
`Always` or when `SiteUsers:Cookie:Name` is empty.

### Permission catalog

`StarterApp.SiteUsers.SiteUserPermissionCatalog` registers two sample
permissions:

| Key | Policy name |
|---|---|
| `site.profile.read` | `platform:permission:site.profile.read` |
| `site.profile.update` | `platform:permission:site.profile.update` |

Unknown permission keys resolve to `AuthorizationDecision.Denied` because
the platform's permission handler only succeeds when the current user's
`PermissionSet` contains the key.

### Bootstrap owner (Development only)

A `bootstrap-owner` sub-command generates a one-time random password,
hashes it through ASP.NET Identity, and prints the password once. It
refuses to run in Production.

```bash
ASPNETCORE_ENVIRONMENT=Development dotnet run -- bootstrap-owner --email owner@example.com
```

The first run applies the included `SiteUsers/Migrations/InitialIdentitySchema`
migration to the configured connection string. Subsequent runs fail with a
clear error if the owner already exists.

### Test environment

The test project (`tests/StarterApp.Tests/StarterApp.Tests.csproj`)
references `Platform.Identity.Testing` to support deterministic fake
authentication in integration tests. Production code never references
the testing package.
//#endif

## Versions

Package references pin exact versions (`Platform.* 0.1.0`, third-party
versions as generated). To adopt newer platform packages, update the
`Version` attributes and rebuild; to abandon the template, keep or delete
this source — uninstalling the template pack never affects it.
