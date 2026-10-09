//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::core::ForgeError;
use crate::policy::redact_credentials;
use chrono::{DateTime, Duration, Utc};
use rand::RngCore;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

use super::constants::{ADMIN_PERMISSION, IDENTITY_DIR, SESSION_ID_LEN, VALUE_LEN};
use super::model::{
    AdminSession, AuthCallback, AuthChallenge, IdentityConfig, IdentityOutcome, IdentityRejection,
    ProviderClaims, SessionState,
};

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

pub(super) fn require_nonempty(field: Option<&str>, name: &str) -> Result<String, ForgeError> {
    match field {
        Some(value) if !value.trim().is_empty() => Ok(value.trim().to_string()),
        _ => Err(IdentityRejection::invalid(format!(
            "identity `{name}` is required"
        ))),
    }
}

pub(super) fn validate_url(value: &str, field: &str) -> Result<(), ForgeError> {
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

pub(super) fn is_kebab_or_token(value: &str) -> bool {
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

pub(super) fn is_admin_claim(value: &str) -> bool {
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
