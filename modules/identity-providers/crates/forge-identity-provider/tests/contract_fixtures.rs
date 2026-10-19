//! Contract fixture tests.
//!
//! The contract is the only thing Forge Core and a provider pack share, so it is
//! tested against the refusals it must produce, not only against the happy path.

use forge_identity_provider::{
    plan_digest, validate_request, validate_response, Population, ProposedChange,
    ProviderOperation, ProviderRequest, ProviderResponse, ProviderStatus, CONTRACT_ID,
};
use serde_json::json;

fn request(operation: ProviderOperation) -> ProviderRequest {
    ProviderRequest::new(
        "keycloak-local",
        "op-1",
        "demo-app",
        "sha256:project",
        operation,
        json!({ "state_dir": "/tmp/state", "port": 18080 }),
    )
}

fn ready_response(operation: ProviderOperation) -> ProviderResponse {
    let changes = vec![ProposedChange {
        kind: "provider.realm".to_string(),
        target: "realm:forge-demo-app".to_string(),
        summary: "Create the isolated Keycloak realm.".to_string(),
    }];
    ProviderResponse {
        contract: CONTRACT_ID.to_string(),
        provider_id: "keycloak-local".to_string(),
        operation_id: "op-1".to_string(),
        project_id: "demo-app".to_string(),
        project_revision: "sha256:project".to_string(),
        operation,
        status: ProviderStatus::Ready,
        summary: "The plan is ready.".to_string(),
        plan_digest: Some(
            plan_digest("demo-app", "keycloak-local", operation.as_str(), &changes).unwrap(),
        ),
        proposed_changes: changes,
        population: None,
        secret_references: vec!["file:///tmp/state/admin-password".to_string()],
        recovery: Vec::new(),
        details: json!({}),
    }
}

#[test]
fn a_matching_pair_validates() {
    let request = request(ProviderOperation::Plan);
    let response = ready_response(ProviderOperation::Plan);

    assert!(validate_request(&request).is_ok());
    assert!(validate_response(&request, &response).is_ok());
}

#[test]
fn an_unknown_contract_major_is_refused() {
    let request = request(ProviderOperation::Plan);
    let mut response = ready_response(ProviderOperation::Plan);
    response.contract = "forge-identity-provider/2.0".to_string();

    let error = validate_response(&request, &response).unwrap_err();
    assert!(
        error.to_string().contains("unsupported contract"),
        "{error}"
    );
}

#[test]
fn a_response_for_another_project_is_refused() {
    let request = request(ProviderOperation::Plan);
    let mut response = ready_response(ProviderOperation::Plan);
    response.project_id = "other-app".to_string();

    let error = validate_response(&request, &response).unwrap_err();
    assert!(error.to_string().contains("identity mismatch"), "{error}");
}

#[test]
fn a_response_for_another_operation_is_refused() {
    let request = request(ProviderOperation::Plan);
    let mut response = ready_response(ProviderOperation::Plan);
    response.operation = ProviderOperation::Apply;

    let error = validate_response(&request, &response).unwrap_err();
    assert!(error.to_string().contains("identity mismatch"), "{error}");
}

#[test]
fn a_response_for_a_stale_project_revision_is_refused() {
    let request = request(ProviderOperation::Plan);
    let mut response = ready_response(ProviderOperation::Plan);
    response.project_revision = "sha256:other".to_string();

    let error = validate_response(&request, &response).unwrap_err();
    assert!(error.to_string().contains("identity mismatch"), "{error}");
}

#[test]
fn a_response_that_carries_secret_material_is_refused() {
    let request = request(ProviderOperation::Plan);
    let mut response = ready_response(ProviderOperation::Plan);
    response.summary = "The client_secret is hunter2".to_string();

    let error = validate_response(&request, &response).unwrap_err();
    assert!(error.to_string().contains("secret material"), "{error}");
}

#[test]
fn a_secret_reference_that_is_not_a_reference_is_refused() {
    let request = request(ProviderOperation::Plan);
    let mut response = ready_response(ProviderOperation::Plan);
    response.secret_references = vec!["hunter2".to_string()];

    let error = validate_response(&request, &response).unwrap_err();
    assert!(error.to_string().contains("secret material"), "{error}");
}

#[test]
fn a_registration_url_on_another_host_is_refused() {
    let request = request(ProviderOperation::Plan);
    let mut response = ready_response(ProviderOperation::Plan);
    response.population = Some(Population {
        population_id: "keycloak-local/forge-demo-app".to_string(),
        issuer: "https://127.0.0.1:18080/realms/forge-demo-app".to_string(),
        client_id: "forge-demo-app".to_string(),
        callback_path: "/signin-oidc".to_string(),
        scopes: vec!["openid".to_string()],
        registration_url: Some("https://attacker.example/register".to_string()),
    });

    let error = validate_response(&request, &response).unwrap_err();
    assert!(error.to_string().contains("unsafe url"), "{error}");
}

#[test]
fn a_plain_http_issuer_that_is_not_loopback_is_refused() {
    let request = request(ProviderOperation::Plan);
    let mut response = ready_response(ProviderOperation::Plan);
    response.population = Some(Population {
        population_id: "keycloak-local/forge-demo-app".to_string(),
        issuer: "http://identity.example/realms/forge-demo-app".to_string(),
        client_id: "forge-demo-app".to_string(),
        callback_path: "/signin-oidc".to_string(),
        scopes: vec!["openid".to_string()],
        registration_url: None,
    });

    let error = validate_response(&request, &response).unwrap_err();
    assert!(error.to_string().contains("unsafe url"), "{error}");
}

#[test]
fn a_plan_without_a_digest_is_refused() {
    let request = request(ProviderOperation::Plan);
    let mut response = ready_response(ProviderOperation::Plan);
    response.plan_digest = None;

    let error = validate_response(&request, &response).unwrap_err();
    assert!(error.to_string().contains("plan digest"), "{error}");
}

#[test]
fn the_digest_changes_when_the_plan_changes() {
    let changes = vec![ProposedChange {
        kind: "provider.realm".to_string(),
        target: "realm:forge-demo-app".to_string(),
        summary: "Create the isolated Keycloak realm.".to_string(),
    }];
    let first = plan_digest("demo-app", "keycloak-local", "plan", &changes).unwrap();
    let mut other = changes.clone();
    other[0].summary = "Create a different realm.".to_string();
    let second = plan_digest("demo-app", "keycloak-local", "plan", &other).unwrap();

    assert_ne!(first, second);
}

#[test]
fn an_empty_plan_has_no_digest() {
    assert!(plan_digest("demo-app", "keycloak-local", "plan", &[]).is_err());
}

#[test]
fn a_request_without_a_project_is_refused() {
    let mut request = request(ProviderOperation::Probe);
    request.project_id = String::new();

    let error = validate_request(&request).unwrap_err();
    assert!(error.to_string().contains("project_id"), "{error}");
}
