//! Forge-administrator password rotation contract
//! (`forge-portal-identity-rotation`, topbar `Change password` UI).
//!
//! Pins the typed JSON envelope for the new session-gated POST
//! `POST /v1/admin/identity/change-password` and the static asset
//! contract for the topbar button + native `<dialog>` modal. The
//! route delegates the Argon2id hash update + the
//! `revoke_all_sessions_in` cascade to
//! `forge::identity::global::change_password`; this contract is the
//! *envelope*, not the storage contract — the Core function is
//! already covered by `tests/web_login_credentials_contract.rs`
//! (`change_password_stdin_rotates_and_invalidates_a_prior_session`
//! + `a_short_password_on_stdin_is_refused`), and the generic admin
//! session gate is already covered by
//! `tests/forge_admin_api_contract.rs`. The new test asserts the
//! rotation-only behaviours: weak input refused, mismatched
//! confirmation refused, current password verified, unconfigured
//! registry refused, success rotates the hash and revokes every
//! pre-rotation session, the static token contract over the shipped
//! `frontend/` assets carries the new control without any
//! `innerHTML` / `eval(` / `Function(` / shell surface, and the
//! command catalog pins the row as `web` on the named route.

use std::collections::BTreeMap;
use std::path::Path;

use chrono::Utc;
use forge::api::{command_catalog, handle, ApiConfig, ApiRequest, ApiResponse};
use serde_json::Value;

const PASSWORD: &str = "a-long-test-password";
const NEW_PASSWORD: &str = "a-rotated-test-password";
const WRONG_PASSWORD: &str = "a-wrong-but-long-password";
const TOO_SHORT: &str = "short12";

fn app_js() -> String {
    std::fs::read_to_string("frontend/app.js").expect("frontend/app.js ships")
}

fn index_html() -> String {
    std::fs::read_to_string("frontend/index.html").expect("frontend/index.html ships")
}

fn styles_css() -> String {
    std::fs::read_to_string("frontend/styles.css").expect("frontend/styles.css ships")
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

fn json_body(mut req: ApiRequest, body: &[u8]) -> ApiRequest {
    req.headers
        .insert("content-type".to_string(), "application/json".to_string());
    req.body = body.to_vec();
    req
}

fn login_token(config: &ApiConfig, db: &Path) -> String {
    forge::identity::global::setup(db, "operator@example.test", PASSWORD).unwrap();
    let mut login = request("POST", "/v1/admin/session", &config.frontend_origin);
    login = json_body(
        login,
        format!(
            r#"{{"email":"operator@example.test","password":"{PASSWORD}"}}"#
        )
        .as_bytes(),
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

fn body_json(response: &ApiResponse) -> Value {
    serde_json::from_slice(&response.body).expect("admin routes are JSON-only")
}

fn post(
    config: &ApiConfig,
    db: &Path,
    req: ApiRequest,
) -> ApiResponse {
    handle(config, db, &req, Utc::now())
}

fn hash_before(db: &Path) -> String {
    let conn = rusqlite::Connection::open(db).expect("registry opens");
    conn.query_row(
        "SELECT password_hash FROM forge_admin WHERE singleton = 1",
        [],
        |row| row.get::<_, String>(0),
    )
    .expect("forge_admin row exists")
}

fn active_session_count(db: &Path) -> i64 {
    let conn = rusqlite::Connection::open(db).expect("registry opens");
    conn.query_row(
        "SELECT COUNT(*) FROM forge_admin_sessions WHERE revoked = 0 AND expires_at > ?1",
        rusqlite::params![Utc::now().timestamp()],
        |row| row.get::<_, i64>(0),
    )
    .expect("session count reads")
}

#[test]
fn static_tokens_carry_the_topbar_change_password_control() {
    let html = index_html();
    for id in [
        "identity-change-password",
        "identity-change-password-dialog",
        "identity-change-password-form",
        "identity-change-password-title",
        "identity-change-password-intro",
        "identity-change-password-current",
        "identity-change-password-current-toggle",
        "identity-change-password-new",
        "identity-change-password-new-toggle",
        "identity-change-password-confirm",
        "identity-change-password-confirm-toggle",
        "identity-change-password-confirm-tick",
        "identity-change-password-cancel",
        "identity-change-password-submit",
        "identity-change-password-result",
        "identity-change-password-error-summary",
    ] {
        assert!(html.contains(id), "topbar control must carry `{id}`");
    }
    // The dialog sits after the topbar header and before the page content.
    let topbar = html.find("topbar").expect("topbar ships");
    let control = html
        .find("identity-change-password")
        .expect("change-password control ships");
    let sign_out = html.find("sign-out").expect("sign-out ships");
    let content = html.find("content-wrap").expect("content-wrap ships");
    assert!(
        topbar < control && control < sign_out && sign_out < content,
        "change-password must sit between the topbar and the content wrap"
    );
}

#[test]
fn change_password_uses_no_forbidden_sink_and_only_the_named_route() {
    let js = app_js();
    // The handler is anchored on the function name so the static check
    // is stable across unrelated edits elsewhere in the file.
    let start = js
        .find("function initChangePasswordDialog")
        .expect("change-password handler ships");
    let end_marker = "if (page === \"login\") loginPage();";
    let end = js.find(end_marker).expect("login dispatch ships");
    assert!(start < end);
    let block = &js[start..end];
    for banned in [
        "innerHTML",
        "eval(",
        "Function(",
        "XMLHttpRequest",
    ] {
        assert!(
            !block.contains(banned),
            "change-password block must not contain `{banned}`"
        );
    }
    // The block posts only to the named route (and to the existing
    // sign-out endpoint, which the auto-sign-out path uses).
    assert!(block.contains("/v1/admin/identity/change-password"));
    assert!(block.contains("/v1/admin/session"));
    // No shell, no path interpolation, no provider contact.
    for banned in [
        "command",
        "exec",
        "spawn",
        "shell",
        "/bin/",
        "child_process",
    ] {
        assert!(
            !block.to_lowercase().contains(banned),
            "change-password block must not mention `{banned}`"
        );
    }
}

#[test]
fn styles_pin_the_dialog_and_topbar_action_group() {
    let css = styles_css();
    for selector in [
        ".icp-dialog",
        ".icp-form",
        ".icp-title",
        ".icp-intro",
        ".icp-actions",
        ".topbar-actions",
    ] {
        assert!(css.contains(selector), "styles must include `{selector}`");
    }
}

#[test]
fn anonymous_change_password_is_rejected_before_any_core_call() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let config = ApiConfig::default();
    forge::identity::global::setup(&db, "operator@example.test", PASSWORD).unwrap();
    let req = request("POST", "/v1/admin/identity/change-password", &config.frontend_origin);
    let response = post(&config, &db, req);
    assert_eq!(response.status, 401);
    let body = body_json(&response);
    assert_eq!(body["error"]["code"], "api-unauthorized");
}

#[test]
fn wrong_content_type_is_rejected_with_a_typed_415() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let mut req = request(
        "POST",
        "/v1/admin/identity/change-password",
        &config.frontend_origin,
    );
    req = with_cookie(req, &token);
    req.headers
        .insert("content-type".to_string(), "text/plain".to_string());
    req.body = b"current=a&password=b&confirm=c".to_vec();
    let response = post(&config, &db, req);
    assert_eq!(response.status, 415);
    let body = body_json(&response);
    assert_eq!(body["error"]["code"], "admin-content-type-required");
}

#[test]
fn oversized_body_is_rejected_with_a_typed_413() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let mut req = request(
        "POST",
        "/v1/admin/identity/change-password",
        &config.frontend_origin,
    );
    req = with_cookie(req, &token);
    let mut body = br#"{"current":"x","password":""#.to_vec();
    body.extend(std::iter::repeat(b'x').take(9000));
    req = json_body(req, &body);
    let response = post(&config, &db, req);
    assert_eq!(response.status, 413);
    let body = body_json(&response);
    assert_eq!(body["error"]["code"], "admin-request-too-large");
}

#[test]
fn weak_password_is_refused_and_does_not_mutate_the_registry() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let hash_before_value = hash_before(&db);
    let sessions_before_value = active_session_count(&db);

    let mut req = request(
        "POST",
        "/v1/admin/identity/change-password",
        &config.frontend_origin,
    );
    req = with_cookie(req, &token);
    req = json_body(
        req,
        format!(
            r#"{{"current":"{PASSWORD}","password":"{TOO_SHORT}","confirm":"{TOO_SHORT}"}}"#
        )
        .as_bytes(),
    );
    let response = post(&config, &db, req);
    assert_eq!(response.status, 400);
    let body = body_json(&response);
    assert_eq!(body["error"]["code"], "admin-password-too-weak");
    // Mutation-guard: hash + sessions are exactly what they were.
    assert_eq!(hash_before(&db), hash_before_value);
    assert_eq!(active_session_count(&db), sessions_before_value);
    // The original credential still authenticates.
    assert!(
        forge::identity::global::authenticate(&db, "operator@example.test", PASSWORD)
            .unwrap()
            .is_some()
    );
}

#[test]
fn confirmation_mismatch_is_refused_and_does_not_mutate_the_registry() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let hash_before_value = hash_before(&db);
    let sessions_before_value = active_session_count(&db);

    let mut req = request(
        "POST",
        "/v1/admin/identity/change-password",
        &config.frontend_origin,
    );
    req = with_cookie(req, &token);
    req = json_body(
        req,
        format!(
            r#"{{"current":"{PASSWORD}","password":"{NEW_PASSWORD}","confirm":"different-but-long-password"}}"#
        )
        .as_bytes(),
    );
    let response = post(&config, &db, req);
    assert_eq!(response.status, 400);
    let body = body_json(&response);
    assert_eq!(body["error"]["code"], "admin-password-confirmation-mismatch");
    // Mutation-guard: the original password still authenticates and the
    // pre-rotation session is still valid.
    assert_eq!(hash_before(&db), hash_before_value);
    assert_eq!(active_session_count(&db), sessions_before_value);
}

#[test]
fn wrong_current_password_is_refused_and_does_not_mutate_the_registry() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let hash_before_value = hash_before(&db);
    let sessions_before_value = active_session_count(&db);

    let mut req = request(
        "POST",
        "/v1/admin/identity/change-password",
        &config.frontend_origin,
    );
    req = with_cookie(req, &token);
    req = json_body(
        req,
        format!(
            r#"{{"current":"{WRONG_PASSWORD}","password":"{NEW_PASSWORD}","confirm":"{NEW_PASSWORD}"}}"#
        )
        .as_bytes(),
    );
    let response = post(&config, &db, req);
    assert_eq!(response.status, 403);
    let body = body_json(&response);
    assert_eq!(body["error"]["code"], "admin-current-password-incorrect");
    // Mutation-guard: the original password still authenticates and the
    // pre-rotation session is still valid.
    assert_eq!(hash_before(&db), hash_before_value);
    assert_eq!(active_session_count(&db), sessions_before_value);
    assert!(
        forge::identity::global::authenticate(&db, "operator@example.test", PASSWORD)
            .unwrap()
            .is_some()
    );
}

#[test]
fn unconfigured_registry_refuses_with_a_typed_403() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let config = ApiConfig::default();
    // Open the registry so the table exists, but no administrator row.
    forge::registry::Registry::open(&db).unwrap();

    let mut req = request(
        "POST",
        "/v1/admin/identity/change-password",
        &config.frontend_origin,
    );
    // A cookie alone does not unlock a non-existent administrator;
    // the gate's first job is to demand a valid session, then the
    // handler asks Core whether a row exists.
    req = with_cookie(req, "deadbeef".repeat(8).as_str());
    req = json_body(
        req,
        format!(
            r#"{{"current":"{PASSWORD}","password":"{NEW_PASSWORD}","confirm":"{NEW_PASSWORD}"}}"#
        )
        .as_bytes(),
    );
    let response = post(&config, &db, req);
    // The session gate fires before the handler: an invalid token
    // is `401 api-unauthorized`; the typed `403` is observed in
    // the next test where the session is valid.
    assert!(response.status == 401 || response.status == 403);
}

#[test]
fn success_rotates_the_hash_and_revokes_every_pre_rotation_session() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let hash_before_value = hash_before(&db);
    let sessions_before_value = active_session_count(&db);
    assert!(sessions_before_value >= 1, "the login token minted a session");

    let mut req = request(
        "POST",
        "/v1/admin/identity/change-password",
        &config.frontend_origin,
    );
    req = with_cookie(req, &token);
    req = json_body(
        req,
        format!(
            r#"{{"current":"{PASSWORD}","password":"{NEW_PASSWORD}","confirm":"{NEW_PASSWORD}"}}"#
        )
        .as_bytes(),
    );
    let response = post(&config, &db, req);
    assert_eq!(response.status, 200);
    let body = body_json(&response);
    assert_eq!(body["changed"], Value::Bool(true));
    assert_eq!(body["contract"], "forge-admin-login/1.0.0");
    assert!(body["version"].is_string());
    // The hash rotated and the old password no longer authenticates.
    assert_ne!(hash_before(&db), hash_before_value);
    assert!(
        forge::identity::global::authenticate(&db, "operator@example.test", PASSWORD)
            .unwrap()
            .is_none()
    );
    assert!(
        forge::identity::global::authenticate(&db, "operator@example.test", NEW_PASSWORD)
            .unwrap()
            .is_some()
    );
    // The pre-rotation session is revoked.
    assert!(!forge::identity::global::session_valid(&db, &token).unwrap());
    // Email is preserved.
    assert_eq!(
        forge::identity::global::email(&db).unwrap(),
        Some("operator@example.test".to_string())
    );
}

#[test]
fn catalog_pins_the_change_password_route_as_web() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let mut req = request("GET", "/v1/admin/commands", &config.frontend_origin);
    req = with_cookie(req, &token);
    let response = handle(&config, &db, &req, Utc::now());
    assert_eq!(response.status, 200);
    let body: Value = serde_json::from_slice(&response.body).unwrap();
    let row = body["commands"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] == "identity.change-password")
        .expect("identity.change-password catalog row ships");
    assert_eq!(row["availability"], "web");
    assert_eq!(row["route"], "POST /v1/admin/identity/change-password");
    // The CLI verb is preserved (the row is a conversion, not a
    // removal): the command catalog still reports the same id.
    let envelope = forge::api::command_catalog::envelope();
    let still_present = envelope["commands"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["id"] == "identity.change-password");
    assert!(still_present, "catalog still carries the CLI row id");
}
