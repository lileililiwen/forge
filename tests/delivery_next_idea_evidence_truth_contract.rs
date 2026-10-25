//! API contract for `delivery-next-idea-evidence-truth`.
//!
//! The next-idea row states the transition the server recorded. It never
//! claims a publish the server did not observe, and a caller's note is
//! kept in the row marked as the caller's report.
//!
//! - a confirmed request with no note writes a row that claims no
//!   publish;
//! - a confirmed request with a note keeps the note marked as the
//!   caller's report;
//! - the unconfirmed, over-long and credential-shaped refusals are
//!   unchanged and journal nothing.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::sync::Mutex;

use chrono::Utc;
use forge::api::{handle, ApiConfig, ApiRequest};
use forge::registry::Registry;
use tempfile::tempdir;

static SERIAL: Mutex<()> = Mutex::new(());

fn lock() -> std::sync::MutexGuard<'static, ()> {
    match SERIAL.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

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

fn with_cookie(mut req: ApiRequest, token: &str) -> ApiRequest {
    req.cookies
        .insert("forge_admin_session".to_string(), token.to_string());
    req
}

fn json_body(mut req: ApiRequest, body: &str) -> ApiRequest {
    req.headers
        .insert("content-type".to_string(), "application/json".to_string());
    req.body = body.as_bytes().to_vec();
    req
}

fn login_token(config: &ApiConfig, db: &Path) -> String {
    forge::identity::global::setup(db, "operator@example.test", "a-long-test-password").unwrap();
    let mut login = request("POST", "/v1/admin/session", &config.frontend_origin);
    login = json_body(
        login,
        r#"{"email":"operator@example.test","password":"a-long-test-password"}"#,
    );
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

fn write_project(dir: &Path, id: &str) {
    fs::create_dir_all(dir).unwrap();
    fs::write(
        dir.join("forge.yaml"),
        format!(
            "schema: 1\nproject:\n  id: {id}\n  name: Test {id}\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\nfeatures: {{}}\n"
        ),
    )
    .unwrap();
}

fn register_project(db: &Path, dir: &Path) {
    let mut registry = Registry::open(db).unwrap();
    registry.register(dir, None).unwrap();
}

/// The `delivery.next-idea` journal rows of one project, oldest first.
fn next_idea_rows(db: &Path, project: &str) -> Vec<(String, String)> {
    Registry::open(db)
        .unwrap()
        .journal_entries()
        .unwrap()
        .iter()
        .filter(|e| e.kind == "delivery.next-idea" && e.project_id == project)
        .map(|e| (e.state.clone(), e.detail.clone().unwrap_or_default()))
        .collect()
}

fn next_idea_request(config: &ApiConfig, body: &str, token: &str) -> ApiRequest {
    with_cookie(
        json_body(
            request(
                "POST",
                "/v1/admin/projects/p1/delivery/next-idea",
                &config.frontend_origin,
            ),
            body,
        ),
        token,
    )
}

#[test]
fn a_next_idea_row_claims_no_publish() {
    let _guard = lock();
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let proj = dir.path().join("p1");
    write_project(&proj, "p1");
    register_project(&db, &proj);

    // A confirmed request with no note: the server recorded the
    // transition and read no publish, so it claims none.
    let req = next_idea_request(&config, r#"{"confirm":true}"#, &token);
    let response = handle(&config, &db, &req, Utc::now());
    assert_eq!(
        response.status,
        200,
        "{}",
        String::from_utf8_lossy(&response.body)
    );
    let body: serde_json::Value =
        serde_json::from_slice(&response.body).expect("the route answers JSON");
    assert_eq!(body["accepted"], true);
    assert_eq!(body["project_id"], "p1");
    assert!(body["operation_id"].as_i64().is_some(), "{body}");

    let rows = next_idea_rows(&db, "p1");
    assert_eq!(rows.len(), 1, "one row per transition");
    let (state, detail) = &rows[0];
    assert_eq!(state, "done", "the route recorded the transition");
    assert_eq!(
        detail, "delivery.next-idea: transition=proposed",
        "the row states the transition and claims no publish"
    );
    assert!(!detail.contains("publish landed"), "{detail}");
}

#[test]
fn a_callers_note_is_recorded_as_the_callers_report() {
    let _guard = lock();
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let proj = dir.path().join("p1");
    write_project(&proj, "p1");
    register_project(&db, &proj);

    let req = next_idea_request(
        &config,
        r#"{"confirm":true,"note":"publish landed"}"#,
        &token,
    );
    let response = handle(&config, &db, &req, Utc::now());
    assert_eq!(
        response.status,
        200,
        "{}",
        String::from_utf8_lossy(&response.body)
    );

    let rows = next_idea_rows(&db, "p1");
    assert_eq!(rows.len(), 1, "one row per transition");
    let (_, detail) = &rows[0];
    assert_eq!(
        detail, "delivery.next-idea: transition=proposed reported=publish landed",
        "the caller's report is kept and marked as the caller's"
    );
}

#[test]
fn the_refusals_are_unchanged() {
    let _guard = lock();
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let proj = dir.path().join("p1");
    write_project(&proj, "p1");
    register_project(&db, &proj);

    // Without `confirm` the route refuses and journals nothing.
    let req = next_idea_request(&config, r#"{"confirm":false}"#, &token);
    let response = handle(&config, &db, &req, Utc::now());
    assert_eq!(response.status, 409);
    assert!(next_idea_rows(&db, "p1").is_empty());

    // An over-long note is refused.
    let long = "x".repeat(2001);
    let req = next_idea_request(
        &config,
        &format!(r#"{{"confirm":true,"note":"{long}"}}"#),
        &token,
    );
    let response = handle(&config, &db, &req, Utc::now());
    assert_eq!(response.status, 400);
    assert!(next_idea_rows(&db, "p1").is_empty());

    // A credential-shaped note is refused.
    let req = next_idea_request(
        &config,
        r#"{"confirm":true,"note":"AKIAIOSFODNN7EXAMPLE"}"#,
        &token,
    );
    let response = handle(&config, &db, &req, Utc::now());
    assert_eq!(response.status, 400);
    assert!(next_idea_rows(&db, "p1").is_empty());
}
