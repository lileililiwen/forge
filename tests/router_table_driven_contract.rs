//! Mutation contract tests for the table-driven route dispatch
//! (`forge-api-router-table-driven`).
//!
//! The refactor moves three cross-cutting `match` arms in
//! `src/api/router.rs` (the `required_permission` table, the admin
//! pre-dispatch `matches!` arm, and the final-dispatch fallthrough)
//! into per-variant methods in `src/api/router_classify.rs`. The
//! public API is unchanged; the wire shape is byte-identical for
//! every existing variant.
//!
//! These tests pin the new dispatch surface through the public
//! `forge::api::handle` entry so a future change that breaks the
//! classification tables is caught by the test suite:
//! - **Permission table:** the `admin:access` cluster is exercised
//!   through `handle` and the response is asserted to carry the
//!   same `401 api-unauthorized` envelope for anonymous callers as
//!   the pre-refactor code produced.
//! - **Admin-dispatch table:** the new `Route::AdminProjectGateRun`
//!   / `Route::AdminProjectGateStatus` variants added by the
//!   previous subagent's `forge-web-gate-controls` are dispatched
//!   through `handle` and the response is asserted to carry the
//!   typed envelope only the `super::admin::handle` path produces.
//!   If `is_admin_dispatch` were broken (returned `false` for these
//!   variants), the request would fall through to the
//!   bearer-handler dispatch and the response envelope would
//!   differ — the test would fail, proving the classification
//!   table is the load-bearing surface.
//! - **Exhaustiveness:** the `Route::AdminOptions` short-circuit
//!   (the only admin variant NOT covered by `is_admin_dispatch`)
//!   is exercised through `handle` and the response is asserted to
//!   carry the CORS preflight `204` envelope.
//! - **Project-not-found:** an admin route targeting an
//!   unmanaged project id is dispatched through `handle` and the
//!   response is asserted to carry the typed
//!   `admin-project-not-managed` envelope the `super::admin::handle`
//!   path produces. If the dispatch were broken the request would
//!   fall through to `not_found()` with a different envelope.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use forge::api::{handle, ApiConfig, ApiRequest, ApiResponse};
use forge::registry::Registry;

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
    forge::identity::global::setup(db, "operator@example.test", "a-long-test-password").unwrap();
    let mut login = request("POST", "/v1/admin/session", &config.frontend_origin);
    login = json_body(
        login,
        br#"{"email":"operator@example.test","password":"a-long-test-password"}"#,
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
            "schema: 1\nproject:\n  id: {id}\n  name: Test {id}\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\n"
        ),
    )
    .unwrap();
}

fn fixture() -> (tempfile::TempDir, PathBuf, ApiConfig) {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let alpha = tmp.path().join("alpha");
    write_project(&alpha, "alpha");
    {
        let mut registry = Registry::open(&db).unwrap();
        registry.register(&alpha, None).unwrap();
    }
    let config = ApiConfig::default();
    (tmp, db, config)
}

fn get(config: &ApiConfig, db: &Path, token: &str, path: &str) -> ApiResponse {
    let req = request("GET", path, &config.frontend_origin);
    handle(config, db, &with_cookie(req, token), Utc::now())
}

fn body_text(response: &ApiResponse) -> String {
    String::from_utf8_lossy(&response.body).to_string()
}

// ----- Dispatch-table coverage -----------------------------------------

#[test]
fn healthz_does_not_require_a_session() {
    // Permission table — `Route::Healthz` requires no session and
    // no extra permission; the public route answers 200 directly.
    let (_tmp, db, config) = fixture();
    let response = handle(
        &config,
        &db,
        &request("GET", "/healthz", &config.frontend_origin),
        Utc::now(),
    );
    assert_eq!(response.status, 200, "/healthz must answer 200");
    let body = body_text(&response);
    assert!(body.contains("\"status\":\"ok\""));
    assert!(body.contains("\"contract\":"));
}

#[test]
fn admin_routes_are_anonymous_401() {
    // Admin-dispatch table — every `Route::Admin*` variant is
    // short-circuited to `super::admin::handle`, which enforces
    // the session gate and returns `401 api-unauthorized` for an
    // anonymous request. The test covers the gate-status route
    // added by the previous subagent's `forge-web-gate-controls`
    // to prove the new gate routes are still dispatched
    // correctly: the response is the typed admin envelope, not the
    // `route-not-found` envelope the bearer-handler fallthrough
    // would produce.
    let (_tmp, db, config) = fixture();
    for path in [
        "/v1/admin/projects/alpha/gate/status",
        "/v1/admin/projects/alpha",
        "/v1/admin/projects",
        "/v1/admin/identity/change-password",
        "/v1/admin/session",
    ] {
        let response = handle(
            &config,
            &db,
            &request("GET", path, &config.frontend_origin),
            Utc::now(),
        );
        assert_eq!(response.status, 401, "{path} must be 401 anonymous");
        assert!(
            body_text(&response).contains("api-unauthorized"),
            "{path} must carry the api-unauthorized envelope"
        );
    }
}

#[test]
fn admin_bearer_mutations_are_401_for_anonymous_callers() {
    // Permission table — every bearer-handler variant in the
    // `admin:access` cluster (CreateProject, AddFeature,
    // ApplyDeployment, the share cluster, the interest cluster,
    // the delivery cluster) demands a session and the
    // `admin:access` permission. An anonymous request is rejected
    // with `401 api-unauthorized` before any handler runs.
    let (_tmp, db, config) = fixture();
    for path in [
        "/v1/projects",
        "/v1/projects/alpha/deployments",
        "/v1/projects/alpha/share",
        "/v1/share/approve",
        "/v1/projects/alpha/interest",
    ] {
        let response = handle(
            &config,
            &db,
            &request("POST", path, &config.frontend_origin),
            Utc::now(),
        );
        assert_eq!(response.status, 401, "{path} must be 401 anonymous");
        assert!(
            body_text(&response).contains("api-unauthorized"),
            "{path} must carry the api-unauthorized envelope"
        );
    }
}

#[test]
fn admin_options_short_circuits_to_preflight() {
    // Exhaustiveness — `Route::AdminOptions` is the only admin
    // variant NOT covered by `is_admin_dispatch`; it short-circuits
    // to `super::admin::handle_preflight` above the dispatch and
    // returns a CORS preflight `204`. The test pins the
    // short-circuit so a future `is_admin_dispatch` change that
    // includes `Route::AdminOptions` is caught: if
    // `is_admin_dispatch` were to return `true` for
    // `Route::AdminOptions` the request would reach
    // `super::admin::handle` and the response would not be the
    // CORS preflight `204`.
    let (_tmp, db, config) = fixture();
    for path in [
        "/v1/admin/projects",
        "/v1/admin/projects/alpha",
        "/v1/admin/projects/alpha/gate/status",
    ] {
        let response = handle(
            &config,
            &db,
            &request("OPTIONS", path, &config.frontend_origin),
            Utc::now(),
        );
        assert_eq!(response.status, 204, "{path} OPTIONS must be 204");
    }
}

#[test]
fn unknown_project_id_is_a_typed_400_through_admin_dispatch() {
    // Project-not-found — an admin route targeting an unmanaged
    // project id is dispatched through `super::admin::handle` (via
    // `is_admin_dispatch`) and the response is the typed
    // `400 admin-project-not-managed` envelope. If the dispatch
    // were broken the request would fall through to the bearer
    // dispatch and the response would not carry the typed
    // envelope; the test would fail, proving the classification
    // table is load-bearing.
    let (_tmp, db, config) = fixture();
    let token = login_token(&config, &db);
    let response = get(
        &config,
        &db,
        &token,
        "/v1/admin/projects/nonexistent/gate/status",
    );
    assert_eq!(
        response.status, 400,
        "unknown project id must be 400 admin-project-not-managed"
    );
    let body = body_text(&response);
    assert!(
        body.contains("admin-project-not-managed"),
        "body must carry the typed envelope: {body}"
    );
}

// ----- Mutation contract: gate-run dispatch is load-bearing ------------

#[test]
fn gate_run_dispatch_reaches_admin_handle_with_a_valid_session() {
    // The mutation contract: `Route::AdminProjectGateRun` is
    // classified as `is_admin_dispatch == true` and reaches
    // `super::admin::handle`. A POST without `application/json`
    // returns the typed `415 admin-content-type-required` envelope
    // the `super::admin::handle_preflight` Content-Type gate
    // produces — the same envelope the previous subagent's
    // `gate.rs` test pins. If `is_admin_dispatch` returned `false`
    // for this variant the request would fall through to the
    // bearer-handler dispatch, the body would not carry the
    // gate-specific 415 envelope, and the test would fail.
    let (_tmp, db, config) = fixture();
    let token = login_token(&config, &db);
    let req = with_cookie(
        request(
            "POST",
            "/v1/admin/projects/alpha/gate/run",
            &config.frontend_origin,
        ),
        &token,
    );
    let response = handle(&config, &db, &req, Utc::now());
    assert_eq!(
        response.status, 415,
        "gate/run must reach the admin handler and return 415 for a non-JSON body"
    );
    let body = body_text(&response);
    assert!(
        body.contains("admin-content-type-required"),
        "body must carry the typed 415 envelope: {body}"
    );
}
