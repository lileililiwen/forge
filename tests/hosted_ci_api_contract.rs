use chrono::Utc;
use forge::api::{handle, route_request, ApiConfig, ApiRequest};
use forge::registry::Registry;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

fn request(method: &str, path: &str, origin: &str) -> ApiRequest {
    ApiRequest {
        method: method.into(),
        path: path.into(),
        query: None,
        headers: BTreeMap::from([("origin".into(), origin.into())]),
        body: Vec::new(),
        idempotency_key: None,
        bearer_token: None,
        cookies: BTreeMap::new(),
        remote_addr: None,
        started_at: Utc::now(),
    }
}

fn json(mut request: ApiRequest, body: &[u8]) -> ApiRequest {
    request
        .headers
        .insert("content-type".into(), "application/json".into());
    request.body = body.to_vec();
    request
}

fn project(dir: &Path, db: &Path) {
    fs::create_dir_all(dir).unwrap();
    fs::write(dir.join("forge.yaml"), "schema: 1\nproject:\n  id: sample-service\n  name: Sample Service\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\n").unwrap();
    let run = |args: &[&str]| {
        let output = Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .output()
            .unwrap();
        assert!(output.status.success(), "git {args:?}");
    };
    run(&["init", "-q"]);
    run(&[
        "remote",
        "add",
        "origin",
        "https://github.com/owner/repository.git",
    ]);
    Registry::open(db).unwrap().register(dir, None).unwrap();
}

fn admin_cookie(config: &ApiConfig, db: &Path) -> String {
    forge::identity::global::setup(db, "operator@example.test", "a-long-test-password").unwrap();
    let login = json(
        request("POST", "/v1/admin/session", &config.frontend_origin),
        br#"{"email":"operator@example.test","password":"a-long-test-password"}"#,
    );
    let response = handle(config, db, &login, Utc::now());
    assert_eq!(response.status, 200);
    let cookie = response.headers["set-cookie"]
        .split(';')
        .next()
        .unwrap()
        .split_once('=')
        .unwrap()
        .1
        .to_string();
    cookie
}

fn with_cookie(mut request: ApiRequest, cookie: &str) -> ApiRequest {
    request
        .cookies
        .insert("forge_admin_session".into(), cookie.into());
    request
}

#[test]
fn route_table_separates_public_workload_from_admin_history_and_binding() {
    assert_eq!(
        route_request("POST", "/v1/ci/github/evidence"),
        Some(forge::api::Route::HostedCiIngest)
    );
    assert_eq!(
        route_request("GET", "/v1/admin/projects/sample-service/ci/hosted"),
        Some(forge::api::Route::AdminHostedCiHistory {
            id: "sample-service".into()
        })
    );
    assert_eq!(
        route_request("PUT", "/v1/admin/projects/sample-service/ci/repository"),
        Some(forge::api::Route::AdminHostedCiBind {
            id: "sample-service".into()
        })
    );
}

#[test]
fn workload_route_requires_oidc_and_operator_routes_require_admin_session() {
    let temp = tempfile::tempdir().unwrap();
    let db = temp.path().join("forge.db");
    let config = ApiConfig::default();
    let anonymous = request("POST", "/v1/ci/github/evidence", &config.frontend_origin);
    let response = handle(&config, &db, &anonymous, Utc::now());
    assert_eq!(response.status, 401);
    let response = handle(
        &config,
        &db,
        &request(
            "GET",
            "/v1/admin/projects/sample-service/ci/hosted",
            &config.frontend_origin,
        ),
        Utc::now(),
    );
    assert_eq!(response.status, 401);
}

#[test]
fn workload_route_rejects_oversized_envelopes_before_authentication() {
    let temp = tempfile::tempdir().unwrap();
    let db = temp.path().join("forge.db");
    let config = ApiConfig::default();
    let mut oversized = request("POST", "/v1/ci/github/evidence", &config.frontend_origin);
    oversized.body = vec![b'x'; 256 * 1024 + 1];
    let response = handle(&config, &db, &oversized, Utc::now());
    assert_eq!(response.status, 413);
    assert!(!db.exists(), "oversized request initialized the registry");
}

#[test]
fn operator_binds_and_reads_hosted_ci_without_changing_local_gate_status() {
    let temp = tempfile::tempdir().unwrap();
    let db = temp.path().join("forge.db");
    project(&temp.path().join("project"), &db);
    let config = ApiConfig::default();
    let cookie = admin_cookie(&config, &db);
    let malformed_bind = with_cookie(
        json(
            request(
                "PUT",
                "/v1/admin/projects/sample-service/ci/repository",
                &config.frontend_origin,
            ),
            br#"{"repository_id":"12345","unexpected":"ignored"}"#,
        ),
        &cookie,
    );
    let response = handle(&config, &db, &malformed_bind, Utc::now());
    assert_eq!(response.status, 400);
    let bind = with_cookie(
        json(
            request(
                "PUT",
                "/v1/admin/projects/sample-service/ci/repository",
                &config.frontend_origin,
            ),
            br#"{"repository_id":"12345"}"#,
        ),
        &cookie,
    );
    let response = handle(&config, &db, &bind, Utc::now());
    assert_eq!(
        response.status,
        200,
        "{}",
        String::from_utf8_lossy(&response.body)
    );
    let read = with_cookie(
        request(
            "GET",
            "/v1/admin/projects/sample-service/ci/hosted",
            &config.frontend_origin,
        ),
        &cookie,
    );
    let response = handle(&config, &db, &read, Utc::now());
    assert_eq!(
        response.status,
        200,
        "{}",
        String::from_utf8_lossy(&response.body)
    );
    let body: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
    assert_eq!(body["binding"]["repository_id"], "12345");
    assert_eq!(body["observations"].as_array().unwrap().len(), 0);
}
