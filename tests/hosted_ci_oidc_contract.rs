use forge::ci_evidence::{HostedCiEvidenceEnvelope, HostedCiRun};
use forge::gate::{GateAggregate, GateCheckState, GateEvidence};
use forge::hosted_ci::{
    github_slug, validate_claims_for_envelope, verify_github_actions_token, GITHUB_ACTIONS_AUDIENCE,
};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde_json::json;

fn token(audience: &str, repository_id: &str, sha: &str) -> String {
    let claims = json!({
        "iss": "https://token.actions.githubusercontent.com",
        "aud": audience,
        "exp": chrono::Utc::now().timestamp() + 600,
        "iat": chrono::Utc::now().timestamp(),
        "repository": "owner/repository",
        "repository_id": repository_id,
        "run_id": "123456789",
        "run_attempt": "1",
        "workflow_ref": "owner/repository/.github/workflows/forge-gate.yml@refs/heads/main",
        "sha": sha,
        "event_name": "push",
        "ref": "refs/heads/main"
    });
    encode(
        &Header::new(Algorithm::RS256),
        &claims,
        &EncodingKey::from_rsa_pem(include_bytes!("fixtures/hosted-ci/test-private.pem")).unwrap(),
    )
    .unwrap()
}

fn token_with_audience(audience: serde_json::Value) -> String {
    let claims = json!({
        "iss": "https://token.actions.githubusercontent.com",
        "aud": audience,
        "exp": chrono::Utc::now().timestamp() + 600,
        "iat": chrono::Utc::now().timestamp(),
        "repository": "owner/repository",
        "repository_id": "12345",
        "run_id": "123456789",
        "run_attempt": "1",
        "workflow_ref": "owner/repository/.github/workflows/forge-gate.yml@refs/heads/main",
        "sha": "a".repeat(40),
        "event_name": "push",
        "ref": "refs/heads/main"
    });
    encode(
        &Header::new(Algorithm::RS256),
        &claims,
        &EncodingKey::from_rsa_pem(include_bytes!("fixtures/hosted-ci/test-private.pem")).unwrap(),
    )
    .unwrap()
}

fn envelope() -> HostedCiEvidenceEnvelope {
    HostedCiEvidenceEnvelope {
        contract: "forge-hosted-ci-evidence/1.0".into(),
        project_id: "sample-service".into(),
        repository: "owner/repository".into(),
        run: HostedCiRun {
            provider: "github-actions".into(),
            workflow: "owner/repository/.github/workflows/forge-gate.yml@refs/heads/main".into(),
            run_id: "123456789".into(),
            run_attempt: 1,
            job: "upload-hosted-evidence".into(),
            url: "https://github.com/owner/repository/actions/runs/123456789".into(),
        },
        source_revision: "a".repeat(40),
        gate: GateEvidence {
            contract: forge::gate::GATE_CONTRACT_VERSION.into(),
            project_id: "sample-service".into(),
            runtime: "forge".into(),
            profile: forge::gate::GateProfile::Full,
            runtime_version: Some("0.1.0".into()),
            revision: Some("a".repeat(40)),
            manifest_digest: None,
            rule_pack_version: None,
            aggregate: GateAggregate::Passed,
            checks: vec![forge::gate::GateCheck {
                id: "baseline".into(),
                state: GateCheckState::Pass,
                note: None,
            }],
            observed_at: "2026-10-18T10:00:00Z".into(),
            dry_run: false,
            note: None,
        },
        observed_at: "2026-10-18T10:01:00Z".into(),
    }
}

#[test]
fn accepts_only_fixed_issuer_audience_and_signed_github_claims() {
    let claims = verify_github_actions_token(
        &token(GITHUB_ACTIONS_AUDIENCE, "12345", &"a".repeat(40)),
        &jsonwebtoken::DecodingKey::from_rsa_pem(include_bytes!(
            "fixtures/hosted-ci/test-public.pem"
        ))
        .unwrap(),
    )
    .unwrap();
    assert_eq!(claims.repository, "owner/repository");
    assert_eq!(claims.repository_id, "12345");
    assert_eq!(claims.run_id, "123456789");
}

#[test]
fn rejects_wrong_audience_and_unsigned_algorithm() {
    let key = jsonwebtoken::DecodingKey::from_rsa_pem(include_bytes!(
        "fixtures/hosted-ci/test-public.pem"
    ))
    .unwrap();
    assert!(
        verify_github_actions_token(&token("other-audience", "12345", &"a".repeat(40)), &key)
            .is_err()
    );
    assert!(verify_github_actions_token(
        &token_with_audience(json!([GITHUB_ACTIONS_AUDIENCE, "another-audience"])),
        &key
    )
    .is_err());
    let unsigned = format!("{}.{}.", "eyJhbGciOiJub25lIn0", "e30");
    assert!(verify_github_actions_token(&unsigned, &key).is_err());
}

#[test]
fn rejects_expired_tokens_and_missing_identity_claims() {
    let mut claims = json!({
        "iss": "https://token.actions.githubusercontent.com",
        "aud": GITHUB_ACTIONS_AUDIENCE,
        "exp": 1,
        "iat": 1
    });
    let expired = encode(
        &Header::new(Algorithm::RS256),
        &claims,
        &EncodingKey::from_rsa_pem(include_bytes!("fixtures/hosted-ci/test-private.pem")).unwrap(),
    )
    .unwrap();
    let key = jsonwebtoken::DecodingKey::from_rsa_pem(include_bytes!(
        "fixtures/hosted-ci/test-public.pem"
    ))
    .unwrap();
    assert!(verify_github_actions_token(&expired, &key).is_err());
    claims["exp"] = json!(chrono::Utc::now().timestamp() + 600);
    let missing = encode(
        &Header::new(Algorithm::RS256),
        &claims,
        &EncodingKey::from_rsa_pem(include_bytes!("fixtures/hosted-ci/test-private.pem")).unwrap(),
    )
    .unwrap();
    assert!(verify_github_actions_token(&missing, &key).is_err());
}

#[test]
fn binds_signed_claims_to_envelope_and_registered_repository_id() {
    let key = jsonwebtoken::DecodingKey::from_rsa_pem(include_bytes!(
        "fixtures/hosted-ci/test-public.pem"
    ))
    .unwrap();
    let claims = verify_github_actions_token(
        &token(GITHUB_ACTIONS_AUDIENCE, "12345", &"a".repeat(40)),
        &key,
    )
    .unwrap();
    validate_claims_for_envelope(&claims, "12345", &envelope()).unwrap();
    assert!(validate_claims_for_envelope(&claims, "99999", &envelope()).is_err());
    let mut wrong_repository = claims.clone();
    wrong_repository.repository = "owner/other-repository".into();
    assert!(validate_claims_for_envelope(&wrong_repository, "12345", &envelope()).is_err());
    let mut wrong_run = claims.clone();
    wrong_run.run_id = "987654321".into();
    assert!(validate_claims_for_envelope(&wrong_run, "12345", &envelope()).is_err());
    let mut mismatch = envelope();
    mismatch.run.run_attempt = 2;
    assert!(validate_claims_for_envelope(&claims, "12345", &mismatch).is_err());
    mismatch = envelope();
    mismatch.source_revision = "b".repeat(40);
    assert!(validate_claims_for_envelope(&claims, "12345", &mismatch).is_err());
    mismatch = envelope();
    mismatch.run.workflow = "owner/repository/.github/workflows/other.yml@refs/heads/main".into();
    let mut other_workflow = claims.clone();
    other_workflow.workflow_ref = mismatch.run.workflow.clone();
    assert!(validate_claims_for_envelope(&other_workflow, "12345", &mismatch).is_err());
    let mut pull_request = claims.clone();
    pull_request.event_name = "pull_request".into();
    assert!(validate_claims_for_envelope(&pull_request, "12345", &envelope()).is_err());
}

#[test]
fn normalizes_only_canonical_github_remotes() {
    for remote in [
        "https://github.com/Owner/Repo.git",
        "git@github.com:Owner/Repo.git",
        "ssh://git@github.com/Owner/Repo.git",
    ] {
        assert_eq!(github_slug(remote).as_deref(), Some("owner/repo"));
    }
    for remote in [
        "https://attacker.example/owner/repo.git",
        "http://github.com/owner/repo.git",
        "https://github.com/owner/repo/extra.git",
    ] {
        assert_eq!(github_slug(remote), None);
    }
}
