//! Portfolio share HTTP contract (`portfolio-share-publish`).
//!
//! Every share route — record read, record write, withdraw, preview,
//! approve, publish, reconcile and audit — answers through the existing
//! `admin:access` boundary, so an unauthorized request reaches no write
//! and discloses no private record. The approval and publication flow
//! records the *authenticated* session subject rather than a claimed
//! one, which is what makes the audit trail worth keeping.

#[path = "support/share.rs"]
mod support;

use serde_json::Value;
use support::{
    api, api_json, api_request, fleet, lossy, mint_session_token, registered, run_json, share_set,
};

#[test]
fn every_share_route_demands_a_session() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, _token) = fleet(tmp.path());
    for (method, path, body) in [
        ("GET", "/v1/projects/alethefy/share", ""),
        ("POST", "/v1/projects/alethefy/share", "{}"),
        ("POST", "/v1/projects/alethefy/share/remove", "{}"),
        ("GET", "/v1/share/manifest", ""),
        ("POST", "/v1/share/approve", "{}"),
        ("POST", "/v1/share/publish", "{}"),
        ("POST", "/v1/share/reconcile", "{}"),
        ("GET", "/v1/share/audit", ""),
    ] {
        let response = api(&db, &api_request(method, path, None, body));
        assert_eq!(response.status, 401, "{method} {path}");
        assert_eq!(api_json(&response)["error"]["code"], "api-unauthorized");
    }
    // Nothing was written by an unauthenticated request.
    assert!(run_json(&db, &["portfolio", "share", "list"])["records"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[test]
fn preview_requires_admin_and_does_not_disclose_private_records() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, token) = fleet(tmp.path());
    share_set(&db, "forge", "Forge");
    let response = api(
        &db,
        &api_request("GET", "/v1/share/manifest", Some(&token), ""),
    );
    assert_eq!(response.status, 200);
    let body = api_json(&response);
    assert_eq!(body["contract"], "forge-portfolio-share/0.1.0");
    assert_eq!(body["share"]["project_count"], 1);
    assert_eq!(body["share"]["approvable"], true);
    assert_eq!(body["share"]["findings"].as_array().unwrap().len(), 0);
}

#[test]
fn the_share_record_routes_answer_through_the_authorization_boundary() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, token) = fleet(tmp.path());
    // A project without a record reports `shared: false` rather than
    // inventing an empty entry.
    let response = api(
        &db,
        &api_request("GET", "/v1/projects/alethefy/share", Some(&token), ""),
    );
    assert_eq!(response.status, 200);
    assert_eq!(api_json(&response)["share"]["shared"], false);

    let created = api(
        &db,
        &api_request(
            "POST",
            "/v1/projects/alethefy/share",
            Some(&token),
            r#"{"title":"Alethefy","summary":"Deterministic evidence.","category":"platform",
                "source_url":"https://github.com/lileililiwen/alethefy","showcase_status":"beta",
                "featured":true,"surfaces":[{"label":"Docs","url":"https://example.com/alethefy/docs"}]}"#,
        ),
    );
    assert_eq!(created.status, 200, "{}", lossy(&created.body));
    let body = api_json(&created);
    assert_eq!(body["share"]["record"]["state"], "validated");
    assert_eq!(body["share"]["record"]["featured"], true);
    assert_eq!(
        body["share"]["record"]["surfaces"]
            .as_array()
            .unwrap()
            .len(),
        1
    );

    // The CLI sees the same record.
    let shown = run_json(&db, &["portfolio", "share", "show", "alethefy"]);
    assert_eq!(shown["share"]["title"], "Alethefy");
    assert_eq!(shown["share"]["showcase_status"], "beta");

    let removed = api(
        &db,
        &api_request(
            "POST",
            "/v1/projects/alethefy/share/remove",
            Some(&token),
            "{}",
        ),
    );
    assert_eq!(removed.status, 200);
    assert_eq!(api_json(&removed)["share"]["removed"], true);
}

#[test]
fn a_cross_project_session_cannot_write_another_projects_share_record() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, dir) = registered(tmp.path(), "alethefy");
    registered(tmp.path(), "forge");
    let token = mint_session_token(&db, &dir, "ops-admin");
    let response = api(
        &db,
        &api_request(
            "POST",
            "/v1/projects/forge/share",
            Some(&token),
            r#"{"title":"Forge","summary":"s","category":"platform","source_url":"https://example.com/forge"}"#,
        ),
    );
    assert_eq!(response.status, 403);
    assert_eq!(api_json(&response)["error"]["code"], "api-project-mismatch");
    assert!(run_json(&db, &["portfolio", "share", "list"])["records"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[test]
fn the_api_approval_and_publication_flow_records_the_session_subject() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, dir) = registered(tmp.path(), "alethefy");
    registered(tmp.path(), "forge");
    let token = mint_session_token(&db, &dir, "release-bot");
    let target = tmp.path().join("catalog.json");
    let created = api(
        &db,
        &api_request(
            "POST",
            "/v1/projects/alethefy/share",
            Some(&token),
            r#"{"title":"Alethefy","summary":"Deterministic evidence.","category":"platform",
                "source_url":"https://github.com/lileililiwen/alethefy"}"#,
        ),
    );
    assert_eq!(created.status, 200, "{}", lossy(&created.body));

    let preview = api(
        &db,
        &api_request("GET", "/v1/share/manifest", Some(&token), ""),
    );
    let hash = api_json(&preview)["share"]["manifest_sha256"]
        .as_str()
        .unwrap()
        .to_string();
    let approved = api(
        &db,
        &api_request(
            "POST",
            "/v1/share/approve",
            Some(&token),
            &format!("{{\"manifest_sha256\":\"{hash}\"}}"),
        ),
    );
    assert_eq!(approved.status, 200, "{}", lossy(&approved.body));
    // The audit records the authenticated subject, not a claimed one.
    assert_eq!(
        api_json(&approved)["share"]["approval"]["actor"],
        "release-bot"
    );

    let published = api(
        &db,
        &api_request(
            "POST",
            "/v1/share/publish",
            Some(&token),
            &format!(
                "{{\"target\":{},\"operation_key\":\"op-http\"}}",
                serde_json::json!(target.to_str().unwrap())
            ),
        ),
    );
    assert_eq!(published.status, 200, "{}", lossy(&published.body));
    assert_eq!(
        api_json(&published)["share"]["publication"]["status"],
        "published"
    );
    assert!(target.exists());

    let audit = api(
        &db,
        &api_request("GET", "/v1/share/audit", Some(&token), ""),
    );
    assert_eq!(audit.status, 200);
    let body = api_json(&audit);
    assert_eq!(body["share"]["approvals"][0]["actor"], "release-bot");
    assert_eq!(body["share"]["publications"][0]["actor"], "release-bot");
    assert_eq!(body["share"]["unreconciled"], Value::Null);
}

#[test]
fn a_stale_approval_hash_is_a_typed_conflict_over_http() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, token) = fleet(tmp.path());
    let created = api(
        &db,
        &api_request(
            "POST",
            "/v1/projects/alethefy/share",
            Some(&token),
            r#"{"title":"Alethefy","summary":"s","category":"platform","source_url":"https://example.com/alethefy"}"#,
        ),
    );
    assert_eq!(created.status, 200);
    let response = api(
        &db,
        &api_request(
            "POST",
            "/v1/share/approve",
            Some(&token),
            &format!("{{\"manifest_sha256\":\"{}\"}}", "0".repeat(64)),
        ),
    );
    assert_eq!(response.status, 409);
    assert_eq!(
        api_json(&response)["error"]["code"],
        "portfolio-share-conflict"
    );
}

#[test]
fn an_invalid_share_write_is_a_typed_400_over_http() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, token) = fleet(tmp.path());
    let response = api(
        &db,
        &api_request(
            "POST",
            "/v1/projects/alethefy/share",
            Some(&token),
            r#"{"title":"Alethefy","summary":"s","category":"platform",
                "source_url":"https://example.com/alethefy/admin/settings"}"#,
        ),
    );
    assert_eq!(response.status, 400);
    assert_eq!(
        api_json(&response)["error"]["code"],
        "portfolio-share-invalid"
    );
    assert_eq!(
        api(
            &db,
            &api_request("GET", "/v1/projects/alethefy/share", Some(&token), "")
        )
        .status,
        200
    );
    assert_eq!(
        api_json(&api(
            &db,
            &api_request("GET", "/v1/projects/alethefy/share", Some(&token), "")
        ))["share"]["shared"],
        false
    );
}

#[test]
fn the_share_audit_limit_is_bounded() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, token) = fleet(tmp.path());
    // The wire parser splits the query off the path, so the handler
    // reads `request.query`; an absent query is the house default.
    let mut request = api_request("GET", "/v1/share/audit", Some(&token), "");
    request.query = Some("limit=0".to_string());
    let response = api(&db, &request);
    assert_eq!(response.status, 400);
    assert_eq!(api_json(&response)["error"]["code"], "api-invalid");
    request.query = Some("limit=abc".to_string());
    assert_eq!(api(&db, &request).status, 400);
    request.query = Some("limit=9000".to_string());
    assert_eq!(api(&db, &request).status, 400);
    request.query = Some("depth=2".to_string());
    assert_eq!(api(&db, &request).status, 400);
    request.query = None;
    assert_eq!(api(&db, &request).status, 200);
    request.query = Some("limit=10".to_string());
    assert_eq!(api(&db, &request).status, 200);
}
