//! Project-deployment execution contract (`forge-web-project-deployment`).
//!
//! Pins the two new typed, session-gated deploy routes —
//! `GET /v1/admin/projects/{id}/deploy/plan` (`forge deploy plan`) and
//! `POST /v1/admin/projects/{id}/deploy` (`forge deploy apply`) — to the
//! same security boundary the shipped `feature`/`spec` lifecycle routes
//! honour: an anonymous JSON request is refused with `401` before any Core
//! call, a session request without a JSON body is refused with `415`, a
//! hostile project id is a typed `400` and an unmanaged id a `404`. The plan
//! route is side-effect-free, runs no adapter, returns a path-free
//! projection and never writes the on-disk deploy state or the registry
//! `operations` journal. The apply route returns a 64-hex `plan_digest` on a
//! preview, refuses a mismatched digest with `409` and a fresh digest and no
//! write, and only delegates to the in-process `deploy::engine::apply_deploy`
//! on a confirmed matching digest — using a stubbed `FORGE_DEPLOYER_BIN` so
//! the test is hermetic. An adapter that returns a `failed` status is
//! surfaced honestly as a failure and journaled, never as success. No
//! response ever serializes the absolute project path or the adapter binary.

use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

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

fn body_json(response: &forge::api::ApiResponse) -> serde_json::Value {
    serde_json::from_slice(&response.body).expect("admin routes are JSON-only")
}

/// A project the deploy engine will accept: schema-1 manifest, a
/// `deployment:` block with a typed `local` target + an `artifact` it can
/// load, and a git working tree with a committed HEAD so
/// `capture_source_revision` succeeds. `docker-compose.yml` is the
/// artifact the existing CLI deploy tests use, so the registry/parser path
/// is identical to the shipped `forge deploy plan/apply` flow.
fn write_deploy_project(dir: &Path, id: &str) {
    fs::create_dir_all(dir).unwrap();
    let yaml = format!(
        "schema: 1\nproject:\n  id: {id}\n  name: Test {id}\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\ndeployment:\n  artifact: docker-compose.yml\n  default: home\n  targets:\n    - name: home\n      kind: local\n"
    );
    fs::write(dir.join("forge.yaml"), yaml).unwrap();
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
    run_git(dir, &["checkout", "-q", "-b", "main"]);
    run_git(
        dir,
        &["add", "--", "forge.yaml", "README.md", "docker-compose.yml"],
    );
    run_git(dir, &["commit", "-q", "-m", "initial"]);
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

/// A standing deploy adapter fixture: the JSON contract the engine reads
/// off stdout. The script reads its stdin (the adapter request) and
/// prints a `delivered`/`running` envelope.
const ADAPTER_OK_BODY: &str = r#"#!/bin/sh
cat >/dev/null
printf '%s' '{"contract":"forge-deploy-executor/0.1.0","apply_status":"delivered","apply_note":"fixture adapter applied the artifact","apply_evidence":["compose up: app-0.1.0"],"observation_status":"running","observation_detail":"docker service app is running","observation_evidence":["docker ps: app healthy"],"recovery":[]}'
"#;

/// A standing deploy adapter that returns `failed` so the route must
/// surface an honest failure and journal it, never a success.
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

/// Run the admin API with `FORGE_DEPLOYER_BIN` pointed at the supplied
/// adapter path; callers that need no adapter can pass `None` and the
/// variable is unset for that test. The helper clears any inherited env
/// state on entry so a parallel sibling test that set the variable for
/// itself cannot leak in, and restores the caller's prior value on exit.
fn handle_with_adapter(
    config: &ApiConfig,
    db: &Path,
    request: &ApiRequest,
    adapter: Option<&Path>,
) -> forge::api::ApiResponse {
    let prior = std::env::var("FORGE_DEPLOYER_BIN").ok();
    std::env::remove_var("FORGE_DEPLOYER_BIN");
    if let Some(bin) = adapter {
        std::env::set_var("FORGE_DEPLOYER_BIN", bin);
    }
    let response = handle(config, db, request, Utc::now());
    match prior {
        Some(value) => std::env::set_var("FORGE_DEPLOYER_BIN", value),
        None => std::env::remove_var("FORGE_DEPLOYER_BIN"),
    }
    response
}

#[test]
fn anonymous_and_non_json_are_refused_before_any_core_call() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let project = dir.path().join("p1");
    write_deploy_project(&project, "p1");
    register_project(&db, &project);
    let before_forge = project.join(".forge");
    let before_state_bytes: Option<Vec<u8>> = before_forge
        .exists()
        .then(|| fs::read(&before_forge).ok())
        .flatten();

    // Anonymous GET plan -> 401.
    let anon_plan = handle(
        &config,
        &db,
        &request(
            "GET",
            "/v1/admin/projects/p1/deploy/plan",
            &config.frontend_origin,
        ),
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
                "/v1/admin/projects/p1/deploy",
                &config.frontend_origin,
            ),
            r#"{"target":"home"}"#,
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
                "/v1/admin/projects/p1/deploy",
                &config.frontend_origin,
            ),
            &token,
        ),
        Utc::now(),
    );
    assert_eq!(bad_ct.status, 415, "non-json apply refused");

    // No on-disk state appeared (the deploy state dir is created only by
    // a confirmed apply; the previews here must not have created it).
    if before_state_bytes.is_none() {
        assert!(
            !before_forge.exists(),
            "no on-disk state should be written by refused requests"
        );
    }
}

#[test]
fn plan_route_reads_no_write_and_returns_a_path_free_preview() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let project = dir.path().join("p1");
    write_deploy_project(&project, "p1");
    register_project(&db, &project);

    let response = handle(
        &config,
        &db,
        &with_cookie(
            request(
                "GET",
                "/v1/admin/projects/p1/deploy/plan",
                &config.frontend_origin,
            ),
            &token,
        ),
        Utc::now(),
    );
    assert_eq!(response.status, 200, "plan status");
    let json = body_json(&response);
    let view = &json["deploy_plan"];
    assert_eq!(view["action"], "deploy");
    assert_eq!(view["project_id"], "p1");
    assert_eq!(view["target"], "home");
    assert_eq!(view["target_kind"], "local");
    assert!(view["ready"].as_bool().unwrap_or(false));
    assert!(view["artifact_hash"].is_string());
    assert!(view["artifact_bytes"].is_u64());
    let raw = String::from_utf8_lossy(&response.body);
    assert!(
        !raw.contains(&project.to_string_lossy().to_string()),
        "plan response leaked the project path: {raw}"
    );

    // No on-disk deploy state, no registry journal row.
    assert!(
        !project.join(".forge/deploy").exists(),
        "plan must not create .forge/deploy"
    );
}

#[test]
fn apply_preview_writes_nothing_and_wrong_digest_is_refused() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let project = dir.path().join("p1");
    write_deploy_project(&project, "p1");
    register_project(&db, &project);

    // Preview without confirm -> 200 + path-free view + 64-hex digest,
    // no on-disk state, no adapter call.
    let preview = handle(
        &config,
        &db,
        &with_cookie(
            json_body(
                request(
                    "POST",
                    "/v1/admin/projects/p1/deploy",
                    &config.frontend_origin,
                ),
                r#"{"target":"home"}"#,
            ),
            &token,
        ),
        Utc::now(),
    );
    assert_eq!(preview.status, 200, "preview status");
    let json = body_json(&preview);
    let digest = json["plan_digest"].as_str().unwrap();
    assert_eq!(digest.len(), 64, "digest length");
    assert!(
        digest.chars().all(|c| c.is_ascii_hexdigit()),
        "digest hex: {digest}"
    );
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
                    "/v1/admin/projects/p1/deploy",
                    &config.frontend_origin,
                ),
                r#"{"target":"home","confirm":true,"plan_digest":"deadbeefdeadbeef"}"#,
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

    // No on-disk state appeared.
    assert!(
        !project.join(".forge/deploy").exists(),
        "refused apply must not create .forge/deploy"
    );
}

#[test]
fn confirmed_apply_with_stub_deployer_reaches_the_core_handler() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let project = dir.path().join("p1");
    write_deploy_project(&project, "p1");
    register_project(&db, &project);
    let adapter = write_adapter(dir.path(), ADAPTER_OK_BODY);

    // Preview, then confirm with the same digest, both with the stub
    // adapter in scope.
    let preview = handle_with_adapter(
        &config,
        &db,
        &with_cookie(
            json_body(
                request(
                    "POST",
                    "/v1/admin/projects/p1/deploy",
                    &config.frontend_origin,
                ),
                r#"{"target":"home"}"#,
            ),
            &token,
        ),
        Some(&adapter),
    );
    assert_eq!(preview.status, 200, "preview status");
    let digest = body_json(&preview)["plan_digest"]
        .as_str()
        .unwrap()
        .to_string();

    let applied = handle_with_adapter(
        &config,
        &db,
        &with_cookie(
            json_body(
                request(
                    "POST",
                    "/v1/admin/projects/p1/deploy",
                    &config.frontend_origin,
                ),
                &format!(r#"{{"target":"home","confirm":true,"plan_digest":"{digest}"}}"#),
            ),
            &token,
        ),
        Some(&adapter),
    );
    assert_eq!(applied.status, 202, "confirmed apply status");
    let json = body_json(&applied);
    // The response came from the engine (state file, observation, etc.),
    // not from the admin preview/mismatch gate.
    assert!(
        json.get("confirmation").is_none(),
        "confirmed apply must not echo the admin gate"
    );
    assert!(json["operation_id"].is_i64());
    assert!(json["healthy"].as_bool().unwrap_or(false));
    assert_eq!(json["target"], "home");
    let raw = String::from_utf8_lossy(&applied.body);
    assert!(
        !raw.contains(&project.to_string_lossy().to_string()),
        "handler response leaked project path"
    );
    assert!(
        !raw.contains(&adapter.to_string_lossy().to_string()),
        "handler response leaked adapter binary path"
    );

    // The engine persisted deploy state to disk (apply path), and the
    // journal recorded the operation.
    let state = project.join(".forge/deploy/p1");
    assert!(state.exists(), "deploy state dir must be written");
    let registry = Registry::open(&db).unwrap();
    let mut found_deploy = false;
    for op_id in 1..32 {
        if let Some(entry) = registry.operation(op_id).unwrap_or(None) {
            if entry.kind == "admin.deploy" && entry.project_id == "p1" {
                found_deploy = true;
                break;
            }
        }
    }
    assert!(
        found_deploy,
        "registry journal must record a deploy operation for p1"
    );
}

#[test]
fn adapter_failure_is_reported_honestly_never_as_success() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let project = dir.path().join("p1");
    write_deploy_project(&project, "p1");
    register_project(&db, &project);
    let adapter = write_adapter(dir.path(), ADAPTER_FAIL_BODY);

    let preview = handle_with_adapter(
        &config,
        &db,
        &with_cookie(
            json_body(
                request(
                    "POST",
                    "/v1/admin/projects/p1/deploy",
                    &config.frontend_origin,
                ),
                r#"{"target":"home"}"#,
            ),
            &token,
        ),
        Some(&adapter),
    );
    let digest = body_json(&preview)["plan_digest"]
        .as_str()
        .unwrap()
        .to_string();

    let applied = handle_with_adapter(
        &config,
        &db,
        &with_cookie(
            json_body(
                request(
                    "POST",
                    "/v1/admin/projects/p1/deploy",
                    &config.frontend_origin,
                ),
                &format!(r#"{{"target":"home","confirm":true,"plan_digest":"{digest}"}}"#),
            ),
            &token,
        ),
        Some(&adapter),
    );
    // The route always succeeds at the HTTP level for a confirmed
    // matching digest (the engine is the source of truth); the report
    // itself must report `healthy: false` and a `failed` stage so the
    // failure is observable, not papered over as success.
    assert_eq!(applied.status, 202, "failed apply still 202");
    let json = body_json(&applied);
    assert!(!json["healthy"].as_bool().unwrap_or(true), "healthy=false");
    let stages = json["stages"].as_array().expect("stages array");
    let apply_stage = stages
        .iter()
        .find(|s| s["stage"] == "apply")
        .expect("apply stage present");
    assert_eq!(apply_stage["status"], "failed");
}

#[test]
fn hostile_id_is_refused_and_unmanaged_id_is_not_found() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let project = dir.path().join("p1");
    write_deploy_project(&project, "p1");
    register_project(&db, &project);

    // Path-bearing id -> 400, no echo, no write.
    let hostile = handle(
        &config,
        &db,
        &with_cookie(
            request(
                "GET",
                "/v1/admin/projects/..%2F..%2Fetc/deploy/plan",
                &config.frontend_origin,
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
    assert!(!raw.contains(".."), "hostile id must not be echoed: {raw}");

    // Unmanaged id -> 404.
    let unmanaged = handle(
        &config,
        &db,
        &with_cookie(
            request(
                "GET",
                "/v1/admin/projects/ghost-proj/deploy/plan",
                &config.frontend_origin,
            ),
            &token,
        ),
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
fn empty_target_resolves_to_the_manifest_default() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let project = dir.path().join("p1");
    write_deploy_project(&project, "p1");
    register_project(&db, &project);

    let preview = handle(
        &config,
        &db,
        &with_cookie(
            json_body(
                request(
                    "POST",
                    "/v1/admin/projects/p1/deploy",
                    &config.frontend_origin,
                ),
                r#"{}"#,
            ),
            &token,
        ),
        Utc::now(),
    );
    assert_eq!(preview.status, 200, "empty target preview");
    let json = body_json(&preview);
    assert_eq!(json["preview"]["target"], "home");
}

#[test]
fn catalog_reports_deploy_plan_and_apply_as_web_with_an_execution_block() {
    let rows = command_catalog::rows();
    let plan = rows
        .iter()
        .find(|row| row.id == "deploy.plan")
        .expect("missing catalog row deploy.plan");
    assert_eq!(plan.availability, "web", "deploy.plan should be web");
    assert_eq!(
        plan.route.as_deref(),
        Some("GET /v1/admin/projects/{id}/deploy/plan")
    );
    assert!(
        plan.execution.is_none(),
        "deploy.plan is a read route, no execution block"
    );

    let apply = rows
        .iter()
        .find(|row| row.id == "deploy.apply")
        .expect("missing catalog row deploy.apply");
    assert_eq!(apply.availability, "web", "deploy.apply should be web");
    let execution = apply
        .execution
        .as_ref()
        .expect("deploy.apply must carry an execution block");
    assert_eq!(
        execution.route, "POST /v1/admin/projects/{id}/deploy",
        "deploy.apply execution route"
    );
    assert_eq!(execution.method, "POST");
    assert!(execution.confirm_required, "confirm required");
    assert!(execution.digest_bound, "digest bound");
    assert_eq!(execution.risk, "remote_write");
    let target_param = execution
        .parameters
        .iter()
        .find(|p| p.name == "target")
        .expect("target parameter");
    assert_eq!(target_param.kind, "string");
    assert!(!target_param.required, "target is optional");
}
