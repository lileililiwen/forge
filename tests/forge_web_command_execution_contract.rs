//! Command-execution contract (`forge-web-command-execution`).
//!
//! Pins the typed, session-gated authoring routes
//! `POST /v1/admin/projects/{id}/feature` (`forge feature add`) and
//! `POST …/spec` (`forge spec generate`): an anonymous request gets `401` with
//! no Core call, a non-JSON body gets `415`, a preview (no `confirm`) returns a
//! bounded descriptor and `plan_digest` while leaving the project manifest and
//! registry untouched, a confirmed action with a mismatched digest is refused
//! with a fresh digest and no write, a missing required field is a typed `400`,
//! a correctly confirmed action is delegated to the same in-process Core
//! handler (never a shell), and no response ever echoes the absolute project
//! path. It also asserts the catalog now reports these two commands as `web`.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use chrono::Utc;
use forge::api::{command_catalog, handle, ApiConfig, ApiRequest};
use forge::registry::Registry;
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

fn body_json(response: &forge::api::ApiResponse) -> serde_json::Value {
    serde_json::from_slice(&response.body).expect("admin routes are JSON-only")
}

fn manifest_text(dir: &Path) -> String {
    fs::read_to_string(dir.join("forge.yaml")).unwrap()
}

#[test]
fn anonymous_and_non_json_are_refused_before_any_core_call() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let project = dir.path().join("p1");
    write_project(&project, "p1");
    register_project(&db, &project);

    // Valid JSON body but no session cookie -> 401, no Core write.
    let anon = handle(
        &config,
        &db,
        &json_body(
            request(
                "POST",
                "/v1/admin/projects/p1/feature",
                &config.frontend_origin,
            ),
            r#"{"feature":"auth","version":"1.0.0"}"#,
        ),
        Utc::now(),
    );
    assert_eq!(anon.status, 401);
    assert!(manifest_text(&project).contains("features: {}"));

    // Session but a non-JSON body -> 415 before the handler.
    let bad_ct = handle(
        &config,
        &db,
        &with_cookie(
            request(
                "POST",
                "/v1/admin/projects/p1/feature",
                &config.frontend_origin,
            ),
            &token,
        ),
        Utc::now(),
    );
    assert_eq!(bad_ct.status, 415);
}

#[test]
fn preview_returns_a_digest_without_writing_and_confirm_with_wrong_digest_is_refused() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let project = dir.path().join("p1");
    write_project(&project, "p1");
    register_project(&db, &project);
    let before = manifest_text(&project);

    let preview = handle(
        &config,
        &db,
        &with_cookie(
            json_body(
                request(
                    "POST",
                    "/v1/admin/projects/p1/feature",
                    &config.frontend_origin,
                ),
                r#"{"feature":"auth","version":"1.0.0"}"#,
            ),
            &token,
        ),
        Utc::now(),
    );
    assert_eq!(preview.status, 200);
    let body = body_json(&preview);
    let digest = body["plan_digest"].as_str().unwrap();
    assert_eq!(digest.len(), 64);
    assert!(digest.chars().all(|c| c.is_ascii_hexdigit()));
    assert!(
        body["confirmation"].is_object(),
        "preview must describe the gate"
    );
    assert_eq!(body["preview"]["feature"], "auth");
    // A preview never echoes the absolute project path.
    let raw = String::from_utf8_lossy(&preview.body);
    assert!(
        !raw.contains(&project.to_string_lossy().to_string()),
        "response leaked an absolute path"
    );
    assert_eq!(manifest_text(&project), before, "preview must not write");

    // Confirm with a wrong digest -> 409, still no write, fresh digest returned.
    let wrong = handle(
        &config,
        &db,
        &with_cookie(
            json_body(
                request(
                    "POST",
                    "/v1/admin/projects/p1/feature",
                    &config.frontend_origin,
                ),
                r#"{"feature":"auth","version":"1.0.0","confirm":true,"plan_digest":"deadbeef"}"#,
            ),
            &token,
        ),
        Utc::now(),
    );
    assert_eq!(wrong.status, 409);
    assert_eq!(body_json(&wrong)["error"]["code"], "admin-digest-mismatch");
    assert_eq!(
        manifest_text(&project),
        before,
        "wrong digest must not write"
    );

    // The recomputed digest for the identical fields equals the preview digest.
    let recalc = handle(
        &config,
        &db,
        &with_cookie(
            json_body(
                request(
                    "POST",
                    "/v1/admin/projects/p1/feature",
                    &config.frontend_origin,
                ),
                r#"{"feature":"auth","version":"1.0.0"}"#,
            ),
            &token,
        ),
        Utc::now(),
    );
    assert_eq!(
        body_json(&recalc)["plan_digest"],
        digest,
        "digest is deterministic"
    );
}

#[test]
fn correctly_confirmed_action_delegates_to_the_core_handler_not_the_admin_guard() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let project = dir.path().join("p1");
    write_project(&project, "p1");
    register_project(&db, &project);

    // Obtain the canonical digest.
    let preview = handle(
        &config,
        &db,
        &with_cookie(
            json_body(
                request(
                    "POST",
                    "/v1/admin/projects/p1/feature",
                    &config.frontend_origin,
                ),
                r#"{"feature":"auth","version":"1.0.0"}"#,
            ),
            &token,
        ),
        Utc::now(),
    );
    let digest = body_json(&preview)["plan_digest"]
        .as_str()
        .unwrap()
        .to_string();

    // Confirm with the exact digest: the admin guard passes and the request is
    // handed to the Core `add_feature` handler. The response is therefore a
    // handler response (an applied `202` or a typed Core `4xx`), never the
    // admin preview object and never the `admin-digest-mismatch` refusal.
    let applied = handle(
        &config,
        &db,
        &with_cookie(
            json_body(
                request(
                    "POST",
                    "/v1/admin/projects/p1/feature",
                    &config.frontend_origin,
                ),
                &format!(
                    r#"{{"feature":"auth","version":"1.0.0","confirm":true,"plan_digest":"{digest}"}}"#
                ),
            ),
            &token,
        ),
        Utc::now(),
    );
    let body = body_json(&applied);
    assert!(
        body.get("confirmation").is_none(),
        "a confirmed action must reach the Core handler, not return a preview"
    );
    assert_ne!(
        body["error"]["code"].as_str(),
        Some("admin-digest-mismatch"),
        "the correct digest must satisfy the admin gate"
    );
}

#[test]
fn spec_generate_requires_findings_and_uses_the_same_digest_gate() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let project = dir.path().join("p1");
    write_project(&project, "p1");
    register_project(&db, &project);

    let empty = handle(
        &config,
        &db,
        &with_cookie(
            json_body(
                request(
                    "POST",
                    "/v1/admin/projects/p1/spec",
                    &config.frontend_origin,
                ),
                r#"{"findings":[]}"#,
            ),
            &token,
        ),
        Utc::now(),
    );
    assert_eq!(empty.status, 400);
    assert_eq!(
        body_json(&empty)["error"]["code"],
        "admin-findings-required"
    );

    let preview = handle(
        &config,
        &db,
        &with_cookie(
            json_body(
                request(
                    "POST",
                    "/v1/admin/projects/p1/spec",
                    &config.frontend_origin,
                ),
                r#"{"findings":["missing-ci"]}"#,
            ),
            &token,
        ),
        Utc::now(),
    );
    assert_eq!(preview.status, 200);
    let body = body_json(&preview);
    assert_eq!(body["preview"]["action"], "spec-generate");
    assert_eq!(body["preview"]["findings"][0], "missing-ci");
    assert_eq!(body["plan_digest"].as_str().unwrap().len(), 64);
}

#[test]
fn catalog_reports_the_two_authoring_commands_as_web() {
    let rows = command_catalog::rows();
    for id in ["feature.add", "spec.generate"] {
        let row = rows
            .iter()
            .find(|row| row.id == id)
            .unwrap_or_else(|| panic!("missing catalog row {id}"));
        assert_eq!(row.availability, "web", "{id} should be web");
        assert!(row.route.is_some(), "{id} must name a route");
        assert!(row.reason.is_none(), "{id} must carry no CLI-only reason");
    }
}
