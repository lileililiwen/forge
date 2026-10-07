//! Project-actions execution contract (`forge-web-project-actions`).
//!
//! Pins the three new typed, session-gated lifecycle write routes —
//! `POST /v1/admin/projects/{id}/feature/remove` (`forge feature remove`),
//! `POST …/feature/upgrade` (`forge feature upgrade`) and
//! `POST …/spec/apply` (`forge spec apply`) — to the same security boundary the
//! shipped `feature add` / `spec generate` routes honour: an anonymous JSON
//! request is refused with `401` before any Core call, a session request without
//! a JSON body is refused with `415`, a hostile project id is a typed `400` and
//! an unmanaged id a `404`, a preview returns a path-free descriptor and a
//! 64-hex `plan_digest` while leaving the manifest byte-identical, a confirmed
//! action with a stale or mismatched digest is refused with `409` and no write,
//! a missing required field is a typed `400` whose body never echoes the input,
//! a correctly confirmed action is delegated to the in-process Core handler
//! (never a preview and never the mismatch refusal), and no response ever
//! serializes the absolute project path. It also asserts the catalog now reports
//! these three commands as `web`, each with a well-formed `execution` block.

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

/// A project with one installed feature so a remove/upgrade has real target
/// state; a preview of either must leave this manifest byte-identical.
fn write_project_with_feature(dir: &Path, id: &str) {
    fs::create_dir_all(dir).unwrap();
    fs::write(
        dir.join("forge.yaml"),
        format!(
            "schema: 1\nproject:\n  id: {id}\n  name: Test {id}\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\nfeatures:\n  auth: \"0.0.9\"\n"
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

/// (route suffix, confirm-required JSON body with a valid field set) for each of
/// the three new lifecycle write commands.
const LIFECYCLE: &[(&str, &str)] = &[
    ("feature/remove", r#"{"feature":"auth"}"#),
    ("feature/upgrade", r#"{"feature":"auth","version":"1.0.0"}"#),
    ("spec/apply", r#"{"findings":["missing-ci"]}"#),
];

#[test]
fn anonymous_and_non_json_are_refused_before_any_core_call() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let project = dir.path().join("p1");
    write_project_with_feature(&project, "p1");
    register_project(&db, &project);

    for (suffix, body) in LIFECYCLE {
        let path = format!("/v1/admin/projects/p1/{suffix}");

        // Valid JSON body, no session cookie -> 401, manifest untouched.
        let anon = handle(
            &config,
            &db,
            &json_body(request("POST", &path, &config.frontend_origin), body),
            Utc::now(),
        );
        assert_eq!(anon.status, 401, "{suffix} anonymous");
        assert!(
            manifest_text(&project).contains("auth"),
            "{suffix} anonymous must not write"
        );

        // Session, non-JSON body -> 415 before the handler runs.
        let bad_ct = handle(
            &config,
            &db,
            &with_cookie(request("POST", &path, &config.frontend_origin), &token),
            Utc::now(),
        );
        assert_eq!(bad_ct.status, 415, "{suffix} non-json");
    }
}

#[test]
fn preview_returns_a_digest_without_writing_and_leaves_no_path() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let project = dir.path().join("p1");
    write_project_with_feature(&project, "p1");
    register_project(&db, &project);
    let before = manifest_text(&project);

    for (suffix, body) in LIFECYCLE {
        let path = format!("/v1/admin/projects/p1/{suffix}");
        let preview = handle(
            &config,
            &db,
            &with_cookie(
                json_body(request("POST", &path, &config.frontend_origin), body),
                &token,
            ),
            Utc::now(),
        );
        assert_eq!(preview.status, 200, "{suffix} preview status");
        let json = body_json(&preview);
        let digest = json["plan_digest"].as_str().unwrap();
        assert_eq!(digest.len(), 64, "{suffix} digest length");
        assert!(
            digest.chars().all(|c| c.is_ascii_hexdigit()),
            "{suffix} digest hex"
        );
        assert!(json["confirmation"].is_object(), "{suffix} gate described");
        // The descriptor is path-free.
        let raw = String::from_utf8_lossy(&preview.body);
        assert!(
            !raw.contains(&project.to_string_lossy().to_string()),
            "{suffix} response leaked an absolute path"
        );
        // The expected canonical action names.
        let action = json["preview"]["action"].as_str().unwrap();
        assert!(
            ["feature-remove", "feature-upgrade", "spec-apply"].contains(&action),
            "{suffix} unexpected action {action}"
        );
        assert_eq!(
            manifest_text(&project),
            before,
            "{suffix} preview must not write"
        );

        // Confirm with a wrong digest -> 409, no write, fresh digest returned.
        let wrong_body = format!(
            r#"{{{},"confirm":true,"plan_digest":"deadbeefdeadbeef"}}"#,
            body.trim_start_matches('{').trim_end_matches('}'),
        );
        let wrong = handle(
            &config,
            &db,
            &with_cookie(
                json_body(request("POST", &path, &config.frontend_origin), &wrong_body),
                &token,
            ),
            Utc::now(),
        );
        assert_eq!(wrong.status, 409, "{suffix} wrong digest status");
        assert_eq!(
            body_json(&wrong)["error"]["code"],
            "admin-digest-mismatch",
            "{suffix} mismatch code"
        );
        assert_eq!(
            manifest_text(&project),
            before,
            "{suffix} wrong digest must not write"
        );
    }
}

#[test]
fn confirmed_matching_digest_reaches_the_core_handler_not_the_preview_or_guard() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let project = dir.path().join("p1");
    write_project_with_feature(&project, "p1");
    register_project(&db, &project);

    for (suffix, body) in LIFECYCLE {
        let path = format!("/v1/admin/projects/p1/{suffix}");
        let preview = handle(
            &config,
            &db,
            &with_cookie(
                json_body(request("POST", &path, &config.frontend_origin), body),
                &token,
            ),
            Utc::now(),
        );
        let digest = body_json(&preview)["plan_digest"]
            .as_str()
            .unwrap()
            .to_string();

        let confirmed_body = format!(
            r#"{{{},"confirm":true,"plan_digest":"{digest}"}}"#,
            body.trim_start_matches('{').trim_end_matches('}'),
        );
        let applied = handle(
            &config,
            &db,
            &with_cookie(
                json_body(
                    request("POST", &path, &config.frontend_origin),
                    &confirmed_body,
                ),
                &token,
            ),
            Utc::now(),
        );
        let json = body_json(&applied);
        // Reaching the Core handler means the response is the handler's, never
        // the admin preview object and never the digest-mismatch refusal.
        assert!(
            json.get("confirmation").is_none(),
            "{suffix} confirmed action must reach the Core handler"
        );
        assert_ne!(
            json["error"]["code"].as_str(),
            Some("admin-digest-mismatch"),
            "{suffix} correct digest must satisfy the gate"
        );
        // And the handler response never leaks the absolute project path.
        let raw = String::from_utf8_lossy(&applied.body);
        assert!(
            !raw.contains(&project.to_string_lossy().to_string()),
            "{suffix} handler response leaked an absolute path"
        );
    }
}

#[test]
fn spec_apply_requires_findings_and_missing_feature_is_a_typed_four_hundred() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let project = dir.path().join("p1");
    write_project_with_feature(&project, "p1");
    register_project(&db, &project);

    // Empty findings -> 400, never echoed, no write.
    let empty = handle(
        &config,
        &db,
        &with_cookie(
            json_body(
                request(
                    "POST",
                    "/v1/admin/projects/p1/spec/apply",
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
        "admin-findings-required",
        "spec apply requires findings"
    );

    // feature remove/upgrade without a feature -> typed 400.
    for suffix in ["feature/remove", "feature/upgrade"] {
        let missing = handle(
            &config,
            &db,
            &with_cookie(
                json_body(
                    request(
                        "POST",
                        &format!("/v1/admin/projects/p1/{suffix}"),
                        &config.frontend_origin,
                    ),
                    r#"{"version":"1.0.0"}"#,
                ),
                &token,
            ),
            Utc::now(),
        );
        assert_eq!(missing.status, 400, "{suffix} missing feature");
        assert_eq!(
            body_json(&missing)["error"]["code"],
            "admin-feature-required",
            "{suffix} feature required"
        );
    }
}

#[test]
fn hostile_id_is_refused_and_unmanaged_id_is_not_found() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let project = dir.path().join("p1");
    write_project_with_feature(&project, "p1");
    register_project(&db, &project);

    // A path-bearing, invalid id is a typed 400 before any filesystem access.
    let hostile = handle(
        &config,
        &db,
        &with_cookie(
            json_body(
                request(
                    "POST",
                    "/v1/admin/projects/..%2F..%2Fetc/feature/remove",
                    &config.frontend_origin,
                ),
                r#"{"feature":"auth"}"#,
            ),
            &token,
        ),
        Utc::now(),
    );
    assert_eq!(hostile.status, 400, "hostile id refused");
    assert_eq!(
        body_json(&hostile)["error"]["code"],
        "admin-invalid-project-id",
        "hostile id typed"
    );
    let raw = String::from_utf8_lossy(&hostile.body);
    assert!(
        !raw.contains("..") && !raw.contains('/'),
        "hostile id must not be echoed: {raw}"
    );

    // A well-formed but unmanaged id is an honest 404, never a write.
    let unmanaged = handle(
        &config,
        &db,
        &with_cookie(
            json_body(
                request(
                    "POST",
                    "/v1/admin/projects/ghost-proj/feature/upgrade",
                    &config.frontend_origin,
                ),
                r#"{"feature":"auth","version":"1.0.0"}"#,
            ),
            &token,
        ),
        Utc::now(),
    );
    assert_eq!(unmanaged.status, 404, "unmanaged id not found");
    assert_eq!(
        body_json(&unmanaged)["error"]["code"],
        "admin-project-unmanaged",
        "unmanaged typed reason"
    );
}

#[test]
fn catalog_reports_the_three_lifecycle_commands_as_web_with_execution_blocks() {
    let rows = command_catalog::rows();
    for (id, route, action) in [
        (
            "feature.remove",
            "POST /v1/admin/projects/{id}/feature/remove",
            "feature",
        ),
        (
            "feature.upgrade",
            "POST /v1/admin/projects/{id}/feature/upgrade",
            "feature",
        ),
        (
            "spec.apply",
            "POST /v1/admin/projects/{id}/spec/apply",
            "findings",
        ),
    ] {
        let row = rows
            .iter()
            .find(|row| row.id == id)
            .unwrap_or_else(|| panic!("missing catalog row {id}"));
        assert_eq!(row.availability, "web", "{id} should be web");
        assert_eq!(row.route, Some(route), "{id} route");
        let execution = row
            .execution
            .as_ref()
            .unwrap_or_else(|| panic!("{id} must carry an execution block"));
        assert_eq!(execution.route, route, "{id} execution route");
        assert_eq!(execution.method, "POST", "{id} method");
        assert!(execution.confirm_required, "{id} confirm required");
        assert!(execution.digest_bound, "{id} digest bound");
        assert!(
            execution
                .parameters
                .iter()
                .any(|p| p.name == action && p.required),
            "{id} must declare required `{action}` parameter"
        );
    }
}
