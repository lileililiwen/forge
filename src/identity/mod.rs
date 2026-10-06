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

mod oidc;

pub use oidc::LibraryBrowserAuthVerifier;

use std::sync::Mutex;

use chrono::{DateTime, Duration, Utc};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::core::manifest::{IdentityMeta, Manifest};
use crate::core::ForgeError;
use crate::policy::redact_credentials;

/// Contract data version for the identity API, the persisted
/// session records, and the CLI/MCP envelopes. Bumped when
/// a new field is added or a typed error code changes.
pub const IDENTITY_CONTRACT_VERSION: &str = "0.1.0";

/// Default per-project client id placeholder used by the
/// fixtures when a project does not declare one in its
/// manifest. Real manifests always carry an explicit
/// client id; the placeholder exists so the contract tests
/// can stand in for a real provider configuration.
pub const IDENTITY_FAKE_PROVIDER: &str = "okta-fixture";

/// Maximum number of scopes a project's identity block may
/// request. Larger lists are refused with
/// `identity-invalid` so the manifest stays bounded.
pub const MAX_SCOPES: usize = 16;

/// Maximum number of admin-claim values the manifest may
/// enumerate. Larger lists are refused with
/// `identity-invalid`.
pub const MAX_ADMIN_VALUES: usize = 16;

/// Minimum state/nonce lifetime in seconds. A shorter TTL
/// is refused because the round trip is unlikely to
/// complete in time.
pub const MIN_STATE_TTL_SECONDS: i64 = 30;

/// Maximum state/nonce lifetime in seconds. A longer TTL
/// enlarges the replay window and is refused.
pub const MAX_STATE_TTL_SECONDS: i64 = 600;

/// Minimum admin session lifetime in seconds.
pub const MIN_SESSION_TTL_SECONDS: i64 = 60;

/// Maximum admin session lifetime in seconds. The brief
/// (`Session and project isolation`) calls for short-lived
/// project sessions; a longer TTL is refused.
pub const MAX_SESSION_TTL_SECONDS: i64 = 86_400;

/// Directory (relative to the project root) holding the
/// per-project session records.
pub const IDENTITY_DIR: &str = ".forge/identity";

/// Directory (relative to the project root) holding
/// pending auth challenges awaiting the OIDC callback.
/// Each challenge is named by its `state` value so the
/// callback handler can find it deterministically.
pub const IDENTITY_PENDING_DIR: &str = ".forge/identity";

/// Permission granted to a session after a successful
/// admin-claim check. The string set is bounded on
/// purpose so the planner and the doctor can map a
/// permission to a known capability without parsing
/// free-form text.
pub const ADMIN_PERMISSION: &str = "admin:access";

/// OIDC provider names accepted in the manifest. The
/// contract is provider-agnostic: every supported
/// provider is a real OIDC provider, so the only
/// difference between providers is the issuer, the
/// JWKS URI and the admin claim convention. The list
/// is bounded so the validator can refuse unknown
/// providers without inventing a provider registry
/// during the contract cycle.
pub const SUPPORTED_PROVIDERS: &[&str] = &[
    "okta",
    "auth0",
    "keycloak",
    "azure-ad",
    "google",
    "github",
    "okta-fixture",
];

/// OIDC code challenge methods accepted by the
/// `build-challenge` contract. Only `S256` is supported
/// in v0.1; `plain` is refused because it offers no
/// replay protection.
pub const SUPPORTED_CODE_CHALLENGE_METHODS: &[&str] = &["S256"];

/// Length of the random `state`, `nonce` and
/// `code_verifier` values (in raw bytes; the values are
/// hex-encoded so the wire length is `2 * VALUE_LEN`).
pub const VALUE_LEN: usize = 32;

/// Length of the session id (in raw bytes; hex-encoded
/// for storage).
pub const SESSION_ID_LEN: usize = 16;

/// Typed identity rejection. The `code` field is the
/// stable machine code Core returns on the wire; the
/// `reason` is the human-readable explanation. The
/// session reference is optional: config validation
/// has no session yet, complete-auth has the
/// candidate, and session validation has the existing
/// record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentityRejection {
    pub code: String,
    pub reason: String,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
}

impl IdentityRejection {
    fn invalid(reason: impl Into<String>) -> ForgeError {
        ForgeError::IdentityInvalid {
            reason: reason.into(),
        }
    }
}

/// OIDC identity configuration, normalized from the
/// manifest's `identity:` block. The manifest validator
/// is the only path that produces this struct; the
/// identity module never reads `forge.yaml` itself so
/// Core owns the schema boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentityConfig {
    pub provider: String,
    pub issuer: String,
    pub client_id: String,
    pub audience: String,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
    pub jwks_uri: Option<String>,
    pub state_ttl_seconds: i64,
    pub session_ttl_seconds: i64,
    pub admin_claim: String,
    pub admin_values: Vec<String>,
    pub client_secret_ref: Option<String>,
}

impl IdentityConfig {
    /// Normalize the manifest's `identity:` block into a
    /// typed `IdentityConfig`. Refuses empty fields, bad
    /// URLs, weak TTLs, unknown providers and oversized
    /// lists with the typed `identity-invalid` code.
    pub fn from_manifest(project_id: &str, meta: &IdentityMeta) -> Result<Self, ForgeError> {
        if let Err(reason) = crate::core::validate_project_id(project_id) {
            return Err(IdentityRejection::invalid(format!(
                "project id `{project_id}` is invalid: {reason}"
            )));
        }
        let provider = require_nonempty(meta.provider.as_deref(), "provider")?;
        if !SUPPORTED_PROVIDERS.contains(&provider.as_str()) {
            return Err(IdentityRejection::invalid(format!(
                "identity provider `{provider}` is not in the supported set; \
                 supported providers: {}",
                SUPPORTED_PROVIDERS.join(", ")
            )));
        }
        let issuer = require_nonempty(meta.issuer.as_deref(), "issuer")?;
        validate_url(&issuer, "issuer")?;
        if !issuer.starts_with("https://") {
            return Err(IdentityRejection::invalid(format!(
                "identity issuer `{issuer}` must use https; cleartext issuers \
                 are refused because the OIDC discovery document and JWKS \
                 response would be vulnerable to tampering"
            )));
        }
        let client_id = require_nonempty(meta.client_id.as_deref(), "client_id")?;
        if !is_kebab_or_token(&client_id) {
            return Err(IdentityRejection::invalid(format!(
                "identity client_id `{client_id}` must be kebab-case or a \
                 provider-assigned opaque token; whitespace and shell \
                 metacharacters are refused"
            )));
        }
        let audience = match meta.audience.as_deref() {
            Some(value) if !value.trim().is_empty() => value.to_string(),
            _ => client_id.clone(),
        };
        let redirect_uri = require_nonempty(meta.redirect_uri.as_deref(), "redirect_uri")?;
        validate_url(&redirect_uri, "redirect_uri")?;
        if !(redirect_uri.starts_with("https://")
            || redirect_uri.starts_with("http://localhost")
            || redirect_uri.starts_with("http://127.0.0.1")
            || redirect_uri.starts_with("forge://"))
        {
            return Err(IdentityRejection::invalid(format!(
                "identity redirect_uri `{redirect_uri}` must use https, \
                 http://localhost, http://127.0.0.1, or the forge:// scheme; \
                 cleartext remote redirects are refused"
            )));
        }
        if meta.scopes.is_empty() {
            return Err(IdentityRejection::invalid(
                "identity scopes must include at least `openid`",
            ));
        }
        if meta.scopes.len() > MAX_SCOPES {
            return Err(IdentityRejection::invalid(format!(
                "identity declares {} scopes; the maximum is {MAX_SCOPES}",
                meta.scopes.len()
            )));
        }
        for scope in &meta.scopes {
            if scope.trim().is_empty() {
                return Err(IdentityRejection::invalid(
                    "identity scope entries must not be empty",
                ));
            }
            if scope.contains(char::is_whitespace) {
                return Err(IdentityRejection::invalid(format!(
                    "identity scope `{scope}` must not contain whitespace"
                )));
            }
        }
        if !meta.scopes.iter().any(|s| s == "openid") {
            return Err(IdentityRejection::invalid(
                "identity scopes must include `openid` (OIDC requires it)",
            ));
        }
        let jwks_uri = match meta.jwks_uri.as_deref() {
            Some(value) if !value.trim().is_empty() => {
                validate_url(value, "jwks_uri")?;
                if !value.starts_with("https://") {
                    return Err(IdentityRejection::invalid(format!(
                        "identity jwks_uri `{value}` must use https"
                    )));
                }
                Some(value.to_string())
            }
            _ => None,
        };
        let state_ttl_seconds = meta.state_ttl_seconds.unwrap_or(MIN_STATE_TTL_SECONDS + 30);
        if !(MIN_STATE_TTL_SECONDS..=MAX_STATE_TTL_SECONDS).contains(&state_ttl_seconds) {
            return Err(IdentityRejection::invalid(format!(
                "identity state_ttl_seconds {state_ttl_seconds} is outside the \
                 allowed range [{MIN_STATE_TTL_SECONDS}, {MAX_STATE_TTL_SECONDS}]"
            )));
        }
        let session_ttl_seconds = meta.session_ttl_seconds.unwrap_or(MAX_SESSION_TTL_SECONDS);
        if !(MIN_SESSION_TTL_SECONDS..=MAX_SESSION_TTL_SECONDS).contains(&session_ttl_seconds) {
            return Err(IdentityRejection::invalid(format!(
                "identity session_ttl_seconds {session_ttl_seconds} is outside the \
                 allowed range [{MIN_SESSION_TTL_SECONDS}, {MAX_SESSION_TTL_SECONDS}]"
            )));
        }
        let admin_claim = require_nonempty(meta.admin_claim.as_deref(), "admin_claim")?;
        if !is_admin_claim(&admin_claim) {
            return Err(IdentityRejection::invalid(format!(
                "identity admin_claim `{admin_claim}` must be a kebab-case, \
                 snake_case, namespaced URI, or single word; whitespace and \
                 shell metacharacters are refused"
            )));
        }
        if meta.admin_values.is_empty() {
            return Err(IdentityRejection::invalid(
                "identity admin_values must list at least one value; an empty \
                 list means no value grants admin access, and the project would \
                 never mint a session"
                    .to_string(),
            ));
        }
        if meta.admin_values.len() > MAX_ADMIN_VALUES {
            return Err(IdentityRejection::invalid(format!(
                "identity declares {} admin_values; the maximum is {MAX_ADMIN_VALUES}",
                meta.admin_values.len()
            )));
        }
        for value in &meta.admin_values {
            if value.trim().is_empty() {
                return Err(IdentityRejection::invalid(
                    "identity admin_values entries must not be empty",
                ));
            }
        }
        let client_secret_ref = match meta.client_secret_ref.as_deref() {
            Some(value) if !value.trim().is_empty() => {
                if !value.contains("://") {
                    return Err(IdentityRejection::invalid(format!(
                        "identity client_secret_ref `{value}` must be a reference \
                         (e.g. `env://OIDC_CLIENT_SECRET`); raw secrets are refused \
                         in the manifest"
                    )));
                }
                Some(value.to_string())
            }
            _ => None,
        };
        Ok(IdentityConfig {
            provider,
            issuer,
            client_id,
            audience,
            redirect_uri,
            scopes: meta.scopes.clone(),
            jwks_uri,
            state_ttl_seconds,
            session_ttl_seconds,
            admin_claim,
            admin_values: meta.admin_values.clone(),
            client_secret_ref,
        })
    }

    /// Build a config from a manifest's `identity:` block,
    /// returning `Ok(None)` when the block is absent.
    pub fn from_manifest_opt(
        project_id: &str,
        manifest: &Manifest,
    ) -> Result<Option<Self>, ForgeError> {
        match &manifest.identity {
            Some(meta) => Ok(Some(Self::from_manifest(project_id, meta)?)),
            None => Ok(None),
        }
    }
}

/// Authorization request the caller hands to the OIDC
/// provider. The `code_verifier` is the secret the
/// provider never sees; the `code_challenge` is the
/// SHA-256 hash the provider stores. Both are returned
/// to the caller so the redirect and the callback share
/// the same cryptographic binding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthChallenge {
    pub project_id: String,
    pub provider: String,
    pub client_id: String,
    pub state: String,
    pub nonce: String,
    pub code_verifier: String,
    pub code_challenge: String,
    pub code_challenge_method: String,
    pub redirect_uri: String,
    pub scope: String,
    pub audience: String,
    pub issuer: String,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

impl AuthChallenge {
    pub fn is_expired(&self, now: DateTime<Utc>) -> bool {
        now >= self.expires_at
    }
}

/// The OIDC callback the caller receives after the
/// provider round trip. Either `code` is present (success)
/// or `error` is present (provider-reported failure);
/// never both.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthCallback {
    pub project_id: String,
    pub state: String,
    pub code: String,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub error_description: Option<String>,
}

/// Provider-issued claims, normalized to the typed
/// surface Forge validates. The raw `claims` map is
/// preserved so the admin-claim lookup can use the
/// project-defined claim name without changing the
/// typed contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderClaims {
    pub issuer: String,
    pub audience: String,
    pub subject: String,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub nonce: String,
    pub scopes: Vec<String>,
    pub claims: BTreeMap<String, String>,
}

impl ProviderClaims {
    pub fn claim(&self, name: &str) -> Option<&str> {
        self.claims.get(name).map(String::as_str)
    }
}

/// Per-project admin session state. A session is bound
/// to the project id that minted it; `Active` is the only
/// state that grants the `admin:access` permission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionState {
    Active,
    Expired,
    Revoked,
}

/// Per-project admin session. Carries the subject the
/// provider authenticated and the permissions Core
/// granted after the admin-claim check. The session id
/// is the only piece the caller presents; the rest is
/// Core-owned and never leaves the registry or the
/// session file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdminSession {
    pub session_id: String,
    pub project_id: String,
    pub provider: String,
    pub subject: String,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub permissions: Vec<String>,
    pub state: SessionState,
    #[serde(default)]
    pub note: Option<String>,
}

impl AdminSession {
    pub fn is_expired(&self, now: DateTime<Utc>) -> bool {
        now >= self.expires_at || self.state == SessionState::Expired
    }
}

/// Result of `complete-auth`. The successful case carries
/// the freshly minted session; the failure case carries
/// the typed `IdentityRejection` so a partial run is
/// observable on stdout before the CLI renders the
/// summary. The JSON envelope is the default external
/// tagging: `{"Session": {...}}` or `{"Rejected": {...}}`
/// so the contract is unambiguous when a CLI or MCP
/// caller reads `outcome.Session.project_id` without
/// inspecting a discriminator field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IdentityOutcome {
    Session(AdminSession),
    Rejected(IdentityRejection),
}

impl IdentityOutcome {
    pub fn is_session(&self) -> bool {
        matches!(self, IdentityOutcome::Session(_))
    }
}

/// Compute the SHA-256 code challenge from a code
/// verifier, then base64url-encode the digest (no
/// padding). This is the OIDC PKCE `S256` method.
pub fn code_challenge_s256(verifier: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let digest = hasher.finalize();
    base64url(&digest)
}

fn base64url(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    let mut i = 0;
    while i + 3 <= bytes.len() {
        let n = ((bytes[i] as u32) << 16) | ((bytes[i + 1] as u32) << 8) | (bytes[i + 2] as u32);
        out.push(ALPHABET[((n >> 18) & 0x3f) as usize] as char);
        out.push(ALPHABET[((n >> 12) & 0x3f) as usize] as char);
        out.push(ALPHABET[((n >> 6) & 0x3f) as usize] as char);
        out.push(ALPHABET[(n & 0x3f) as usize] as char);
        i += 3;
    }
    let rem = bytes.len() - i;
    if rem == 1 {
        let n = (bytes[i] as u32) << 16;
        out.push(ALPHABET[((n >> 18) & 0x3f) as usize] as char);
        out.push(ALPHABET[((n >> 12) & 0x3f) as usize] as char);
    } else if rem == 2 {
        let n = ((bytes[i] as u32) << 16) | ((bytes[i + 1] as u32) << 8);
        out.push(ALPHABET[((n >> 18) & 0x3f) as usize] as char);
        out.push(ALPHABET[((n >> 12) & 0x3f) as usize] as char);
        out.push(ALPHABET[((n >> 6) & 0x3f) as usize] as char);
    }
    out
}

fn random_hex(len: usize) -> String {
    let mut buf = vec![0u8; len];
    rand::thread_rng().fill_bytes(&mut buf);
    hex_lower(&buf)
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(HEX[((*b >> 4) & 0x0f) as usize] as char);
        out.push(HEX[(*b & 0x0f) as usize] as char);
    }
    out
}

fn require_nonempty(field: Option<&str>, name: &str) -> Result<String, ForgeError> {
    match field {
        Some(value) if !value.trim().is_empty() => Ok(value.trim().to_string()),
        _ => Err(IdentityRejection::invalid(format!(
            "identity `{name}` is required"
        ))),
    }
}

fn validate_url(value: &str, field: &str) -> Result<(), ForgeError> {
    if value.contains(char::is_whitespace) {
        return Err(IdentityRejection::invalid(format!(
            "identity {field} `{value}` must not contain whitespace"
        )));
    }
    if value.contains(';') || value.contains('|') || value.contains('`') {
        return Err(IdentityRejection::invalid(format!(
            "identity {field} `{value}` must not contain shell metacharacters"
        )));
    }
    if !value.contains("://") {
        return Err(IdentityRejection::invalid(format!(
            "identity {field} `{value}` must include a scheme (e.g. `https://`)"
        )));
    }
    Ok(())
}

fn is_kebab_or_token(value: &str) -> bool {
    if value.is_empty() {
        return false;
    }
    let mut chars = value.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphanumeric() => {}
        _ => return false,
    }
    for c in chars {
        if !(c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.') {
            return false;
        }
    }
    true
}

fn is_admin_claim(value: &str) -> bool {
    if value.is_empty() {
        return false;
    }
    if value.contains(char::is_whitespace) {
        return false;
    }
    if value.contains(';') || value.contains('|') || value.contains('`') {
        return false;
    }
    for c in value.chars() {
        if !(c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' || c == ':' || c == '/')
        {
            return false;
        }
    }
    true
}

/// Build a fresh OIDC authorization request. The state,
/// nonce and PKCE verifier are random; the issued_at
/// and expires_at are derived from the configured
/// `state_ttl_seconds`. The `complete-auth` call needs
/// both the challenge and the original code verifier to
/// finish the round trip; both are returned here so the
/// caller can store the challenge for the lifetime of
/// the request and the verifier for the lifetime of
/// the session.
pub fn build_challenge(
    project_id: &str,
    config: &IdentityConfig,
    now: DateTime<Utc>,
) -> Result<AuthChallenge, ForgeError> {
    if let Err(reason) = crate::core::validate_project_id(project_id) {
        return Err(IdentityRejection::invalid(format!(
            "auth challenge project id `{project_id}` is invalid: {reason}"
        )));
    }
    if config.client_id.trim().is_empty() {
        return Err(IdentityRejection::invalid(
            "identity client_id is required to build a challenge",
        ));
    }
    let state = random_hex(VALUE_LEN);
    let nonce = random_hex(VALUE_LEN);
    let code_verifier = random_hex(VALUE_LEN);
    let code_challenge = code_challenge_s256(&code_verifier);
    let expires_at = now + Duration::seconds(config.state_ttl_seconds);
    Ok(AuthChallenge {
        project_id: project_id.to_string(),
        provider: config.provider.clone(),
        client_id: config.client_id.clone(),
        state,
        nonce,
        code_verifier,
        code_challenge,
        code_challenge_method: "S256".to_string(),
        redirect_uri: config.redirect_uri.clone(),
        scope: config.scopes.join(" "),
        audience: config.audience.clone(),
        issuer: config.issuer.clone(),
        issued_at: now,
        expires_at,
    })
}

/// Validate an OIDC callback against the original
/// challenge. Refuses mismatched state, expired
/// challenge, missing code, provider-reported errors
/// and shell metacharacter smuggling.
pub fn validate_callback(
    callback: &AuthCallback,
    challenge: &AuthChallenge,
    now: DateTime<Utc>,
) -> Result<(), ForgeError> {
    if callback.project_id != challenge.project_id {
        return Err(IdentityRejection::invalid(format!(
            "auth callback project id `{}` does not match the challenge \
             project id `{}`; callbacks are project-scoped",
            callback.project_id, challenge.project_id
        )));
    }
    if callback.state != challenge.state {
        return Err(IdentityRejection::invalid(
            "auth callback state does not match the issued challenge; \
             the provider or the caller is replaying or substituting a \
             challenge",
        ));
    }
    if challenge.is_expired(now) {
        return Err(IdentityRejection::invalid(format!(
            "auth challenge expired at {} (state_ttl_seconds={}); a new \
             challenge must be built before completing the round trip",
            challenge.expires_at.to_rfc3339(),
            (challenge.expires_at - challenge.issued_at).num_seconds()
        )));
    }
    if let Some(err) = &callback.error {
        let description = callback.error_description.as_deref().unwrap_or("");
        let description_clause = if description.trim().is_empty() {
            String::new()
        } else {
            format!(" description=`{}`", redact_credentials(description))
        };
        return Err(IdentityRejection::invalid(format!(
            "auth callback returned provider error `{err}`{description_clause}; \
             the project did not complete the OIDC round trip"
        )));
    }
    if callback.code.trim().is_empty() {
        return Err(IdentityRejection::invalid(
            "auth callback is missing the authorization code; the provider \
             did not return a usable code",
        ));
    }
    if callback.code.contains(char::is_whitespace)
        || callback.code.contains(';')
        || callback.code.contains('|')
    {
        return Err(IdentityRejection::invalid(
            "auth callback code contains shell metacharacters; the OIDC \
             code is opaque and must not embed shell-active characters",
        ));
    }
    Ok(())
}

/// Validate provider claims against the issued
/// challenge and the project's identity configuration.
/// Refuses wrong issuer, wrong audience, expired token,
/// mismatched nonce, missing scope and empty subject.
pub fn validate_claims(
    claims: &ProviderClaims,
    challenge: &AuthChallenge,
    config: &IdentityConfig,
    now: DateTime<Utc>,
) -> Result<(), ForgeError> {
    if claims.issuer != config.issuer {
        return Err(IdentityRejection::invalid(format!(
            "provider claims issuer `{}` does not match the project's \
             configured issuer `{}`",
            redact_credentials(&claims.issuer),
            redact_credentials(&config.issuer)
        )));
    }
    if claims.audience != config.audience {
        return Err(IdentityRejection::invalid(format!(
            "provider claims audience `{}` does not match the project's \
             configured audience `{}`",
            redact_credentials(&claims.audience),
            redact_credentials(&config.audience)
        )));
    }
    if claims.subject.trim().is_empty() {
        return Err(IdentityRejection::invalid(
            "provider claims subject is empty; OIDC requires a non-empty \
             `sub` claim",
        ));
    }
    if claims.expires_at <= claims.issued_at {
        return Err(IdentityRejection::invalid(format!(
            "provider claims exp {} is not after iat {}; the token is \
             already expired or the timestamps are invalid",
            claims.expires_at.to_rfc3339(),
            claims.issued_at.to_rfc3339()
        )));
    }
    if now >= claims.expires_at {
        return Err(IdentityRejection::invalid(format!(
            "provider claims exp {} is in the past; the token is expired \
             and must be refreshed before completing the round trip",
            claims.expires_at.to_rfc3339()
        )));
    }
    if claims.nonce != challenge.nonce {
        return Err(IdentityRejection::invalid(
            "provider claims nonce does not match the issued challenge; \
             the response is replayed or substituted",
        ));
    }
    for required in &config.scopes {
        if !claims.scopes.iter().any(|s| s == required) {
            return Err(IdentityRejection::invalid(format!(
                "provider claims scopes {:?} do not include the configured \
                 scope `{required}`",
                claims.scopes
            )));
        }
    }
    Ok(())
}

/// Resolve the admin-claim value from the provider
/// claims and the project's identity configuration.
/// Returns `Ok(true)` when the claim is present and
/// the value is in the configured allow list, `Ok(false)`
/// when the claim is missing or the value is not in the
/// allow list (the caller maps `false` to
/// `identity-permission-denied`).
pub fn admin_claim_grants(claims: &ProviderClaims, config: &IdentityConfig) -> bool {
    match claims.claim(&config.admin_claim) {
        // The claim may be a scalar (exact match) or a
        // comma-joined list of group/role values produced by
        // the OIDC verifier from an array claim; every
        // configured allow-list value is checked against each
        // element.
        Some(value) => value
            .split(',')
            .map(str::trim)
            .any(|part| config.admin_values.iter().any(|v| v == part)),
        None => false,
    }
}

/// Mint a per-project admin session. Refuses when the
/// admin claim does not grant admin access; the typed
/// `identity-permission-denied` code makes the boundary
/// observable so a project that authenticates a user
/// but lacks the project admin permission is refused
/// rather than silently granted access.
pub fn mint_session(
    config: &IdentityConfig,
    claims: &ProviderClaims,
    challenge: &AuthChallenge,
    now: DateTime<Utc>,
) -> Result<AdminSession, ForgeError> {
    if !admin_claim_grants(claims, config) {
        let claim_value = claims.claim(&config.admin_claim).unwrap_or("").to_string();
        return Err(ForgeError::IdentityPermissionDenied {
            reason: format!(
                "provider login for subject `{}` does not include the \
                 project admin permission; admin_claim `{}` value `{}` is \
                 not in the configured allow list; provider login does not \
                 imply admin authorization",
                redact_credentials(&claims.subject),
                redact_credentials(&config.admin_claim),
                redact_credentials(&claim_value)
            ),
        });
    }
    let session_id = random_hex(SESSION_ID_LEN);
    let expires_at = now + Duration::seconds(config.session_ttl_seconds);
    Ok(AdminSession {
        session_id,
        project_id: challenge.project_id.clone(),
        provider: config.provider.clone(),
        subject: claims.subject.clone(),
        issued_at: now,
        expires_at,
        permissions: vec![ADMIN_PERMISSION.to_string()],
        state: SessionState::Active,
        note: None,
    })
}

/// Validate a session against a project and a
/// permission. Refuses cross-project tokens (R2 failure
/// scenario), expired sessions, revoked sessions, and
/// missing permissions. The session id is the only
/// field the caller has to present; the rest is
/// resolved from the persisted state.
pub fn validate_session(
    session: &AdminSession,
    project_id: &str,
    required_permission: &str,
    now: DateTime<Utc>,
) -> Result<(), ForgeError> {
    if session.project_id != project_id {
        return Err(ForgeError::IdentitySessionCrossProject {
            reason: format!(
                "session `{}` was minted for project `{}`; presenting it \
                 to project `{project_id}` is refused",
                session.session_id, session.project_id
            ),
        });
    }
    if session.state == SessionState::Revoked {
        return Err(ForgeError::IdentitySessionExpired {
            reason: format!(
                "session `{}` is revoked; a new challenge must be built",
                session.session_id
            ),
        });
    }
    if session.is_expired(now) {
        return Err(ForgeError::IdentitySessionExpired {
            reason: format!(
                "session `{}` expired at {}",
                session.session_id,
                session.expires_at.to_rfc3339()
            ),
        });
    }
    if !session.permissions.iter().any(|p| p == required_permission) {
        return Err(ForgeError::IdentityPermissionDenied {
            reason: format!(
                "session `{}` does not carry the required permission \
                 `{required_permission}`; minted permissions: {:?}",
                session.session_id, session.permissions
            ),
        });
    }
    Ok(())
}

/// Mark a session as revoked. The session file is
/// updated in place; the revocation is project-scoped
/// (R2 boundary scenario: revoking one project's
/// session does not implicitly revoke another
/// project's session).
pub fn terminate_session(session: &mut AdminSession) {
    session.state = SessionState::Revoked;
    session.note = Some("revoked by forge identity terminate".to_string());
}

/// Per-project session file path. The path is lexically
/// scoped to the project directory, the project id and
/// the session id so two projects (or two sessions on
/// the same project) can never share a file.
pub fn session_path_for(
    project_dir: &Path,
    project_id: &str,
    session_id: &str,
) -> Result<PathBuf, ForgeError> {
    if project_id.trim().is_empty() {
        return Err(IdentityRejection::invalid(
            "session path requires a project id",
        ));
    }
    if session_id.trim().is_empty() {
        return Err(IdentityRejection::invalid(
            "session path requires a session id",
        ));
    }
    if !is_hex(session_id) {
        return Err(IdentityRejection::invalid(format!(
            "session id `{session_id}` is not a hex token; refusing to \
             resolve a path that could escape the per-project directory"
        )));
    }
    Ok(project_dir
        .join(IDENTITY_DIR)
        .join(project_id)
        .join("sessions")
        .join(format!("{session_id}.json")))
}

/// Per-project pending challenge file path. The path is
/// lexically scoped to the project directory, the project
/// id and the challenge's `state` value so two concurrent
/// challenges on the same project can never share a
/// file.
pub fn challenge_path_for(
    project_dir: &Path,
    project_id: &str,
    state: &str,
) -> Result<PathBuf, ForgeError> {
    if project_id.trim().is_empty() {
        return Err(IdentityRejection::invalid(
            "challenge path requires a project id",
        ));
    }
    if state.trim().is_empty() {
        return Err(IdentityRejection::invalid(
            "challenge path requires a state value",
        ));
    }
    if !is_hex(state) {
        return Err(IdentityRejection::invalid(format!(
            "challenge state `{state}` is not a hex token; refusing to \
             resolve a path that could escape the per-project directory"
        )));
    }
    Ok(project_dir
        .join(IDENTITY_DIR)
        .join(project_id)
        .join("pending")
        .join(format!("{state}.json")))
}

fn is_hex(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c) || ('A'..='F').contains(&c))
}

/// Load a session from disk. A missing file is
/// `Ok(None)` so a fresh project does not report a
/// missing-session error before the first challenge
/// completes.
pub fn load_session(
    project_dir: &Path,
    project_id: &str,
    session_id: &str,
) -> Result<Option<AdminSession>, ForgeError> {
    let path = session_path_for(project_dir, project_id, session_id)?;
    if !path.exists() {
        return Ok(None);
    }
    let bytes = fs::read(&path).map_err(|err| ForgeError::IdentityInvalid {
        reason: format!("cannot read session file {}: {err}", path.display()),
    })?;
    if bytes.is_empty() {
        return Ok(None);
    }
    let session: AdminSession =
        serde_json::from_slice(&bytes).map_err(|err| ForgeError::IdentityInvalid {
            reason: format!(
                "session file at {} is not valid JSON: {err}",
                path.display()
            ),
        })?;
    Ok(Some(session))
}

/// Persist a pending auth challenge (atomic `.tmp` + rename).
/// `complete-auth` reads the challenge back by `state` so the
/// caller can finish the round trip across processes.
pub fn save_challenge(
    project_dir: &Path,
    project_id: &str,
    challenge: &AuthChallenge,
) -> Result<(), ForgeError> {
    if challenge.project_id != project_id {
        return Err(ForgeError::IdentityInvalid {
            reason: format!(
                "challenge `{}` reports project id `{}`; refusing to write it \
                 under `{project_id}`",
                challenge.state, challenge.project_id
            ),
        });
    }
    let path = challenge_path_for(project_dir, project_id, &challenge.state)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| ForgeError::IdentityInvalid {
            reason: format!(
                "cannot create identity pending directory {}: {err}",
                parent.display()
            ),
        })?;
    }
    let bytes =
        serde_json::to_vec_pretty(challenge).map_err(|err| ForgeError::IdentityInvalid {
            reason: format!("cannot serialize auth challenge: {err}"),
        })?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, &bytes).map_err(|err| ForgeError::IdentityInvalid {
        reason: format!(
            "cannot write identity challenge tmp {}: {err}",
            tmp.display()
        ),
    })?;
    fs::rename(&tmp, &path).map_err(|err| ForgeError::IdentityInvalid {
        reason: format!("cannot rename identity challenge {}: {err}", path.display()),
    })?;
    Ok(())
}

/// Load a pending auth challenge by state. `Ok(None)` is
/// returned for an unknown or expired challenge so the caller
/// can surface a typed `identity-invalid` rejection.
pub fn load_challenge(
    project_dir: &Path,
    project_id: &str,
    state: &str,
) -> Result<Option<AuthChallenge>, ForgeError> {
    let path = challenge_path_for(project_dir, project_id, state)?;
    if !path.exists() {
        return Ok(None);
    }
    let bytes = fs::read(&path).map_err(|err| ForgeError::IdentityInvalid {
        reason: format!("cannot read challenge file {}: {err}", path.display()),
    })?;
    if bytes.is_empty() {
        return Ok(None);
    }
    let challenge: AuthChallenge =
        serde_json::from_slice(&bytes).map_err(|err| ForgeError::IdentityInvalid {
            reason: format!(
                "challenge file at {} is not valid JSON: {err}",
                path.display()
            ),
        })?;
    Ok(Some(challenge))
}

/// Remove a pending challenge file. Called by
/// `complete-auth` after a successful or failed round trip
/// so a stale challenge cannot be replayed.
pub fn delete_challenge_file(
    project_dir: &Path,
    project_id: &str,
    state: &str,
) -> Result<(), ForgeError> {
    let path = challenge_path_for(project_dir, project_id, state)?;
    if !path.exists() {
        return Ok(());
    }
    fs::remove_file(&path).map_err(|err| ForgeError::IdentityInvalid {
        reason: format!("cannot delete identity challenge {}: {err}", path.display()),
    })?;
    Ok(())
}

/// Persist a session to disk (atomic `.tmp` + rename).
pub fn save_session(
    project_dir: &Path,
    project_id: &str,
    session: &AdminSession,
) -> Result<(), ForgeError> {
    if session.project_id != project_id {
        return Err(ForgeError::IdentityInvalid {
            reason: format!(
                "session `{}` reports project id `{}`; refusing to write it \
                 under `{project_id}`",
                session.session_id, session.project_id
            ),
        });
    }
    let path = session_path_for(project_dir, project_id, &session.session_id)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| ForgeError::IdentityInvalid {
            reason: format!(
                "cannot create identity directory {}: {err}",
                parent.display()
            ),
        })?;
    }
    let bytes = serde_json::to_vec_pretty(session).map_err(|err| ForgeError::IdentityInvalid {
        reason: format!("cannot serialize admin session: {err}"),
    })?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, &bytes).map_err(|err| ForgeError::IdentityInvalid {
        reason: format!("cannot write identity session tmp {}: {err}", tmp.display()),
    })?;
    fs::rename(&tmp, &path).map_err(|err| ForgeError::IdentityInvalid {
        reason: format!("cannot rename identity session {}: {err}", path.display()),
    })?;
    Ok(())
}

/// List every persisted session for the named project.
/// The list is returned in stable id order so a future
/// CLI surface can render it deterministically.
pub fn list_sessions(
    project_dir: &Path,
    project_id: &str,
) -> Result<Vec<AdminSession>, ForgeError> {
    let dir = project_dir
        .join(IDENTITY_DIR)
        .join(project_id)
        .join("sessions");
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(&dir).map_err(|err| ForgeError::IdentityInvalid {
        reason: format!("cannot read identity directory {}: {err}", dir.display()),
    })? {
        let entry = match entry {
            Ok(value) => value,
            Err(_) => continue,
        };
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let bytes = match fs::read(&path) {
            Ok(value) => value,
            Err(_) => continue,
        };
        if let Ok(session) = serde_json::from_slice::<AdminSession>(&bytes) {
            if session.project_id == project_id {
                out.push(session);
            }
        }
    }
    out.sort_by(|a, b| a.session_id.cmp(&b.session_id));
    Ok(out)
}

/// Delete a session file. Used by `terminate` to make
/// sure a revoked session cannot be re-presented. The
/// `state` field is updated in memory and on disk
/// before the file is removed so a partial run still
/// reports `Revoked`.
pub fn delete_session_file(
    project_dir: &Path,
    project_id: &str,
    session_id: &str,
) -> Result<(), ForgeError> {
    let path = session_path_for(project_dir, project_id, session_id)?;
    if !path.exists() {
        return Ok(());
    }
    fs::remove_file(&path).map_err(|err| ForgeError::IdentityInvalid {
        reason: format!("cannot delete identity session {}: {err}", path.display()),
    })?;
    Ok(())
}

/// Search every registered project for the named
/// session. Returns the session and the project that
/// minted it so the caller can surface the
/// `identity-session-cross-project` rejection with
/// both project ids named. The session is found by
/// walking each registered project's
/// `.forge/identity/<project>/sessions/<id>.json`; the
/// returned `AdminSession::project_id` is the source of
/// truth so two projects can never share a session id
/// without the lookup discovering the source.
pub fn lookup_session_across_projects<I>(
    session_id: &str,
    projects: I,
) -> Result<Option<(AdminSession, String, std::path::PathBuf)>, ForgeError>
where
    I: IntoIterator<Item = (String, std::path::PathBuf)>,
{
    for (project_id, dir) in projects {
        if let Some(session) = load_session(&dir, &project_id, session_id)? {
            return Ok(Some((session, project_id, dir)));
        }
    }
    Ok(None)
}

/// Search the parent directory of the target project
/// for sibling project directories that own a session
/// with the given id. This is the fallback when the
/// session is not found in the target project's
/// identity directory and no registry is available
/// (e.g. a sandbox test fixture that does not register
/// the projects). The walk is bounded to the parent
/// directory's immediate children so a runaway scan
/// cannot escape the test sandbox. The session file's
/// `project_id` is the source of truth for the owner
/// so two projects can never share a session id
/// without the lookup discovering the source.
pub fn lookup_session_in_sibling_projects(
    project_dir: &Path,
    session_id: &str,
) -> Result<Option<(AdminSession, String, PathBuf)>, ForgeError> {
    let parent = match project_dir.parent() {
        Some(value) => value.to_path_buf(),
        None => return Ok(None),
    };
    let entries = match fs::read_dir(&parent) {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };
    for entry in entries {
        let entry = match entry {
            Ok(value) => value,
            Err(_) => continue,
        };
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let identity_root = path.join(IDENTITY_DIR);
        if !identity_root.is_dir() {
            continue;
        }
        let project_dirs = match fs::read_dir(&identity_root) {
            Ok(value) => value,
            Err(_) => continue,
        };
        for project_entry in project_dirs {
            let project_entry = match project_entry {
                Ok(value) => value,
                Err(_) => continue,
            };
            let project_path = project_entry.path();
            if !project_path.is_dir() {
                continue;
            }
            let sessions_dir = project_path.join("sessions");
            if !sessions_dir.is_dir() {
                continue;
            }
            let session_file = sessions_dir.join(format!("{session_id}.json"));
            if !session_file.is_file() {
                continue;
            }
            let bytes = match fs::read(&session_file) {
                Ok(value) => value,
                Err(_) => continue,
            };
            if let Ok(session) = serde_json::from_slice::<AdminSession>(&bytes) {
                let owner_id = session.project_id.clone();
                return Ok(Some((session, owner_id, project_path)));
            }
        }
    }
    Ok(None)
}

/// Redact credential-shaped substrings in identity
/// evidence. Delegates to
/// [`crate::policy::redact_credentials`] so the
/// identity contract shares one definition of "secret"
/// with the policy, release, distribution, docs and
/// deploy adapters.
pub fn redact_identity_evidence(text: &str) -> String {
    redact_credentials(text)
}

/// Render a challenge in human-readable form. Used by
/// the CLI; the JSON envelope is rendered by serde.
pub fn render_challenge_human(challenge: &AuthChallenge) -> String {
    format!(
        "challenge project={} provider={} state={} nonce={} code_challenge={} method={} \
         redirect_uri={} scope={} audience={} expires_at={}",
        challenge.project_id,
        challenge.provider,
        challenge.state,
        challenge.nonce,
        challenge.code_challenge,
        challenge.code_challenge_method,
        challenge.redirect_uri,
        challenge.scope,
        challenge.audience,
        challenge.expires_at.to_rfc3339(),
    )
}

/// Render a session in human-readable form.
pub fn render_session_human(session: &AdminSession) -> String {
    let state = match session.state {
        SessionState::Active => "active",
        SessionState::Expired => "expired",
        SessionState::Revoked => "revoked",
    };
    format!(
        "session project={} provider={} subject={} permissions={} state={} \
         issued_at={} expires_at={} note={}",
        session.project_id,
        session.provider,
        session.subject,
        session.permissions.join(","),
        state,
        session.issued_at.to_rfc3339(),
        session.expires_at.to_rfc3339(),
        session.note.as_deref().unwrap_or(""),
    )
}

/// Render an `IdentityOutcome` in human-readable form.
pub fn render_outcome_human(outcome: &IdentityOutcome) -> String {
    match outcome {
        IdentityOutcome::Session(session) => {
            format!("session minted: {}", render_session_human(session))
        }
        IdentityOutcome::Rejected(rejection) => format!(
            "identity rejected [{}]: {}",
            rejection.code, rejection.reason
        ),
    }
}

/// Browser authentication verifier. The HTTP callback handler
/// hands the verifier the parsed [`AuthCallback`] and the
/// originating [`AuthChallenge`]; the verifier performs the
/// OIDC code exchange and ID-token verification and returns
/// the normalised [`ProviderClaims`]. The trait abstracts
/// the OIDC client so tests can supply a deterministic fake
/// without touching the network, and so a future live
/// provider implementation can be swapped in without
/// changing the call sites.
pub trait BrowserAuthVerifier {
    /// Exchange the authorization code for tokens and verify
    /// the ID-token cryptographically against the project's
    /// configured issuer, audience, redirect URI, PKCE
    /// challenge, nonce, time bounds, and admin-claim
    /// allowlist. The returned claims are the typed surface
    /// [`validate_claims`] and [`mint_session`] consume.
    fn exchange_and_verify(
        &self,
        callback: &AuthCallback,
        challenge: &AuthChallenge,
        config: &IdentityConfig,
        now: DateTime<Utc>,
    ) -> Result<ProviderClaims, ForgeError>;
}

/// Test-only verifier that pops pre-canned outcomes off a
/// queue. Every `exchange_and_verify` call consumes one
/// queued outcome; a missing queued outcome produces a
/// `identity-auth-failed` refusal so a stale test fixture
/// cannot quietly mint a session.
pub struct FakeBrowserAuthVerifier {
    queue: Mutex<Vec<Result<ProviderClaims, ForgeError>>>,
}

impl FakeBrowserAuthVerifier {
    /// Empty queue. Every call returns the typed
    /// `identity-auth-failed` refusal until at least one
    /// outcome is queued.
    pub fn new() -> Self {
        Self {
            queue: Mutex::new(Vec::new()),
        }
    }

    /// Queue a successful outcome. The most recent
    /// `queue_success`/`queue_error` is consumed first.
    pub fn queue_success(&self, claims: ProviderClaims) {
        self.queue
            .lock()
            .expect("fake verifier mutex")
            .push(Ok(claims));
    }

    /// Queue a failure outcome. The verifier returns the
    /// queued [`ForgeError`] on the next call.
    pub fn queue_error(&self, err: ForgeError) {
        self.queue
            .lock()
            .expect("fake verifier mutex")
            .push(Err(err));
    }
}

impl Default for FakeBrowserAuthVerifier {
    fn default() -> Self {
        Self::new()
    }
}

impl BrowserAuthVerifier for FakeBrowserAuthVerifier {
    fn exchange_and_verify(
        &self,
        _callback: &AuthCallback,
        _challenge: &AuthChallenge,
        _config: &IdentityConfig,
        _now: DateTime<Utc>,
    ) -> Result<ProviderClaims, ForgeError> {
        let mut queue = self.queue.lock().expect("fake verifier mutex");
        match queue.pop() {
            Some(outcome) => outcome,
            None => Err(ForgeError::IdentityAuthFailed {
                reason: "FakeBrowserAuthVerifier has no queued outcome; queue a \
                         success or error before driving the callback"
                    .to_string(),
            }),
        }
    }
}

/// Complete a browser OIDC round trip. Validates the
/// callback against the persisted challenge, asks the
/// verifier to exchange the code and verify the ID-token,
/// validates the claims against the project's identity
/// configuration, mints a session, persists it, and removes
/// the consumed challenge so a replay attempt finds no
/// challenge to consume. The trait parameter makes the
/// network-free fake usable in tests while production uses
/// [`LibraryBrowserAuthVerifier`], which performs the real
/// discovery/code-exchange/ID-token verification through
/// the `openidconnect` crate.
pub fn complete_browser_auth<V: BrowserAuthVerifier + ?Sized>(
    project_dir: &Path,
    config: &IdentityConfig,
    callback: &AuthCallback,
    challenge: &AuthChallenge,
    verifier: &V,
    now: DateTime<Utc>,
) -> Result<AdminSession, ForgeError> {
    validate_callback(callback, challenge, now)?;
    let claims = verifier.exchange_and_verify(callback, challenge, config, now)?;
    validate_claims(&claims, challenge, config, now)?;
    let session = mint_session(config, &claims, challenge, now)?;
    save_session(project_dir, &challenge.project_id, &session)?;
    delete_challenge_file(project_dir, &challenge.project_id, &challenge.state)?;
    Ok(session)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_meta() -> IdentityMeta {
        IdentityMeta {
            provider: Some("okta".to_string()),
            issuer: Some("https://example.okta.com".to_string()),
            client_id: Some("forge-admin".to_string()),
            audience: Some("forge-admin".to_string()),
            redirect_uri: Some("https://admin.example.com/oidc/callback".to_string()),
            scopes: vec!["openid".to_string(), "profile".to_string()],
            jwks_uri: Some("https://example.okta.com/.well-known/jwks.json".to_string()),
            state_ttl_seconds: Some(120),
            session_ttl_seconds: Some(3600),
            admin_claim: Some("groups".to_string()),
            admin_values: vec!["forge-admins".to_string()],
            client_secret_ref: Some("env://OIDC_CLIENT_SECRET".to_string()),
        }
    }

    fn now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-01-15T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    #[test]
    fn validate_config_accepts_well_formed_block() {
        let cfg = IdentityConfig::from_manifest("forge-admin", &sample_meta()).unwrap();
        assert_eq!(cfg.provider, "okta");
        assert_eq!(cfg.audience, "forge-admin");
        assert_eq!(cfg.scopes, vec!["openid", "profile"]);
    }

    #[test]
    fn validate_config_audience_defaults_to_client_id() {
        let mut meta = sample_meta();
        meta.audience = None;
        let cfg = IdentityConfig::from_manifest("forge-admin", &meta).unwrap();
        assert_eq!(cfg.audience, "forge-admin");
    }

    #[test]
    fn validate_config_rejects_empty_provider() {
        let mut meta = sample_meta();
        meta.provider = Some(String::new());
        let err = IdentityConfig::from_manifest("forge-admin", &meta).unwrap_err();
        assert_eq!(err.code(), "identity-invalid");
    }

    #[test]
    fn validate_config_rejects_unknown_provider() {
        let mut meta = sample_meta();
        meta.provider = Some("custom-idp".to_string());
        let err = IdentityConfig::from_manifest("forge-admin", &meta).unwrap_err();
        assert_eq!(err.code(), "identity-invalid");
    }

    #[test]
    fn validate_config_rejects_cleartext_issuer() {
        let mut meta = sample_meta();
        meta.issuer = Some("http://example.okta.com".to_string());
        let err = IdentityConfig::from_manifest("forge-admin", &meta).unwrap_err();
        assert_eq!(err.code(), "identity-invalid");
    }

    #[test]
    fn validate_config_rejects_missing_openid_scope() {
        let mut meta = sample_meta();
        meta.scopes = vec!["profile".to_string()];
        let err = IdentityConfig::from_manifest("forge-admin", &meta).unwrap_err();
        assert_eq!(err.code(), "identity-invalid");
    }

    #[test]
    fn validate_config_rejects_empty_admin_values() {
        let mut meta = sample_meta();
        meta.admin_values.clear();
        let err = IdentityConfig::from_manifest("forge-admin", &meta).unwrap_err();
        assert_eq!(err.code(), "identity-invalid");
    }

    #[test]
    fn validate_config_rejects_state_ttl_out_of_range() {
        let mut meta = sample_meta();
        meta.state_ttl_seconds = Some(MAX_STATE_TTL_SECONDS + 1);
        let err = IdentityConfig::from_manifest("forge-admin", &meta).unwrap_err();
        assert_eq!(err.code(), "identity-invalid");
    }

    #[test]
    fn validate_config_rejects_session_ttl_out_of_range() {
        let mut meta = sample_meta();
        meta.session_ttl_seconds = Some(MAX_SESSION_TTL_SECONDS + 1);
        let err = IdentityConfig::from_manifest("forge-admin", &meta).unwrap_err();
        assert_eq!(err.code(), "identity-invalid");
    }

    #[test]
    fn validate_config_rejects_raw_secret() {
        let mut meta = sample_meta();
        meta.client_secret_ref = Some("super-secret".to_string());
        let err = IdentityConfig::from_manifest("forge-admin", &meta).unwrap_err();
        assert_eq!(err.code(), "identity-invalid");
    }

    #[test]
    fn validate_config_rejects_shell_metacharacters_in_redirect() {
        let mut meta = sample_meta();
        meta.redirect_uri = Some("https://admin.example.com/;rm -rf".to_string());
        let err = IdentityConfig::from_manifest("forge-admin", &meta).unwrap_err();
        assert_eq!(err.code(), "identity-invalid");
    }

    #[test]
    fn validate_config_rejects_bad_project_id() {
        let err = IdentityConfig::from_manifest("Bad_ID", &sample_meta()).unwrap_err();
        assert_eq!(err.code(), "identity-invalid");
    }

    #[test]
    fn validate_config_rejects_too_many_scopes() {
        let mut meta = sample_meta();
        meta.scopes = (0..MAX_SCOPES + 1).map(|i| format!("scope-{i}")).collect();
        let err = IdentityConfig::from_manifest("forge-admin", &meta).unwrap_err();
        assert_eq!(err.code(), "identity-invalid");
    }

    #[test]
    fn build_challenge_is_deterministic_in_shape_but_random_in_values() {
        let cfg = IdentityConfig::from_manifest("forge-admin", &sample_meta()).unwrap();
        let a = build_challenge("forge-admin", &cfg, now()).unwrap();
        let b = build_challenge("forge-admin", &cfg, now()).unwrap();
        assert_eq!(a.project_id, "forge-admin");
        assert_eq!(a.code_challenge_method, "S256");
        assert_ne!(a.state, b.state);
        assert_ne!(a.nonce, b.nonce);
        assert_ne!(a.code_verifier, b.code_verifier);
        assert_eq!(a.code_challenge, code_challenge_s256(&a.code_verifier));
    }

    #[test]
    fn build_challenge_rejects_bad_project_id() {
        let cfg = IdentityConfig::from_manifest("forge-admin", &sample_meta()).unwrap();
        let err = build_challenge("Bad_ID", &cfg, now()).unwrap_err();
        assert_eq!(err.code(), "identity-invalid");
    }

    #[test]
    fn validate_callback_accepts_matching_state_within_ttl() {
        let cfg = IdentityConfig::from_manifest("forge-admin", &sample_meta()).unwrap();
        let challenge = build_challenge("forge-admin", &cfg, now()).unwrap();
        let callback = AuthCallback {
            project_id: "forge-admin".to_string(),
            state: challenge.state.clone(),
            code: "abcd1234".to_string(),
            error: None,
            error_description: None,
        };
        validate_callback(&callback, &challenge, now()).unwrap();
    }

    #[test]
    fn validate_callback_rejects_mismatched_state() {
        let cfg = IdentityConfig::from_manifest("forge-admin", &sample_meta()).unwrap();
        let challenge = build_challenge("forge-admin", &cfg, now()).unwrap();
        let callback = AuthCallback {
            project_id: "forge-admin".to_string(),
            state: "deadbeef".to_string(),
            code: "abcd1234".to_string(),
            error: None,
            error_description: None,
        };
        let err = validate_callback(&callback, &challenge, now()).unwrap_err();
        assert_eq!(err.code(), "identity-invalid");
    }

    #[test]
    fn validate_callback_rejects_expired_challenge() {
        let cfg = IdentityConfig::from_manifest("forge-admin", &sample_meta()).unwrap();
        let challenge = build_challenge("forge-admin", &cfg, now()).unwrap();
        let callback = AuthCallback {
            project_id: "forge-admin".to_string(),
            state: challenge.state.clone(),
            code: "abcd1234".to_string(),
            error: None,
            error_description: None,
        };
        let later = challenge.expires_at + Duration::seconds(1);
        let err = validate_callback(&callback, &challenge, later).unwrap_err();
        assert_eq!(err.code(), "identity-invalid");
    }

    #[test]
    fn validate_callback_rejects_provider_error() {
        let cfg = IdentityConfig::from_manifest("forge-admin", &sample_meta()).unwrap();
        let challenge = build_challenge("forge-admin", &cfg, now()).unwrap();
        let callback = AuthCallback {
            project_id: "forge-admin".to_string(),
            state: challenge.state.clone(),
            code: String::new(),
            error: Some("access_denied".to_string()),
            error_description: Some("user denied".to_string()),
        };
        let err = validate_callback(&callback, &challenge, now()).unwrap_err();
        assert_eq!(err.code(), "identity-invalid");
    }

    #[test]
    fn validate_callback_rejects_cross_project_state() {
        let cfg = IdentityConfig::from_manifest("forge-admin", &sample_meta()).unwrap();
        let challenge = build_challenge("forge-admin", &cfg, now()).unwrap();
        let callback = AuthCallback {
            project_id: "other".to_string(),
            state: challenge.state.clone(),
            code: "abcd1234".to_string(),
            error: None,
            error_description: None,
        };
        let err = validate_callback(&callback, &challenge, now()).unwrap_err();
        assert_eq!(err.code(), "identity-invalid");
    }

    #[test]
    fn validate_callback_rejects_shell_metacharacters_in_code() {
        let cfg = IdentityConfig::from_manifest("forge-admin", &sample_meta()).unwrap();
        let challenge = build_challenge("forge-admin", &cfg, now()).unwrap();
        let callback = AuthCallback {
            project_id: "forge-admin".to_string(),
            state: challenge.state.clone(),
            code: "abc;rm -rf /".to_string(),
            error: None,
            error_description: None,
        };
        let err = validate_callback(&callback, &challenge, now()).unwrap_err();
        assert_eq!(err.code(), "identity-invalid");
    }

    fn sample_claims(challenge: &AuthChallenge, cfg: &IdentityConfig) -> ProviderClaims {
        let mut claims = BTreeMap::new();
        claims.insert(cfg.admin_claim.clone(), "forge-admins".to_string());
        ProviderClaims {
            issuer: cfg.issuer.clone(),
            audience: cfg.audience.clone(),
            subject: "user-1".to_string(),
            issued_at: now(),
            expires_at: now() + Duration::seconds(60),
            nonce: challenge.nonce.clone(),
            scopes: cfg.scopes.clone(),
            claims,
        }
    }

    #[test]
    fn validate_claims_accepts_matching_token() {
        let cfg = IdentityConfig::from_manifest("forge-admin", &sample_meta()).unwrap();
        let challenge = build_challenge("forge-admin", &cfg, now()).unwrap();
        let claims = sample_claims(&challenge, &cfg);
        validate_claims(&claims, &challenge, &cfg, now()).unwrap();
    }

    #[test]
    fn validate_claims_rejects_wrong_issuer() {
        let cfg = IdentityConfig::from_manifest("forge-admin", &sample_meta()).unwrap();
        let challenge = build_challenge("forge-admin", &cfg, now()).unwrap();
        let mut claims = sample_claims(&challenge, &cfg);
        claims.issuer = "https://attacker.example.com".to_string();
        let err = validate_claims(&claims, &challenge, &cfg, now()).unwrap_err();
        assert_eq!(err.code(), "identity-invalid");
    }

    #[test]
    fn validate_claims_rejects_wrong_audience() {
        let cfg = IdentityConfig::from_manifest("forge-admin", &sample_meta()).unwrap();
        let challenge = build_challenge("forge-admin", &cfg, now()).unwrap();
        let mut claims = sample_claims(&challenge, &cfg);
        claims.audience = "other-audience".to_string();
        let err = validate_claims(&claims, &challenge, &cfg, now()).unwrap_err();
        assert_eq!(err.code(), "identity-invalid");
    }

    #[test]
    fn validate_claims_rejects_expired_token() {
        let cfg = IdentityConfig::from_manifest("forge-admin", &sample_meta()).unwrap();
        let challenge = build_challenge("forge-admin", &cfg, now()).unwrap();
        let mut claims = sample_claims(&challenge, &cfg);
        claims.expires_at = now() - Duration::seconds(1);
        let err = validate_claims(&claims, &challenge, &cfg, now()).unwrap_err();
        assert_eq!(err.code(), "identity-invalid");
    }

    #[test]
    fn validate_claims_rejects_nonce_mismatch() {
        let cfg = IdentityConfig::from_manifest("forge-admin", &sample_meta()).unwrap();
        let challenge = build_challenge("forge-admin", &cfg, now()).unwrap();
        let mut claims = sample_claims(&challenge, &cfg);
        claims.nonce = "deadbeef".to_string();
        let err = validate_claims(&claims, &challenge, &cfg, now()).unwrap_err();
        assert_eq!(err.code(), "identity-invalid");
    }

    #[test]
    fn validate_claims_rejects_missing_required_scope() {
        let cfg = IdentityConfig::from_manifest("forge-admin", &sample_meta()).unwrap();
        let challenge = build_challenge("forge-admin", &cfg, now()).unwrap();
        let mut claims = sample_claims(&challenge, &cfg);
        claims.scopes = vec!["openid".to_string()];
        let err = validate_claims(&claims, &challenge, &cfg, now()).unwrap_err();
        assert_eq!(err.code(), "identity-invalid");
    }

    #[test]
    fn admin_claim_grants_returns_false_when_value_not_in_allow_list() {
        let cfg = IdentityConfig::from_manifest("forge-admin", &sample_meta()).unwrap();
        let challenge = build_challenge("forge-admin", &cfg, now()).unwrap();
        let mut claims = sample_claims(&challenge, &cfg);
        claims
            .claims
            .insert(cfg.admin_claim.clone(), "intern".to_string());
        assert!(!admin_claim_grants(&claims, &cfg));
    }

    #[test]
    fn admin_claim_grants_returns_true_when_value_in_allow_list() {
        let cfg = IdentityConfig::from_manifest("forge-admin", &sample_meta()).unwrap();
        let challenge = build_challenge("forge-admin", &cfg, now()).unwrap();
        let claims = sample_claims(&challenge, &cfg);
        assert!(admin_claim_grants(&claims, &cfg));
    }

    #[test]
    fn mint_session_refuses_non_admin_claim() {
        let cfg = IdentityConfig::from_manifest("forge-admin", &sample_meta()).unwrap();
        let challenge = build_challenge("forge-admin", &cfg, now()).unwrap();
        let mut claims = sample_claims(&challenge, &cfg);
        claims
            .claims
            .insert(cfg.admin_claim.clone(), "intern".to_string());
        let err = mint_session(&cfg, &claims, &challenge, now()).unwrap_err();
        assert_eq!(err.code(), "identity-permission-denied");
    }

    #[test]
    fn mint_session_accepts_admin_claim() {
        let cfg = IdentityConfig::from_manifest("forge-admin", &sample_meta()).unwrap();
        let challenge = build_challenge("forge-admin", &cfg, now()).unwrap();
        let claims = sample_claims(&challenge, &cfg);
        let session = mint_session(&cfg, &claims, &challenge, now()).unwrap();
        assert_eq!(session.project_id, "forge-admin");
        assert_eq!(session.state, SessionState::Active);
        assert!(session.permissions.contains(&ADMIN_PERMISSION.to_string()));
    }

    #[test]
    fn validate_session_rejects_cross_project_token() {
        let cfg = IdentityConfig::from_manifest("forge-admin", &sample_meta()).unwrap();
        let challenge = build_challenge("forge-admin", &cfg, now()).unwrap();
        let claims = sample_claims(&challenge, &cfg);
        let session = mint_session(&cfg, &claims, &challenge, now()).unwrap();
        let err = validate_session(&session, "other", ADMIN_PERMISSION, now()).unwrap_err();
        assert_eq!(err.code(), "identity-session-cross-project");
    }

    #[test]
    fn validate_session_rejects_expired() {
        let cfg = IdentityConfig::from_manifest("forge-admin", &sample_meta()).unwrap();
        let challenge = build_challenge("forge-admin", &cfg, now()).unwrap();
        let claims = sample_claims(&challenge, &cfg);
        let mut session = mint_session(&cfg, &claims, &challenge, now()).unwrap();
        session.expires_at = now() - Duration::seconds(1);
        let err = validate_session(&session, "forge-admin", ADMIN_PERMISSION, now()).unwrap_err();
        assert_eq!(err.code(), "identity-session-expired");
    }

    #[test]
    fn validate_session_rejects_revoked() {
        let cfg = IdentityConfig::from_manifest("forge-admin", &sample_meta()).unwrap();
        let challenge = build_challenge("forge-admin", &cfg, now()).unwrap();
        let claims = sample_claims(&challenge, &cfg);
        let mut session = mint_session(&cfg, &claims, &challenge, now()).unwrap();
        session.state = SessionState::Revoked;
        let err = validate_session(&session, "forge-admin", ADMIN_PERMISSION, now()).unwrap_err();
        assert_eq!(err.code(), "identity-session-expired");
    }

    #[test]
    fn validate_session_rejects_missing_permission() {
        let cfg = IdentityConfig::from_manifest("forge-admin", &sample_meta()).unwrap();
        let challenge = build_challenge("forge-admin", &cfg, now()).unwrap();
        let claims = sample_claims(&challenge, &cfg);
        let mut session = mint_session(&cfg, &claims, &challenge, now()).unwrap();
        session.permissions.clear();
        let err = validate_session(&session, "forge-admin", ADMIN_PERMISSION, now()).unwrap_err();
        assert_eq!(err.code(), "identity-permission-denied");
    }

    #[test]
    fn validate_session_accepts_active_session() {
        let cfg = IdentityConfig::from_manifest("forge-admin", &sample_meta()).unwrap();
        let challenge = build_challenge("forge-admin", &cfg, now()).unwrap();
        let claims = sample_claims(&challenge, &cfg);
        let session = mint_session(&cfg, &claims, &challenge, now()).unwrap();
        validate_session(&session, "forge-admin", ADMIN_PERMISSION, now()).unwrap();
    }

    #[test]
    fn terminate_session_marks_revoked() {
        let cfg = IdentityConfig::from_manifest("forge-admin", &sample_meta()).unwrap();
        let challenge = build_challenge("forge-admin", &cfg, now()).unwrap();
        let claims = sample_claims(&challenge, &cfg);
        let mut session = mint_session(&cfg, &claims, &challenge, now()).unwrap();
        terminate_session(&mut session);
        assert_eq!(session.state, SessionState::Revoked);
    }

    #[test]
    fn session_path_rejects_non_hex_id() {
        let tmp = tempfile::tempdir().unwrap();
        let err = session_path_for(tmp.path(), "forge-admin", "not-hex!").unwrap_err();
        assert_eq!(err.code(), "identity-invalid");
    }

    #[test]
    fn save_and_load_session_round_trip() {
        let cfg = IdentityConfig::from_manifest("forge-admin", &sample_meta()).unwrap();
        let challenge = build_challenge("forge-admin", &cfg, now()).unwrap();
        let claims = sample_claims(&challenge, &cfg);
        let session = mint_session(&cfg, &claims, &challenge, now()).unwrap();
        let tmp = tempfile::tempdir().unwrap();
        save_session(tmp.path(), "forge-admin", &session).unwrap();
        let loaded = load_session(tmp.path(), "forge-admin", &session.session_id)
            .unwrap()
            .unwrap();
        assert_eq!(loaded, session);
    }

    #[test]
    fn save_session_refuses_project_mismatch() {
        let cfg = IdentityConfig::from_manifest("forge-admin", &sample_meta()).unwrap();
        let challenge = build_challenge("forge-admin", &cfg, now()).unwrap();
        let claims = sample_claims(&challenge, &cfg);
        let session = mint_session(&cfg, &claims, &challenge, now()).unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let err = save_session(tmp.path(), "other", &session).unwrap_err();
        assert_eq!(err.code(), "identity-invalid");
    }

    #[test]
    fn load_session_returns_none_for_missing_file() {
        let tmp = tempfile::tempdir().unwrap();
        let session = load_session(tmp.path(), "forge-admin", "deadbeef").unwrap();
        assert!(session.is_none());
    }

    #[test]
    fn list_sessions_returns_stable_order() {
        let cfg = IdentityConfig::from_manifest("forge-admin", &sample_meta()).unwrap();
        let challenge = build_challenge("forge-admin", &cfg, now()).unwrap();
        let claims = sample_claims(&challenge, &cfg);
        let tmp = tempfile::tempdir().unwrap();
        let mut first_ids = Vec::new();
        for _ in 0..3 {
            let session = mint_session(&cfg, &claims, &challenge, now()).unwrap();
            first_ids.push(session.session_id.clone());
            save_session(tmp.path(), "forge-admin", &session).unwrap();
        }
        let listed = list_sessions(tmp.path(), "forge-admin").unwrap();
        let listed_ids: Vec<String> = listed.iter().map(|s| s.session_id.clone()).collect();
        let mut expected = first_ids.clone();
        expected.sort();
        assert_eq!(listed_ids, expected);
    }

    #[test]
    fn list_sessions_returns_empty_for_missing_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let listed = list_sessions(tmp.path(), "forge-admin").unwrap();
        assert!(listed.is_empty());
    }

    #[test]
    fn terminate_does_not_implicitly_revoke_other_projects() {
        // Two projects, one provider, two sessions. Revoking
        // project A's session must not affect project B's
        // session (R2 boundary scenario).
        let cfg = IdentityConfig::from_manifest("forge-admin", &sample_meta()).unwrap();
        let challenge_a = build_challenge("project-a", &cfg, now()).unwrap();
        let challenge_b = build_challenge("project-b", &cfg, now()).unwrap();
        let claims_a = sample_claims(&challenge_a, &cfg);
        let claims_b = sample_claims(&challenge_b, &cfg);
        let mut session_a = mint_session(&cfg, &claims_a, &challenge_a, now()).unwrap();
        let session_b = mint_session(&cfg, &claims_b, &challenge_b, now()).unwrap();
        terminate_session(&mut session_a);
        assert_eq!(session_a.state, SessionState::Revoked);
        assert_eq!(session_b.state, SessionState::Active);
    }

    #[test]
    fn redact_identity_evidence_delegates_to_policy() {
        let sample = "token=ghp_abcdefghijklmnopqrstuvwxyz0123456789";
        let redacted = redact_identity_evidence(sample);
        assert!(redacted.contains("[REDACTED]"));
        assert!(!redacted.contains("ghp_abcdefghijklmnopqrstuvwxyz"));
    }

    #[test]
    fn human_renderers_carry_required_fields() {
        let cfg = IdentityConfig::from_manifest("forge-admin", &sample_meta()).unwrap();
        let challenge = build_challenge("forge-admin", &cfg, now()).unwrap();
        let text = render_challenge_human(&challenge);
        for needle in [
            "forge-admin",
            "okta",
            &challenge.state,
            &challenge.code_challenge,
            "S256",
        ] {
            assert!(text.contains(needle), "{needle} missing in {text}");
        }
        let claims = sample_claims(&challenge, &cfg);
        let session = mint_session(&cfg, &claims, &challenge, now()).unwrap();
        let text = render_session_human(&session);
        for needle in ["forge-admin", "okta", "user-1", "active"] {
            assert!(text.contains(needle), "{needle} missing in {text}");
        }
        let outcome = IdentityOutcome::Session(session);
        let text = render_outcome_human(&outcome);
        assert!(text.contains("session minted"));
        let rejection = IdentityRejection {
            code: "identity-invalid".to_string(),
            reason: "x".to_string(),
            project_id: Some("forge-admin".to_string()),
            session_id: None,
        };
        let outcome = IdentityOutcome::Rejected(rejection);
        let text = render_outcome_human(&outcome);
        assert!(text.contains("identity rejected"));
        assert!(text.contains("identity-invalid"));
    }

    #[test]
    fn supported_provider_list_is_stable() {
        let expected = &[
            "okta",
            "auth0",
            "keycloak",
            "azure-ad",
            "google",
            "github",
            "okta-fixture",
        ];
        assert_eq!(SUPPORTED_PROVIDERS, expected);
        assert_eq!(SUPPORTED_CODE_CHALLENGE_METHODS, &["S256"]);
    }

    /// The verified round trip consumes the challenge and
    /// persists exactly one session for the project. The
    /// verifier is the network-free fake so the test is
    /// deterministic.
    #[test]
    fn complete_browser_auth_mints_and_consumes_challenge() {
        let cfg = IdentityConfig::from_manifest("forge-admin", &sample_meta()).unwrap();
        let challenge = build_challenge("forge-admin", &cfg, now()).unwrap();
        let dir = tempfile::tempdir().unwrap();
        save_challenge(dir.path(), "forge-admin", &challenge).unwrap();
        let verifier = FakeBrowserAuthVerifier::new();
        verifier.queue_success(sample_claims(&challenge, &cfg));
        let callback = AuthCallback {
            project_id: "forge-admin".to_string(),
            state: challenge.state.clone(),
            code: "abcd1234".to_string(),
            error: None,
            error_description: None,
        };
        let session =
            complete_browser_auth(dir.path(), &cfg, &callback, &challenge, &verifier, now())
                .expect("verified round trip");
        assert_eq!(session.project_id, "forge-admin");
        assert_eq!(session.state, SessionState::Active);
        assert!(
            !challenge_path_for(dir.path(), "forge-admin", &challenge.state)
                .unwrap()
                .exists(),
            "the consumed challenge file must be removed"
        );
        let session_path =
            session_path_for(dir.path(), "forge-admin", &session.session_id).unwrap();
        assert!(session_path.exists(), "session file must be persisted");
    }

    #[test]
    fn complete_browser_auth_refuses_verifier_error() {
        let cfg = IdentityConfig::from_manifest("forge-admin", &sample_meta()).unwrap();
        let challenge = build_challenge("forge-admin", &cfg, now()).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let verifier = FakeBrowserAuthVerifier::new();
        verifier.queue_error(ForgeError::IdentityAuthFailed {
            reason: "provider rejected the exchange".to_string(),
        });
        let callback = AuthCallback {
            project_id: "forge-admin".to_string(),
            state: challenge.state.clone(),
            code: "abcd1234".to_string(),
            error: None,
            error_description: None,
        };
        let err = complete_browser_auth(dir.path(), &cfg, &callback, &challenge, &verifier, now())
            .unwrap_err();
        assert_eq!(err.code(), "identity-auth-failed");
        // No session was persisted on a failed verification.
        let sessions_dir = dir.path().join(".forge/identity/forge-admin/sessions");
        assert!(!sessions_dir.exists() || sessions_dir.read_dir().unwrap().next().is_none());
    }

    #[test]
    fn complete_browser_auth_refuses_non_admin_claim() {
        let cfg = IdentityConfig::from_manifest("forge-admin", &sample_meta()).unwrap();
        let challenge = build_challenge("forge-admin", &cfg, now()).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let verifier = FakeBrowserAuthVerifier::new();
        let mut claims = sample_claims(&challenge, &cfg);
        claims
            .claims
            .insert(cfg.admin_claim.clone(), "intern".to_string());
        verifier.queue_success(claims);
        let callback = AuthCallback {
            project_id: "forge-admin".to_string(),
            state: challenge.state.clone(),
            code: "abcd1234".to_string(),
            error: None,
            error_description: None,
        };
        let err = complete_browser_auth(dir.path(), &cfg, &callback, &challenge, &verifier, now())
            .unwrap_err();
        assert_eq!(err.code(), "identity-permission-denied");
    }

    #[test]
    fn library_verifier_refuses_provider_error_without_network() {
        let cfg = IdentityConfig::from_manifest("forge-admin", &sample_meta()).unwrap();
        let challenge = build_challenge("forge-admin", &cfg, now()).unwrap();
        let callback = AuthCallback {
            project_id: "forge-admin".to_string(),
            state: challenge.state.clone(),
            code: String::new(),
            error: Some("access_denied".to_string()),
            error_description: Some("user denied".to_string()),
        };
        let err = LibraryBrowserAuthVerifier
            .exchange_and_verify(&callback, &challenge, &cfg, now())
            .unwrap_err();
        assert_eq!(err.code(), "identity-auth-failed");
    }

    #[test]
    fn library_verifier_refuses_expired_challenge_without_network() {
        let cfg = IdentityConfig::from_manifest("forge-admin", &sample_meta()).unwrap();
        let challenge = build_challenge("forge-admin", &cfg, now()).unwrap();
        let callback = AuthCallback {
            project_id: "forge-admin".to_string(),
            state: challenge.state.clone(),
            code: "abcd1234".to_string(),
            error: None,
            error_description: None,
        };
        let later = challenge.expires_at + Duration::seconds(1);
        let err = LibraryBrowserAuthVerifier
            .exchange_and_verify(&callback, &challenge, &cfg, later)
            .unwrap_err();
        assert_eq!(err.code(), "identity-auth-failed");
    }

    #[test]
    fn library_verifier_refuses_empty_code_without_network() {
        let cfg = IdentityConfig::from_manifest("forge-admin", &sample_meta()).unwrap();
        let challenge = build_challenge("forge-admin", &cfg, now()).unwrap();
        let callback = AuthCallback {
            project_id: "forge-admin".to_string(),
            state: challenge.state.clone(),
            code: "   ".to_string(),
            error: None,
            error_description: None,
        };
        let err = LibraryBrowserAuthVerifier
            .exchange_and_verify(&callback, &challenge, &cfg, now())
            .unwrap_err();
        assert_eq!(err.code(), "identity-auth-failed");
    }

    #[test]
    fn admin_claim_grants_matches_any_array_element() {
        let cfg = IdentityConfig::from_manifest("forge-admin", &sample_meta()).unwrap();
        let challenge = build_challenge("forge-admin", &cfg, now()).unwrap();
        let mut claims = sample_claims(&challenge, &cfg);
        claims.claims.insert(
            cfg.admin_claim.clone(),
            "intern,forge-admins,operators".to_string(),
        );
        assert!(admin_claim_grants(&claims, &cfg));
    }
}
