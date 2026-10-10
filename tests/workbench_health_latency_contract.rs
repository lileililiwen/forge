//! Workbench health latency contract (`workbench-health-latency`).
//!
//! Sentinel oracle: with `FORGE_DRIFTWATCH_BIN` pointed at a script that
//! touches a file, the detail GET leaves the file untouched (fast local
//! pass) while the explicit refresh route touches it (live policy pass).
//! Env overrides are process-scoped, so the tests in this file serialize
//! on a mutex; run this target with `--test-threads=1` if combined with
//! other env-mutating suites in one process.

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

/// A rust-web project with every L1 evidence marker, so the local doctor
/// pass is genuinely clean and the fast path reports `deferred`.
fn write_healthy_project(dir: &Path, id: &str) {
    fs::create_dir_all(dir.join(".github/workflows")).unwrap();
    fs::write(
        dir.join("forge.yaml"),
        format!(
            "schema: 1\nproject:\n  id: {id}\n  name: Healthy {id}\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\nfeatures: {{}}\n"
        ),
    )
    .unwrap();
    fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"healthy\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    fs::write(dir.join("README.md"), "# Healthy project\n").unwrap();
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
        &[
            "remote",
            "add",
            "origin",
            "https://example.test/healthy.git",
        ],
    );
}

/// A minimal project whose local checks fail, so the fast path reports
/// `issues` without ever invoking the checker.
fn write_broken_project(dir: &Path, id: &str) {
    fs::create_dir_all(dir).unwrap();
    fs::write(
        dir.join("forge.yaml"),
        format!(
            "schema: 1\nproject:\n  id: {id}\n  name: Broken {id}\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\nfeatures:\n  auth: \"0.0.9\"\n"
        ),
    )
    .unwrap();
}

fn register_project(db: &Path, dir: &Path) {
    let mut registry = Registry::open(db).unwrap();
    registry.register(dir, None).unwrap();
}

/// Sentinel checker: the `--version` probe answers without touching the
/// sentinel; any real check invocation touches `$SENTINEL` and prints a
/// minimal legacy adapter report with no findings.
fn write_sentinel_script(dir: &Path) -> PathBuf {
    let path = dir.join("sentinel-driftwatch.sh");
    fs::write(
        &path,
        "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then\necho \"driftwatch 0.1.0-sentinel\"\nexit 0\nfi\ntouch \"$SENTINEL\"\nprintf '%s' '{\"tool\":\"driftwatch\",\"tool_version\":\"0.1.0\",\"contract\":\"0.1.0\",\"source_revision\":null,\"findings\":[]}'\n",
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

fn journal_len(db: &Path) -> usize {
    Registry::open_read_only(db)
        .unwrap()
        .journal_entries()
        .unwrap()
        .len()
}

#[test]
fn detail_get_does_not_invoke_the_checker_and_reports_deferred() {
    let _guard = ENV_LOCK.lock().unwrap();
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let proj = dir.path().join("proj");
    write_healthy_project(&proj, "wb-fast");
    register_project(&db, &proj);
    let script = write_sentinel_script(dir.path());
    let sentinel = dir.path().join("checker-ran");
    assert!(!sentinel.exists());
    std::env::set_var("FORGE_DRIFTWATCH_BIN", &script);
    std::env::set_var("SENTINEL", &sentinel);
    let journal_before = journal_len(&db);

    let response = handle(
        &config,
        &db,
        &with_cookie(
            request("GET", "/v1/admin/projects/wb-fast", &config.frontend_origin),
            &token,
        ),
        Utc::now(),
    );

    std::env::remove_var("FORGE_DRIFTWATCH_BIN");
    std::env::remove_var("SENTINEL");
    assert_eq!(response.status, 200);
    let body = body_json(&response);
    assert_eq!(body["contract"], "forge-project-workbench/0.1.0");
    assert_eq!(
        body["health"]["state"], "deferred",
        "a locally-clean project must report deferred, never healthy, on the fast path: {}",
        body["health"]
    );
    let findings = body["health"]["findings"].as_array().unwrap();
    let deferred = findings
        .iter()
        .find(|f| f["id"] == "policy-deferred")
        .expect("the fast path appends the policy-deferred finding");
    assert_eq!(deferred["status"], "unavailable");
    assert_eq!(deferred["applicable"], false);
    assert!(
        !sentinel.exists(),
        "the detail GET must not invoke the checker binary"
    );
    assert_eq!(
        journal_len(&db),
        journal_before,
        "reads write no journal rows"
    );
    let text = String::from_utf8_lossy(&response.body).to_string();
    assert!(
        !text.contains(&proj.canonicalize().unwrap().display().to_string()),
        "the registered absolute path leaked into the detail projection"
    );
}

#[test]
fn refresh_runs_the_live_check_and_returns_the_full_document() {
    let _guard = ENV_LOCK.lock().unwrap();
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let proj = dir.path().join("proj");
    write_healthy_project(&proj, "wb-refresh");
    register_project(&db, &proj);
    let script = write_sentinel_script(dir.path());
    let sentinel = dir.path().join("checker-ran");
    std::env::set_var("FORGE_DRIFTWATCH_BIN", &script);
    std::env::set_var("SENTINEL", &sentinel);
    let journal_before = journal_len(&db);

    let detail = handle(
        &config,
        &db,
        &with_cookie(
            request(
                "GET",
                "/v1/admin/projects/wb-refresh",
                &config.frontend_origin,
            ),
            &token,
        ),
        Utc::now(),
    );
    assert_eq!(detail.status, 200);
    assert!(
        !sentinel.exists(),
        "the detail GET must not invoke the checker binary"
    );

    let response = handle(
        &config,
        &db,
        &with_cookie(
            request(
                "POST",
                "/v1/admin/projects/wb-refresh/health/refresh",
                &config.frontend_origin,
            ),
            &token,
        ),
        Utc::now(),
    );

    std::env::remove_var("FORGE_DRIFTWATCH_BIN");
    std::env::remove_var("SENTINEL");
    assert_eq!(response.status, 200);
    let body = body_json(&response);
    assert_eq!(body["contract"], "forge-project-workbench/0.1.0");
    assert_eq!(body["project_id"], "wb-refresh");
    assert!(
        sentinel.exists(),
        "the refresh route must execute the configured checker binary"
    );
    let state = body["health"]["state"].as_str().unwrap();
    assert!(
        ["healthy", "issues", "stale"].contains(&state),
        "the refresh returns the full live document, got {state}"
    );
    let findings = body["health"]["findings"].as_array().unwrap();
    assert!(
        findings.iter().all(|f| f["id"] != "policy-deferred"),
        "the full pass carries no deferred finding"
    );
    assert_eq!(
        journal_len(&db),
        journal_before,
        "the refresh is read-only: no journal row"
    );
    let text = String::from_utf8_lossy(&response.body).to_string();
    assert!(
        !text.contains(&proj.canonicalize().unwrap().display().to_string()),
        "the registered absolute path leaked into the refresh document"
    );
}

#[test]
fn local_problems_still_surface_immediately_without_the_checker() {
    let _guard = ENV_LOCK.lock().unwrap();
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let proj = dir.path().join("proj");
    write_broken_project(&proj, "wb-broken");
    register_project(&db, &proj);
    let script = write_sentinel_script(dir.path());
    let sentinel = dir.path().join("checker-ran");
    std::env::set_var("FORGE_DRIFTWATCH_BIN", &script);
    std::env::set_var("SENTINEL", &sentinel);

    let response = handle(
        &config,
        &db,
        &with_cookie(
            request(
                "GET",
                "/v1/admin/projects/wb-broken",
                &config.frontend_origin,
            ),
            &token,
        ),
        Utc::now(),
    );

    std::env::remove_var("FORGE_DRIFTWATCH_BIN");
    std::env::remove_var("SENTINEL");
    assert_eq!(response.status, 200);
    let body = body_json(&response);
    assert_eq!(
        body["health"]["state"], "issues",
        "local failures still report issues on the fast path: {}",
        body["health"]
    );
    assert!(
        !sentinel.exists(),
        "local issues must surface without invoking the checker"
    );
}

#[test]
fn refresh_is_session_gated_and_leaks_no_data_anonymously() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    forge::identity::global::setup(&db, "operator@example.test", "a-long-test-password").unwrap();
    let proj = dir.path().join("proj");
    write_broken_project(&proj, "wb-gated");
    register_project(&db, &proj);

    let response = handle(
        &config,
        &db,
        &request(
            "POST",
            "/v1/admin/projects/wb-gated/health/refresh",
            &config.frontend_origin,
        ),
        Utc::now(),
    );
    assert_eq!(response.status, 401);
    let text = String::from_utf8_lossy(&response.body).to_string();
    assert!(!text.contains("manifest"), "401 body leaked data: {text}");
    assert!(
        !text.contains(&proj.display().to_string()),
        "absolute path echoed: {text}"
    );
}

#[test]
fn frontend_health_card_pins_the_deferred_state_and_refresh_control() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let app = fs::read_to_string(root.join("frontend/app.js")).unwrap();

    assert!(
        app.contains("deferred"),
        "HEALTH_LABELS must gain the deferred state"
    );
    assert!(
        app.contains("/health/refresh"),
        "the card must call the explicit refresh route"
    );
    assert!(
        app.contains("wb-health-refresh"),
        "the refresh control marker must exist"
    );
    assert!(
        app.contains("Run full health check"),
        "the deferred card offers the full check"
    );
    assert!(
        app.contains("Running full check"),
        "the refresh shows progress while the live pass runs"
    );
    assert!(
        app.contains("policy-deferred"),
        "the card special-cases the deferred finding row"
    );
    // The deferred row must never offer a remediate shortcut: the finding
    // id is branched before the remediate button is built.
    let region = &app[app.find("policy-deferred").unwrap()..];
    assert!(
        region.contains("healthRefreshButton") || region.contains("wb-health-refresh"),
        "the deferred row renders the refresh control"
    );
}
