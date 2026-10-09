//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::core::ForgeError;
use chrono::{DateTime, Utc};
use std::path::Path;

use super::model::{AdminSession, AuthCallback, AuthChallenge, IdentityConfig, ProviderClaims};
use super::protocol::{
    delete_challenge_file, mint_session, save_session, validate_callback, validate_claims,
};

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
pub(super) mod tests {
    use super::*;
    use crate::core::manifest::IdentityMeta;
    use crate::identity::constants::{
        ADMIN_PERMISSION, MAX_SCOPES, MAX_SESSION_TTL_SECONDS, MAX_STATE_TTL_SECONDS,
        SUPPORTED_CODE_CHALLENGE_METHODS, SUPPORTED_PROVIDERS,
    };
    use crate::identity::model::{
        FakeBrowserAuthVerifier, IdentityOutcome, IdentityRejection, SessionState,
    };
    use crate::identity::oidc::LibraryBrowserAuthVerifier;
    use crate::identity::protocol::{
        admin_claim_grants, build_challenge, challenge_path_for, code_challenge_s256,
        list_sessions, load_session, redact_identity_evidence, render_challenge_human,
        render_outcome_human, render_session_human, save_challenge, session_path_for,
        terminate_session, validate_session,
    };
    use chrono::Duration;
    use std::collections::BTreeMap;

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
