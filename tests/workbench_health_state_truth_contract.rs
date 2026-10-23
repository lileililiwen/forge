//! Workbench health state-truth contract (`workbench-health-state-truth`).
//!
//! The workbench health document and the project-status doctor sub-check
//! read the same doctor report, so they must reach the same judgement of
//! it. One report, one partition:
//!
//! - a report with blocking evidence (a failing or unavailable finding, or
//!   an unmet maturity control) is `issues`;
//! - a report with no blocking evidence whose observation is stale is
//!   `stale` — never `issues`, because its document names no failing
//!   finding;
//! - a report with no blocking evidence and a current observation is the
//!   surface's own healthy state — `deferred` on the fast path (the live
//!   policy check did not run) and `healthy` on the explicit refresh.
//!
//! The routes are driven through the public `forge::api::handle`, so the
//! contract is read exactly as the operator reads it. `FORGE_DRIFTWATCH_BIN`
//! is pointed at a fixture script so the refresh route's live policy pass
//! runs and succeeds; env overrides are process-scoped, so the tests in
//! this file serialize on a mutex.

use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use chrono::Utc;
use forge::api::{handle, ApiConfig, ApiRequest};
use forge::registry::Registry;
use tempfile::tempdir;

static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Take the env lock. A panic in one test must not turn the other tests'
/// failures into lock-poison noise: each test asserts its own claim.
fn lock_env() -> std::sync::MutexGuard<'static, ()> {
    ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

const WORKBENCH_CONTRACT: &str = "forge-project-workbench/0.1.0";
const STATUS_CONTRACT: &str = "forge-project-status/0.1.0";

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

/// Seed the global administrator and return a live session token.
fn login_token(config: &ApiConfig, db: &Path) -> String {
    forge::identity::global::setup(db, "operator@example.test", "a-long-test-password").unwrap();
    let mut login = request("POST", "/v1/admin/session", &config.frontend_origin);
    login
        .headers
        .insert("content-type".to_string(), "application/json".to_string());
    login.body = br#"{"email":"operator@example.test","password":"a-long-test-password"}"#.to_vec();
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

fn git(dir: &Path, args: &[&str]) {
    let status = std::process::Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .expect("git is available");
    assert!(status.success(), "git {args:?} failed");
}

fn manifest(id: &str) -> String {
    format!(
        "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\nfeatures: {{}}\n"
    )
}

/// A rust-web project at L1 with every local evidence marker, so the
/// doctor's local pass is clean and the report carries no blocking
/// finding. The only thing that can make such a report unhealthy is a
/// stale registry observation.
fn write_clean_project(dir: &Path, id: &str) {
    fs::create_dir_all(dir.join(".github/workflows")).unwrap();
    fs::write(dir.join("forge.yaml"), manifest(id)).unwrap();
    fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"clean\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    fs::write(dir.join("README.md"), "# Clean project\n").unwrap();
    fs::write(dir.join("Dockerfile"), "FROM scratch\n").unwrap();
    fs::write(
        dir.join(".github/workflows/ci.yml"),
        "name: ci\non: [push]\njobs: {}\n",
    )
    .unwrap();
    fs::write(dir.join("driftwatch.yaml"), "version: 1\n").unwrap();
    git(dir, &["init", "-q"]);
    git(dir, &["config", "user.email", "test@example.test"]);
    git(dir, &["config", "user.name", "Forge Test"]);
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-q", "-m", "init"]);
    git(
        dir,
        &["remote", "add", "origin", "https://example.test/clean.git"],
    );
}

/// A rust-web project whose requested capability is incompatible with the
/// profile, so the local doctor pass carries one **failing** finding.
fn write_broken_project(dir: &Path, id: &str) {
    fs::create_dir_all(dir).unwrap();
    fs::write(
        dir.join("forge.yaml"),
        format!(
            "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\nfeatures:\n  auth: \"0.0.9\"\n"
        ),
    )
    .unwrap();
}

fn register_project(db: &Path, dir: &Path) {
    let mut registry = Registry::open(db).unwrap();
    registry.register(dir, None).unwrap();
}

/// A fixture checker: the `--version` probe answers without touching the
/// sentinel; a real check invocation reports one passing policy finding,
/// so the live pass is judged `passed` rather than `unjudged`.
fn write_clean_checker(dir: &Path) -> PathBuf {
    let path = dir.join("clean-driftwatch.sh");
    fs::write(
        &path,
        "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then\necho \"driftwatch 0.1.0-fixture\"\nexit 0\nfi\nprintf '%s' '{\"tool\":\"driftwatch\",\"tool_version\":\"0.1.0\",\"contract\":\"0.1.0\",\"source_revision\":null,\"findings\":[{\"id\":\"policy-baseline\",\"category\":\"architecture\",\"severity\":\"pass\",\"applicable\":true,\"message\":\"the project satisfies the policy baseline\",\"evidence\":[]}]}'\n",
    )
    .unwrap();
    let mut perms = fs::metadata(&path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&path, perms).unwrap();
    path
}

fn body_json(response: &forge::api::ApiResponse) -> serde_json::Value {
    serde_json::from_slice(&response.body).expect("workbench routes are JSON-only")
}

/// The workbench detail document for one project.
fn detail(config: &ApiConfig, db: &Path, token: &str, id: &str) -> serde_json::Value {
    let response = handle(
        config,
        db,
        &with_cookie(
            request(
                "GET",
                &format!("/v1/admin/projects/{id}"),
                &config.frontend_origin,
            ),
            token,
        ),
        Utc::now(),
    );
    assert_eq!(response.status, 200);
    let body = body_json(&response);
    assert_eq!(body["contract"], WORKBENCH_CONTRACT);
    body
}

/// The explicit full health document for one project.
fn refresh(config: &ApiConfig, db: &Path, token: &str, id: &str) -> serde_json::Value {
    let response = handle(
        config,
        db,
        &with_cookie(
            request(
                "POST",
                &format!("/v1/admin/projects/{id}/health/refresh"),
                &config.frontend_origin,
            ),
            token,
        ),
        Utc::now(),
    );
    assert_eq!(response.status, 200);
    let body = body_json(&response);
    assert_eq!(body["contract"], WORKBENCH_CONTRACT);
    body
}

/// The doctor sub-check of the project-status route.
fn doctor_sub_check(config: &ApiConfig, db: &Path, token: &str, id: &str) -> serde_json::Value {
    let response = handle(
        config,
        db,
        &with_cookie(
            request(
                "GET",
                &format!("/v1/admin/projects/{id}/status"),
                &config.frontend_origin,
            ),
            token,
        ),
        Utc::now(),
    );
    assert_eq!(response.status, 200);
    let body = body_json(&response);
    assert_eq!(body["contract"], STATUS_CONTRACT);
    body["checks"]
        .as_array()
        .expect("checks array")
        .iter()
        .find(|check| check["id"] == "doctor")
        .cloned()
        .expect("the doctor sub-check must be present")
}

fn health_state(health: &serde_json::Value) -> &str {
    health["state"].as_str().expect("a health state")
}

fn finding<'a>(health: &'a serde_json::Value, id: &str) -> Option<&'a serde_json::Value> {
    health["findings"]
        .as_array()
        .expect("a findings array")
        .iter()
        .find(|finding| finding["id"] == id)
}

#[test]
fn a_stale_observation_is_stale_and_not_issues_on_every_surface() {
    let _guard = lock_env();
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let proj = dir.path().join("proj");
    write_clean_project(&proj, "stale-truth");
    register_project(&db, &proj);
    let checker = write_clean_checker(dir.path());
    std::env::set_var("FORGE_DRIFTWATCH_BIN", &checker);

    // The manifest is written again after the registration, so the registry
    // observation now predates the current manifest: the doctor's report is
    // stale and carries nothing else.
    fs::write(proj.join("forge.yaml"), manifest("stale-truth")).unwrap();

    let detail_body = detail(&config, &db, &token, "stale-truth");
    let refresh_body = refresh(&config, &db, &token, "stale-truth");
    let status_doctor = doctor_sub_check(&config, &db, &token, "stale-truth");

    std::env::remove_var("FORGE_DRIFTWATCH_BIN");

    assert_eq!(
        health_state(&detail_body["health"]),
        "stale",
        "a stale observation carries no failing finding, so it is not a set of issues: {}",
        detail_body["health"]
    );
    assert_eq!(
        health_state(&refresh_body["health"]),
        "stale",
        "the explicit refresh reports the same partition: {}",
        refresh_body["health"]
    );
    assert_eq!(
        status_doctor["state"], "stale",
        "the sibling projection reports the same partition: {status_doctor}"
    );
    assert_eq!(
        status_doctor["summary"],
        "the registry observation predates the current manifest; re-register to refresh",
        "the project-status wording is unchanged: {status_doctor}"
    );

    // The document still names the stale observation it is reporting.
    let observation = finding(&detail_body["health"], "registry-observation")
        .expect("the stale observation is named in the health document");
    assert_eq!(observation["status"], "warn");
    assert!(
        finding(&detail_body["health"], "policy-deferred").is_some(),
        "the fast path still appends the policy-deferred finding"
    );
    let blocking = detail_body["health"]["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|finding| finding["id"] != "policy-deferred")
        .filter(|finding| finding["status"] == "fail" || finding["status"] == "unavailable")
        .count();
    assert_eq!(
        blocking, 0,
        "the stale document names no blocking finding: {}",
        detail_body["health"]
    );
}

#[test]
fn a_failing_local_check_is_issues_on_every_surface() {
    let _guard = lock_env();
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let proj = dir.path().join("proj");
    write_broken_project(&proj, "issues-truth");
    register_project(&db, &proj);
    let checker = write_clean_checker(dir.path());
    std::env::set_var("FORGE_DRIFTWATCH_BIN", &checker);

    let detail_body = detail(&config, &db, &token, "issues-truth");
    let refresh_body = refresh(&config, &db, &token, "issues-truth");
    let status_doctor = doctor_sub_check(&config, &db, &token, "issues-truth");

    std::env::remove_var("FORGE_DRIFTWATCH_BIN");

    assert_eq!(
        health_state(&detail_body["health"]),
        "issues",
        "a failing local check is a set of issues: {}",
        detail_body["health"]
    );
    assert_eq!(
        health_state(&refresh_body["health"]),
        "issues",
        "the explicit refresh reports the same partition: {}",
        refresh_body["health"]
    );
    assert_eq!(
        status_doctor["state"], "issues",
        "the sibling projection reports the same partition: {status_doctor}"
    );
    let failing = finding(&detail_body["health"], "build-config")
        .expect("the failing local check is named in the health document");
    assert_eq!(failing["status"], "fail");
}

#[test]
fn a_locally_clean_project_is_deferred_on_the_detail_and_healthy_on_the_refresh() {
    let _guard = lock_env();
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let proj = dir.path().join("proj");
    write_clean_project(&proj, "clean-truth");
    register_project(&db, &proj);
    let checker = write_clean_checker(dir.path());
    std::env::set_var("FORGE_DRIFTWATCH_BIN", &checker);

    let detail_body = detail(&config, &db, &token, "clean-truth");
    let refresh_body = refresh(&config, &db, &token, "clean-truth");
    let status_doctor = doctor_sub_check(&config, &db, &token, "clean-truth");

    std::env::remove_var("FORGE_DRIFTWATCH_BIN");

    assert_eq!(
        health_state(&detail_body["health"]),
        "deferred",
        "the fast path did not run the live policy check, so it never claims healthy: {}",
        detail_body["health"]
    );
    let deferred = finding(&detail_body["health"], "policy-deferred")
        .expect("the fast path appends the policy-deferred finding");
    assert_eq!(deferred["status"], "unavailable");
    assert_eq!(deferred["applicable"], false);
    assert_eq!(
        health_state(&refresh_body["health"]),
        "healthy",
        "the live pass ran and found no blocking evidence: {}",
        refresh_body["health"]
    );
    assert!(
        finding(&refresh_body["health"], "policy-deferred").is_none(),
        "the full pass carries no deferred finding"
    );
    assert_eq!(
        status_doctor["state"], "healthy",
        "the sibling projection reports the same partition: {status_doctor}"
    );
}

#[test]
fn an_unreadable_project_directory_is_unavailable() {
    let _guard = lock_env();
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let proj = dir.path().join("proj");
    write_clean_project(&proj, "gone-truth");
    register_project(&db, &proj);
    fs::remove_dir_all(&proj).unwrap();

    let detail_body = detail(&config, &db, &token, "gone-truth");

    assert_eq!(
        health_state(&detail_body["health"]),
        "unavailable",
        "a project directory that cannot be read is the workbench's own unavailable state: {}",
        detail_body["health"]
    );
    assert!(
        detail_body["health"]["note"].as_str().is_some(),
        "the unavailable state explains itself: {}",
        detail_body["health"]
    );
}
