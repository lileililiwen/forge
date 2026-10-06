use std::collections::BTreeMap;

use chrono::Utc;
use forge::api::{handle, ApiConfig, ApiRequest};
use tempfile::tempdir;

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

#[test]
fn admin_login_fleet_and_logout_use_a_forge_wide_cookie() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    forge::identity::global::setup(&db, "operator@example.test", "a-long-test-password").unwrap();

    let mut login = request("POST", "/v1/admin/session", &config.frontend_origin);
    login
        .headers
        .insert("content-type".to_string(), "application/json".to_string());
    login.body = br#"{"email":"operator@example.test","password":"a-long-test-password"}"#.to_vec();
    let response = handle(&config, &db, &login, Utc::now());
    assert_eq!(response.status, 200);
    assert_eq!(
        response
            .headers
            .get("access-control-allow-credentials")
            .map(String::as_str),
        Some("true")
    );
    let cookie = response.headers.get("set-cookie").unwrap();
    assert!(cookie.contains("forge_admin_session="));
    assert!(cookie.contains("HttpOnly"));
    let token = cookie
        .split(';')
        .next()
        .unwrap()
        .split_once('=')
        .unwrap()
        .1
        .to_string();

    let mut state = request("GET", "/v1/admin/session", &config.frontend_origin);
    state
        .cookies
        .insert("forge_admin_session".to_string(), token.clone());
    assert_eq!(handle(&config, &db, &state, Utc::now()).status, 200);

    let mut projects = request("GET", "/v1/admin/projects", &config.frontend_origin);
    projects
        .cookies
        .insert("forge_admin_session".to_string(), token.clone());
    let response = handle(&config, &db, &projects, Utc::now());
    assert_eq!(response.status, 200);
    let json: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
    assert_eq!(json["projects"].as_array().unwrap().len(), 0);

    let mut logout = request("DELETE", "/v1/admin/session", &config.frontend_origin);
    logout
        .cookies
        .insert("forge_admin_session".to_string(), token.clone());
    let response = handle(&config, &db, &logout, Utc::now());
    assert_eq!(response.status, 200);
    assert!(response
        .headers
        .get("set-cookie")
        .unwrap()
        .contains("Max-Age=0"));

    projects
        .cookies
        .insert("forge_admin_session".to_string(), token);
    assert_eq!(handle(&config, &db, &projects, Utc::now()).status, 401);
}

#[test]
fn admin_api_rejects_bad_credentials_and_untrusted_origins() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    forge::identity::global::setup(&db, "operator@example.test", "a-long-test-password").unwrap();

    let mut bad_login = request("POST", "/v1/admin/session", &config.frontend_origin);
    bad_login
        .headers
        .insert("content-type".to_string(), "application/json".to_string());
    bad_login.body = br#"{"email":"operator@example.test","password":"wrong-password"}"#.to_vec();
    assert_eq!(handle(&config, &db, &bad_login, Utc::now()).status, 401);

    let mut hostile = request("GET", "/v1/admin/session", "https://attacker.example");
    assert_eq!(handle(&config, &db, &hostile, Utc::now()).status, 403);
    hostile.method = "OPTIONS".to_string();
    assert_eq!(handle(&config, &db, &hostile, Utc::now()).status, 403);
}

/// Log in through the real handler and return the raw session token.
fn login_token(config: &ApiConfig, db: &std::path::Path) -> String {
    let mut login = request("POST", "/v1/admin/session", &config.frontend_origin);
    login
        .headers
        .insert("content-type".to_string(), "application/json".to_string());
    login.body = br#"{"email":"operator@example.test","password":"a-long-test-password"}"#.to_vec();
    let response = handle(config, db, &login, Utc::now());
    assert_eq!(response.status, 200);
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

fn with_cookie(method: &str, path: &str, origin: &str, token: &str) -> ApiRequest {
    let mut req = request(method, path, origin);
    req.cookies
        .insert("forge_admin_session".to_string(), token.to_string());
    req
}

#[test]
fn admin_api_requires_a_session_and_isolates_the_project_oidc_cookie() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    forge::identity::global::setup(&db, "operator@example.test", "a-long-test-password").unwrap();

    // Anonymous (no cookie) admin requests authorize nothing.
    let anon = request("GET", "/v1/admin/projects", &config.frontend_origin);
    assert_eq!(handle(&config, &db, &anon, Utc::now()).status, 401);
    let anon_state = request("GET", "/v1/admin/session", &config.frontend_origin);
    let response = handle(&config, &db, &anon_state, Utc::now());
    assert_eq!(response.status, 200);
    let json: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
    assert_eq!(json["authenticated"], serde_json::Value::Bool(false));

    // A successful login sets only the Forge-wide cookie, whose name is distinct
    // from the project-scoped OIDC `forge_session` cookie.
    let token = login_token(&config, &db);
    let mut login = request("POST", "/v1/admin/session", &config.frontend_origin);
    login
        .headers
        .insert("content-type".to_string(), "application/json".to_string());
    login.body = br#"{"email":"operator@example.test","password":"a-long-test-password"}"#.to_vec();
    let cookie = handle(&config, &db, &login, Utc::now())
        .headers
        .get("set-cookie")
        .cloned()
        .unwrap();
    assert!(cookie.contains("forge_admin_session="));
    assert!(!cookie.contains("forge_session="));
    assert!(cookie.contains("HttpOnly"));
    assert!(cookie.contains("SameSite=Lax"));
    let _ = token;

    // An expired session authorizes nothing: force the stored row past its bound.
    let token = login_token(&config, &db);
    let conn = rusqlite::Connection::open(&db).unwrap();
    conn.execute(
        "UPDATE forge_admin_sessions SET expires_at = ?1",
        rusqlite::params![Utc::now().timestamp() - 1],
    )
    .unwrap();
    drop(conn);
    let expired = with_cookie("GET", "/v1/admin/projects", &config.frontend_origin, &token);
    assert_eq!(handle(&config, &db, &expired, Utc::now()).status, 401);
}

#[test]
fn logout_without_the_configured_origin_is_refused_and_does_not_revoke() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    forge::identity::global::setup(&db, "operator@example.test", "a-long-test-password").unwrap();
    let token = login_token(&config, &db);

    // A mismatched Origin cannot revoke: the request is refused before any
    // session or cookie state changes.
    let mismatched = with_cookie(
        "DELETE",
        "/v1/admin/session",
        "https://attacker.example",
        &token,
    );
    assert_eq!(handle(&config, &db, &mismatched, Utc::now()).status, 403);

    // The session therefore still authorizes the fleet.
    let still_valid = with_cookie("GET", "/v1/admin/projects", &config.frontend_origin, &token);
    assert_eq!(handle(&config, &db, &still_valid, Utc::now()).status, 200);

    // Correct-origin logout then revokes it.
    let logout = with_cookie(
        "DELETE",
        "/v1/admin/session",
        &config.frontend_origin,
        &token,
    );
    assert_eq!(handle(&config, &db, &logout, Utc::now()).status, 200);
    assert_eq!(handle(&config, &db, &still_valid, Utc::now()).status, 401);
}
