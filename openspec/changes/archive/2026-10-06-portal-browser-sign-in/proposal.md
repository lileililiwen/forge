# Proposal: Let operators sign in to the Forge browser portal

## Why

`GET /ui` requires a session ID, but ordinary browser navigation cannot attach Forge's bearer header; its `?token=` escape hatch exposes the credential in history and referrers. The existing identity surface can validate and persist project-scoped sessions but does not complete a browser OIDC provider round trip, leaving the portal without a normal sign-in experience.

## What Changes

- Implement browser OIDC authorization-code sign-in with PKCE, state and nonce, using a registered project identity configuration to initiate and complete provider authentication.
- Validate discovery issuer, authorization response state, token signature, issuer, audience, expiry, nonce, and the configured admin claim before issuing a Forge session.
- Keep the resulting project-scoped session server-side and deliver only its opaque session identifier in a host-only, HttpOnly cookie scoped to `/ui`; do not accept credentials in query strings or browser form fields.
- Add sign-in, callback, and sign-out UI routes; validate the session on every protected UI request and preserve JSON API bearer authentication unchanged.
- Keep this package limited to browser sign-in/session lifecycle. The separate responsive/accessibility visual redesign is listed below as an independently verifiable follow-up, not implemented by this change.

## Package Boundary and Split Assessment

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `portal-browser-sign-in` | Browser users authenticate through configured OIDC and receive a Forge project session without URL credentials. | Forge / Rust 1.87+, existing synchronous HTTP API and SQLite/file-backed identity state. | `/ui/sign-in`, `/ui/auth/callback`, `/ui/sign-out`; existing `IdentityConfig`, `AuthChallenge`, and `AdminSession` contracts. | Existing `central-admin-identity`; no other active package. | Local fake OIDC provider completes valid PKCE flow and creates a session; invalid protocol/claim cases create none. |
| `portal-accessible-responsive-ui` | Existing fleet/project screens become responsive and meet the specified WCAG 2.2 AA interaction baseline. | Forge / Rust 1.87+, existing Maud server-rendered UI. | `portal-web-ui` rendering contract; no auth or persistence changes. | None; can be delivered independently after sign-in. | Rendered UI checks plus browser viewport, keyboard, accessibility-tree, and measured contrast evidence. |

These are separate outcomes: authentication can be accepted through protocol/session tests without visual redesign, and the visual package can be accepted using an existing authorized test session. They share only existing Forge portal ownership, not a state transition, so combining them would obscure acceptance. This package is the first dependency-ready item; the follow-up must not be smuggled into its implementation.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| Forge identity | `src/identity/mod.rs`: `IdentityConfig`, `AuthChallenge`, `validate_callback`, `validate_claims`, `mint_session`, session persistence; `openspec/specs/central-admin-identity/spec.md` | Project-scoped config, state/nonce/PKCE challenge, admin allow-list, session lifecycle and session files. | Current API/CLI completion surface does not perform discovery, code exchange, or cryptographic ID-token verification; browser callback wiring is absent. | Forge owns operator identity and its Rust API listener. | **adopt** the existing Forge identity model and extend its provider boundary; do not duplicate session storage. |
| Devloom | `openspec/specs/owner-settings-console/spec.md`; `src/Devloom.Infrastructure/Persistence/AddDevloomPersistenceExtensions.cs` | Product-local verified-owner authorization, HttpOnly owner sessions, synchronizer CSRF, server-side session forwarding. | ASP.NET product-owner identity and database lifecycle differ from Forge's project-scoped operator session and Rust process. | Devloom owns user/owner identity for its application. | **keep local**; importing it would create a cross-runtime/session-owner dependency and conflate app users with Forge operators. |
| Chronicleaf | Checkout is under active development; no verified Forge-compatible browser identity contract was established by this targeted search. | No reusable provider/session interface established. | Product-local, changing implementation and distinct owner lifecycle. | Chronicleaf owns its product identity. | **keep local**; no source dependency or shared runtime is justified. |
| Workspace Governance | `docs/toolchain-decisions.md` records a planned C# product OIDC client in `dotnet-platform-libs`; no Forge browser session API found in targeted search. | Portfolio governance and language-platform direction. | C# library adoption cannot be Forge's Rust runtime dependency; no stack-neutral operator-session contract exists. | Workspace Governance owns portfolio policy; `dotnet-platform-libs` owns C# packages. | **keep local** for Forge session orchestration; future common protocol requirements may be standardized separately. |
| Platform Contracts | `README.md` states authentication, sessions, and UI are not owned there. | Stack-neutral wire contracts when a cross-project exchange is needed. | This is one server's local browser-session boundary, not inter-project data exchange. | Platform Contracts owns versioned cross-project wire schemas. | **keep local**; no shared contract needed. |

No sibling files or runtime calls are introduced. This design keeps project identity independent and follows the existing “optional, versioned artifact/adaptor” boundary between portfolio projects.

## BFS Impact Map

- **Actors/flow:** operator opens `/ui`; unauthenticated user chooses a registered project identity; Forge redirects to the configured provider; callback validates the one-time challenge and provider response; Forge mints a session; browser returns to the original same-origin portal path; sign-out revokes only the owning project's session.
- **Contracts:** modifies `portal-web-ui` and `central-admin-identity`; adds explicit browser-route behavior while leaving `/v1` request/response envelopes byte-compatible.
- **Modules/callers:** `src/api/mod.rs` routing and authorization; `src/api/ui/{routes,auth,render}.rs`; `src/identity/mod.rs`; identity configuration/secret resolver; browser and HTTP integration tests.
- **Persistence:** reuse `.forge/identity/<project>/challenges` and `sessions`; challenge is one-use and bounded by configured state TTL. No SQLite migration or shared cookie store.
- **Provider/dependency:** add a maintained OIDC client implementation with pinned lockfile closure; provider endpoints are reached only from Forge's server. Do not hand-parse or hand-verify JWT/JWKS.
- **Security/failure boundaries:** reject missing/unknown project identity, disabled or unsupported provider, discovery issuer mismatch, state replay/mismatch/expiry, callback error, code exchange error/timeout, signature/issuer/audience/expiry/nonce mismatch, missing admin claim, cross-project session, revoked/expired session, bad cookie, cross-origin state-changing request, and malformed return path. Never log authorization code, state, nonce, token, cookie, or secret. Fail closed; do not mint a session on any error.
- **Compatibility:** header bearer auth remains supported for API and existing UI automation during migration; query-string `token` is removed from browser UI paths and is never generated. No sibling is required to build/run Forge.
- **Tests:** deterministic OIDC test issuer/JWKS; route contracts for start/callback/sign-out; session owner/admin/expiry boundaries; API compatibility; redaction and URL/referrer checks; strict OpenSpec and native Rust checks.
- **Unaffected:** publish semantics, portfolio writes, Studio preview behavior, Forge registry schema, generated project output, and static-site publishing.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `portal-web-ui`: add browser sign-in, callback, cookie, sign-out, and unauthenticated behavior requirements.
- `central-admin-identity`: require real provider-backed browser callback verification before a browser session can be minted.

## Non-goals

- Responsive visual redesign and broader WCAG remediation (separate `portal-accessible-responsive-ui` package).
- Changes to project end-user authentication, Devloom/Chronicleaf auth, or shared login across unrelated product sites.
- New identity-provider administration UI, account provisioning, multi-factor enrollment, or provider-specific SDKs.
- Publicly exposing the loopback API, reverse-proxy deployment automation, or changing operator bind policy.
- Treating authentication as permission: a successful provider login without the configured Forge admin claim is denied.
