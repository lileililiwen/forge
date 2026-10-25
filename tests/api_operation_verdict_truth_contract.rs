//! HTTP contract for the API operation journal state.
//!
//! One rule names what an operation row records: the outcome the request
//! earned. A request that only planned is a planned operation, a request
//! that applied the work is a completed operation, a request the shared
//! rule refused is a blocked operation and a run that did not finish is a
//! failed operation. The shared operation boundary reads that rule; no
//! route writes a state of its own.
//!
//! - a dry-run deployment journals a planned operation;
//! - a blocked deploy plan journals a blocked operation;
//! - a dry-run upgrade journals a planned operation;
//! - an applied feature request still journals a completed operation;
//! - a failed deploy run journals a failed operation, never a completed
//!   one.
//!
//! The journal rows are read from the registry's own SQLite database, the
//! way the other API contract tests read them.

use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

use chrono::Utc;
use forge::api::{handle, ApiConfig, ApiRequest, ApiResponse};
use forge::registry::Registry;

/// The bearer session the routes authorize against.
const SESSION_ID: &str = "a1b2c3d4e5f60718";

/// `FORGE_DEPLOYER_BIN` is a process-global setting read by the deploy
/// engine inside the apply route, so every test in this binary serializes
/// on one lock to keep parallel tests from racing each other's adapter.
static SERIAL: Mutex<()> = Mutex::new(());

/// Lock the serialization guard, tolerating a poisoned lock so a prior
/// panic does not cascade into unrelated failures.
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

fn json_body(mut req: ApiRequest, body: &[u8]) -> ApiRequest {
    req.headers
        .insert("content-type".to_string(), "application/json".to_string());
    req.body = body.to_vec();
    req
}

/// The project the deploy engine accepts: a schema-1 manifest with a
/// `deployment:` block (a typed `local` target and an artifact it can
/// load) and a git working tree with a committed HEAD, so the revision
/// the plan names is a real one.
fn write_project(dir: &Path, id: &str) {
    fs::create_dir_all(dir).unwrap();
    let text = format!(
        "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\nfeatures: {{}}\ndeployment:\n  artifact: docker-compose.yml\n  default: home\n  targets:\n    - name: home\n      kind: local\n"
    );
    fs::write(dir.join("forge.yaml"), text).unwrap();
    fs::write(dir.join("README.md"), "v1\n").unwrap();
    fs::write(
        dir.join("docker-compose.yml"),
        "services:\n  app:\n    image: app:0.1.0\n",
    )
    .unwrap();
    run_git(dir, &["init", "-q"]);
    run_git(dir, &["config", "user.email", "forge@example.com"]);
    run_git(dir, &["config", "user.name", "Forge Test"]);
    run_git(dir, &["config", "init.defaultBranch", "main"]);
    run_git(
        dir,
        &["add", "--", "forge.yaml", "README.md", "docker-compose.yml"],
    );
    run_git(dir, &["commit", "-q", "-m", "initial"]);
}

fn run_git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .expect("git");
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// The project's committed HEAD.
fn head_revision(dir: &Path) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .arg("rev-parse")
        .arg("HEAD")
        .output()
        .expect("git rev-parse");
    assert!(out.status.success(), "git rev-parse failed");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// The Gate evidence that satisfies the shared delivery rule, so a plan
/// is ready and an apply is allowed.
fn write_gate_evidence(dir: &Path, id: &str, revision: &str) {
    let evidence = serde_json::json!({
        "contract": forge::gate::GATE_CONTRACT_VERSION,
        "project_id": id,
        "runtime": "forge",
        "profile": "full",
        "revision": revision,
        "aggregate": "passed",
        "checks": [],
        "observed_at": "2026-10-25T00:00:00Z",
        "dry_run": false
    });
    let path = dir.join(".forge/gate").join(id).join("evidence.json");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, serde_json::to_vec(&evidence).unwrap()).unwrap();
}

/// An operator session with the `admin:access` permission the routes
/// demand, seeded through the same file shape the Core persists.
fn seed_identity_session(project: &Path, project_id: &str, session_id: &str) {
    let now = Utc::now();
    let session = forge::identity::AdminSession {
        session_id: session_id.to_string(),
        project_id: project_id.to_string(),
        provider: "okta".to_string(),
        subject: "user-1".to_string(),
        issued_at: now,
        expires_at: now + chrono::Duration::seconds(3600),
        permissions: vec!["admin:access".to_string()],
        state: forge::identity::SessionState::Active,
        note: None,
    };
    forge::identity::save_session(project, project_id, &session).unwrap();
}

/// A registered project with a deploy target, and with the Gate evidence
/// when the caller asks for a ready plan.
fn fixture(id: &str, gated: bool) -> (tempfile::TempDir, PathBuf, ApiConfig, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let project = tmp.path().join("proj");
    write_project(&project, id);
    if gated {
        write_gate_evidence(&project, id, &head_revision(&project));
    }
    {
        let mut registry = Registry::open(&db).unwrap();
        registry.register(&project, None).unwrap();
    }
    seed_identity_session(&project, id, SESSION_ID);
    let config = ApiConfig::default();
    (tmp, db, config, project)
}

/// The journal rows of one kind, oldest first.
fn journal_rows(db: &Path, kind: &str) -> Vec<(String, String)> {
    Registry::open(db)
        .unwrap()
        .journal_entries()
        .unwrap()
        .iter()
        .filter(|e| e.kind == kind)
        .map(|e| (e.state.clone(), e.detail.clone().unwrap_or_default()))
        .collect()
}

/// Post a JSON body to a project route with the bearer session.
fn post(
    config: &ApiConfig,
    db: &Path,
    token: &str,
    path: &str,
    body: &[u8],
) -> (u16, serde_json::Value) {
    post_with_adapter(config, db, token, path, body, None)
}

/// The same call, with the deploy adapter pointed at a fixture.
///
/// The adapter variable is process-global, so it is only touched when a
/// fixture is supplied: a call that needs no adapter must not clear the
/// variable a sibling test is using.
fn post_with_adapter(
    config: &ApiConfig,
    db: &Path,
    token: &str,
    path: &str,
    body: &[u8],
    adapter: Option<&Path>,
) -> (u16, serde_json::Value) {
    let prior = std::env::var("FORGE_DEPLOYER_BIN").ok();
    if let Some(path) = adapter {
        std::env::set_var("FORGE_DEPLOYER_BIN", path);
    }
    let mut req = request("POST", path, &config.frontend_origin);
    req.bearer_token = Some(token.to_string());
    req = json_body(req, body);
    let response: ApiResponse = handle(config, db, &req, Utc::now());
    if adapter.is_some() {
        match prior {
            Some(value) => std::env::set_var("FORGE_DEPLOYER_BIN", value),
            None => std::env::remove_var("FORGE_DEPLOYER_BIN"),
        }
    }
    let value = serde_json::from_slice(&response.body).unwrap_or_else(|err| {
        panic!(
            "invalid json: {err}; body={}",
            String::from_utf8_lossy(&response.body)
        )
    });
    (response.status, value)
}

/// A deploy adapter that acknowledges the apply and reports a `running`
/// observation.
const ADAPTER_OK_BODY: &str = r#"#!/bin/sh
cat >/dev/null
printf '%s' '{"contract":"forge-deploy-executor/0.1.0","apply_status":"delivered","apply_note":"fixture adapter applied the artifact","apply_evidence":["compose up: app-0.1.0"],"observation_status":"running","observation_detail":"docker service app is running","observation_evidence":["docker ps: app healthy"],"recovery":[]}'
"#;

/// A deploy adapter that cannot apply the artifact.
const ADAPTER_FAIL_BODY: &str = r#"#!/bin/sh
cat >/dev/null
printf '%s' '{"contract":"forge-deploy-executor/0.1.0","apply_status":"failed","apply_note":"fixture adapter could not apply the artifact","apply_evidence":["docker compose up failed"],"observation_status":"unknown","observation_detail":"target unreachable","observation_evidence":[],"recovery":["re-run forge deploy apply with --confirm"]}'
"#;

fn write_adapter(dir: &Path, body: &str) -> PathBuf {
    let path = dir.join("forge-deployer-fixture.sh");
    fs::write(&path, body).unwrap();
    let mut perms = fs::metadata(&path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&path, perms).unwrap();
    path
}

#[test]
fn a_dry_run_deployment_journals_a_planned_operation() {
    // The route computed a plan: the body is the plan, nothing was
    // deployed, and the row must not claim a completed operation.
    let (_tmp, db, config, _project) = fixture("api-op-plan", true);
    let (status, value) = post(
        &config,
        &db,
        SESSION_ID,
        "/v1/projects/api-op-plan/deployments",
        br#"{"confirm":true,"dry_run":true}"#,
    );
    assert_eq!(status, 202, "the route answers with its plan: {value}");
    assert_eq!(value["deploy"]["ready"], true, "{value}");

    let rows = journal_rows(&db, "api.deploy");
    assert_eq!(rows.len(), 1, "one row per request: {rows:?}");
    let (state, detail) = &rows[0];
    assert_eq!(
        state, "planned",
        "a plan is not a completed operation; detail={detail}"
    );
    assert_ne!(state, "done", "detail={detail}");
    assert!(
        detail.contains("api.deploy planned"),
        "the detail names the run: {detail}"
    );
}

#[test]
fn a_blocked_deploy_plan_journals_a_blocked_operation() {
    // Without the Gate evidence the shared delivery rule refuses the
    // revision, so the plan is blocked and the row says so.
    let (_tmp, db, config, _project) = fixture("api-op-blocked", false);
    let (status, value) = post(
        &config,
        &db,
        SESSION_ID,
        "/v1/projects/api-op-blocked/deployments",
        br#"{"confirm":true,"dry_run":true}"#,
    );
    assert_eq!(status, 202, "the route answers with its plan: {value}");
    assert_eq!(value["deploy"]["ready"], false, "{value}");

    let rows = journal_rows(&db, "api.deploy");
    assert_eq!(rows.len(), 1, "one row per request: {rows:?}");
    let (state, detail) = &rows[0];
    assert_eq!(state, "blocked", "detail={detail}");
    assert!(
        detail.contains("api.deploy blocked"),
        "the detail names the run: {detail}"
    );
}

#[test]
fn a_dry_run_upgrade_journals_a_planned_operation() {
    // The upgrade route planned without changing anything, so its row
    // names a plan.
    let (_tmp, db, config, _project) = fixture("api-op-upgrade", true);
    let (status, value) = post(
        &config,
        &db,
        SESSION_ID,
        "/v1/projects/api-op-upgrade/upgrade",
        br#"{"confirm":true,"dry_run":true}"#,
    );
    assert_eq!(status, 202, "the route answers with its plan: {value}");
    assert!(
        value["upgrade"]["steps"].is_array(),
        "the body is the plan: {value}"
    );

    let rows = journal_rows(&db, "api.upgrade");
    assert_eq!(rows.len(), 1, "one row per request: {rows:?}");
    let (state, detail) = &rows[0];
    assert_eq!(
        state, "planned",
        "a plan is not a completed upgrade; detail={detail}"
    );
    assert!(
        detail.contains("api.upgrade planned"),
        "the detail names the run: {detail}"
    );
}

#[test]
fn an_applied_request_still_journals_a_completed_operation() {
    // The applied work keeps the completed operation it had.
    let (_tmp, db, config, _project) = fixture("api-op-feature", true);
    let (status, value) = post(
        &config,
        &db,
        SESSION_ID,
        "/v1/projects/api-op-feature/features",
        br#"{"feature":"auth"}"#,
    );
    assert_eq!(status, 202, "the route answers with its outcome: {value}");

    let rows = journal_rows(&db, "api.add_feature");
    assert_eq!(rows.len(), 1, "one row per request: {rows:?}");
    let (state, detail) = &rows[0];
    assert_eq!(state, "done", "detail={detail}");
    assert!(
        detail.contains("api.add_feature completed"),
        "the detail names the run: {detail}"
    );
}

#[test]
fn a_failed_deploy_run_is_not_a_completed_operation() {
    // The adapter could not apply the artifact: the report's own verdict
    // is `failed`, so the row must not claim a completed operation.
    let _guard = lock();
    let (tmp, db, config, _project) = fixture("api-op-failed", true);
    let adapter = write_adapter(tmp.path(), ADAPTER_FAIL_BODY);

    let (status, value) = post_with_adapter(
        &config,
        &db,
        SESSION_ID,
        "/v1/projects/api-op-failed/deployments",
        br#"{"confirm":true}"#,
        Some(&adapter),
    );
    assert_eq!(status, 202, "the route answers with the report: {value}");
    assert_eq!(value["deploy"]["verdict"], "failed", "{value}");

    let rows = journal_rows(&db, "api.deploy");
    assert_eq!(rows.len(), 1, "one row per request: {rows:?}");
    let (state, detail) = &rows[0];
    assert_eq!(
        state, "failed",
        "a run that did not finish is not a completed operation; detail={detail}"
    );
    assert_ne!(state, "done", "detail={detail}");
    assert!(
        detail.contains("api.deploy failed"),
        "the detail names the run: {detail}"
    );
}

#[test]
fn a_delivered_deploy_run_journals_a_completed_operation() {
    // The applied deploy is unchanged: it still journals a completed
    // operation.
    let _guard = lock();
    let (tmp, db, config, _project) = fixture("api-op-delivered", true);
    let adapter = write_adapter(tmp.path(), ADAPTER_OK_BODY);

    let (status, value) = post_with_adapter(
        &config,
        &db,
        SESSION_ID,
        "/v1/projects/api-op-delivered/deployments",
        br#"{"confirm":true}"#,
        Some(&adapter),
    );
    assert_eq!(status, 202, "the route answers with the report: {value}");
    assert_eq!(value["deploy"]["verdict"], "delivered", "{value}");

    let rows = journal_rows(&db, "api.deploy");
    assert_eq!(rows.len(), 1, "one row per request: {rows:?}");
    let (state, detail) = &rows[0];
    assert_eq!(state, "done", "detail={detail}");
    assert!(
        detail.contains("api.deploy completed"),
        "the detail names the run: {detail}"
    );
}
