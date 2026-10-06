# Design: portal-browser-sign-in

## Context and boundary

Implement only the Forge-hosted browser authentication lifecycle in `/home/paul/code/forge`, Rust 1.87+, on the existing synchronous `forge api serve` listener. Inspect/change `src/api/mod.rs`, `src/api/ui/{routes,auth,render}.rs`, `src/identity/mod.rs`, the manifest identity loader, Cargo dependency/lock/policy files, `tests/portal_ui_contract.rs`, and identity contract tests. Do not modify Devloom, Chronicleaf, Workspace Governance, Platform Contracts, publish flows, or the separately package-mapped visual redesign.

Forge remains identity/session owner. Reuse `IdentityConfig`, `AuthChallenge`, `AdminSession`, session persistence, redaction and project registry. Do not share source, databases, cookies, secrets, or runtime calls with siblings. The platform-contracts repo explicitly does not own auth/session/UI; Devloom has a distinct ASP.NET product-owner session and CSRF boundary; neither is a compatible Forge operator-session provider.

## Runtime and dependencies

- Language: Rust 2021, minimum Rust 1.87, current Cargo workspace.
- Provider protocol: `openidconnect` Rust crate, exact version `4.0.1`, with its blocking `reqwest` client (`reqwest-blocking`) and rustls TLS feature enabled. Use the crate's discovery, authorization URL, code exchange, and ID-token verification APIs; no hand-built JWT/JWKS validation and no shell subprocess. Revision from the originally drafted `3.5.0` pin: the 3.x line depends on `reqwest` 0.11 / `rustls` 0.21, whose `rustls-webpki` and `h2` revisions carry open advisories with no in-line fix. The 4.x line uses `rustls` 0.23 and a modern `hyper`, clearing those. The remaining `rsa` timing advisory (`RUSTSEC-2023-0071`) is a private-key operation not reachable from Forge's public-key signature verification; it is recorded as an explicit, reasoned exception in `deny.toml` and `.cargo/audit.toml`. The provider HTTP client enforces connect and total deadlines of 10 seconds and disables redirects.
- Preserve the existing single-threaded synchronous HTTP handler. Provider requests must use explicit connect and total deadlines bounded to 10 seconds; never hold a SQLite connection while waiting on the network.
- Do not add JavaScript, a Node build, a browser framework, a new server, or a sibling crate. Review Cargo.lock and `deny.toml` for the complete transitive closure and license/security policy.
- Tests: `cargo test --test portal_ui_contract`, `cargo test --test identity_contract`, `cargo fmt --check`, `cargo build`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, strict OpenSpec validation, and the repository release check when environment prerequisites are available.

## Routes and state flow

| Route | Authorization | Behavior |
|---|---|---|
| `GET /ui/sign-in?project=<id>&return=<local-path>` | Anonymous; loopback API listener | Validate project is registered and has valid identity config; create and persist a bounded state/nonce/PKCE challenge; 303 to the provider authorization endpoint. Return path must be relative, same-origin, and under `/ui`; otherwise use `/ui`. |
| `GET /ui/auth/callback?code=…&state=…` | Anonymous callback; challenge is the proof-in-progress | Atomically consume the one-time challenge before code exchange. Exchange using configured redirect URI, client credentials resolved from approved secret references, and PKCE verifier. Verify signature/JWKS, exact issuer, client audience, expiry/issued-at policy, nonce, and admin claim before minting a project-scoped session. Save session, set cookie, then 303 to validated return path. |
| `POST /ui/sign-out` | Valid browser session or expired-session cleanup | Require same-origin `Origin` and same-site request policy; revoke/remove only the session in its owner project; clear cookie; 303 to `/ui/sign-in`. |
| `GET /ui`, `/ui/projects/{id}`, `/ui/studio/{id}` and UI POSTs | Valid session on every request | Resolve the cookie as the session ID and route through the same project/session/permission checks used by API bearer authorization. Never trust cookie presence alone. |

Challenge states: `absent → issued → consumed → completed|rejected`; only `issued` can transition to `consumed`, and every callback attempt consumes it (including provider errors) so replay cannot succeed. Challenge expiry uses configured `state_ttl_seconds`; a cleanup pass may remove expired files but is not required for rejection. Session state uses existing active/expired/revoked model and configured project TTL. A callback may create one session at most.

Bind the authorization transaction to the initiating browser as well: set a random, short-lived `forge_oidc_state` HttpOnly cookie with `Path=/ui/auth/callback; SameSite=Lax`, `Secure` on HTTPS, and an expiry no later than the challenge. The callback must match both its `state` query value and this cookie to the persisted challenge, then expire the transient cookie on every callback outcome. Reject duplicate state-cookie names rather than selecting one ambiguously. Persist challenge consumption with an atomic create-new consumed marker (or an equivalently atomic state transition); concurrent callbacks have exactly one winner.

The callback URL is the configured `IdentityConfig.redirect_uri`; it must exactly match the provider registration. Forge must refuse startup/route initiation for remote HTTP or a redirect host not matching the request's allowed origin policy. For reverse proxies, do not trust forwarded headers by default. This package targets loopback/local operator use; a future proxy-aware HTTPS deployment requires an explicit trusted-proxy contract.

## Session and browser credential contract

- Cookie name: `forge_session`; value is the existing 64-character lowercase hex session ID. Cookie attributes: `Path=/ui; HttpOnly; SameSite=Lax; Secure` for HTTPS. For loopback HTTP development, omit `Secure` only when the actual listener origin is a loopback HTTP origin; never omit it for non-loopback. Do not set `Domain`, so it remains host-only. Cookie expiry cannot exceed `AdminSession.expires_at`.
- Cookie is accepted only on `/ui` routes. It is converted to a session ID and passed through `authorize`; API `/v1` continues to require `Authorization: Bearer` and ignores cookies.
- Do not accept `?token=` on any UI route; reject it without reflecting its value. Remove credential-bearing hidden fields from UI forms. Existing bearer-header UI automation remains supported but is validated through `authorize` too.
- Protected UI POSTs require exact same-origin `Origin`; missing or mismatched origin is refused. SameSite is defense in depth, not the sole CSRF defense. GET is read-only; no mutation via callback GET other than consuming the login challenge and creating an authenticated session.
- Never render, log, persist outside identity storage, or redirect with provider code, access/ID token, client secret, cookie value, state, nonce, or PKCE verifier. Error pages expose stable codes and redacted messages only.

## Provider configuration and ownership

`forge.yaml` remains the desired-configuration owner. The existing typed identity loader validates provider, HTTPS issuer, client ID, exact redirect URI, `openid` scope and admin claim allowlist. `client_secret_ref` is resolved only by the existing approved secret-reference mechanism; never put a secret in `forge.yaml`, SQLite, HTML, or logs. Public clients use PKCE and no secret. Discovery metadata's issuer must equal the configured issuer; redirect endpoint and JWKS requests must use HTTPS outside the explicit loopback test fixture. No configurable arbitrary HTTP endpoint is added.

For deterministic tests, the OIDC verifier accepts an injected test transport only inside the identity module/test harness; production construction always uses HTTPS rustls transport and the configured issuer. Test servers may use loopback HTTP with a test-only constructor that cannot be enabled by environment variables in production.

## Authorization and failure policy

- Unknown project, missing identity config, malformed config, provider discovery outage, timeout, malformed metadata, callback provider error, missing/duplicate code or state, challenge missing/expired/replayed, PKCE/code exchange failure, invalid signature/key, issuer/audience/nonce/time mismatch, missing admin claim, and non-admin claim all fail closed; no session is created.
- Session must be active, unexpired, owned by the selected project, and carry `admin:access` for mutating UI routes. A valid provider identity without the configured admin claim receives a generic 403 and a redacted audit/journal refusal; it is not retried as another project.
- Cross-project cookie use is 403; unknown or expired cookie is cleared and redirected to sign-in. Revocation is checked on every request, not cached in browser state.
- Provider HTTP calls have bounded response size and deadlines; discovery/JWKS failure is unavailable, not a permissive fallback. No stale-key verification fallback beyond the OIDC library's documented cache behavior.
- An invalid return path falls back to `/ui`; it is never used as an external redirect.
- Session storage errors fail the callback with a generic 500 page and do not report success. Challenge is already consumed; operator restarts login.

## Compatibility and migration

JSON API envelopes and bearer semantics remain byte-identical. Browser UI query tokens and hidden-form token propagation are removed because they expose session material; forms submit same-origin without a credential field and rely on cookie plus Origin validation. Existing authorized UI requests using an Authorization header remain valid. The schema and database are unchanged. Historical `complete-auth` remains a development/test helper and is not upgraded into the browser trust boundary; UI only accepts sessions produced after cryptographically verified provider completion. Update the existing `portal-web-ui` wording that currently implies query-token browser auth.

## Verification oracles

Use an in-process deterministic OIDC issuer and signing keys; do not depend on public IdP availability for test pass criteria.

1. Success: valid discovery, PKCE code exchange, signed ID token, exact issuer/audience, nonce, expiry, and configured admin claim produce exactly one persisted active project session, a correctly scoped cookie, and a same-origin redirect to the sanitized return path.
2. Failure matrix: each protocol/claim/config/provider error above produces non-2xx or safe sign-in redirect, no session, no secret/code/token in body/headers/log capture, and a consumed single-use challenge where callback was reached.
3. Boundaries: replay callback, use session on another project, use expired/revoked/no-admin session, request `/v1` with only cookie, use external return path, cross-origin sign-out/write, missing cookie, and cookie on non-UI route are all rejected as specified.
4. Compatibility: bearer-authenticated JSON fixtures before/after compare byte-for-byte; valid Authorization bearer continues to work on UI and is checked against persisted session state.
5. Run the exact test/build/format/lint/OpenSpec commands above. Live external-provider success is a separate evidence item and is not inferred from the fake issuer tests.

## Decision ledger

- Resolved: browser OIDC is a real provider round trip, not a pasted session ID or a fake local sign-in screen.
- Resolved: project-scoped Forge session remains the authorization unit; browser cookie is only a transport for its opaque ID.
- Resolved: visual refresh is separate and explicitly excluded from this package.
- Resolved: no sibling runtime/source adoption; use Forge identity and an OIDC protocol crate.
- Deferred, not blocking this local package: live-provider acceptance evidence, trusted reverse-proxy deployment, and shared cross-site SSO.
- Security boundary: if the selected crate cannot provide blocking bounded HTTP and cryptographic ID-token verification on Rust 1.87, stop and revise this design before implementation; do not replace it with hand-written validation.
