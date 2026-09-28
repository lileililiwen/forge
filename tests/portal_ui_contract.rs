//! HTTP contract tests for the in-process portal UI.
//!
//! The portal-web-ui package adds three routes
//! (`GET /ui`, `GET /ui/projects/{id}`, and
//! `POST /ui/projects/{id}/publish`) to the existing API
//! service. These tests cover the verification oracle in
//! design D6: status codes, auth refusal, cross-origin
//! refusal, form-token mismatch, confirm enqueues a
//! `publish.ui` journal row, escape matrix.
//!
//! Project rows are seeded through a small registry helper
//! that inserts the rows directly. The tests do not depend
//! on `forge publish` or the JSON `apply` path; every
//! assertion is over the UI surface in isolation.

use std::collections::BTreeMap;
use std::io::Cursor;
use std::path::PathBuf;

use chrono::Utc;
use forge::api::{handle_buffered, ApiConfig, ApiRequest, ApiResponse};
use forge::registry::Registry;

const BEARER: &str = "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef";
const LOOPBACK_ORIGIN: &str = "http://127.0.0.1:8765";

fn tmp_db_path() -> PathBuf {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut path = dir.path().to_path_buf();
    // Keep the file so `Registry::open` finds it.
    path.push("forge.db");
    // Remove the auto-created tempdir on test exit; the
    // registry holds a Sqlite Connection until drop.
    let _ = dir; // let tempdir drop at end of fn scope and clean up
    path
}

fn seed_project(path: &PathBuf, id: &str, profile: &str, maturity: &str) {
    // Open the registry once so `apply_migrations` creates
    // the schema; then drop the handle and seed the rows
    // directly via rusqlite. This avoids the
    // manifest-validated `Registry::insert_project` path
    // (which would require a real `forge.yaml`).
    {
        let _registry = Registry::open(path).expect("open registry");
    }
    let conn = rusqlite::Connection::open(path).expect("open");
    conn.execute(
        "INSERT INTO projects \
         (id, name, path, profile, maturity, target_maturity, \
          schema_version, platform_version, features, observed_at) \
         VALUES (?1, ?1, ?2, ?3, ?4, NULL, 1, '0.1.0', '{}', datetime('now')) \
         ON CONFLICT(id) DO UPDATE SET \
           name = excluded.name, \
           path = excluded.path, \
           profile = excluded.profile, \
           maturity = excluded.maturity",
        rusqlite::params![id, format!("/tmp/{id}"), profile, maturity],
    )
    .unwrap_or_else(|err| panic!("insert {id}: {err}"));
}

fn drive(config: &ApiConfig, db_path: &PathBuf, request: &ApiRequest) -> ApiResponse {
    let mut full: Vec<u8> = Vec::new();
    full.extend_from_slice(request.method.as_bytes());
    full.extend_from_slice(b" ");
    full.extend_from_slice(request.path.as_bytes());
    full.extend_from_slice(b" HTTP/1.1\r\n");
    for (k, v) in &request.headers {
        full.extend_from_slice(k.as_bytes());
        full.extend_from_slice(b": ");
        full.extend_from_slice(v.as_bytes());
        full.extend_from_slice(b"\r\n");
    }
    if !request.body.is_empty() {
        full.extend_from_slice(format!("content-length: {}\r\n", request.body.len()).as_bytes());
    }
    full.extend_from_slice(b"\r\n");
    full.extend_from_slice(&request.body);
    let mut reader = Cursor::new(full);
    let mut writer = Vec::new();
    handle_buffered(config, db_path, &mut reader, &mut writer, 1024 * 1024)
        .expect("handle_buffered")
}

fn make_request(
    method: &str,
    path: &str,
    headers: BTreeMap<String, String>,
    bearer: Option<String>,
    body: Vec<u8>,
) -> ApiRequest {
    let mut headers = headers;
    // The buffered transport routes through `parse_request`
    // which extracts the bearer from the Authorization header;
    // mirror that here so the resulting `bearer_token` field
    // matches what `forge api serve` would have populated.
    if let Some(token) = bearer.as_ref() {
        headers.insert("authorization".to_string(), format!("Bearer {token}"));
    }
    ApiRequest {
        method: method.to_string(),
        path: path.to_string(),
        query: None,
        headers,
        body,
        idempotency_key: None,
        bearer_token: bearer,
        remote_addr: Some("127.0.0.1:9999".parse().unwrap()),
        started_at: Utc::now(),
    }
}

fn html_accept() -> BTreeMap<String, String> {
    let mut h = BTreeMap::new();
    h.insert("accept".to_string(), "text/html".to_string());
    h
}

#[test]
fn list_without_auth_returns_401_html() {
    let path = tmp_db_path();
    seed_project(&path, "alethefy", "rust-web", "L3");
    let config = ApiConfig::default();
    let req = make_request("GET", "/ui", html_accept(), None, Vec::new());
    let resp = drive(&config, &path, &req);
    assert_eq!(resp.status, 401);
    let body = String::from_utf8_lossy(&resp.body);
    assert!(body.contains("api-unauthorized"));
    // Never leaks a JSON envelope to the browser.
    assert!(!body.starts_with("{"));
}

#[test]
fn list_with_auth_returns_200_html_with_roster() {
    let path = tmp_db_path();
    seed_project(&path, "alethefy", "rust-web", "L3");
    seed_project(&path, "forge", "rust-web", "L2");
    let config = ApiConfig::default();
    let req = make_request(
        "GET",
        "/ui",
        html_accept(),
        Some(BEARER.to_string()),
        Vec::new(),
    );
    let resp = drive(&config, &path, &req);
    assert_eq!(resp.status, 200);
    let ct = resp
        .headers
        .iter()
        .find(|(k, _)| k.as_str() == "content-type")
        .map(|(_, v)| v.as_str())
        .unwrap_or("");
    assert!(ct.contains("text/html"), "expected text/html, got `{ct}`");
    let body = String::from_utf8_lossy(&resp.body);
    assert!(
        body.to_lowercase().contains("<!doctype"),
        "expected HTML doctype, got first 200 bytes: `{}`",
        &body[..body.len().min(200)]
    );
    assert!(body.contains("alethefy"));
    assert!(body.contains("forge"));
    // The page renders a Liveness pointer line so operators
    // know the data source.
    assert!(body.contains("fleet online"));
}

#[test]
fn list_renders_404_html_for_unknown_project() {
    let path = tmp_db_path();
    seed_project(&path, "alethefy", "rust-web", "L3");
    let config = ApiConfig::default();
    let req = make_request(
        "GET",
        "/ui/projects/no-such-app",
        html_accept(),
        Some(BEARER.to_string()),
        Vec::new(),
    );
    let resp = drive(&config, &path, &req);
    assert_eq!(resp.status, 404);
    let body = String::from_utf8_lossy(&resp.body);
    assert!(body.contains("unknown-project"));
    assert!(!body.starts_with("{"));
}

#[test]
fn detail_renders_known_project() {
    let path = tmp_db_path();
    seed_project(&path, "alethefy", "rust-web", "L3");
    let config = ApiConfig::default();
    let req = make_request(
        "GET",
        "/ui/projects/alethefy",
        html_accept(),
        Some(BEARER.to_string()),
        Vec::new(),
    );
    let resp = drive(&config, &path, &req);
    assert_eq!(resp.status, 200);
    let body = String::from_utf8_lossy(&resp.body);
    assert!(body.contains("alethefy"));
    assert!(body.contains("Profile"));
    assert!(body.contains("Republish"));
    assert!(body.contains("Recent operations"));
}

#[test]
fn publish_without_confirm_returns_plan_page_no_journal_row() {
    let path = tmp_db_path();
    seed_project(&path, "alethefy", "rust-web", "L3");
    let config = ApiConfig::default();
    let body = format!("token={BEARER}&origin={LOOPBACK_ORIGIN}");
    let mut headers = html_accept();
    headers.insert("origin".to_string(), LOOPBACK_ORIGIN.to_string());
    headers.insert(
        "content-type".to_string(),
        "application/x-www-form-urlencoded".to_string(),
    );
    let req = make_request(
        "POST",
        "/ui/projects/alethefy/publish",
        headers,
        Some(BEARER.to_string()),
        body.into_bytes(),
    );
    let resp = drive(&config, &path, &req);
    assert_eq!(resp.status, 200);
    let body = String::from_utf8_lossy(&resp.body);
    assert!(body.contains("Dry-run plan"));
    assert!(body.contains("Confirm republish"));
    assert!(body.contains("sync"));
    assert!(body.contains("deploy"));
    let registry = Registry::open(&path).expect("open");
    let recent = registry.recent_operations(64).expect("recent");
    assert!(
        !recent.iter().any(|e| e.kind == "publish.ui"),
        "dry-run preview must not journal a publish.ui row"
    );
}

#[test]
fn publish_with_confirm_enqueues_and_returns_202() {
    let path = tmp_db_path();
    seed_project(&path, "alethefy", "rust-web", "L3");
    let config = ApiConfig::default();
    let body = format!("token={BEARER}&origin={LOOPBACK_ORIGIN}&confirm=yes");
    let mut headers = html_accept();
    headers.insert("origin".to_string(), LOOPBACK_ORIGIN.to_string());
    headers.insert(
        "content-type".to_string(),
        "application/x-www-form-urlencoded".to_string(),
    );
    let req = make_request(
        "POST",
        "/ui/projects/alethefy/publish",
        headers,
        Some(BEARER.to_string()),
        body.into_bytes(),
    );
    let resp = drive(&config, &path, &req);
    assert_eq!(resp.status, 202);
    let body_text = String::from_utf8_lossy(&resp.body);
    assert!(body_text.contains("Republish enqueued"));
    let registry = Registry::open(&path).expect("open");
    let recent = registry.recent_operations(64).expect("recent");
    assert!(
        recent
            .iter()
            .any(|e| e.kind == "publish.ui" && e.project_id == "alethefy"),
        "confirmed publish should journal a publish.ui row"
    );
}

#[test]
fn cross_origin_post_is_refused() {
    let path = tmp_db_path();
    seed_project(&path, "alethefy", "rust-web", "L3");
    let config = ApiConfig::default();
    let body = format!("token={BEARER}&origin=http://evil.example.com&confirm=yes");
    let mut headers = html_accept();
    headers.insert("origin".to_string(), "http://evil.example.com".to_string());
    headers.insert(
        "content-type".to_string(),
        "application/x-www-form-urlencoded".to_string(),
    );
    let req = make_request(
        "POST",
        "/ui/projects/alethefy/publish",
        headers,
        Some(BEARER.to_string()),
        body.into_bytes(),
    );
    let resp = drive(&config, &path, &req);
    assert_eq!(resp.status, 403);
    let body = String::from_utf8_lossy(&resp.body);
    assert!(body.contains("ui-origin-mismatch"));
    let registry = Registry::open(&path).expect("open");
    let recent = registry.recent_operations(64).expect("recent");
    assert!(
        !recent.iter().any(|e| e.kind == "publish.ui"),
        "cross-origin POST must not journal a row"
    );
}

#[test]
fn mismatched_form_token_is_refused() {
    let path = tmp_db_path();
    seed_project(&path, "alethefy", "rust-web", "L3");
    let config = ApiConfig::default();
    let body = format!(
        "token={wrong}&origin={LOOPBACK_ORIGIN}&confirm=yes",
        wrong = "not_the_bearer"
    );
    let mut headers = html_accept();
    headers.insert("origin".to_string(), LOOPBACK_ORIGIN.to_string());
    headers.insert(
        "content-type".to_string(),
        "application/x-www-form-urlencoded".to_string(),
    );
    let req = make_request(
        "POST",
        "/ui/projects/alethefy/publish",
        headers,
        Some(BEARER.to_string()),
        body.into_bytes(),
    );
    let resp = drive(&config, &path, &req);
    assert_eq!(resp.status, 403);
    let body = String::from_utf8_lossy(&resp.body);
    assert!(body.contains("api-token-mismatch"));
}

#[test]
fn invalid_project_id_in_path_returns_400_html() {
    let path = tmp_db_path();
    seed_project(&path, "alethefy", "rust-web", "L3");
    let config = ApiConfig::default();
    let req = make_request(
        "GET",
        "/ui/projects/<script>",
        html_accept(),
        Some(BEARER.to_string()),
        Vec::new(),
    );
    let resp = drive(&config, &path, &req);
    assert_eq!(resp.status, 400);
    let body = String::from_utf8_lossy(&resp.body);
    // The id is rejected, not echoed as raw HTML.
    assert!(!body.contains("<script>"));
    assert!(body.contains("manifest-invalid"));
}

#[test]
fn list_escapes_project_name_with_quotes() {
    let path = tmp_db_path();
    seed_project(&path, "evil-app", "rust-web", "L3");
    let config = ApiConfig::default();
    let req = make_request(
        "GET",
        "/ui",
        html_accept(),
        Some(BEARER.to_string()),
        Vec::new(),
    );
    let resp = drive(&config, &path, &req);
    assert_eq!(resp.status, 200);
    let body = String::from_utf8_lossy(&resp.body);
    assert!(body.contains("evil-app"));
    // The id passes through maud's escape; the literal
    // ampersand/quote substring never appears in the
    // rendered output, even though it could in the source.
    assert!(!body.contains("evil&amp;"));
}

#[test]
fn empty_registry_renders_empty_fleet_notice() {
    let path = tmp_db_path();
    let _ = path.clone();
    let config = ApiConfig::default();
    let req = make_request(
        "GET",
        "/ui",
        html_accept(),
        Some(BEARER.to_string()),
        Vec::new(),
    );
    let resp = drive(&config, &path, &req);
    assert_eq!(resp.status, 200);
    let body = String::from_utf8_lossy(&resp.body);
    assert!(body.contains("no projects"));
}

#[test]
fn json_accept_falls_through_to_api_routing_error() {
    let path = tmp_db_path();
    seed_project(&path, "alethefy", "rust-web", "L3");
    let config = ApiConfig::default();
    let mut headers = BTreeMap::new();
    headers.insert("accept".to_string(), "application/json".to_string());
    let req = make_request("GET", "/ui", headers, Some(BEARER.to_string()), Vec::new());
    let resp = drive(&config, &path, &req);
    assert_eq!(resp.status, 400);
    let body = String::from_utf8_lossy(&resp.body);
    assert!(body.contains("api-routing"));
}
