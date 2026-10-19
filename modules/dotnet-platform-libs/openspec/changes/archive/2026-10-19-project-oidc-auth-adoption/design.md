## Implementation Boundary

Owner: `forge/modules/dotnet-platform-libs`, C#/.NET module. Add a separate independently packaged `Platform.Identity.OpenIdConnect.AspNetCore` package. It integrates with ASP.NET Core authentication and the existing `Platform.Identity.Contracts` / `Platform.Identity.AspNetCore` surfaces. It does not edit Forge source or provider packs, `Platform.Identity.Contracts`, existing local-auth templates, `platform-contracts`, or Rust libraries.

## Language and Runtime

C# targeting `net10.0`, following the current module package baseline. ASP.NET Core 10 shared framework; Microsoft-maintained `Microsoft.AspNetCore.Authentication.OpenIdConnect` dependency pinned centrally to the module's compatible ASP.NET Core version. Use the module's existing solution, package-manifest, architecture-test, quality-gate, feed-packaging and public-API conventions. Do not add a third-party vendor SDK.

## Ownership and Shared Code

`Platform.Identity.Contracts` remains the provider-neutral user/identity contract owner. `Platform.Identity.AspNetCore` remains current-user and authorization projection owner. The new package owns only OpenID Connect authentication-scheme registration and token/callback validation. The consuming application owns local user lookup/creation, its external identity link keyed by issuer and subject, session lifetime/cookie policy, registration policy, claims-to-local-role mapping, persistence, and permissions. Package dependency direction: OIDC ASP.NET adapter → identity contracts + host adapter; consumer → these packages. Package/feed release is independent; consumers pin exact published versions. No source project references across sibling repos.

## User Experience and Interface

N/A for a library package: it provides no route, form, or browser asset. Evidence: `docs/packages.md` lists reusable library APIs; `platform-site-user-auth-starter` owns site login/register pages. The consuming application must expose a labeled login action, distinguish provider-unavailable and denied states, return focus after callback/error, and preserve its existing accessible local-registration experience when enabled. Forge's provider setup package owns browser verification of the end-to-end generated app.

## Behavioral Model

1. Application explicitly registers the OIDC scheme and provides provider configuration.
2. Anonymous protected request redirects only to the configured issuer using authorization code + PKCE, one-use state and nonce.
3. Callback validation checks discovery issuer, signature/JWKS, audience/client ID, redirect URI, state, nonce, token lifetime and required subject.
4. On success, app callback resolves or creates one app-local identity keyed by normalized `(issuer, subject)` according to the app's explicit enrollment policy. New local user creation is disabled unless app opts into it.
5. App maps only explicitly configured claims to local authorization. External roles/permissions are not trusted as local grants by default.
6. App creates its own local session/cookie and applies its own sign-out/session revocation policy.

The package has no global state and makes no cross-project identity link. An exact repeat callback/state is rejected after one use; no automatic retry of an authorization callback.

## Contract and Compatibility

- Public package: `Platform.Identity.OpenIdConnect.AspNetCore`; explicit `AddPlatformOpenIdConnect(...)` registration extension and typed options; no automatic registration through existing `AddPlatformAspNetCore`.
- Required options: absolute HTTPS issuer/authority, client ID, callback path, sign-in scheme, credential source/reference when required by selected confidential-client mode, explicit scope allowlist, and token/claim mapping policy. No implicit discovery fallback to another issuer.
- Successful authentication returns a validated external principal; application code resolves local users through its existing `IIdentityStore`/external identity contract. Missing application mapping fails closed with a typed, redacted outcome.
- Failures classify as invalid configuration, invalid response/token, access denied, provider unavailable, and application mapping failure. Responses and logs never include code, token, secret, or full claim values.
- Existing local authentication and platform contracts retain behavior. OIDC is opt-in and does not alter schema or cookie semantics of existing users.

## Failure and Boundary Policy

| Condition | Result |
|---|---|
| Missing/invalid issuer, client, callback or secret reference | Fail startup validation; do not start an insecure scheme |
| Non-HTTPS external issuer | Refuse outside explicitly isolated local development; local HTTP policy is governed by a separate tested development-only option, not silent relaxation |
| Issuer, audience, signature, state, nonce, redirect or expiry mismatch | Reject callback; no session, user creation, or authorization |
| Provider discovery/JWKS unavailable | Return provider-unavailable; no local identity mutation |
| Valid external identity but local enrollment disabled/no mapping | Deny sign-in; do not auto-create account |
| Missing/unknown role/permission mapping | Authenticate only if app policy permits; grant no local permission |
| Duplicate issuer+subject link | Reject as conflict; do not relink or merge accounts |
| Logout | Clear only the application's own session; provider-wide logout is not implied |

## Verification Oracle

After implementation authorization: add deterministic unit tests for configuration validation and claim mapping; security-focused integration tests using a local OIDC test issuer with signing keys for valid callback, wrong issuer/audience/signature/state/nonce/expiry, one-use state, provider outage, duplicate link, no auto-enrollment, and isolated local authorization. Run the package test suite, module architecture tests, package manifest/public API checks, full module quality gate, and strict OpenSpec validation. A generated consumer browser journey and accessibility evidence are required in the separate Forge adoption package; library tests alone do not prove login UX.

## Implementation Plan

**Package.** `src/Platform.Identity.OpenIdConnect.AspNetCore` targets `net10.0`,
declares the `Microsoft.AspNetCore.App` framework reference, the pinned
`Microsoft.AspNetCore.Authentication.OpenIdConnect` package, and one project
reference to `Platform.Identity.Contracts` for the shared external identity
shape. It does not reference `Platform.Identity.AspNetCore`: the adapter adds
no current-user projection of its own, so the host adapter stays optional for
the application. The ASP.NET Core OpenID Connect handler ships as its own
Microsoft package rather than inside the shared framework, which is why the
package reference exists and why the architecture test allows exactly that one
package for this project.

**Public surface.**

| Type | Purpose |
|---|---|
| `PlatformOpenIdConnectOptions` | Issuer, client id, callback path, sign-in scheme, scope allowlist, client authentication mode, secret reference, loopback-only HTTP escape hatch, role claim type, clock skew, state lifetime, identity-resolution mode, scheme name, backchannel handler, event hook. |
| `AddPlatformOpenIdConnect(...)` | Explicit opt-in registration on `IServiceCollection`. |
| `PlatformOpenIdConnectOptionsValidator` | Startup validation of the options above. |
| `IPlatformOpenIdConnectSecretResolver` / `PlatformOpenIdConnectSecretResolver` | Resolves `config:Section:Key` and `env:VARIABLE_NAME` references; never holds a secret value in the options. |
| `IPlatformOpenIdConnectStateReplayGuard` / `InMemoryPlatformOpenIdConnectStateReplayGuard` | One-use callback state, per process by default, replaceable for multi-instance deployments. |
| `PlatformOpenIdConnectExternalIdentity` / `PlatformOpenIdConnectPrincipal` | The validated `(issuer, subject)` pair and the way to read it back. |
| `IPlatformOpenIdConnectIdentityResolver` / `PlatformOpenIdConnectIdentityResolution` | The application's own identity seam and its answers. |
| `PlatformOpenIdConnectException` / `PlatformOpenIdConnectFailureKind` | Redacted failure classification. |

**Protocol posture.** The adapter registers the ASP.NET Core OpenID Connect
scheme with `ResponseType = code`, `UsePkce = true`, `SaveTokens = false`,
`GetClaimsFromUserInfoEndpoint = false`, `MapInboundClaims = false`, the
configured scope allowlist, and `RequireHttpsMetadata` unless the loopback
development option is set. A post-configure step pins `ValidIssuer` to the
configured issuer, `ValidAudience` to the client id, lifetime validation, the
configured clock skew, and the role claim type.

**Adapter validation.** The adapter's own event handlers run before any
application handler, so an application can observe the flow but cannot skip the
adapter's guarantees. They check the issuer claim against the configured
issuer, the audience against the client id, the presence of a subject and a
nonce, consume the callback state once, remove unmapped provider role claims,
and — when the application asks for it — call the application's identity
resolver. The validated issuer and subject are written into the ticket
properties, because the ASP.NET Core handler removes the protocol claims such
as `iss` from the signed-in principal.

**Failure classification.** The provider backchannel is wrapped so a transport
failure or a provider server error becomes a `ProviderUnavailable` outcome
instead of an opaque exception. Provider-signalled errors, token validation
failures, and application mapping refusals are classified as `AccessDenied`,
`InvalidResponse`, and `ApplicationMapping`. Every message is fixed text; no
code, token, secret, or raw claim value is ever included.

**Tests.** `tests/Platform.Identity.OpenIdConnect.AspNetCore.Tests` contains a
local OpenID Connect issuer with its own RSA signing key, a discovery
document, a JWKS, an authorization endpoint, and a token endpoint that checks
the PKCE verifier. The tests drive a real application over `TestServer` and
cover the valid callback, a token signed with another key, a token from another
issuer, a token for another audience, a token with another nonce, an expired
token, a replayed callback, an unreachable provider, a provider server error,
a provider denial, failure redaction, the option validation, the one-use state
guard, and the secret reference resolver. The ownership tests cover the
unmapped provider role, the explicit role mapping, the linked identity, the
disabled enrollment, the duplicate-link conflict, and two applications that
resolve the same subject from their own stores.

## Decision Ledger

- **Resolved:** separate optional package; no vendor SDK; OIDC authorization-code + PKCE; local application user/authorization ownership.
- **Resolved:** OIDC does not define registration; application enrollment must be an explicit policy and cannot be inferred from provider login.
- **Resolved:** shared administrator SSO is a separate workspace concern; product-user identity is not shared by default.
- **Resolved:** the adapter depends on `Platform.Identity.Contracts` only; the host adapter stays optional.
- **Deferred:** provider-specific self-service registration and recovery; owned by provider packs or app-local auth, not this OIDC client.
- **Implementation authorized (2026-10-19):** the owner directed implementation of this package; the new package, its tests, and the module records that describe them are the only authorized surfaces.

## Requirement Traceability

| Requirement | Proposal outcome | UX flow / N/A | Design decision | Implementation boundary | Success/failure/boundary scenarios | Tasks | Verification oracle |
|---|---|---|---|---|---|---|---|
| OIDC host integration is optional and provider-neutral | Optional generic OIDC package | N/A: library has no UI; consuming app flow is owned by Forge adoption | Separate explicit package, no vendor SDK | New `Platform.Identity.OpenIdConnect.AspNetCore` | Opt-in, absent package, compatible provider | 2.1, 3.1 | Package consumer tests; no behavior/config changes when absent |
| OIDC responses are validated before sign-in | Secure validated callback | N/A: library boundary; end-to-end browser flow in Forge package | PKCE, one-use state/nonce, full issuer/token validation | New ASP.NET Core adapter | Valid callback, invalid callback, replay | 2.2, 3.3 | Signed local OIDC issuer integration tests |
| Product user records and permissions remain application-owned | Isolated app users and local permissions | N/A: consuming app owns enrollment and authorization UI | App resolves issuer/subject locally; no auto-enrollment/grant | Adapter plus application identity store | Existing link, enrollment disabled, two apps, unmapped roles | 2.3–2.4, 3.2 | Two independent stores with same external subject; denied unmapped claim |
| OIDC configuration and failures are safe | Safe configuration and provider failure | N/A: library options and typed outcomes; consumer renders errors | Startup validation, redacted failures, no mutation on outage/conflict | Adapter options and identity-store boundary | Invalid configuration, provider outage, duplicate link | 2.2–2.5, 3.3 | Config, outage, duplicate, secret-redaction tests |
