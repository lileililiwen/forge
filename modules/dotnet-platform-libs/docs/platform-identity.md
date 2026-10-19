# Platform identity and authorization

The identity capability is split into provider-neutral contracts, an optional ASP.NET Core adapter, an optional EF Core adapter, and deterministic test providers.

## Local development

Register `AddPlatformIdentity()` in the application service collection. Tests and local tools can register `FakeCurrentUserAccessor`, `FakeCredentialVerifier`, `FakeExternalIdentityProvider`, or `FakeVerificationProvider` explicitly. The default current-user value is anonymous and does not throw when no request identity exists.

Applications register their own permission catalog entries and then call `RequirePlatformPermission("resource.action")`. Platform packages do not define product roles or permissions.

## Production configuration

Applications own authentication handlers, token formats, user entities, tenant resolution, and external provider credentials. Call `AddPlatformIdentityAuthentication()` only when the application intentionally owns the platform scheme name; application-owned schemes may be registered separately. Claims are projected into `CurrentUser` through `HttpCurrentUserAccessor` using subject, email, tenant, role, and permission claims.

Provider adapters must return `IdentityProviderResult<T>` with a normalized `IdentityFailureReason`. Provider response bodies, secrets, and vendor exceptions must not be placed in the result.

## Persistence and replacement

`Platform.Identity.EntityFrameworkCore` supplies `IIdentityStore` and `IdentityDbContextAdapter`, but no user entity, schema, migration, or business relationship. Applications derive the adapter and configure their own model. Replace the store and all external providers through the public interfaces.

Authorization decisions can be observed through `IAuthorizationDecisionAuditor`. Security-sensitive identity mutations use `IIdentityAuditHook`; the later admin/audit capability can provide the durable implementation.

## Migration guidance

Adopt contracts first, map the application's existing user and claims model to `CurrentUser`, then add the ASP.NET Core adapter and named policies. Add the EF adapter only after the application has selected its entity model and migration ownership. Keep provider SDK references in application/provider adapter projects, never in contract packages.

## Host integration

`Platform.Identity.AspNetCore` is the optional host adapter. It projects configured claims into `CurrentUser`, exposes permission/role policies, provides replaceable store seams, records authorization audit events, and validates explicitly selected JWT configuration.

### Claim mapping

`PlatformIdentityOptions` exposes configurable claim types: `SubjectClaimType` (default `ClaimTypes.NameIdentifier`, falls back to `sub`), `EmailClaimType` (default `ClaimTypes.Email`, falls back to `email`), `TenantClaimType`, `PermissionClaimType`, and `AuthenticationScheme`. Unauthenticated requests always resolve to `CurrentUser.Anonymous`. Roles and permissions are deduplicated case-insensitively.

### Authorization and audit

`RequirePlatformPermission("resource.action")` and `RequirePlatformRole("role")` register named policies. The permission handler succeeds the requirement for authenticated subjects carrying the permission and records a denied `IdentityAuditEvent` ("authorization.denied") to a registered `IIdentityAuditHook` without any token contents. `IAuthorizationDecisionAuditor` continues to receive the normalized authorization decision.

### Replaceable stores

Applications provide their own persistence through `AddPlatformIdentityCredentialVerifier<T>`, `AddPlatformIdentityExternalProvider<T>`, `AddPlatformIdentityVerificationProvider<T>`, `AddPlatformIdentitySessionStore<T>`, and `AddPlatformIdentityAuditHook<T>` (all `TryAdd`). `AddPlatformIdentityStore<T>` lives on `Platform.Identity.EntityFrameworkCore`. `IIdentitySessionService` wraps the registered `ISessionStore`, preserving `IdentityProviderResult<T>` outcomes and reporting `ProviderUnavailable` when no store is registered.

### JWT configuration validation

When an application selects JWT authentication it calls `AddPlatformIdentityJwt(o => { o.Enabled = true; ... })`. On host startup the platform validates that `SigningKey`, `Issuer`, and `Audience` are present and fails fast with a secret-free message when configuration is missing. `PlatformIdentityJwtOptions.GetDiagnosticName()` returns a redacted view for logs and health surfaces that never includes the signing key.

## OpenID Connect sign-in

`Platform.Identity.OpenIdConnect.AspNetCore` is a separate, opt-in adapter. An application calls `AddPlatformOpenIdConnect(...)` with its own issuer, client id, callback path, sign-in scheme, and scope allowlist; an application that does not call it keeps its local authentication unchanged and gains no OIDC scheme, route, or configuration requirement.

The adapter owns the scheme registration and the callback validation only: authorization code with PKCE, one-use state, discovery issuer, signature, audience, redirect URI, nonce, and token time bounds. Provider role claims are removed from the validated principal unless the application names a `RoleClaimType`, so an external role grants no local permission by default.

Application users, enrollment, sessions, roles, and permissions stay with the application. The adapter ships no user table, migration, or session store, and it never creates or links an identity: the application resolves the validated `(issuer, subject)` pair through its own store, either by registering an `IPlatformOpenIdConnectIdentityResolver` or by reading the identity that the adapter stores in the ticket properties. A confidential client names a secret *reference* (`config:Section:Key` or `env:VARIABLE_NAME`); the adapter never holds a secret value in its options and never logs one.

Failures arrive as a `PlatformOpenIdConnectException` with a classified kind (`InvalidConfiguration`, `InvalidResponse`, `AccessDenied`, `ProviderUnavailable`, `ApplicationMapping`) and a stable reason code. The message never carries a code, token, secret, or raw claim value.

## Migration from the starter Identity module

- Register `AddPlatformIdentity()` before endpoint mapping; keep the application's own authentication scheme and token issuance unchanged.
- Map the starter's claims to the configured `SubjectClaimType`/`EmailClaimType`/`TenantClaimType`/`PermissionClaimType` rather than copying starter claim names into the platform.
- Replace individual starter services incrementally: register the application's `ISessionStore`, `ICredentialVerifier`, or `IVerificationProvider` through the platform seams instead of the starter singletons.
- Keep product roles and permissions in application code; the platform only evaluates whatever claims the application projects.

## Rollback

Remove `AddPlatformIdentityJwt` and the platform seams to revert to the starter's identity host. No platform-owned user entity, migration, JWT key, or permission catalog is created, so removing the adapter registration leaves the application's existing identity module intact. Claim compatibility is preserved because the platform reads whichever claim types the application configures.
