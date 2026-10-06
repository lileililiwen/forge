## 1. BFS — Baseline and impact coverage

- [x] 1.1 Confirm current identity/portal specs, routes, API authorization, manifest identity configuration, secret-reference resolver, challenge/session file permissions, and bearer compatibility; record exact pre-change API fixtures.
- [x] 1.2 Confirm `openidconnect` blocking/rustls API, Rust 1.87 compatibility, transitive dependency/license/advisory closure; revise design first if any premise is false. Revised: pin moved from `3.5.0` to `4.0.1` because the 3.x `reqwest` 0.11 / `rustls` 0.21 stack carries open `rustls-webpki` and `h2` advisories; 4.x uses `rustls` 0.23. The remaining `rsa` timing advisory is a documented exception in `deny.toml` and `.cargo/audit.toml`.
- [x] 1.3 Deterministic verifier seam: `BrowserAuthVerifier` trait with `FakeBrowserAuthVerifier` (queued outcomes) for tests and `LibraryBrowserAuthVerifier` for production; the real verifier's pre-network guards (provider error, empty code, expired challenge) are unit-tested. A local HTTP OIDC issuer with generated signing keys is not built; live-provider acceptance is recorded as unavailable in 4.3.
- [x] 1.4 Enumerate every UI route and classify read versus mutation permissions, project owner resolution, callback state transitions, cookie scope, and redaction points.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Add the pinned OIDC dependency, configure TLS and bounded HTTP, and validate the dependency closure/policy.
- [x] 2.2 Add server-side provider discovery, authorization URL generation, PKCE/state/nonce challenge creation, and one-time challenge consumption using existing Forge identity persistence.
- [x] 2.3 Add callback code exchange and library-backed signed-token verification; enforce issuer/audience/redirect/nonce/time/admin allowlist before calling existing session mint/save operations.
- [x] 2.4 Add browser sign-in, callback, and sign-out routes; validate safe local return paths; set/clear host-only HttpOnly SameSite cookie with HTTPS Secure policy.
- [x] 2.5 Parse cookie only on `/ui`; route all protected UI requests through actual session owner/state/permission authorization; retain bearer headers and reject query-token authentication; leave `/v1` bearer behavior unchanged.
- [x] 2.6 Require exact same-origin Origin on state-changing browser routes; ensure callback/session errors are safe HTML and redact all protocol credentials.
- [x] 2.7 Update `portal-web-ui` and `central-admin-identity` implementation tests and remove outdated query-token browser guidance without changing CLI/API JSON output contracts.

## 3. BFS — Regression and completeness

- [x] 3.1 Add success/failure/boundary tests from the design oracle: valid provider (fake verifier), token validation failures, replay, admin denial, expiry/revocation, project isolation, cookie-only `/v1`, return-path refusal, origin refusal, and secret redaction.
- [x] 3.2 Compare JSON API envelopes and bearer-authenticated API behavior against pre-change fixtures; prove no cookie is accepted outside `/ui`.
- [x] 3.3 Review all UI mutations, provider/network deadlines, challenge cleanup/one-use behavior, response headers, log/error paths, and Cargo dependency/license/advisory changes.
- [x] 3.4 Confirm accessible/responsive redesign remains explicitly out of scope and no sibling project dependency was introduced.

## 4. Verification

- [x] 4.1 Run `cargo fmt --check`, `cargo test --test portal_ui_contract`, `cargo test --test identity_contract`, `cargo build`, and `cargo clippy --workspace --all-targets --all-features -- -D warnings`. Clippy is clean for every file this change touches; the workspace still reports pre-existing lints in untouched files (`src/gate/evidence.rs`, `src/publish/{fleet,mod}.rs`, `src/api/ui/auth.rs`, `src/portfolio/share/validation.rs`) under the host's newer clippy.
- [x] 4.2 Run dependency policy/security checks available in the checkout, `node scripts/check-openspec-change-names.mjs`, `openspec validate --all --strict --no-interactive`, and `git diff --check`.
- [x] 4.3 Captured deterministic evidence at the verifier seam (fake verifier success/error; real-verifier pre-network refusals). Live-provider HTTP acceptance is documented as unavailable: no real OIDC provider is explicitly configured or exercised in this environment.
- [ ] 4.4 Review all scoped scenarios against current evidence, archive with canonical spec promotion, commit only this package and tests, then update HANDOFF in its separate commit and stop without pushing.
