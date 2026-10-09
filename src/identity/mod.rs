//! Centralized OIDC admin federation with per-project sessions
//! (`central-admin-identity`).
//!
//! Forge Core owns the versioned OIDC identity contract, the
//! state/nonce validator, the per-project session lifecycle and
//! the typed `identity-*` rejection codes. The CLI is the
//! first transport; future MCP, HTTP API and portal surfaces
//! consume the same Core contract so a project that proves a
//! session is valid here is also valid through every other
//! transport.
//!
//! ## Why
//!
//! [requirement.md](../../requirement.md) §31 (and the maturity
//! gate at §25) call for centralized admin authentication
//! using standard OIDC, with per-project sessions and no
//! shared cookies across unrelated applications. The
//! contract covers the v0.1 outcome: each project owns its
//! own per-project client and admin session; the provider
//! authenticates the user; Forge grants the per-project
//! admin session only after a separate, evidence-bound
//! admin-claim check.
//!
//! ## Boundary
//!
//! - **Provider login does not imply admin authorization.**
//!   The OIDC code path validates the issuer, audience,
//!   redirect URI, state, nonce and the token's `exp`/`iat`
//!   window; the admin claim is checked separately, and a
//!   mismatch is refused with `identity-permission-denied`
//!   (R1 boundary scenario).
//! - **Per-project sessions, no cross-project tokens.** A
//!   session is bound to the project id that minted it.
//!   When a session is presented to a different project,
//!   the validator returns `identity-session-cross-project`
//!   and refuses the call without inspecting any other
//!   claim (R2 failure scenario).
//! - **Termination is project-scoped.** Revoking or expiring
//!   one project's session never implicitly revokes or
//!   validates another project's session; each session has
//!   its own state and its own expiry (R2 boundary).
//! - **Credentials are references, not embedded values.**
//!   The manifest carries a `client_secret_ref`; the resolved
//!   secret never reaches the journal or the evidence. A
//!   credential-shaped substring in evidence is redacted
//!   through `policy::redact_credentials` so a leaked secret
//!   cannot appear in the registry or the CLI output.
//!
//! ## Persistence
//!
//! Sessions are stored under `.forge/identity/<project>/`
//! (atomic `.tmp` + rename) so a project's identity
//! evidence is project-scoped and never shared with another
//! project. The Core registry's `operations` table receives
//! one `identity` row per `validate-config` / `build-challenge`
//! / `complete-auth` / `terminate` call with a `done` /
//! `rejected` / `blocked` verdict and the project id (no
//! synthetic id is invented; identity is always project-
//! scoped).
//!
//! ## Risk model
//!
//! The browser sign-in path verifies a real OIDC provider
//! response through the pinned `openidconnect` crate
//! (discovery, PKCE code exchange, signed ID-token
//! verification) before minting a session; see
//! [`oidc`]. The contract tests drive a deterministic fake
//! verifier so the local gate never depends on a public
//! provider. The redaction rule set is the same
//! `policy::redact_credentials` consumed by every other
//! adapter.

pub mod browser_auth;
pub mod constants;
pub mod fake_verifier;
pub mod global;
pub mod model;
mod oidc;
pub mod protocol;

pub use oidc::LibraryBrowserAuthVerifier;

// Re-export all types
pub use browser_auth::*;
pub use constants::*;
pub use model::*;
pub use protocol::*;
