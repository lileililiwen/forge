use std::collections::BTreeMap;
use std::path::Path;

use chrono::Utc;
use forge::api::{handle, ApiConfig, ApiRequest};
use serde_json::Value;
use tempfile::TempDir;

const EMAIL: &str = "module-operator@example.test";
const PASSWORD: &str = "a-long-test-password";

fn request(method: &str, path: &str, origin: &str) -> ApiRequest {
    ApiRequest {
        method: method.to_string(),
        path: path.to_string(),
        query: None,
        headers: BTreeMap::from([("origin".to_string(), origin.to_string())]),
        body: Vec::new(),
        idempotency_key: None,
        bearer_token: None,
        cookies: BTreeMap::new(),
        remote_addr: None,
        started_at: Utc::now(),
    }
}

fn setup_admin(db: &Path) {
    forge::identity::global::setup(db, EMAIL, PASSWORD).unwrap();
}

fn login_token(config: &ApiConfig, db: &Path) -> String {
    let mut login = request("POST", "/v1/admin/session", &config.frontend_origin);
    login
        .headers
        .insert("content-type".to_string(), "application/json".to_string());
    login.body = serde_json::json!({ "email": EMAIL, "password": PASSWORD })
        .to_string()
        .into_bytes();
    let response = handle(config, db, &login, Utc::now());
    assert_eq!(response.status, 200, "admin login succeeds");
    response
        .headers
        .get("set-cookie")
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .split_once('=')
        .unwrap()
        .1
        .to_string()
}

fn body_json(response: &forge::api::ApiResponse) -> Value {
    serde_json::from_slice(&response.body).expect("API response is JSON")
}

#[test]
fn modules_page_exposes_a_real_authenticated_module_workflow() {
    let html = include_str!("../frontend/index.html");
    let js = include_str!("../frontend/app.js");
    assert!(html.contains("data-route=\"modules\""));
    assert!(html.contains("id=\"modules\""));
    assert!(js.contains("/v1/admin/modules"));
    assert!(js.contains("/gate/run"));
}

#[test]
fn module_inventory_requires_an_admin_session_and_returns_only_relative_paths() {
    let temp = TempDir::new().unwrap();
    let db = temp.path().join("registry.db");
    setup_admin(&db);
    let config = ApiConfig::default();

    let anonymous = handle(
        &config,
        &db,
        &request("GET", "/v1/admin/modules", &config.frontend_origin),
        Utc::now(),
    );
    assert_eq!(anonymous.status, 401);

    let token = login_token(&config, &db);
    let mut authenticated = request("GET", "/v1/admin/modules", &config.frontend_origin);
    authenticated
        .cookies
        .insert("forge_admin_session".to_string(), token.clone());
    let response = handle(&config, &db, &authenticated, Utc::now());
    assert_eq!(
        response.status,
        200,
        "{}",
        String::from_utf8_lossy(&response.body)
    );
    let body = body_json(&response);
    assert_eq!(body["contract"], "forge-module-workflow/0.1.0");
    let modules = body["modules"].as_array().expect("module array");
    assert!(modules
        .iter()
        .any(|row| row["id"] == "workspace-governance"));
    let serialized = serde_json::to_string(&body).unwrap();
    assert!(!serialized.contains("/home/"));
    assert!(!serialized.contains("/modules/workspace-governance"));

    let mut status = request(
        "GET",
        "/v1/admin/modules/workspace-governance/gate/status",
        &config.frontend_origin,
    );
    status
        .cookies
        .insert("forge_admin_session".to_string(), token);
    let response = handle(&config, &db, &status, Utc::now());
    assert_eq!(
        response.status,
        200,
        "{}",
        String::from_utf8_lossy(&response.body)
    );
    let body = body_json(&response);
    assert_eq!(body["contract"], "forge-module-workflow/0.1.0");
    assert_eq!(body["module"]["id"], "workspace-governance");
    assert!(body["gate"]["aggregate"].is_string());
}

#[test]
fn module_gate_run_rejects_invalid_profiles_and_untrusted_fields() {
    let temp = TempDir::new().unwrap();
    let db = temp.path().join("registry.db");
    setup_admin(&db);
    let config = ApiConfig::default();
    let token = login_token(&config, &db);

    for body in [
        serde_json::json!({ "profile": "quick" }),
        serde_json::json!({ "profile": "fast", "path": "/tmp/outside" }),
        serde_json::json!({ "command": "sh -c true" }),
    ] {
        let mut run = request(
            "POST",
            "/v1/admin/modules/workspace-governance/gate/run",
            &config.frontend_origin,
        );
        run.cookies
            .insert("forge_admin_session".to_string(), token.clone());
        run.headers
            .insert("content-type".to_string(), "application/json".to_string());
        run.body = body.to_string().into_bytes();
        let response = handle(&config, &db, &run, Utc::now());
        assert_eq!(response.status, 400);
        assert!(String::from_utf8_lossy(&response.body).contains("gate-profile-invalid"));
    }
}
