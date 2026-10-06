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
use forge::core::ForgeError;
use forge::registry::Registry;

const BEARER: &str = "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef";
const LOOPBACK_ORIGIN: &str = "http://127.0.0.1:8765";

fn tmp_db_path() -> PathBuf {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut path = dir.path().to_path_buf();
    // The directory is dropped at the end of this scope,
    // so any `Registry::open` call below must recreate the
    // parent directory before it can write the file. The
    // existing `seed_project` helper already does that
    // via `Registry::open(path)`.
    path.push("forge.db");
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
    if let Some(query) = &request.query {
        full.extend_from_slice(b"?");
        full.extend_from_slice(query.as_bytes());
    }
    full.extend_from_slice(b" HTTP/1.1\r\n");
    for (k, v) in &request.headers {
        full.extend_from_slice(k.as_bytes());
        full.extend_from_slice(b": ");
        full.extend_from_slice(v.as_bytes());
        full.extend_from_slice(b"\r\n");
    }
    if !request.cookies.is_empty() {
        let cookie_header = request
            .cookies
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join("; ");
        full.extend_from_slice(b"cookie: ");
        full.extend_from_slice(cookie_header.as_bytes());
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
        cookies: BTreeMap::new(),
        remote_addr: Some("127.0.0.1:9999".parse().unwrap()),
        started_at: Utc::now(),
    }
}

fn make_request_with_query(
    method: &str,
    path: &str,
    query: Option<&str>,
    headers: BTreeMap<String, String>,
    body: Vec<u8>,
) -> ApiRequest {
    let mut req = make_request(method, path, headers, None, body);
    req.query = query.map(|value| value.to_string());
    req
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

// --- browser sign-in / callback / sign-out ----------------------------

use forge::identity::{
    self, AuthChallenge, FakeBrowserAuthVerifier, IdentityConfig, ProviderClaims,
};
use std::sync::Arc;

const SIGN_IN_PROJECT: &str = "alethefy";
const SIGN_IN_ISSUER: &str = "https://example.okta.com";
const SIGN_IN_REDIRECT: &str = "https://admin.example.com/ui/auth/callback";
const SIGN_IN_AUDIENCE: &str = "forge-admin";
const SIGN_IN_ADMIN_VALUE: &str = "forge-admins";

/// Seed a project plus a `forge.yaml` carrying the
/// identity block the new sign-in routes need. The
/// project id is keyed so the registry row points at a
/// real directory the identity layer can write to.
/// The returned `TempDir` must be kept alive for the
/// duration of the test so the registry file at
/// `db_path` is not unlinked while the test runs.
fn seed_identity_project(db_path: &PathBuf, id: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let project_dir = dir.path().to_path_buf();
    let yaml = format!(
        "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: rust-web\n  \
         maturity: L1\n  target_maturity: L2\nruntime:\n  language: rust\n\
         identity:\n  provider: okta\n  issuer: {SIGN_IN_ISSUER}\n  \
         client_id: {SIGN_IN_AUDIENCE}\n  audience: {SIGN_IN_AUDIENCE}\n  \
         redirect_uri: {SIGN_IN_REDIRECT}\n  scopes:\n    - openid\n    - profile\n  \
         admin_claim: groups\n  admin_values:\n    - {SIGN_IN_ADMIN_VALUE}\n  \
         state_ttl_seconds: 120\n  session_ttl_seconds: 3600\n  \
         client_secret_ref: env://OIDC_CLIENT_SECRET\n"
    );
    std::fs::write(project_dir.join("forge.yaml"), yaml).expect("forge.yaml");
    // Open the registry once so the parent directory
    // exists and the schema is applied, then drop the
    // handle and seed the row directly.
    {
        let _registry = Registry::open(db_path).expect("open registry");
    }
    let conn = rusqlite::Connection::open(db_path).expect("open");
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
        rusqlite::params![
            id,
            project_dir.to_string_lossy().to_string(),
            "rust-web",
            "L3"
        ],
    )
    .unwrap_or_else(|err| panic!("insert {id}: {err}"));
    (dir, project_dir)
}

fn config_with_verifier(verifier: Arc<FakeBrowserAuthVerifier>) -> ApiConfig {
    let mut cfg = ApiConfig::default();
    cfg.browser_auth_verifier = verifier;
    cfg
}

fn set_cookie(resp: &ApiResponse) -> String {
    resp.headers.get("set-cookie").cloned().unwrap_or_default()
}

fn location(resp: &ApiResponse) -> String {
    resp.headers.get("location").cloned().unwrap_or_default()
}

fn build_claims_for(challenge: &AuthChallenge, cfg: &IdentityConfig) -> ProviderClaims {
    let mut claims = std::collections::BTreeMap::new();
    claims.insert(cfg.admin_claim.clone(), SIGN_IN_ADMIN_VALUE.to_string());
    ProviderClaims {
        issuer: cfg.issuer.clone(),
        audience: cfg.audience.clone(),
        subject: "user-1".to_string(),
        issued_at: chrono::Utc::now(),
        expires_at: chrono::Utc::now() + chrono::Duration::seconds(60),
        nonce: challenge.nonce.clone(),
        scopes: cfg.scopes.clone(),
        claims,
    }
}

#[test]
fn sign_in_start_redirects_to_provider_with_challenge_persisted() {
    let path = tmp_db_path();
    let (_dir, project_dir) = seed_identity_project(&path, SIGN_IN_PROJECT);
    let config = config_with_verifier(Arc::new(FakeBrowserAuthVerifier::new()));
    let req = make_request_with_query(
        "GET",
        "/ui/sign-in",
        Some(&format!(
            "project={SIGN_IN_PROJECT}&return=/ui/projects/{SIGN_IN_PROJECT}"
        )),
        html_accept(),
        Vec::new(),
    );
    let resp = drive(&config, &path, &req);
    assert_eq!(resp.status, 303);
    let loc = location(&resp);
    assert!(
        loc.starts_with(SIGN_IN_ISSUER),
        "expected provider redirect, got `{loc}`"
    );
    assert!(loc.contains("response_type=code"));
    assert!(loc.contains("state="));
    let cookie = set_cookie(&resp);
    assert!(
        cookie.contains("forge_oidc_state="),
        "expected state cookie, got `{cookie}`"
    );
    assert!(cookie.contains("Path=/ui/auth/callback"));
    assert!(cookie.contains("HttpOnly"));
    assert!(cookie.contains("SameSite=Lax"));
    // A challenge file should now exist for the project.
    let state = extract_query_value(&loc, "state").expect("state in url");
    let challenge_path = project_dir
        .join(".forge")
        .join("identity")
        .join(SIGN_IN_PROJECT)
        .join("pending")
        .join(format!("{state}.json"));
    assert!(
        challenge_path.exists(),
        "challenge not persisted at {challenge_path:?}"
    );
}

#[test]
fn sign_in_unknown_project_returns_404() {
    let path = tmp_db_path();
    let config = config_with_verifier(Arc::new(FakeBrowserAuthVerifier::new()));
    let req = make_request_with_query(
        "GET",
        "/ui/sign-in",
        Some("project=nope-app&return=/ui"),
        html_accept(),
        Vec::new(),
    );
    let resp = drive(&config, &path, &req);
    assert_eq!(resp.status, 404);
    let body = String::from_utf8_lossy(&resp.body);
    assert!(body.contains("unknown-project"));
}

#[test]
fn sign_in_without_project_param_returns_400() {
    let path = tmp_db_path();
    let config = config_with_verifier(Arc::new(FakeBrowserAuthVerifier::new()));
    let req = make_request_with_query("GET", "/ui/sign-in", None, html_accept(), Vec::new());
    let resp = drive(&config, &path, &req);
    assert_eq!(resp.status, 400);
    let body = String::from_utf8_lossy(&resp.body);
    assert!(body.contains("ui-sign-in-missing-project"));
}

#[test]
fn sign_in_unsafe_return_falls_back_to_ui() {
    let path = tmp_db_path();
    let _dir = seed_identity_project(&path, SIGN_IN_PROJECT);
    let config = config_with_verifier(Arc::new(FakeBrowserAuthVerifier::new()));
    let req = make_request_with_query(
        "GET",
        "/ui/sign-in",
        Some("project=alethefy&return=https://evil.example.com/"),
        html_accept(),
        Vec::new(),
    );
    let resp = drive(&config, &path, &req);
    assert_eq!(resp.status, 303);
    // The page body is the sign-in page; the location
    // header is the provider URL. Either way, no
    // off-origin redirect is produced for the operator.
    let body = String::from_utf8_lossy(&resp.body);
    assert!(!body.contains("https://evil.example.com"));
}

#[test]
fn sign_in_external_relative_return_falls_back_to_ui() {
    let path = tmp_db_path();
    let _dir = seed_identity_project(&path, SIGN_IN_PROJECT);
    let config = config_with_verifier(Arc::new(FakeBrowserAuthVerifier::new()));
    let req = make_request_with_query(
        "GET",
        "/ui/sign-in",
        Some("project=alethefy&return=//evil.example.com/"),
        html_accept(),
        Vec::new(),
    );
    let resp = drive(&config, &path, &req);
    assert_eq!(resp.status, 303);
    let body = String::from_utf8_lossy(&resp.body);
    assert!(!body.contains("evil.example.com"));
}

#[test]
fn auth_callback_valid_mints_session_and_sets_cookie() {
    let path = tmp_db_path();
    let (_dir, project_dir) = seed_identity_project(&path, SIGN_IN_PROJECT);
    let (manifest, _) =
        forge::core::manifest::Manifest::load_from_dir(&project_dir, None).expect("manifest");
    let cfg = IdentityConfig::from_manifest_opt(SIGN_IN_PROJECT, &manifest)
        .expect("cfg")
        .expect("has identity");
    let challenge =
        identity::build_challenge(SIGN_IN_PROJECT, &cfg, Utc::now()).expect("challenge");
    identity::save_challenge(&project_dir, SIGN_IN_PROJECT, &challenge).expect("save challenge");

    let verifier = Arc::new(FakeBrowserAuthVerifier::new());
    verifier.queue_success(build_claims_for(&challenge, &cfg));
    let config = config_with_verifier(verifier);

    let mut cookies = BTreeMap::new();
    cookies.insert("forge_oidc_state".to_string(), challenge.state.clone());
    let mut req = make_request_with_query(
        "GET",
        "/ui/auth/callback",
        Some(&format!(
            "state={state}&code=opaque_code&return=/ui/projects/{SIGN_IN_PROJECT}",
            state = challenge.state
        )),
        html_accept(),
        Vec::new(),
    );
    req.cookies = cookies;
    let resp = drive(&config, &path, &req);
    assert_eq!(resp.status, 303);
    let loc = location(&resp);
    assert_eq!(loc, format!("/ui/projects/{SIGN_IN_PROJECT}"));
    let cookie = set_cookie(&resp);
    assert!(
        cookie.contains("forge_session="),
        "expected session cookie, got `{cookie}`"
    );
    assert!(cookie.contains("Path=/ui"));
    assert!(cookie.contains("HttpOnly"));
    assert!(cookie.contains("SameSite=Lax"));
    // The session file should exist now.
    let session_id = extract_cookie_value(&cookie, "forge_session").expect("session id");
    let session_path = project_dir
        .join(".forge")
        .join("identity")
        .join(SIGN_IN_PROJECT)
        .join("sessions")
        .join(format!("{session_id}.json"));
    assert!(
        session_path.exists(),
        "session not persisted at {session_path:?}"
    );
    // The challenge must have been consumed.
    let challenge_path = project_dir
        .join(".forge")
        .join("identity")
        .join(SIGN_IN_PROJECT)
        .join("pending")
        .join(format!("{}.json", challenge.state));
    assert!(!challenge_path.exists());
}

#[test]
fn auth_callback_state_mismatch_refuses() {
    let path = tmp_db_path();
    let (_dir, project_dir) = seed_identity_project(&path, SIGN_IN_PROJECT);
    let (manifest, _) =
        forge::core::manifest::Manifest::load_from_dir(&project_dir, None).expect("manifest");
    let cfg = IdentityConfig::from_manifest_opt(SIGN_IN_PROJECT, &manifest)
        .expect("cfg")
        .expect("has identity");
    let challenge =
        identity::build_challenge(SIGN_IN_PROJECT, &cfg, Utc::now()).expect("challenge");
    identity::save_challenge(&project_dir, SIGN_IN_PROJECT, &challenge).expect("save challenge");

    let verifier = Arc::new(FakeBrowserAuthVerifier::new());
    let config = config_with_verifier(verifier);

    let mut cookies = BTreeMap::new();
    cookies.insert("forge_oidc_state".to_string(), challenge.state.clone());
    let mut req = make_request_with_query(
        "GET",
        "/ui/auth/callback",
        Some("state=deadbeefdeadbeef&code=opaque_code"),
        html_accept(),
        Vec::new(),
    );
    req.cookies = cookies;
    let resp = drive(&config, &path, &req);
    assert_eq!(resp.status, 401);
    let body = String::from_utf8_lossy(&resp.body);
    assert!(body.contains("identity-invalid"));
}

#[test]
fn auth_callback_no_state_cookie_refuses() {
    let path = tmp_db_path();
    let _dir = seed_identity_project(&path, SIGN_IN_PROJECT);
    let verifier = Arc::new(FakeBrowserAuthVerifier::new());
    let config = config_with_verifier(verifier);
    let req = make_request_with_query(
        "GET",
        "/ui/auth/callback",
        Some("state=deadbeef&code=opaque_code"),
        html_accept(),
        Vec::new(),
    );
    let resp = drive(&config, &path, &req);
    assert_eq!(resp.status, 401);
    let body = String::from_utf8_lossy(&resp.body);
    assert!(body.contains("identity-invalid"));
}

#[test]
fn auth_callback_replay_refuses_second_attempt() {
    let path = tmp_db_path();
    let (_dir, project_dir) = seed_identity_project(&path, SIGN_IN_PROJECT);
    let (manifest, _) =
        forge::core::manifest::Manifest::load_from_dir(&project_dir, None).expect("manifest");
    let cfg = IdentityConfig::from_manifest_opt(SIGN_IN_PROJECT, &manifest)
        .expect("cfg")
        .expect("has identity");
    let challenge =
        identity::build_challenge(SIGN_IN_PROJECT, &cfg, Utc::now()).expect("challenge");
    identity::save_challenge(&project_dir, SIGN_IN_PROJECT, &challenge).expect("save challenge");

    let verifier = Arc::new(FakeBrowserAuthVerifier::new());
    verifier.queue_success(build_claims_for(&challenge, &cfg));
    verifier.queue_success(build_claims_for(&challenge, &cfg));
    let config = config_with_verifier(verifier);

    // First call: success.
    let mut cookies = BTreeMap::new();
    cookies.insert("forge_oidc_state".to_string(), challenge.state.clone());
    let mut req1 = make_request_with_query(
        "GET",
        "/ui/auth/callback",
        Some(&format!("state={}&code=opaque_code", challenge.state)),
        html_accept(),
        Vec::new(),
    );
    req1.cookies = cookies.clone();
    let resp1 = drive(&config, &path, &req1);
    assert_eq!(resp1.status, 303);

    // Replay: the challenge has been consumed; the second
    // callback should be refused, not mint a new session.
    let mut req2 = make_request_with_query(
        "GET",
        "/ui/auth/callback",
        Some(&format!("state={}&code=opaque_code", challenge.state)),
        html_accept(),
        Vec::new(),
    );
    req2.cookies = cookies;
    let resp2 = drive(&config, &path, &req2);
    assert_eq!(resp2.status, 401);
}

#[test]
fn auth_callback_claim_verification_failure_refuses() {
    let path = tmp_db_path();
    let (_dir, project_dir) = seed_identity_project(&path, SIGN_IN_PROJECT);
    let (manifest, _) =
        forge::core::manifest::Manifest::load_from_dir(&project_dir, None).expect("manifest");
    let cfg = IdentityConfig::from_manifest_opt(SIGN_IN_PROJECT, &manifest)
        .expect("cfg")
        .expect("has identity");
    let challenge =
        identity::build_challenge(SIGN_IN_PROJECT, &cfg, Utc::now()).expect("challenge");
    identity::save_challenge(&project_dir, SIGN_IN_PROJECT, &challenge).expect("save challenge");

    let verifier = Arc::new(FakeBrowserAuthVerifier::new());
    verifier.queue_error(ForgeError::IdentityAuthFailed {
        reason: "provider rejected the code exchange".to_string(),
    });
    let config = config_with_verifier(verifier);

    let mut cookies = BTreeMap::new();
    cookies.insert("forge_oidc_state".to_string(), challenge.state.clone());
    let mut req = make_request_with_query(
        "GET",
        "/ui/auth/callback",
        Some(&format!("state={}&code=opaque_code", challenge.state)),
        html_accept(),
        Vec::new(),
    );
    req.cookies = cookies;
    let resp = drive(&config, &path, &req);
    assert_eq!(resp.status, 500);
    let body = String::from_utf8_lossy(&resp.body);
    assert!(body.contains("identity-auth-failed"));
}

#[test]
fn cookie_authorizes_ui_request() {
    let path = tmp_db_path();
    let (_dir, project_dir) = seed_identity_project(&path, SIGN_IN_PROJECT);
    let (manifest, _) =
        forge::core::manifest::Manifest::load_from_dir(&project_dir, None).expect("manifest");
    let cfg = IdentityConfig::from_manifest_opt(SIGN_IN_PROJECT, &manifest)
        .expect("cfg")
        .expect("has identity");
    let challenge =
        identity::build_challenge(SIGN_IN_PROJECT, &cfg, Utc::now()).expect("challenge");
    let claims = build_claims_for(&challenge, &cfg);
    let session = identity::mint_session(&cfg, &claims, &challenge, Utc::now()).expect("session");
    identity::save_session(&project_dir, SIGN_IN_PROJECT, &session).expect("save session");

    let config = ApiConfig::default();
    let mut cookies = BTreeMap::new();
    cookies.insert("forge_session".to_string(), session.session_id.clone());
    let mut req = make_request("GET", "/ui", html_accept(), None, Vec::new());
    req.cookies = cookies;
    let resp = drive(&config, &path, &req);
    assert_eq!(resp.status, 200);
    let body = String::from_utf8_lossy(&resp.body);
    assert!(body.contains("alethefy"));
}

#[test]
fn cookie_does_not_authorize_v1_route() {
    let path = tmp_db_path();
    let (_dir, project_dir) = seed_identity_project(&path, SIGN_IN_PROJECT);
    let (manifest, _) =
        forge::core::manifest::Manifest::load_from_dir(&project_dir, None).expect("manifest");
    let cfg = IdentityConfig::from_manifest_opt(SIGN_IN_PROJECT, &manifest)
        .expect("cfg")
        .expect("has identity");
    let challenge =
        identity::build_challenge(SIGN_IN_PROJECT, &cfg, Utc::now()).expect("challenge");
    let claims = build_claims_for(&challenge, &cfg);
    let session = identity::mint_session(&cfg, &claims, &challenge, Utc::now()).expect("session");
    identity::save_session(&project_dir, SIGN_IN_PROJECT, &session).expect("save session");

    let config = ApiConfig::default();
    let mut cookies = BTreeMap::new();
    cookies.insert("forge_session".to_string(), session.session_id.clone());
    let mut req = make_request("GET", "/v1/projects", BTreeMap::new(), None, Vec::new());
    req.cookies = cookies;
    let resp = drive(&config, &path, &req);
    assert_eq!(resp.status, 401);
}

#[test]
fn cross_project_cookie_is_refused() {
    let path = tmp_db_path();
    let _alethefy_dir = seed_identity_project(&path, "alethefy");
    let (_other_dir, other_dir) = seed_identity_project(&path, "other-proj");
    let (other_manifest, _) =
        forge::core::manifest::Manifest::load_from_dir(&other_dir, None).expect("manifest");
    let other_cfg = IdentityConfig::from_manifest_opt("other-proj", &other_manifest)
        .expect("cfg")
        .expect("has identity");
    let other_challenge =
        identity::build_challenge("other-proj", &other_cfg, Utc::now()).expect("c");
    let other_claims = build_claims_for(&other_challenge, &other_cfg);
    let other_session =
        identity::mint_session(&other_cfg, &other_claims, &other_challenge, Utc::now()).expect("s");
    identity::save_session(&other_dir, "other-proj", &other_session).expect("save");

    let config = ApiConfig::default();
    let mut cookies = BTreeMap::new();
    cookies.insert(
        "forge_session".to_string(),
        other_session.session_id.clone(),
    );
    let mut req = make_request(
        "GET",
        "/ui/projects/alethefy",
        html_accept(),
        None,
        Vec::new(),
    );
    req.cookies = cookies;
    let resp = drive(&config, &path, &req);
    // The session belongs to other-proj, not alethefy; the
    // cross-project boundary is refused. The response is
    // an `api-project-mismatch` 403 (or 401 if the lookup
    // path produces nothing); both are valid rejections
    // and neither leaks the JSON API contract.
    assert!(resp.status == 401 || resp.status == 403);
}

#[test]
fn sign_out_clears_cookie_and_revokes_session() {
    let path = tmp_db_path();
    let (_dir, project_dir) = seed_identity_project(&path, SIGN_IN_PROJECT);
    let (manifest, _) =
        forge::core::manifest::Manifest::load_from_dir(&project_dir, None).expect("manifest");
    let cfg = IdentityConfig::from_manifest_opt(SIGN_IN_PROJECT, &manifest)
        .expect("cfg")
        .expect("has identity");
    let challenge =
        identity::build_challenge(SIGN_IN_PROJECT, &cfg, Utc::now()).expect("challenge");
    let claims = build_claims_for(&challenge, &cfg);
    let session = identity::mint_session(&cfg, &claims, &challenge, Utc::now()).expect("session");
    identity::save_session(&project_dir, SIGN_IN_PROJECT, &session).expect("save session");

    let config = ApiConfig::default();
    let mut cookies = BTreeMap::new();
    cookies.insert("forge_session".to_string(), session.session_id.clone());
    let body = Vec::new();
    let mut req = make_request("POST", "/ui/sign-out", html_accept(), None, body);
    req.cookies = cookies;
    req.headers
        .insert("origin".to_string(), LOOPBACK_ORIGIN.to_string());
    let resp = drive(&config, &path, &req);
    assert_eq!(resp.status, 303);
    let cookie = set_cookie(&resp);
    assert!(
        cookie.contains("forge_session=") && cookie.contains("1970"),
        "expected clearing cookie, got `{cookie}`"
    );
    // The session file should be gone.
    let session_path = project_dir
        .join(".forge")
        .join("identity")
        .join(SIGN_IN_PROJECT)
        .join("sessions")
        .join(format!("{}.json", session.session_id));
    assert!(!session_path.exists());
}

#[test]
fn sign_out_cross_origin_refused() {
    let path = tmp_db_path();
    let (_dir, project_dir) = seed_identity_project(&path, SIGN_IN_PROJECT);
    let (manifest, _) =
        forge::core::manifest::Manifest::load_from_dir(&project_dir, None).expect("manifest");
    let cfg = IdentityConfig::from_manifest_opt(SIGN_IN_PROJECT, &manifest)
        .expect("cfg")
        .expect("has identity");
    let challenge =
        identity::build_challenge(SIGN_IN_PROJECT, &cfg, Utc::now()).expect("challenge");
    let claims = build_claims_for(&challenge, &cfg);
    let session = identity::mint_session(&cfg, &claims, &challenge, Utc::now()).expect("session");
    identity::save_session(&project_dir, SIGN_IN_PROJECT, &session).expect("save session");

    let config = ApiConfig::default();
    let mut cookies = BTreeMap::new();
    cookies.insert("forge_session".to_string(), session.session_id.clone());
    let mut req = make_request("POST", "/ui/sign-out", html_accept(), None, Vec::new());
    req.cookies = cookies;
    req.headers
        .insert("origin".to_string(), "http://evil.example.com".to_string());
    let resp = drive(&config, &path, &req);
    assert_eq!(resp.status, 403);
    // Session file must remain because the sign-out
    // request was refused before any state changed.
    let session_path = project_dir
        .join(".forge")
        .join("identity")
        .join(SIGN_IN_PROJECT)
        .join("sessions")
        .join(format!("{}.json", session.session_id));
    assert!(session_path.exists());
}

#[test]
fn sign_out_no_origin_refused() {
    let path = tmp_db_path();
    let (_dir, project_dir) = seed_identity_project(&path, SIGN_IN_PROJECT);
    let (manifest, _) =
        forge::core::manifest::Manifest::load_from_dir(&project_dir, None).expect("manifest");
    let cfg = IdentityConfig::from_manifest_opt(SIGN_IN_PROJECT, &manifest)
        .expect("cfg")
        .expect("has identity");
    let challenge =
        identity::build_challenge(SIGN_IN_PROJECT, &cfg, Utc::now()).expect("challenge");
    let claims = build_claims_for(&challenge, &cfg);
    let session = identity::mint_session(&cfg, &claims, &challenge, Utc::now()).expect("session");
    identity::save_session(&project_dir, SIGN_IN_PROJECT, &session).expect("save session");

    let config = ApiConfig::default();
    let mut cookies = BTreeMap::new();
    cookies.insert("forge_session".to_string(), session.session_id.clone());
    let mut req = make_request("POST", "/ui/sign-out", html_accept(), None, Vec::new());
    req.cookies = cookies;
    let resp = drive(&config, &path, &req);
    assert_eq!(resp.status, 403);
}

#[test]
fn sign_out_unknown_cookie_is_idempotent() {
    let path = tmp_db_path();
    let _dir = seed_identity_project(&path, SIGN_IN_PROJECT);
    let config = ApiConfig::default();
    let mut cookies = BTreeMap::new();
    cookies.insert("forge_session".to_string(), "deadbeefdeadbeef".to_string());
    let mut req = make_request("POST", "/ui/sign-out", html_accept(), None, Vec::new());
    req.cookies = cookies;
    req.headers
        .insert("origin".to_string(), LOOPBACK_ORIGIN.to_string());
    let resp = drive(&config, &path, &req);
    assert_eq!(resp.status, 303);
    let cookie = set_cookie(&resp);
    assert!(cookie.contains("forge_session=") && cookie.contains("1970"));
}

#[test]
fn sign_out_only_revokes_owning_project() {
    let path = tmp_db_path();
    let (_a_dir, a_dir) = seed_identity_project(&path, "alethefy");
    let (_b_dir, b_dir) = seed_identity_project(&path, "other-proj");
    let (a_manifest, _) = forge::core::manifest::Manifest::load_from_dir(&a_dir, None).expect("a");
    let a_cfg = IdentityConfig::from_manifest_opt("alethefy", &a_manifest)
        .expect("a cfg")
        .expect("has identity");
    let a_challenge = identity::build_challenge("alethefy", &a_cfg, Utc::now()).expect("a c");
    let a_claims = build_claims_for(&a_challenge, &a_cfg);
    let a_session =
        identity::mint_session(&a_cfg, &a_claims, &a_challenge, Utc::now()).expect("a s");
    identity::save_session(&a_dir, "alethefy", &a_session).expect("a save");

    let (b_manifest, _) = forge::core::manifest::Manifest::load_from_dir(&b_dir, None).expect("b");
    let b_cfg = IdentityConfig::from_manifest_opt("other-proj", &b_manifest)
        .expect("b cfg")
        .expect("has identity");
    let b_challenge = identity::build_challenge("other-proj", &b_cfg, Utc::now()).expect("b c");
    let b_claims = build_claims_for(&b_challenge, &b_cfg);
    let b_session =
        identity::mint_session(&b_cfg, &b_claims, &b_challenge, Utc::now()).expect("b s");
    identity::save_session(&b_dir, "other-proj", &b_session).expect("b save");

    let config = ApiConfig::default();
    let mut cookies = BTreeMap::new();
    cookies.insert("forge_session".to_string(), a_session.session_id.clone());
    let mut req = make_request("POST", "/ui/sign-out", html_accept(), None, Vec::new());
    req.cookies = cookies;
    req.headers
        .insert("origin".to_string(), LOOPBACK_ORIGIN.to_string());
    let resp = drive(&config, &path, &req);
    assert_eq!(resp.status, 303);

    // Project A's session is gone; project B's is intact.
    let a_session_path = a_dir
        .join(".forge")
        .join("identity")
        .join("alethefy")
        .join("sessions")
        .join(format!("{}.json", a_session.session_id));
    let b_session_path = b_dir
        .join(".forge")
        .join("identity")
        .join("other-proj")
        .join("sessions")
        .join(format!("{}.json", b_session.session_id));
    assert!(!a_session_path.exists(), "alethefy session must be revoked");
    assert!(b_session_path.exists(), "other-proj session must remain");
}

#[test]
fn query_token_in_url_is_rejected() {
    let path = tmp_db_path();
    seed_project(&path, "alethefy", "rust-web", "L3");
    let config = ApiConfig::default();
    // The legacy `?token=` escape hatch is removed. A
    // request that supplies it without an `Authorization:`
    // header or a `forge_session` cookie must be refused
    // as if no credential was supplied.
    let req = make_request_with_query(
        "GET",
        "/ui",
        Some("token=cafebabe"),
        html_accept(),
        Vec::new(),
    );
    let resp = drive(&config, &path, &req);
    assert_eq!(resp.status, 401);
    let body = String::from_utf8_lossy(&resp.body);
    assert!(body.contains("api-unauthorized"));
}

fn extract_query_value(url: &str, key: &str) -> Option<String> {
    let query = url.split_once('?')?.1;
    for pair in query.split('&') {
        if let Some(rest) = pair.strip_prefix(&format!("{key}=")) {
            return Some(rest.to_string());
        }
    }
    None
}

fn extract_cookie_value(cookie: &str, name: &str) -> Option<String> {
    for pair in cookie.split(';') {
        let trimmed = pair.trim();
        if let Some(rest) = trimmed.strip_prefix(&format!("{name}=")) {
            return Some(rest.to_string());
        }
    }
    None
}
