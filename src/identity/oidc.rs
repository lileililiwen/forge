//! Production OpenID Connect browser-auth verifier.
//!
//! The browser sign-in round trip is verified against a real
//! provider using the pinned `openidconnect` crate with its
//! blocking `reqwest` client and rustls TLS transport. No
//! JWT/JWKS validation is hand-built: discovery, PKCE code
//! exchange, and ID-token signature/issuer/audience/nonce/time
//! verification all go through the library.
//!
//! This module owns only the provider boundary. The
//! project-scoped session model, challenge persistence, and
//! admin-claim allow-list stay in [`super`].
//!
//! The dependency is pinned to `openidconnect` 4.x: the 3.x
//! line pulls `reqwest` 0.11 / `rustls` 0.21, whose
//! `rustls-webpki` and `h2` revisions carry open advisories.
//! The 4.x line uses `rustls` 0.23 and a modern `hyper`; the
//! only remaining advisory is the RSA timing sidechannel
//! (`RUSTSEC-2023-0071`), which affects private-key
//! operations and is not reachable from public-key signature
//! verification. The `deny.toml` exception records that.

use std::collections::BTreeMap;
use std::time::Duration;

use chrono::{DateTime, Utc};
use openidconnect::core::{
    CoreAuthDisplay, CoreAuthPrompt, CoreErrorResponseType, CoreGenderClaim, CoreJsonWebKey,
    CoreJweContentEncryptionAlgorithm, CoreJwsSigningAlgorithm, CoreProviderMetadata,
    CoreRevocableToken, CoreRevocationErrorResponse, CoreTokenIntrospectionResponse, CoreTokenType,
};
use openidconnect::{
    AdditionalClaims, AuthorizationCode, Client, ClientId, ClientSecret, EmptyExtraTokenFields,
    EndpointNotSet, IdTokenFields, IssuerUrl, Nonce, OAuth2TokenResponse, PkceCodeVerifier,
    RedirectUrl, StandardErrorResponse, StandardTokenResponse, TokenResponse,
};
use serde::{Deserialize, Serialize};

use super::{
    redact_credentials, AuthCallback, AuthChallenge, BrowserAuthVerifier, ForgeError,
    IdentityConfig, ProviderClaims,
};

/// Provider network bound. Discovery, JWKS retrieval, and
/// the PKCE code exchange share one bounded connection so a
/// stalled provider cannot pin the single-threaded listener.
const PROVIDER_TIMEOUT_SECONDS: u64 = 10;

/// Additional ID-token claims Forge needs beyond the OIDC
/// standard set. The admin claim name is configured per
/// project, so the extras are captured as a free-form map
/// inside the library's own claim deserialization (the
/// signature has already been verified by the time this is
/// read).
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct ForgeAdditionalClaims {
    #[serde(flatten)]
    extra: BTreeMap<String, serde_json::Value>,
}

impl AdditionalClaims for ForgeAdditionalClaims {}

type ForgeIdTokenFields = IdTokenFields<
    ForgeAdditionalClaims,
    EmptyExtraTokenFields,
    CoreGenderClaim,
    CoreJweContentEncryptionAlgorithm,
    CoreJwsSigningAlgorithm,
>;

type ForgeTokenResponse = StandardTokenResponse<ForgeIdTokenFields, CoreTokenType>;

/// Core client carrying Forge's free-form additional claims.
/// The endpoint-state parameters default exactly like the
/// library's own `CoreClient`, so `from_provider_metadata`
/// can infer the concrete states it produces.
type ForgeClient<
    HasAuthUrl = EndpointNotSet,
    HasDeviceAuthUrl = EndpointNotSet,
    HasIntrospectionUrl = EndpointNotSet,
    HasRevocationUrl = EndpointNotSet,
    HasTokenUrl = EndpointNotSet,
    HasUserInfoUrl = EndpointNotSet,
> = Client<
    ForgeAdditionalClaims,
    CoreAuthDisplay,
    CoreGenderClaim,
    CoreJweContentEncryptionAlgorithm,
    CoreJsonWebKey,
    CoreAuthPrompt,
    StandardErrorResponse<CoreErrorResponseType>,
    ForgeTokenResponse,
    CoreTokenIntrospectionResponse,
    CoreRevocableToken,
    CoreRevocationErrorResponse,
    HasAuthUrl,
    HasDeviceAuthUrl,
    HasIntrospectionUrl,
    HasRevocationUrl,
    HasTokenUrl,
    HasUserInfoUrl,
>;

/// Production verifier. The flow is:
///
/// 1. Refuse a provider-reported callback error.
/// 2. Fetch discovery metadata for the configured issuer and
///    require its `issuer` to equal the configured issuer.
/// 3. Exchange the code with the stored PKCE verifier over a
///    bounded blocking HTTPS client.
/// 4. Verify the signed ID token (issuer, audience, expiry,
///    nonce) through the library and map the claims into the
///    typed [`ProviderClaims`] surface.
pub struct LibraryBrowserAuthVerifier;

impl BrowserAuthVerifier for LibraryBrowserAuthVerifier {
    fn exchange_and_verify(
        &self,
        callback: &AuthCallback,
        challenge: &AuthChallenge,
        config: &IdentityConfig,
        now: DateTime<Utc>,
    ) -> Result<ProviderClaims, ForgeError> {
        if let Some(error) = callback.error.as_deref() {
            return Err(auth_failed(format!(
                "provider returned `{}`",
                redact_credentials(error)
            )));
        }
        if callback.code.trim().is_empty() {
            return Err(auth_failed(
                "auth callback did not carry an authorization code",
            ));
        }
        if challenge.is_expired(now) {
            return Err(auth_failed(
                "the OIDC challenge has expired; restart sign-in",
            ));
        }
        let issuer = IssuerUrl::new(config.issuer.clone())
            .map_err(|err| auth_failed(format!("configured issuer is not a URL: {err}")))?;
        let http_client = reqwest::blocking::Client::builder()
            .connect_timeout(Duration::from_secs(PROVIDER_TIMEOUT_SECONDS))
            .timeout(Duration::from_secs(PROVIDER_TIMEOUT_SECONDS))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|err| {
                auth_failed(format!(
                    "cannot build the bounded provider client: {}",
                    redact_credentials(&err.to_string())
                ))
            })?;
        let metadata = CoreProviderMetadata::discover(&issuer, &http_client).map_err(|err| {
            auth_failed(format!(
                "OIDC discovery failed for `{}`: {}",
                redact_credentials(&config.issuer),
                redact_credentials(&err.to_string())
            ))
        })?;
        if metadata.issuer() != &issuer {
            return Err(auth_failed(
                "discovery metadata issuer does not match the configured issuer",
            ));
        }
        let client_secret = resolve_client_secret(config.client_secret_ref.as_deref())?;
        let redirect = RedirectUrl::new(challenge.redirect_uri.clone())
            .map_err(|err| auth_failed(format!("configured redirect URI is invalid: {err}")))?;
        let client = ForgeClient::from_provider_metadata(
            metadata,
            ClientId::new(config.client_id.clone()),
            client_secret,
        )
        .set_redirect_uri(redirect);
        let token_response = client
            .exchange_code(AuthorizationCode::new(callback.code.clone()))
            .map_err(|err| {
                auth_failed(format!(
                    "OIDC client is not configured for code exchange: {err}"
                ))
            })?
            .set_pkce_verifier(PkceCodeVerifier::new(challenge.code_verifier.clone()))
            .request(&http_client)
            .map_err(|err| {
                auth_failed(format!(
                    "PKCE code exchange failed: {}",
                    redact_credentials(&err.to_string())
                ))
            })?;
        let id_token = token_response.id_token().ok_or_else(|| {
            auth_failed("token response did not include an ID token; refusing sign-in")
        })?;
        let nonce = Nonce::new(challenge.nonce.clone());
        let verified = id_token
            .claims(&client.id_token_verifier(), &nonce)
            .map_err(|err| {
                auth_failed(format!(
                    "ID-token verification failed: {}",
                    redact_credentials(&err.to_string())
                ))
            })?;
        let audience = verified
            .audiences()
            .iter()
            .map(|value| value.as_str().to_string())
            .find(|value| value == &config.audience)
            .or_else(|| {
                verified
                    .audiences()
                    .first()
                    .map(|value| value.as_str().to_string())
            })
            .unwrap_or_default();
        let scopes = token_response
            .scopes()
            .map(|values| {
                values
                    .iter()
                    .map(|scope| scope.as_str().to_string())
                    .collect::<Vec<_>>()
            })
            .filter(|values| !values.is_empty())
            .unwrap_or_else(|| {
                challenge
                    .scope
                    .split_whitespace()
                    .map(str::to_string)
                    .collect()
            });
        let nonce_claim = verified
            .nonce()
            .map(|value| value.secret().clone())
            .unwrap_or_default();
        let mut claims = BTreeMap::new();
        for (name, value) in &verified.additional_claims().extra {
            if let Some(encoded) = claim_value_to_string(value) {
                claims.insert(name.clone(), encoded);
            }
        }
        Ok(ProviderClaims {
            issuer: verified.issuer().as_str().to_string(),
            audience,
            subject: verified.subject().as_str().to_string(),
            issued_at: verified.issue_time(),
            expires_at: verified.expiration(),
            nonce: nonce_claim,
            scopes,
            claims,
        })
    }
}

/// Resolve a manifest `client_secret_ref` into a concrete
/// secret. Only the `env://` scheme is supported: the secret
/// is read from the named environment variable at request
/// time and never persisted. An unset variable yields a
/// public client (PKCE-only), which is the correct posture
/// for a browser-side client; any other scheme fails closed
/// rather than guessing.
fn resolve_client_secret(reference: Option<&str>) -> Result<Option<ClientSecret>, ForgeError> {
    let Some(reference) = reference else {
        return Ok(None);
    };
    let name = reference.strip_prefix("env://").ok_or_else(|| {
        auth_failed(format!(
            "unsupported client_secret_ref scheme `{}`; only `env://` is resolved",
            redact_credentials(reference)
        ))
    })?;
    match std::env::var(name) {
        Ok(value) if !value.trim().is_empty() => Ok(Some(ClientSecret::new(value))),
        _ => Ok(None),
    }
}

/// Flatten one additional claim value into the string map the
/// existing admin-claim allow-list compares against. Arrays
/// (the common shape of a `groups` claim) are joined with
/// commas and matched element-wise by
/// [`super::admin_claim_grants`].
fn claim_value_to_string(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(value) => Some(value.clone()),
        serde_json::Value::Array(values) => {
            let parts: Vec<String> = values
                .iter()
                .filter_map(|value| value.as_str().map(str::to_string))
                .collect();
            if parts.is_empty() {
                None
            } else {
                Some(parts.join(","))
            }
        }
        serde_json::Value::Null => None,
        other => Some(other.to_string()),
    }
}

/// One stable, redacted auth-failure constructor. Callers
/// receive the typed `identity-auth-failed` code and never a
/// provider secret, code, or token.
fn auth_failed(reason: impl Into<String>) -> ForgeError {
    ForgeError::IdentityAuthFailed {
        reason: reason.into(),
    }
}
