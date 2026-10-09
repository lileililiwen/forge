//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::core::manifest::{IdentityMeta, Manifest};
use crate::core::ForgeError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Mutex;

use super::constants::{
    MAX_ADMIN_VALUES, MAX_SCOPES, MAX_SESSION_TTL_SECONDS, MAX_STATE_TTL_SECONDS,
    MIN_SESSION_TTL_SECONDS, MIN_STATE_TTL_SECONDS, SUPPORTED_PROVIDERS,
};
use super::protocol::{is_admin_claim, is_kebab_or_token, require_nonempty, validate_url};

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
    pub(super) fn invalid(reason: impl Into<String>) -> ForgeError {
        ForgeError::IdentityInvalid {
            reason: reason.into(),
        }
    }
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
/// Test-only verifier that pops pre-canned outcomes off a
/// queue. Every `exchange_and_verify` call consumes one
/// queued outcome; a missing queued outcome produces a
/// `identity-auth-failed` refusal so a stale test fixture
/// cannot quietly mint a session.
pub struct FakeBrowserAuthVerifier {
    pub(super) queue: Mutex<Vec<Result<ProviderClaims, ForgeError>>>,
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
