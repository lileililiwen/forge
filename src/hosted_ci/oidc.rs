//! GitHub Actions OIDC token validation and claim binding.

use crate::ci_evidence::HostedCiEvidenceEnvelope;
use jsonwebtoken::jwk::{JwkSet, PublicKeyUse};
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::Read;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

pub const GITHUB_ACTIONS_ISSUER: &str = "https://token.actions.githubusercontent.com";
pub const GITHUB_ACTIONS_AUDIENCE: &str = "forge-ci-evidence";
const MAX_TOKEN_BYTES: usize = 16 * 1024;
const MAX_JWKS_BYTES: usize = 256 * 1024;
const JWKS_URL: &str = "https://token.actions.githubusercontent.com/.well-known/jwks";
const JWKS_TTL: Duration = Duration::from_secs(300);
const JWKS_REFRESH_INTERVAL: Duration = Duration::from_secs(30);
static JWKS_CACHE: OnceLock<Mutex<Option<(Instant, Instant, JwkSet)>>> = OnceLock::new();

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GitHubActionsClaims {
    pub repository: String,
    pub repository_id: String,
    pub run_id: String,
    pub run_attempt: String,
    pub workflow_ref: String,
    pub sha: String,
    pub event_name: String,
    #[serde(rename = "ref")]
    pub git_ref: String,
}

/// Verify an Actions ID token against an already-selected GitHub RSA key.
/// JWKS acquisition and cache policy belong to the transport boundary.
pub fn verify_github_actions_token(
    token: &str,
    key: &DecodingKey,
) -> Result<GitHubActionsClaims, String> {
    if token.is_empty() || token.len() > MAX_TOKEN_BYTES {
        return Err("GitHub Actions token is empty or exceeds the size limit".into());
    }
    let mut validation = Validation::new(Algorithm::RS256);
    validation.algorithms = vec![Algorithm::RS256];
    validation.set_issuer(&[GITHUB_ACTIONS_ISSUER]);
    validation.set_audience(&[GITHUB_ACTIONS_AUDIENCE]);
    validation.set_required_spec_claims(&["exp", "iss", "aud"]);
    validation.validate_exp = true;
    validation.validate_nbf = true;
    validation.leeway = 0;
    let decoded = decode::<Value>(token, key, &validation).map_err(|_| {
        "GitHub Actions token failed signature or standard claim validation".to_string()
    })?;
    let claims = decoded.claims;
    let audience_is_exact = match claims.get("aud") {
        Some(Value::String(value)) => value == GITHUB_ACTIONS_AUDIENCE,
        Some(Value::Array(values)) => {
            values.len() == 1 && values[0].as_str() == Some(GITHUB_ACTIONS_AUDIENCE)
        }
        _ => false,
    };
    if !audience_is_exact {
        return Err(
            "GitHub Actions token audience must be exactly the Forge evidence audience".into(),
        );
    }
    for required in [
        "iat",
        "repository",
        "repository_id",
        "run_id",
        "run_attempt",
        "workflow_ref",
        "sha",
        "event_name",
        "ref",
    ] {
        if claims.get(required).is_none() {
            return Err(format!(
                "GitHub Actions token is missing required claim `{required}`"
            ));
        }
    }
    let string = |name: &str| -> Result<String, String> {
        match claims.get(name) {
            Some(Value::String(value)) => Ok(value.clone()),
            Some(Value::Number(value)) => Ok(value.to_string()),
            _ => Err(format!(
                "GitHub Actions token claim `{name}` has an invalid type"
            )),
        }
    };
    let iat = claims["iat"]
        .as_i64()
        .ok_or_else(|| "GitHub Actions token claim `iat` has an invalid type".to_string())?;
    let now = chrono::Utc::now().timestamp();
    if iat > now + 60 {
        return Err("GitHub Actions token was issued in the future".into());
    }
    let result = GitHubActionsClaims {
        repository: string("repository")?,
        repository_id: string("repository_id")?,
        run_id: string("run_id")?,
        run_attempt: string("run_attempt")?,
        workflow_ref: string("workflow_ref")?,
        sha: string("sha")?,
        event_name: string("event_name")?,
        git_ref: string("ref")?,
    };
    if result.repository.is_empty()
        || result.repository_id.is_empty()
        || !result
            .repository_id
            .bytes()
            .all(|byte| byte.is_ascii_digit())
        || result.repository_id.starts_with('0')
        || result.run_id.is_empty()
        || !result.run_id.bytes().all(|byte| byte.is_ascii_digit())
        || result.run_attempt.is_empty()
        || !result.run_attempt.bytes().all(|byte| byte.is_ascii_digit())
        || result.workflow_ref.is_empty()
        || result.event_name != "push"
        || !result.git_ref.starts_with("refs/heads/")
        || result.sha.len() != 40
        || !result.sha.bytes().all(|byte| byte.is_ascii_hexdigit())
        || result.event_name.is_empty()
        || result.git_ref.is_empty()
    {
        return Err("GitHub Actions token contains malformed identity claims".into());
    }
    Ok(result)
}

/// Select an RS256 public key by the token's `kid` from GitHub's fixed JWKS
/// endpoint. The cache is bounded by age and size; an unknown `kid` forces one
/// refresh so key rotation can take effect without accepting an unknown key.
pub fn verify_github_actions_token_from_jwks(token: &str) -> Result<GitHubActionsClaims, String> {
    if token.is_empty() || token.len() > MAX_TOKEN_BYTES {
        return Err("GitHub Actions token is empty or exceeds the size limit".into());
    }
    let header = decode_header(token).map_err(|_| "GitHub Actions token header is malformed")?;
    if header.alg != Algorithm::RS256 {
        return Err("GitHub Actions token must use RS256".into());
    }
    let kid = header
        .kid
        .filter(|value| !value.is_empty() && value.len() <= 256)
        .ok_or("GitHub Actions token has no valid key id")?;
    let cache = JWKS_CACHE.get_or_init(|| Mutex::new(None));
    let cached = cache
        .lock()
        .map_err(|_| "GitHub key cache is unavailable")?;
    let fresh = cached
        .as_ref()
        .is_some_and(|(loaded, _, set)| loaded.elapsed() < JWKS_TTL && set.find(&kid).is_some());
    let refresh_throttled = cached.as_ref().is_some_and(|(loaded, refreshed, _)| {
        loaded.elapsed() < JWKS_TTL && refreshed.elapsed() < JWKS_REFRESH_INTERVAL
    });
    drop(cached);
    if !fresh {
        if refresh_throttled {
            return Err("GitHub signing key id is unknown".into());
        }
        let set = fetch_github_jwks()?;
        let mut cached = cache
            .lock()
            .map_err(|_| "GitHub key cache is unavailable")?;
        let now = Instant::now();
        *cached = Some((now, now, set));
    }
    let cached = cache
        .lock()
        .map_err(|_| "GitHub key cache is unavailable")?;
    let jwks = &cached
        .as_ref()
        .ok_or("GitHub signing keys are unavailable")?
        .2;
    let jwk = jwks.find(&kid).ok_or("GitHub signing key id is unknown")?;
    if jwk.common.public_key_use != Some(PublicKeyUse::Signature)
        || jwk
            .common
            .key_algorithm
            .as_ref()
            .is_none_or(|alg| alg.to_string() != "RS256")
    {
        return Err("GitHub signing key is not an RS256 signature key".into());
    }
    let key = DecodingKey::from_jwk(jwk).map_err(|_| "GitHub signing key is invalid")?;
    verify_github_actions_token(token, &key)
}

fn fetch_github_jwks() -> Result<JwkSet, String> {
    let client = reqwest::blocking::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(3))
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|_| "GitHub signing key client is unavailable")?;
    let response = client
        .get(JWKS_URL)
        .send()
        .map_err(|_| "GitHub signing keys are temporarily unavailable")?;
    if !response.status().is_success() {
        return Err("GitHub signing keys are temporarily unavailable".into());
    }
    if response
        .content_length()
        .is_some_and(|length| length > MAX_JWKS_BYTES as u64)
    {
        return Err("GitHub signing key response exceeds the size limit".into());
    }
    let mut bytes = Vec::with_capacity(MAX_JWKS_BYTES.min(16 * 1024));
    response
        .take(MAX_JWKS_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "GitHub signing keys are temporarily unavailable")?;
    if bytes.len() > MAX_JWKS_BYTES {
        return Err("GitHub signing key response exceeds the size limit".into());
    }
    serde_json::from_slice(&bytes).map_err(|_| "GitHub signing key response is invalid".to_string())
}

/// Cross-check every identity field the GitHub Actions token actually signs.
/// The envelope's job label is retained for display but is not an identity
/// check because GitHub does not include it as a standard OIDC claim.
pub fn validate_claims_for_envelope(
    claims: &GitHubActionsClaims,
    repository_id: &str,
    envelope: &HostedCiEvidenceEnvelope,
) -> Result<(), String> {
    if claims.repository != envelope.repository {
        return Err("signed repository does not match hosted evidence".into());
    }
    if claims.repository_id != repository_id {
        return Err("signed repository ID does not match the Forge project binding".into());
    }
    if claims.run_id != envelope.run.run_id
        || claims.run_attempt != envelope.run.run_attempt.to_string()
    {
        return Err("signed run identity does not match hosted evidence".into());
    }
    if claims.workflow_ref != envelope.run.workflow {
        return Err("signed workflow reference does not match hosted evidence".into());
    }
    let trusted_workflow_prefix = format!(
        "{}/.github/workflows/forge-gate.yml@refs/heads/",
        envelope.repository
    );
    if !claims.workflow_ref.starts_with(&trusted_workflow_prefix) {
        return Err("signed workflow is not the Forge generated Gate workflow".into());
    }
    if claims.event_name != "push" || !claims.git_ref.starts_with("refs/heads/") {
        return Err("hosted evidence must come from a branch push".into());
    }
    if claims.sha != envelope.source_revision {
        return Err("signed source revision does not match hosted evidence".into());
    }
    Ok(())
}
