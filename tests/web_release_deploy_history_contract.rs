//! Read-only release/deploy history contract (`web-release-deploy-history`).
//!
//! Pins the five typed, session-gated history GET routes —
//! `GET /v1/admin/projects/{id}/releases`,
//! `GET /v1/admin/projects/{id}/releases/{release_id}`,
//! `GET /v1/admin/projects/{id}/deploys`,
//! `GET /v1/admin/projects/{id}/deploys/{deploy_id}` and
//! `GET /v1/admin/projects/{id}/deploy/status` — to the same read
//! boundary the shipped plan routes honour: an anonymous request is
//! refused with `401` before any Core call, a hostile project id is a
//! typed `400` without echo, an unmanaged id a `404`, a bad inspect
//! id or limit a typed `400` without echo, and an unknown release /
//! deploy id a typed `404`. Lists read back seeded persisted state
//! (written through the same state-file shape the engines persist);
//! the deploy list omits the absolute `state_path`; status answers
//! from the journal's `publish|deploy|publish.github` rows only.
//! No route writes state, journals, runs an adapter or contacts a
//! provider, and no response ever serializes the absolute project
//! path. Static-token pins cover the five catalog rows (`web`, count
//! stays 234) and the Delivery history card (no new dependency, no
//! `innerHTML` in the added controls).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use forge::api::{command_catalog, handle, ApiConfig, ApiRequest};
use forge::registry::Registry;
use serde_json::json;

fn app_js() -> String {
    std::fs::read_to_string("frontend/app.js").expect("frontend/app.js ships")
}

fn index_html() -> String {
    std::fs::read_to_string("frontend/index.html").expect("frontend/index.html ships")
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

fn with_query(mut req: ApiRequest, query: &str) -> ApiRequest {
    req.query = Some(query.to_string());
    req
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

fn body_json(response: &forge::api::ApiResponse) -> serde_json::Value {
    serde_json::from_slice(&response.body).expect("admin routes are JSON-only")
}

fn history_paths(id: &str) -> Vec<String> {
    vec![
        format!("/v1/admin/projects/{id}/releases"),
        format!("/v1/admin/projects/{id}/releases/rel-1"),
        format!("/v1/admin/projects/{id}/deploys"),
        format!("/v1/admin/projects/{id}/deploys/dep-1"),
        format!("/v1/admin/projects/{id}/deploy/status"),
    ]
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
    let project = tmp.path().join("p1");
    write_project(&project, "p1");
    {
        let mut registry = Registry::open(&db).unwrap();
        registry.register(&project, None).unwrap();
    }
    let config = ApiConfig::default();
    (tmp, db, config)
}

fn project_dir(tmp: &tempfile::TempDir) -> PathBuf {
    tmp.path().join("p1")
}

/// Seed one persisted release + one persisted deploy through the same
/// state-file layout the engines read, so the history routes exercise
/// the real loaders without running any adapter.
fn seed_history(project: &Path) {
    let release = json!({
        "contract": "forge-release-state/0.1.0",
        "identity": {
            "project_id": "p1",
            "version": "1.2.3",
            "source_revision": "abc123",
            "id": "rel-1",
        },
        "changelog": null,
        "docs_locales": [],
        "checks": [],
        "stages": ["tag", "push"],
        "stage_outcomes": [],
        "last_run_at": "2026-10-10T00:00:00Z",
    });
    let release_dir = project.join(".forge/release/p1/rel-1");
    fs::create_dir_all(&release_dir).unwrap();
    fs::write(release_dir.join("state.json"), release.to_string()).unwrap();
    let deploy = json!({
        "contract": "forge-deploy-state/0.1.0",
        "identity": {
            "project_id": "p1",
            "target": "local",
            "source_revision": "abc123",
            "id": "dep-1",
        },
        "target": { "name": "local", "kind": "compose" },
        "adapter": "fixture",
        "artifact": null,
        "health": null,
        "stage_outcomes": [],
        "last_observation": null,
        "last_observed_running": null,
        "last_run_at": "2026-10-10T00:00:00Z",
    });
    let deploy_dir = project.join(".forge/deploy/p1/dep-1");
    fs::create_dir_all(&deploy_dir).unwrap();
    fs::write(deploy_dir.join("state.json"), deploy.to_string()).unwrap();
}

fn get(
    config: &ApiConfig,
    db: &Path,
    token: &str,
    path: &str,
    query: Option<&str>,
) -> forge::api::ApiResponse {
    let mut req = request("GET", path, &config.frontend_origin);
    if let Some(q) = query {
        req = with_query(req, q);
    }
    handle(config, db, &with_cookie(req, token), Utc::now())
}

fn journal_len(db: &Path) -> usize {
    let registry = Registry::open(db).unwrap();
    registry.recent_operations(500).unwrap().len()
}

#[test]
fn anonymous_history_reads_are_refused_before_any_core_call() {
    let (_tmp, db, config) = fixture();
    for path in history_paths("p1") {
        let response = handle(
            &config,
            &db,
            &request("GET", &path, &config.frontend_origin),
            Utc::now(),
        );
        assert_eq!(response.status, 401, "anonymous {path} refused");
    }
}

#[test]
fn hostile_id_is_refused_and_unmanaged_id_is_not_found() {
    let (_tmp, db, config) = fixture();
    let token = login_token(&config, &db);
    for path in history_paths("..%2F..%2Fetc") {
        let response = get(&config, &db, &token, &path, None);
        assert_eq!(response.status, 400, "hostile {path} refused");
        assert_eq!(
            body_json(&response)["error"]["code"],
            "admin-invalid-project-id",
            "hostile {path} typed"
        );
        let raw = String::from_utf8_lossy(&response.body);
        assert!(!raw.contains(".."), "hostile id echoed: {raw}");
    }
    for path in history_paths("ghost-proj") {
        let response = get(&config, &db, &token, &path, None);
        assert_eq!(response.status, 404, "unmanaged {path} not found");
        assert_eq!(
            body_json(&response)["error"]["code"],
            "admin-project-unmanaged",
            "unmanaged {path} typed"
        );
    }
}

#[test]
fn bad_inspect_id_and_bad_limit_are_typed_refusals() {
    let (_tmp, db, config) = fixture();
    let token = login_token(&config, &db);
    for path in [
        "/v1/admin/projects/p1/releases/..",
        "/v1/admin/projects/p1/deploys/a%2Fb",
    ] {
        let response = get(&config, &db, &token, path, None);
        assert_eq!(response.status, 400, "bad inspect id {path} refused");
        assert_eq!(
            body_json(&response)["error"]["code"],
            "admin-invalid-history-id",
            "bad inspect id typed"
        );
    }
    let response = get(
        &config,
        &db,
        &token,
        "/v1/admin/projects/p1/deploy/status",
        Some("limit=bogus"),
    );
    assert_eq!(response.status, 400, "bad limit refused");
    assert_eq!(
        body_json(&response)["error"]["code"],
        "admin-invalid-history-limit",
        "bad limit typed"
    );
}

#[test]
fn release_list_and_inspect_read_back_persisted_state() {
    let (tmp, db, config) = fixture();
    let token = login_token(&config, &db);
    let project = project_dir(&tmp);

    // Empty project: 200 with an empty list, no state dir created.
    let empty = get(&config, &db, &token, "/v1/admin/projects/p1/releases", None);
    assert_eq!(empty.status, 200, "empty list status");
    let json = body_json(&empty);
    assert_eq!(json["project_id"], "p1");
    assert_eq!(json["releases"].as_array().unwrap().len(), 0);
    assert!(!project.join(".forge/release").exists());

    seed_history(&project);

    let listed = get(&config, &db, &token, "/v1/admin/projects/p1/releases", None);
    assert_eq!(listed.status, 200, "list status");
    let json = body_json(&listed);
    let entries = json["releases"].as_array().expect("releases array");
    assert_eq!(entries.len(), 1, "one seeded release");
    assert_eq!(entries[0]["release_id"], "rel-1");
    assert_eq!(entries[0]["version"], "1.2.3");
    assert_eq!(entries[0]["stage_count"], 0);
    assert_eq!(entries[0]["last_run_at"], "2026-10-10T00:00:00Z");
    let raw = String::from_utf8_lossy(&listed.body);
    assert!(
        !raw.contains(&project.to_string_lossy().to_string()),
        "release list leaked the project path: {raw}"
    );

    let inspected = get(
        &config,
        &db,
        &token,
        "/v1/admin/projects/p1/releases/rel-1",
        None,
    );
    assert_eq!(inspected.status, 200, "inspect status");
    let json = body_json(&inspected);
    assert_eq!(json["release"]["identity"]["id"], "rel-1");
    assert_eq!(json["release"]["identity"]["version"], "1.2.3");

    let missing = get(
        &config,
        &db,
        &token,
        "/v1/admin/projects/p1/releases/nope",
        None,
    );
    assert_eq!(missing.status, 404, "unknown release not found");
    assert_eq!(
        body_json(&missing)["error"]["code"],
        "admin-history-not-found",
        "unknown release typed"
    );
}

#[test]
fn deploy_list_and_inspect_stay_path_free() {
    let (tmp, db, config) = fixture();
    let token = login_token(&config, &db);
    let project = project_dir(&tmp);

    let empty = get(&config, &db, &token, "/v1/admin/projects/p1/deploys", None);
    assert_eq!(empty.status, 200, "empty list status");
    assert_eq!(body_json(&empty)["deploys"].as_array().unwrap().len(), 0);
    assert!(!project.join(".forge/deploy").exists());

    seed_history(&project);

    let listed = get(&config, &db, &token, "/v1/admin/projects/p1/deploys", None);
    assert_eq!(listed.status, 200, "list status");
    let json = body_json(&listed);
    let entries = json["deploys"].as_array().expect("deploys array");
    assert_eq!(entries.len(), 1, "one seeded deploy");
    assert_eq!(entries[0]["deploy_id"], "dep-1");
    assert_eq!(entries[0]["target"], "local");
    assert_eq!(entries[0]["last_run_at"], "2026-10-10T00:00:00Z");
    assert!(
        entries[0].get("state_path").is_none(),
        "deploy list must omit the absolute state_path"
    );
    let raw = String::from_utf8_lossy(&listed.body);
    assert!(
        !raw.contains(&project.to_string_lossy().to_string()),
        "deploy list leaked the project path: {raw}"
    );

    let inspected = get(
        &config,
        &db,
        &token,
        "/v1/admin/projects/p1/deploys/dep-1",
        None,
    );
    assert_eq!(inspected.status, 200, "inspect status");
    let json = body_json(&inspected);
    assert_eq!(json["deploy"]["identity"]["id"], "dep-1");
    assert_eq!(json["deploy"]["target"]["name"], "local");
    let raw = String::from_utf8_lossy(&inspected.body);
    assert!(
        !raw.contains(&project.to_string_lossy().to_string()),
        "deploy inspect leaked the project path: {raw}"
    );

    let missing = get(
        &config,
        &db,
        &token,
        "/v1/admin/projects/p1/deploys/nope",
        None,
    );
    assert_eq!(missing.status, 404, "unknown deploy not found");
    assert_eq!(
        body_json(&missing)["error"]["code"],
        "admin-history-not-found",
        "unknown deploy typed"
    );
}

#[test]
fn deploy_status_answers_from_the_journal_only() {
    let (tmp, db, config) = fixture();
    let token = login_token(&config, &db);

    // No rows: honest empty state.
    let empty = get(
        &config,
        &db,
        &token,
        "/v1/admin/projects/p1/deploy/status",
        None,
    );
    assert_eq!(empty.status, 200, "empty status");
    let json = body_json(&empty);
    assert_eq!(json["state"], "empty");
    assert_eq!(json["entries"].as_array().unwrap().len(), 0);
    assert_eq!(json["read_only"], true);

    // Journal mixed rows: only publish/deploy rows for p1 survive.
    {
        let registry = Registry::open(&db).unwrap();
        registry.record_operation("publish", "p1", "done", "shipped 1.2.3");
        registry.record_operation("deploy", "p1", "done", "local delivered");
        registry.record_operation("release", "p1", "done", "release cut");
        registry.record_operation("publish", "other", "done", "unrelated");
    }
    let before = journal_len(&db);
    let status = get(
        &config,
        &db,
        &token,
        "/v1/admin/projects/p1/deploy/status",
        None,
    );
    assert_eq!(status.status, 200, "status");
    let json = body_json(&status);
    let entries = json["entries"].as_array().expect("entries array");
    assert_eq!(entries.len(), 2, "only publish+deploy rows: {json}");
    for entry in entries {
        assert!(
            entry["kind"] == "publish" || entry["kind"] == "deploy",
            "filtered kind: {entry}"
        );
        assert_eq!(entry["project_id"], "p1", "project-scoped: {entry}");
    }
    assert_eq!(journal_len(&db), before, "status read journals nothing");
    assert_eq!(body_json(&status)["read_only"], true);

    // Bad limit is refused; a huge limit clamps instead of failing.
    let bad = get(
        &config,
        &db,
        &token,
        "/v1/admin/projects/p1/deploy/status",
        Some("limit=bogus"),
    );
    assert_eq!(bad.status, 400, "bad limit refused");
    let clamped = get(
        &config,
        &db,
        &token,
        "/v1/admin/projects/p1/deploy/status",
        Some("limit=5000"),
    );
    assert_eq!(clamped.status, 200, "clamped limit ok");
    let _ = tmp;
}

#[test]
fn refused_and_read_paths_change_nothing() {
    let (tmp, db, config) = fixture();
    let token = login_token(&config, &db);
    let project = project_dir(&tmp);
    let before = journal_len(&db);
    for path in history_paths("p1") {
        let anon = handle(
            &config,
            &db,
            &request("GET", &path, &config.frontend_origin),
            Utc::now(),
        );
        assert_eq!(anon.status, 401, "anonymous {path} refused");
    }
    for path in history_paths("ghost-proj") {
        let response = get(&config, &db, &token, &path, None);
        assert_eq!(response.status, 404, "unmanaged {path} not found");
    }
    // Reads (empty lists + status) must not create state dirs or journal
    // rows; unknown inspect ids stay honest 404s.
    for path in [
        "/v1/admin/projects/p1/releases",
        "/v1/admin/projects/p1/deploys",
        "/v1/admin/projects/p1/deploy/status",
    ] {
        let response = get(&config, &db, &token, path, None);
        assert_eq!(response.status, 200, "empty {path} ok");
    }
    for path in [
        "/v1/admin/projects/p1/releases/rel-1",
        "/v1/admin/projects/p1/deploys/dep-1",
    ] {
        let response = get(&config, &db, &token, path, None);
        assert_eq!(response.status, 404, "unknown {path} not found");
    }
    assert!(!project.join(".forge/release").exists());
    assert!(!project.join(".forge/deploy").exists());
    assert_eq!(journal_len(&db), before, "reads journal nothing");
}

#[test]
fn catalog_reports_the_five_history_rows_as_web() {
    let rows = command_catalog::rows();
    for (id, route) in [
        ("release.list", "GET /v1/admin/projects/{id}/releases"),
        (
            "release.inspect",
            "GET /v1/admin/projects/{id}/releases/{release_id}",
        ),
        ("deploy.list", "GET /v1/admin/projects/{id}/deploys"),
        (
            "deploy.inspect",
            "GET /v1/admin/projects/{id}/deploys/{deploy_id}",
        ),
        ("deploy.status", "GET /v1/admin/projects/{id}/deploy/status"),
    ] {
        let row = rows
            .iter()
            .find(|row| row.id == id)
            .unwrap_or_else(|| panic!("missing catalog row {id}"));
        assert_eq!(row.availability, "web", "{id} should be web");
        assert_eq!(row.route.as_deref(), Some(route), "{id} route");
        assert!(row.execution.is_none(), "{id} is a read route");
    }
    let observe = rows
        .iter()
        .find(|row| row.id == "deploy.observe")
        .expect("missing catalog row deploy.observe");
    assert_ne!(
        observe.availability, "web",
        "deploy.observe stays out of the browser"
    );
    assert_eq!(
        rows.len(),
        234,
        "catalog conversion must not add rows ({} found)",
        rows.len()
    );
}

#[test]
fn frontend_history_card_uses_no_new_dependency_or_sink() {
    let html = index_html();
    let js = app_js();
    for token in [
        "history-title",
        "history-releases",
        "history-release-inspect",
        "history-deploys",
        "history-deploy-inspect",
        "history-status",
        "history-releases-result",
        "history-deploys-result",
        "history-status-result",
    ] {
        assert!(
            html.contains(token) || js.contains(token),
            "frontend token {token} missing"
        );
    }
    for route in ["/releases", "/deploys", "/deploy/status"] {
        assert!(js.contains(route), "frontend route {route} missing");
    }
    // The added controls render through the existing DOM helpers.
    assert!(js.contains("historyListReleases"), "history loader missing");
    assert!(js.contains("historyReadStatus"), "status loader missing");
    let start = js.find("Release & deploy history").expect("history block");
    let block = &js[start..(start + 12000).min(js.len())];
    assert!(
        !block.contains("innerHTML"),
        "history block must not use innerHTML"
    );
}
