//! Project-release execution contract (`forge-web-project-release`).
//!
//! Pins the two new typed, session-gated release routes —
//! `GET /v1/admin/projects/{id}/release/plan` (`forge release prepare`) and
//! `POST /v1/admin/projects/{id}/release` (`forge release apply`) — to the
//! same security boundary the shipped `feature`/`spec`/`deploy` lifecycle
//! routes honour: an anonymous JSON request is refused with `401` before any
//! Core call, a session request without a JSON body is refused with `415`, a
//! hostile project id is a typed `400`, an unmanaged id a `404`, and a
//! missing or non-semver `version` a typed `400`. The plan route is
//! side-effect-free, runs no adapter, mutates no git state, returns a
//! path-free projection plus a `plan_digest` and never writes the on-disk
//! release state or the registry `operations` journal. The apply route
//! returns a 64-hex `plan_digest` on a preview, refuses a mismatched digest
//! with `409` and a fresh digest and no write, and only delegates to the
//! in-process `release::engine::apply_release` on a confirmed matching digest.
//! The success path uses a real local git working tree with a local bare
//! `origin` so the commit/tag/push stages run without any network. A failing
//! notes adapter is surfaced honestly as a non-healthy report and journaled,
//! never as success. No response ever serializes the absolute project path,
//! the release state path or an adapter binary.

use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

use chrono::Utc;
use forge::api::{command_catalog, handle, ApiConfig, ApiRequest};
use forge::registry::Registry;
use tempfile::tempdir;

/// `FORGE_PACKAGE_BIN` / `FORGE_CONTAINER_BIN` / `FORGE_NOTES_BIN` are
/// process-global settings read by the engine inside the apply route, so every
/// test in this binary serializes on one lock to keep parallel tests from
/// racing each other's adapters.
static SERIAL: Mutex<()> = Mutex::new(());

/// Lock the serialization guard, tolerating a poisoned lock so a prior panic
/// does not cascade into unrelated failures.
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

fn plan_request(config: &ApiConfig, id: &str, version: Option<&str>) -> ApiRequest {
    let mut req = request(
        "GET",
        &format!("/v1/admin/projects/{id}/release/plan"),
        &config.frontend_origin,
    );
    req.query = version.map(|value| format!("version={value}"));
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

fn body_json(response: &forge::api::ApiResponse) -> serde_json::Value {
    serde_json::from_slice(&response.body).expect("admin routes are JSON-only")
}

/// A project the release engine will accept: a schema-1 manifest whose
/// `release` section declares `versioning: semver` and (by default) no checks,
/// so every captured check is `disabled` and the plan is `ready`; a present
/// `CHANGELOG.md`; and a git working tree with a committed HEAD plus a local
/// bare `origin` remote so the commit/tag/push stages succeed without any
/// network.
fn write_release_project(dir: &Path, id: &str, notes: bool) {
    fs::create_dir_all(dir).unwrap();
    let mut yaml = format!(
        "schema: 1\nproject:\n  id: {id}\n  name: Test {id}\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\nrelease:\n  versioning: semver\n"
    );
    if notes {
        yaml.push_str("  notes:\n    template: RELEASE_NOTES.md\n    output: RELEASE_NOTES.md\n");
    }
    fs::write(dir.join("forge.yaml"), yaml).unwrap();
    fs::write(dir.join("README.md"), "v1\n").unwrap();
    fs::write(dir.join("CHANGELOG.md"), "## 1.0.0\n- initial release\n").unwrap();
    run_git(dir, &["init", "-q"]);
    run_git(dir, &["config", "user.email", "forge@example.com"]);
    run_git(dir, &["config", "user.name", "Forge Test"]);
    run_git(dir, &["config", "init.defaultBranch", "main"]);
    run_git(dir, &["checkout", "-q", "-b", "main"]);
    run_git(
        dir,
        &["add", "--", "forge.yaml", "README.md", "CHANGELOG.md"],
    );
    run_git(dir, &["commit", "-q", "-m", "initial"]);

    // A local bare remote named `origin` so the push stage is hermetic.
    let origin = dir.with_file_name(format!("{id}-origin.git"));
    let out = Command::new("git")
        .arg("init")
        .arg("--bare")
        .arg("-q")
        .arg(&origin)
        .output()
        .expect("git init --bare");
    assert!(out.status.success(), "bare origin init failed");
    run_git(dir, &["remote", "add", "origin", origin.to_str().unwrap()]);
}

fn run_git(dir: &Path, args: &[&str]) {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(dir);
    for a in args {
        cmd.arg(a);
    }
    let out = cmd.output().expect("git");
    assert!(
        out.status.success(),
        "git {:?} failed: status={} stderr={}",
        args,
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
}

fn register_project(db: &Path, dir: &Path) {
    let mut registry = Registry::open(db).unwrap();
    registry.register(dir, None).unwrap();
}

fn write_adapter(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, body).unwrap();
    let mut perms = fs::metadata(&path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&path, perms).unwrap();
    path
}

/// Run the admin API with `FORGE_NOTES_BIN` pointed at the supplied adapter;
/// the helper clears any inherited value on entry and restores the caller's
/// prior value on exit so a parallel sibling cannot leak state.
fn handle_with_notes(
    config: &ApiConfig,
    db: &Path,
    request: &ApiRequest,
    notes: &Path,
) -> forge::api::ApiResponse {
    let prior = std::env::var("FORGE_NOTES_BIN").ok();
    std::env::set_var("FORGE_NOTES_BIN", notes);
    let response = handle(config, db, request, Utc::now());
    match prior {
        Some(value) => std::env::set_var("FORGE_NOTES_BIN", value),
        None => std::env::remove_var("FORGE_NOTES_BIN"),
    }
    response
}

#[test]
fn anonymous_and_non_json_are_refused_before_any_core_call() {
    let _guard = lock();
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let project = dir.path().join("p1");
    write_release_project(&project, "p1", false);
    register_project(&db, &project);

    // Anonymous GET plan -> 401.
    let anon_plan = handle(
        &config,
        &db,
        &plan_request(&config, "p1", Some("1.0.0")),
        Utc::now(),
    );
    assert_eq!(anon_plan.status, 401, "anonymous plan refused");

    // Anonymous POST apply -> 401.
    let anon_apply = handle(
        &config,
        &db,
        &json_body(
            request(
                "POST",
                "/v1/admin/projects/p1/release",
                &config.frontend_origin,
            ),
            r#"{"version":"1.0.0"}"#,
        ),
        Utc::now(),
    );
    assert_eq!(anon_apply.status, 401, "anonymous apply refused");

    // Session, non-JSON POST -> 415.
    let bad_ct = handle(
        &config,
        &db,
        &with_cookie(
            request(
                "POST",
                "/v1/admin/projects/p1/release",
                &config.frontend_origin,
            ),
            &token,
        ),
        Utc::now(),
    );
    assert_eq!(bad_ct.status, 415, "non-json apply refused");

    // No on-disk release state appeared.
    assert!(
        !project.join(".forge/release").exists(),
        "refused requests must not create .forge/release"
    );
}

#[test]
fn plan_route_reads_no_write_and_returns_a_path_free_preview() {
    let _guard = lock();
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let project = dir.path().join("p1");
    write_release_project(&project, "p1", false);
    register_project(&db, &project);

    let response = handle(
        &config,
        &db,
        &with_cookie(plan_request(&config, "p1", Some("1.0.0")), &token),
        Utc::now(),
    );
    assert_eq!(response.status, 200, "plan status");
    let json = body_json(&response);
    let view = &json["release_plan"];
    assert_eq!(view["action"], "release");
    assert_eq!(view["project_id"], "p1");
    assert_eq!(view["version"], "1.0.0");
    assert!(view["ready"].as_bool().unwrap_or(false));
    assert_eq!(view["changelog_path"], "CHANGELOG.md");
    assert!(view["changelog_hash"].is_string());
    let digest = json["plan_digest"].as_str().unwrap();
    assert_eq!(digest.len(), 64, "digest length");
    assert!(
        digest.chars().all(|c| c.is_ascii_hexdigit()),
        "digest hex: {digest}"
    );
    let raw = String::from_utf8_lossy(&response.body);
    assert!(
        !raw.contains(&project.to_string_lossy().to_string()),
        "plan response leaked the project path: {raw}"
    );

    // No on-disk release state, no git mutation.
    assert!(
        !project.join(".forge/release").exists(),
        "plan must not create .forge/release"
    );
}

#[test]
fn apply_preview_writes_nothing_and_wrong_digest_is_refused() {
    let _guard = lock();
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let project = dir.path().join("p1");
    write_release_project(&project, "p1", false);
    register_project(&db, &project);

    // Preview without confirm -> 200 + path-free view + 64-hex digest.
    let preview = handle(
        &config,
        &db,
        &with_cookie(
            json_body(
                request(
                    "POST",
                    "/v1/admin/projects/p1/release",
                    &config.frontend_origin,
                ),
                r#"{"version":"1.0.0"}"#,
            ),
            &token,
        ),
        Utc::now(),
    );
    assert_eq!(preview.status, 200, "preview status");
    let json = body_json(&preview);
    let digest = json["plan_digest"].as_str().unwrap();
    assert_eq!(digest.len(), 64, "digest length");
    let raw = String::from_utf8_lossy(&preview.body);
    assert!(
        !raw.contains(&project.to_string_lossy().to_string()),
        "preview leaked project path"
    );
    assert!(json["confirmation"].is_object(), "gate described");
    assert!(json["preview"].is_object(), "preview shape");

    // Wrong digest + confirm -> 409 mismatch + fresh digest + no write.
    let wrong = handle(
        &config,
        &db,
        &with_cookie(
            json_body(
                request(
                    "POST",
                    "/v1/admin/projects/p1/release",
                    &config.frontend_origin,
                ),
                r#"{"version":"1.0.0","confirm":true,"plan_digest":"deadbeefdeadbeef"}"#,
            ),
            &token,
        ),
        Utc::now(),
    );
    assert_eq!(wrong.status, 409, "wrong digest status");
    assert_eq!(
        body_json(&wrong)["error"]["code"],
        "admin-digest-mismatch",
        "mismatch code"
    );
    assert!(
        !project.join(".forge/release").exists(),
        "refused apply must not create .forge/release"
    );
}

#[test]
fn missing_or_non_semver_version_is_refused() {
    let _guard = lock();
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let project = dir.path().join("p1");
    write_release_project(&project, "p1", false);
    register_project(&db, &project);

    // Missing version on the plan route -> 400.
    let missing = handle(
        &config,
        &db,
        &with_cookie(plan_request(&config, "p1", None), &token),
        Utc::now(),
    );
    assert_eq!(missing.status, 400, "missing version refused");
    assert_eq!(
        body_json(&missing)["error"]["code"],
        "admin-invalid-version",
        "missing version typed"
    );

    // Non-semver version on the plan route -> 400, no echo.
    let bad = handle(
        &config,
        &db,
        &with_cookie(plan_request(&config, "p1", Some("not-a-version")), &token),
        Utc::now(),
    );
    assert_eq!(bad.status, 400, "bad version refused");
    assert_eq!(body_json(&bad)["error"]["code"], "admin-invalid-version");
    let raw = String::from_utf8_lossy(&bad.body);
    assert!(
        !raw.contains("not-a-version"),
        "version input must not be echoed: {raw}"
    );

    // Non-semver version on the apply route -> 400.
    let bad_apply = handle(
        &config,
        &db,
        &with_cookie(
            json_body(
                request(
                    "POST",
                    "/v1/admin/projects/p1/release",
                    &config.frontend_origin,
                ),
                r#"{"version":"1.2"}"#,
            ),
            &token,
        ),
        Utc::now(),
    );
    assert_eq!(bad_apply.status, 400, "bad apply version refused");
    assert!(
        !project.join(".forge/release").exists(),
        "bad version must not create .forge/release"
    );
}

#[test]
fn hostile_id_is_refused_and_unmanaged_id_is_not_found() {
    let _guard = lock();
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let project = dir.path().join("p1");
    write_release_project(&project, "p1", false);
    register_project(&db, &project);

    // Path-bearing id -> 400, no echo, no write.
    let hostile = handle(
        &config,
        &db,
        &with_cookie(
            plan_request(&config, "..%2F..%2Fetc", Some("1.0.0")),
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
    assert!(!raw.contains(".."), "hostile id must not be echoed: {raw}");

    // Unmanaged id -> 404.
    let unmanaged = handle(
        &config,
        &db,
        &with_cookie(plan_request(&config, "ghost-proj", Some("1.0.0")), &token),
        Utc::now(),
    );
    assert_eq!(unmanaged.status, 404, "unmanaged id not found");
    assert_eq!(
        body_json(&unmanaged)["error"]["code"],
        "admin-project-unmanaged",
        "unmanaged typed"
    );
}

#[test]
fn confirmed_apply_with_a_local_origin_reaches_the_core_handler() {
    let _guard = lock();
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let project = dir.path().join("p1");
    write_release_project(&project, "p1", false);
    register_project(&db, &project);
    // The commit stage bundles the changelog, so it must have an uncommitted
    // edit; an unrelated tracked edit would be refused by `commit_paths`.
    fs::write(
        project.join("CHANGELOG.md"),
        "## 1.0.0\n- initial release\n- browser release\n",
    )
    .unwrap();

    // Preview, then confirm with the same digest.
    let preview = handle(
        &config,
        &db,
        &with_cookie(
            json_body(
                request(
                    "POST",
                    "/v1/admin/projects/p1/release",
                    &config.frontend_origin,
                ),
                r#"{"version":"1.0.0"}"#,
            ),
            &token,
        ),
        Utc::now(),
    );
    assert_eq!(preview.status, 200, "preview status");
    let digest = body_json(&preview)["plan_digest"]
        .as_str()
        .unwrap()
        .to_string();

    let applied = handle(
        &config,
        &db,
        &with_cookie(
            json_body(
                request(
                    "POST",
                    "/v1/admin/projects/p1/release",
                    &config.frontend_origin,
                ),
                &format!(r#"{{"version":"1.0.0","confirm":true,"plan_digest":"{digest}"}}"#),
            ),
            &token,
        ),
        Utc::now(),
    );
    assert_eq!(applied.status, 202, "confirmed apply status");
    let json = body_json(&applied);
    assert!(
        json.get("confirmation").is_none(),
        "confirmed apply must not echo the admin gate"
    );
    assert!(json["operation_id"].is_i64());
    assert!(
        json["healthy"].as_bool().unwrap_or(false),
        "healthy: {json}"
    );
    assert_eq!(json["version"], "1.0.0");
    let stages = json["stage_outcomes"].as_array().expect("stage_outcomes");
    for stage in ["commit", "tag", "push"] {
        let outcome = stages
            .iter()
            .find(|s| s["stage"] == stage)
            .unwrap_or_else(|| panic!("missing stage {stage}"));
        assert_eq!(outcome["status"], "delivered", "stage {stage}");
    }
    // The state path is absolute and must never be serialized.
    let raw = String::from_utf8_lossy(&applied.body);
    assert!(
        !raw.contains(&project.to_string_lossy().to_string()),
        "handler response leaked project path"
    );
    assert!(
        !raw.contains("state_path"),
        "handler response must omit the absolute state path"
    );

    // The engine persisted release state to disk and the journal recorded it.
    assert!(
        project.join(".forge/release/p1").exists(),
        "release state dir must be written"
    );
    let registry = Registry::open(&db).unwrap();
    let mut found_release = false;
    for op_id in 1..32 {
        if let Some(entry) = registry.operation(op_id).unwrap_or(None) {
            if entry.kind == "release" && entry.project_id == "p1" {
                found_release = true;
                break;
            }
        }
    }
    assert!(
        found_release,
        "registry journal must record a release operation for p1"
    );
}

#[test]
fn adapter_failure_is_reported_honestly_never_as_success() {
    let _guard = lock();
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let project = dir.path().join("p1");
    write_release_project(&project, "p1", true);
    register_project(&db, &project);
    fs::write(
        project.join("CHANGELOG.md"),
        "## 1.0.0\n- initial release\n- browser release\n",
    )
    .unwrap();
    let notes_fail = write_adapter(
        dir.path(),
        "notes-fail.sh",
        "#!/bin/sh\necho 'notes adapter failed: simulated outage' >&2\nexit 1\n",
    );

    let preview = handle_with_notes(
        &config,
        &db,
        &with_cookie(
            json_body(
                request(
                    "POST",
                    "/v1/admin/projects/p1/release",
                    &config.frontend_origin,
                ),
                r#"{"version":"1.0.0"}"#,
            ),
            &token,
        ),
        &notes_fail,
    );
    assert_eq!(preview.status, 200, "preview status");
    let digest = body_json(&preview)["plan_digest"]
        .as_str()
        .unwrap()
        .to_string();

    let applied = handle_with_notes(
        &config,
        &db,
        &with_cookie(
            json_body(
                request(
                    "POST",
                    "/v1/admin/projects/p1/release",
                    &config.frontend_origin,
                ),
                &format!(r#"{{"version":"1.0.0","confirm":true,"plan_digest":"{digest}"}}"#),
            ),
            &token,
        ),
        &notes_fail,
    );
    // The route always succeeds at the HTTP level for a confirmed matching
    // digest (the engine is the source of truth); the report itself must
    // report `healthy: false` and a `failed` notes stage so the failure is
    // observable, not papered over as success.
    assert_eq!(applied.status, 202, "failed apply still 202");
    let json = body_json(&applied);
    assert!(!json["healthy"].as_bool().unwrap_or(true), "healthy=false");
    let stages = json["stage_outcomes"].as_array().expect("stage_outcomes");
    let notes = stages
        .iter()
        .find(|s| s["stage"] == "notes")
        .expect("notes stage present");
    assert_eq!(notes["status"], "failed");

    // The failure is journaled, never a fake success.
    let registry = Registry::open(&db).unwrap();
    let mut found_release = false;
    for op_id in 1..32 {
        if let Some(entry) = registry.operation(op_id).unwrap_or(None) {
            if entry.kind == "release" && entry.project_id == "p1" {
                found_release = true;
                break;
            }
        }
    }
    assert!(found_release, "release operation must still be journaled");
}

#[test]
fn catalog_reports_release_plan_and_apply_as_web_with_an_execution_block() {
    let _guard = lock();
    let rows = command_catalog::rows();
    let plan = rows
        .iter()
        .find(|row| row.id == "release.prepare")
        .expect("missing catalog row release.prepare");
    assert_eq!(plan.availability, "web", "release.prepare should be web");
    assert_eq!(
        plan.route.as_deref(),
        Some("GET /v1/admin/projects/{id}/release/plan")
    );
    assert!(
        plan.execution.is_none(),
        "release.prepare is a read route, no execution block"
    );

    let apply = rows
        .iter()
        .find(|row| row.id == "release.apply")
        .expect("missing catalog row release.apply");
    assert_eq!(apply.availability, "web", "release.apply should be web");
    let execution = apply
        .execution
        .as_ref()
        .expect("release.apply must carry an execution block");
    assert_eq!(
        execution.route, "POST /v1/admin/projects/{id}/release",
        "release.apply execution route"
    );
    assert_eq!(execution.method, "POST");
    assert!(execution.confirm_required, "confirm required");
    assert!(execution.digest_bound, "digest bound");
    assert_eq!(execution.risk, "remote_write");
    let version_param = execution
        .parameters
        .iter()
        .find(|p| p.name == "version")
        .expect("version parameter");
    assert_eq!(version_param.kind, "string");
    assert!(version_param.required, "version is required");
}
