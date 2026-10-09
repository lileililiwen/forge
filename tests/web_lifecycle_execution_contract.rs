//! Web lifecycle execution contract (`web-lifecycle-execution/0.1.0`).
//!
//! Pins the typed, session-gated lifecycle routes the workbench drives
//! end-to-end: graduation preview/import (artifact JSON + profile + id),
//! intent resolve/apply and remediate plan/apply (digest-bound, stale
//! refused), the delivery next-idea journal transition, and the
//! admin-gated studio spec/refine wrappers. Every preview writes nothing;
//! every confirm without a digest or without `confirm:true` is refused;
//! every success records a journal row and never echoes a path.
//!
//! `FORGE_ADMIN_PROJECTS_ROOT` is process-global, so every test serializes.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::sync::Mutex;

use chrono::Utc;
use forge::api::{command_catalog, handle, ApiConfig, ApiRequest};
use forge::registry::Registry;
use tempfile::tempdir;

const ADMIN_ROOT: &str = "FORGE_ADMIN_PROJECTS_ROOT";

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

fn handle_with_root(
    config: &ApiConfig,
    db: &Path,
    request: &ApiRequest,
    root: Option<&Path>,
) -> forge::api::ApiResponse {
    let prior = std::env::var(ADMIN_ROOT).ok();
    std::env::remove_var(ADMIN_ROOT);
    if let Some(value) = root {
        std::env::set_var(ADMIN_ROOT, value);
    }
    let response = handle(config, db, request, Utc::now());
    match prior {
        Some(value) => std::env::set_var(ADMIN_ROOT, value),
        None => std::env::remove_var(ADMIN_ROOT),
    }
    response
}

fn body_json(response: &forge::api::ApiResponse) -> serde_json::Value {
    serde_json::from_slice(&response.body).expect("admin routes are JSON-only")
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

fn journal_kinds(db: &Path, project: &str) -> Vec<String> {
    Registry::open(db)
        .unwrap()
        .journal_entries()
        .unwrap()
        .iter()
        .filter(|e| e.project_id == project)
        .map(|e| e.kind.clone())
        .collect()
}

fn artifact_json() -> String {
    serde_json::json!({
        "contract": "platform.idea-graduation/0.1.0",
        "hypora_project_id": "prj_01H",
        "hypora_revision": "rev-2026-09-20-3",
        "graduated_at": "2026-09-20T00:00:00Z",
        "brief": {
            "title": "GPA Simulator",
            "problem": "Students cannot see how a term changes their GPA.",
            "audience": "University students planning a term.",
            "solution": "A planner that projects cumulative GPA per course set.",
            "requirements": ["Model terms, courses, credits and grades."],
            "success_metrics": [
                { "name": "graded_course_sets_saved", "target": ">= 100", "window": "30d" }
            ],
        },
        "experiment": {
            "summary": "40 students completed the projection task in the probe.",
            "validated": true,
            "evidence": [
                { "kind": "probe-completion", "excerpt": "Aggregate: 40 of 52 completed.", "observed_at": "2026-09-18T00:00:00Z" }
            ],
        },
    })
    .to_string()
}

fn preview_graduation(
    config: &ApiConfig,
    db: &Path,
    token: &str,
    root: &Path,
) -> serde_json::Value {
    let raw = artifact_json();
    let payload = serde_json::json!({
        "artifact_json": raw,
        "profile": "rust-web",
        "id": "gpa-simulator",
    })
    .to_string();
    let req = with_cookie(
        json_body(
            request(
                "POST",
                "/v1/admin/graduation/preview",
                &config.frontend_origin,
            ),
            &payload,
        ),
        &token,
    );
    let response = handle_with_root(config, db, &req, Some(root));
    assert_eq!(
        response.status,
        200,
        "preview: {}",
        String::from_utf8_lossy(&response.body)
    );
    body_json(&response)
}

#[test]
fn graduation_preview_validates_without_writing() {
    let _guard = lock();
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let root = dir.path().join("workspace");
    fs::create_dir_all(&root).unwrap();
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let body = preview_graduation(&config, &db, &token, &root);
    assert_eq!(body["effect"], "none");
    assert!(body["plan_digest"].as_str().unwrap().len() == 64);
    assert_eq!(body["preview"]["proposed_id"], "gpa-simulator");
    // Nothing registered, no journal, no directory created.
    assert!(Registry::open(&db)
        .unwrap()
        .journal_entries()
        .unwrap()
        .is_empty());
    assert!(!root.join("gpa-simulator").exists());
}

#[test]
fn graduation_import_confirms_the_reviewed_digest_and_journals() {
    let _guard = lock();
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let root = dir.path().join("workspace");
    fs::create_dir_all(&root).unwrap();
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let preview = preview_graduation(&config, &db, &token, &root);
    let digest = preview["plan_digest"].as_str().unwrap().to_string();
    let raw = artifact_json();
    let payload = serde_json::json!({
        "artifact_json": raw,
        "profile": "rust-web",
        "id": "gpa-simulator",
        "confirm": true,
        "plan_digest": digest,
    })
    .to_string();
    let req = with_cookie(
        json_body(
            request(
                "POST",
                "/v1/admin/graduation/import",
                &config.frontend_origin,
            ),
            &payload,
        ),
        &token,
    );
    let response = handle_with_root(&config, &db, &req, Some(&root));
    assert_eq!(
        response.status,
        200,
        "import: {}",
        String::from_utf8_lossy(&response.body)
    );
    let body = body_json(&response);
    assert_eq!(body["project_id"], "gpa-simulator");
    assert!(root.join("gpa-simulator").join("forge.yaml").is_file());
    assert!(journal_kinds(&db, "gpa-simulator").contains(&"graduation.import".to_string()));
}

#[test]
fn graduation_confirm_without_digest_is_refused() {
    let _guard = lock();
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let root = dir.path().join("workspace");
    fs::create_dir_all(&root).unwrap();
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let raw = artifact_json();
    let payload = serde_json::json!({
        "artifact_json": raw,
        "profile": "rust-web",
        "id": "gpa-simulator",
        "confirm": true,
    })
    .to_string();
    let req = with_cookie(
        json_body(
            request(
                "POST",
                "/v1/admin/graduation/import",
                &config.frontend_origin,
            ),
            &payload,
        ),
        &token,
    );
    let response = handle_with_root(&config, &db, &req, Some(&root));
    assert!(response.status == 400 || response.status == 409);
    assert!(!root.join("gpa-simulator").exists());
}

#[test]
fn graduation_stale_digest_returns_a_fresh_preview() {
    let _guard = lock();
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let root = dir.path().join("workspace");
    fs::create_dir_all(&root).unwrap();
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let raw = artifact_json();
    let payload = serde_json::json!({
        "artifact_json": raw,
        "profile": "rust-web",
        "id": "gpa-simulator",
        "confirm": true,
        "plan_digest": "0".repeat(64),
    })
    .to_string();
    let req = with_cookie(
        json_body(
            request(
                "POST",
                "/v1/admin/graduation/import",
                &config.frontend_origin,
            ),
            &payload,
        ),
        &token,
    );
    let response = handle_with_root(&config, &db, &req, Some(&root));
    assert_eq!(response.status, 409);
    let body = body_json(&response);
    assert!(body["plan_digest"].as_str().unwrap().len() == 64);
    assert!(!root.join("gpa-simulator").exists());
}

#[test]
fn intent_resolve_previews_and_apply_journals() {
    let _guard = lock();
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let proj = dir.path().join("p1");
    write_project(&proj, "p1");
    register_project(&db, &proj);
    let payload = r#"{"action":"extend_project","profile":"rust-web","required":[],"forbidden":[],"constraints":[]}"#;
    let req = with_cookie(
        json_body(
            request(
                "POST",
                "/v1/admin/projects/p1/intent/resolve",
                &config.frontend_origin,
            ),
            payload,
        ),
        &token,
    );
    let response = handle(&config, &db, &req, Utc::now());
    assert_eq!(
        response.status,
        200,
        "resolve: {}",
        String::from_utf8_lossy(&response.body)
    );
    let body = body_json(&response);
    let digest = body["plan_digest"].as_str().unwrap().to_string();
    assert_eq!(body["effect"], "none");
    // No receipt written on preview.
    assert!(!proj.join(".forge").exists() || !proj.join(".forge/planner").exists());
    let apply_payload = serde_json::json!({
        "action": "extend_project",
        "profile": "rust-web",
        "required": [],
        "forbidden": [],
        "constraints": [],
        "confirm": true,
        "plan_digest": digest,
    })
    .to_string();
    let areq = with_cookie(
        json_body(
            request(
                "POST",
                "/v1/admin/projects/p1/intent/apply",
                &config.frontend_origin,
            ),
            &apply_payload,
        ),
        &token,
    );
    let aresponse = handle(&config, &db, &areq, Utc::now());
    assert_eq!(
        aresponse.status,
        200,
        "apply: {}",
        String::from_utf8_lossy(&aresponse.body)
    );
    assert!(journal_kinds(&db, "p1").contains(&"intent.apply".to_string()));
}

#[test]
fn intent_apply_with_stale_digest_is_refused() {
    let _guard = lock();
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let proj = dir.path().join("p1");
    write_project(&proj, "p1");
    register_project(&db, &proj);
    let payload = serde_json::json!({
        "action": "extend_project",
        "profile": "rust-web",
        "confirm": true,
        "plan_digest": "f".repeat(64),
    })
    .to_string();
    let req = with_cookie(
        json_body(
            request(
                "POST",
                "/v1/admin/projects/p1/intent/apply",
                &config.frontend_origin,
            ),
            &payload,
        ),
        &token,
    );
    let response = handle(&config, &db, &req, Utc::now());
    assert_eq!(response.status, 409);
    let body = body_json(&response);
    assert!(body["plan_digest"].as_str().unwrap().len() == 64);
}

#[test]
fn remediate_plan_needs_a_finding_and_refuses_without_confirm() {
    let _guard = lock();
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let proj = dir.path().join("p1");
    write_project(&proj, "p1");
    register_project(&db, &proj);
    // Missing finding → typed 400, nothing written.
    let req = with_cookie(
        json_body(
            request(
                "POST",
                "/v1/admin/projects/p1/remediate/plan",
                &config.frontend_origin,
            ),
            r#"{"finding":""}"#,
        ),
        &token,
    );
    let response = handle(&config, &db, &req, Utc::now());
    assert_eq!(response.status, 400);
    // Unknown finding → typed refusal (400/409/422 family), nothing written.
    let req = with_cookie(
        json_body(
            request(
                "POST",
                "/v1/admin/projects/p1/remediate/apply",
                &config.frontend_origin,
            ),
            r#"{"finding":"no-such-finding","confirm":true,"plan_digest":"0"}"#,
        ),
        &token,
    );
    let response = handle(&config, &db, &req, Utc::now());
    assert!(response.status == 400 || response.status == 409);
    assert!(!journal_kinds(&db, "p1").contains(&"remediate.apply".to_string()));
}

#[test]
fn next_idea_transition_journals_and_delivery_catalog_is_web() {
    let _guard = lock();
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let proj = dir.path().join("p1");
    write_project(&proj, "p1");
    register_project(&db, &proj);
    // Without confirm → 409, nothing journaled.
    let req = with_cookie(
        json_body(
            request(
                "POST",
                "/v1/admin/projects/p1/delivery/next-idea",
                &config.frontend_origin,
            ),
            r#"{"confirm":false}"#,
        ),
        &token,
    );
    assert_eq!(handle(&config, &db, &req, Utc::now()).status, 409);
    // With confirm → journal row.
    let req = with_cookie(
        json_body(
            request(
                "POST",
                "/v1/admin/projects/p1/delivery/next-idea",
                &config.frontend_origin,
            ),
            r#"{"confirm":true,"note":"publish landed"}"#,
        ),
        &token,
    );
    let response = handle(&config, &db, &req, Utc::now());
    assert_eq!(response.status, 200);
    assert!(journal_kinds(&db, "p1").contains(&"delivery.next-idea".to_string()));
    // Catalog rows for the new web surface are `web` with routes.
    let rows = command_catalog::rows();
    for id in [
        "graduation.preview",
        "graduation.import",
        "intent.resolve",
        "intent.apply",
        "remediate.plan",
        "remediate.apply",
        "studio.spec",
        "studio.refine",
    ] {
        let row = rows.iter().find(|r| r.id == id).expect(id);
        assert_eq!(row.availability, "web", "{id}");
        assert!(row.route.is_some(), "{id}");
        assert!(row.execution.is_some(), "{id}");
    }
}

#[test]
fn frontend_carries_the_lifecycle_execution_controls() {
    let app = fs::read_to_string("frontend/app.js").expect("app.js ships");
    let html = fs::read_to_string("frontend/index.html").expect("index.html ships");
    for token in [
        "idea-artifact",
        "idea-profile",
        "idea-preview",
        "idea-confirm",
        "idea-run",
        "idea-result",
        "idea-error-summary",
        "renderIdeaEntry",
        "delivery.next-idea",
        "Loop transition journaled",
        "spec_revision",
        "app_revision",
    ] {
        assert!(
            app.contains(token) || html.contains(token),
            "missing token {token}"
        );
    }
    assert!(!app.contains("import(") && !app.contains("require("));
}
