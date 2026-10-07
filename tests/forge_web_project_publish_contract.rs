//! Project provider-publish execution contract (`forge-web-project-publish`).
//!
//! Pins the two new typed, session-gated publish routes —
//! `GET /v1/admin/projects/{id}/publish/plan` (the read-only provider plan) and
//! `POST /v1/admin/projects/{id}/publish` (`forge publish` apply) — to the same
//! security boundary the shipped `feature`/`spec`/`deploy`/`release` lifecycle
//! routes honour: an anonymous JSON request is refused with `401` before any
//! Core call, a session request without a JSON body is refused with `415`, a
//! hostile project id is a typed `400`, an unmanaged id a `404`, and an unset
//! provider, an unreadable provider configuration, an unknown provider or a
//! missing commit are typed `409 admin-prerequisite` responses that never echo
//! a value, provider id or absolute path. The plan route is side-effect-free:
//! it resolves the provider and revision server-side, runs no provider and
//! writes no journal row. The apply route returns a 64-hex `plan_digest` on a
//! preview, refuses a mismatched digest with `409` and a fresh digest and no
//! write, and only invokes the server-configured provider on a confirmed,
//! matching digest.
//!
//! The provider is a hermetic `#!/bin/sh` stub staged at run time that speaks
//! the `forge-publish-provider/0.1.0` envelope on stdin/stdout and logs the
//! request it received — no network, no Mac, no Jenkins and no credentials
//! ever leave the process. A failing stub is surfaced honestly as a typed
//! non-success and journaled `failed`, never as success. No response ever
//! serializes the absolute project path or the provider executable.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

use chrono::Utc;
use forge::api::{command_catalog, handle, ApiConfig, ApiRequest};
use forge::registry::{OperationEntry, Registry};
use serde_json::{json, Value};
use tempfile::tempdir;

/// `FORGE_PUBLISH_PROVIDER` / `FORGE_PUBLISH_PROVIDER_CONFIG` are
/// process-global settings read by the route inside the apply/plan call, so
/// every test in this binary serializes on one lock to keep parallel tests from
/// racing each other's provider selection.
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

fn plan_request(config: &ApiConfig, id: &str) -> ApiRequest {
    request(
        "GET",
        &format!("/v1/admin/projects/{id}/publish/plan"),
        &config.frontend_origin,
    )
}

fn apply_request(config: &ApiConfig, id: &str) -> ApiRequest {
    request(
        "POST",
        &format!("/v1/admin/projects/{id}/publish"),
        &config.frontend_origin,
    )
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

fn body_json(response: &forge::api::ApiResponse) -> Value {
    serde_json::from_slice(&response.body).expect("admin routes are JSON-only")
}

/// Set/restore the process-global publish provider env for the duration of a
/// test so a parallel sibling can never observe another test's selection.
struct PublishEnv {
    provider: Option<String>,
    config: Option<OsString>,
}

impl PublishEnv {
    fn set(provider: &str, config: Option<&Path>) -> Self {
        let guard = Self::capture();
        std::env::set_var("FORGE_PUBLISH_PROVIDER", provider);
        match config {
            Some(path) => std::env::set_var("FORGE_PUBLISH_PROVIDER_CONFIG", path),
            None => std::env::remove_var("FORGE_PUBLISH_PROVIDER_CONFIG"),
        }
        guard
    }

    fn unset() -> Self {
        let guard = Self::capture();
        std::env::remove_var("FORGE_PUBLISH_PROVIDER");
        std::env::remove_var("FORGE_PUBLISH_PROVIDER_CONFIG");
        guard
    }

    fn capture() -> Self {
        Self {
            provider: std::env::var("FORGE_PUBLISH_PROVIDER").ok(),
            config: std::env::var_os("FORGE_PUBLISH_PROVIDER_CONFIG"),
        }
    }
}

impl Drop for PublishEnv {
    fn drop(&mut self) {
        match &self.provider {
            Some(value) => std::env::set_var("FORGE_PUBLISH_PROVIDER", value),
            None => std::env::remove_var("FORGE_PUBLISH_PROVIDER"),
        }
        match &self.config {
            Some(value) => std::env::set_var("FORGE_PUBLISH_PROVIDER_CONFIG", value),
            None => std::env::remove_var("FORGE_PUBLISH_PROVIDER_CONFIG"),
        }
    }
}

/// A managed project the publish route accepts: a schema-1 manifest, a git
/// working tree with a committed HEAD, and a `.forge/providers.yaml` naming the
/// supplied stub executable. The provider executable, provider id, revision and
/// project directory all stay server-side.
fn write_publish_project(dir: &Path, id: &str, provider_command: &Path) {
    fs::create_dir_all(dir).unwrap();
    fs::write(
        dir.join("forge.yaml"),
        format!(
            "schema: 1\nproject:\n  id: {id}\n  name: Test {id}\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\n"
        ),
    )
    .unwrap();
    fs::write(dir.join("README.md"), "v1\n").unwrap();
    run_git(dir, &["init", "-q"]);
    run_git(dir, &["config", "user.email", "forge@example.com"]);
    run_git(dir, &["config", "user.name", "Forge Test"]);
    run_git(dir, &["config", "init.defaultBranch", "main"]);
    run_git(dir, &["checkout", "-q", "-b", "main"]);
    run_git(dir, &["add", "-A"]);
    run_git(dir, &["commit", "-q", "-m", "initial"]);
    write_provider_config(dir, provider_command);
}

fn write_provider_config(project_dir: &Path, command: &Path) {
    let forge_dir = project_dir.join(".forge");
    fs::create_dir_all(&forge_dir).unwrap();
    fs::write(
        forge_dir.join("providers.yaml"),
        format!(
            "providers:\n  - id: fixture\n    command: \"{}\"\n",
            command.display()
        ),
    )
    .unwrap();
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

fn git_head(dir: &Path) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["rev-parse", "HEAD"])
        .output()
        .expect("git rev-parse");
    assert!(out.status.success(), "git rev-parse failed");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn register_project(db: &Path, dir: &Path) {
    let mut registry = Registry::open(db).unwrap();
    registry.register(dir, None).unwrap();
}

fn write_stub(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, body).unwrap();
    let mut perms = fs::metadata(&path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&path, perms).unwrap();
    path
}

/// A hermetic provider stub: drains the request on stdin into a log, emits a
/// valid `forge-publish-provider/0.1.0` response carrying the supplied phase
/// fields, then exits with `exit_code`. It contacts nothing.
fn stub_provider(log: &Path, status: &str, health: &str, exit_code: i32) -> String {
    let log = log.display();
    let response = json!({
        "contract": "forge-publish-provider/0.1.0",
        "provider": "fixture",
        "operation_id": "publish-stub",
        "status": status,
        "health": health,
        "evidence": ["stub provider round-tripped the request"],
        "recovery": [],
        "build_status": "succeeded",
        "run_status": "succeeded",
        "container_identity": "forge-stub-deadbeefcafe",
    });
    let body = serde_json::to_string(&response).unwrap();
    format!(
        "#!/bin/sh\n\
         log=\"{log}\"\n\
         cat > \"${{log}}.in\"\n\
         cat \"${{log}}.in\" >> \"$log\"\n\
         printf '%s\\n' '{body}'\n\
         exit {exit_code}\n"
    )
}

/// Every `publish` journal row recorded for project `p1`.
fn publish_rows(db: &Path) -> Vec<OperationEntry> {
    let registry = Registry::open(db).unwrap();
    let mut rows = Vec::new();
    for op_id in 1..64 {
        if let Ok(Some(entry)) = registry.operation(op_id) {
            if entry.kind == "publish" && entry.project_id == "p1" {
                rows.push(entry);
            }
        }
    }
    rows
}

#[test]
fn anonymous_and_non_json_are_refused_before_any_provider_call() {
    let _guard = lock();
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let log = dir.path().join("provider.log");
    let stub = write_stub(
        dir.path(),
        "stub.sh",
        &stub_provider(&log, "done", "healthy", 0),
    );
    let project = dir.path().join("p1");
    write_publish_project(&project, "p1", &stub);
    register_project(&db, &project);
    let _env = PublishEnv::set("fixture", None);

    // Anonymous GET plan -> 401.
    let anon_plan = handle(&config, &db, &plan_request(&config, "p1"), Utc::now());
    assert_eq!(anon_plan.status, 401, "anonymous plan refused");

    // Anonymous JSON POST apply -> 401.
    let anon_apply = handle(
        &config,
        &db,
        &json_body(apply_request(&config, "p1"), "{}"),
        Utc::now(),
    );
    assert_eq!(anon_apply.status, 401, "anonymous apply refused");

    // Session, non-JSON POST -> 415.
    let bad_ct = handle(
        &config,
        &db,
        &with_cookie(apply_request(&config, "p1"), &token),
        Utc::now(),
    );
    assert_eq!(bad_ct.status, 415, "non-json apply refused");

    // The provider was never invoked and no journal row appeared.
    assert!(
        !log.exists(),
        "refused requests must not invoke the provider"
    );
    assert!(
        publish_rows(&db).is_empty(),
        "refused requests must not journal"
    );
}

#[test]
fn plan_route_reads_no_write_and_returns_a_path_free_preview() {
    let _guard = lock();
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let log = dir.path().join("provider.log");
    let stub = write_stub(
        dir.path(),
        "stub.sh",
        &stub_provider(&log, "done", "healthy", 0),
    );
    let project = dir.path().join("p1");
    write_publish_project(&project, "p1", &stub);
    register_project(&db, &project);
    let _env = PublishEnv::set("fixture", None);

    let response = handle(
        &config,
        &db,
        &with_cookie(plan_request(&config, "p1"), &token),
        Utc::now(),
    );
    assert_eq!(response.status, 200, "plan status");
    let json = body_json(&response);
    let view = &json["publish_plan"];
    assert_eq!(view["action"], "publish");
    assert_eq!(view["project_id"], "p1");
    assert_eq!(view["provider"], "fixture");
    assert_eq!(view["operation"], "publish");
    let revision = view["revision"].as_str().expect("revision");
    assert_eq!(revision.len(), 40, "plan revision is a full SHA");
    assert!(revision.chars().all(|c| c.is_ascii_hexdigit()));
    assert_eq!(
        revision,
        git_head(&project),
        "plan binds the committed HEAD"
    );
    let digest = json["plan_digest"].as_str().unwrap();
    assert_eq!(digest.len(), 64, "digest length");
    assert!(digest.chars().all(|c| c.is_ascii_hexdigit()));
    let raw = String::from_utf8_lossy(&response.body);
    assert!(
        !raw.contains(&dir.path().to_string_lossy().to_string()),
        "plan response leaked an absolute path: {raw}"
    );
    assert!(
        !raw.contains(&stub.to_string_lossy().to_string()),
        "plan response leaked the provider executable"
    );

    // The plan invokes no provider and journals nothing.
    assert!(!log.exists(), "plan must not invoke the provider");
    assert!(publish_rows(&db).is_empty(), "plan must not journal");
}

#[test]
fn missing_prerequisite_is_a_typed_409_without_echo() {
    let _guard = lock();
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let log = dir.path().join("provider.log");
    let stub = write_stub(
        dir.path(),
        "stub.sh",
        &stub_provider(&log, "done", "healthy", 0),
    );
    let project = dir.path().join("p1");
    write_publish_project(&project, "p1", &stub);
    register_project(&db, &project);

    // Unset provider -> 409 naming the variable, never a value.
    {
        let _env = PublishEnv::unset();
        let response = handle(
            &config,
            &db,
            &with_cookie(plan_request(&config, "p1"), &token),
            Utc::now(),
        );
        assert_eq!(response.status, 409, "unset provider refused");
        let json = body_json(&response);
        assert_eq!(json["error"]["code"], "admin-prerequisite");
        let message = json["error"]["message"].as_str().unwrap();
        assert!(
            message.contains("FORGE_PUBLISH_PROVIDER"),
            "message must name the variable: {message}"
        );
    }

    // Configured provider but an unreadable configuration path -> 409 without
    // echoing the path.
    {
        let missing = dir.path().join("nope/providers.yaml");
        let _env = PublishEnv::set("fixture", Some(&missing));
        let response = handle(
            &config,
            &db,
            &with_cookie(plan_request(&config, "p1"), &token),
            Utc::now(),
        );
        assert_eq!(response.status, 409, "missing config refused");
        let json = body_json(&response);
        assert_eq!(json["error"]["code"], "admin-prerequisite");
        let raw = String::from_utf8_lossy(&response.body);
        assert!(
            !raw.contains("nope"),
            "configuration path must not be echoed: {raw}"
        );
    }

    // Configured provider id that the configuration does not name -> 409
    // without echoing the id.
    {
        let _env = PublishEnv::set("ghost-provider", None);
        let response = handle(
            &config,
            &db,
            &with_cookie(plan_request(&config, "p1"), &token),
            Utc::now(),
        );
        assert_eq!(response.status, 409, "unknown provider refused");
        let json = body_json(&response);
        assert_eq!(json["error"]["code"], "admin-prerequisite");
        let raw = String::from_utf8_lossy(&response.body);
        assert!(
            !raw.contains("ghost-provider"),
            "provider id must not be echoed: {raw}"
        );
    }

    assert!(
        !log.exists(),
        "a refused prerequisite never invokes the provider"
    );
    assert!(
        publish_rows(&db).is_empty(),
        "a refused prerequisite never journals"
    );
}

#[test]
fn hostile_id_is_refused_and_unmanaged_id_is_not_found() {
    let _guard = lock();
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let log = dir.path().join("provider.log");
    let stub = write_stub(
        dir.path(),
        "stub.sh",
        &stub_provider(&log, "done", "healthy", 0),
    );
    let project = dir.path().join("p1");
    write_publish_project(&project, "p1", &stub);
    register_project(&db, &project);
    let _env = PublishEnv::set("fixture", None);

    // Path-bearing id -> 400, no echo.
    let hostile = handle(
        &config,
        &db,
        &with_cookie(plan_request(&config, "..%2F..%2Fetc"), &token),
        Utc::now(),
    );
    assert_eq!(hostile.status, 400, "hostile id refused");
    assert_eq!(
        body_json(&hostile)["error"]["code"],
        "admin-invalid-project-id"
    );
    let raw = String::from_utf8_lossy(&hostile.body);
    assert!(!raw.contains(".."), "hostile id must not be echoed: {raw}");

    // Unmanaged id -> 404.
    let unmanaged = handle(
        &config,
        &db,
        &with_cookie(plan_request(&config, "ghost-proj"), &token),
        Utc::now(),
    );
    assert_eq!(unmanaged.status, 404, "unmanaged id not found");
    assert_eq!(
        body_json(&unmanaged)["error"]["code"],
        "admin-project-unmanaged"
    );

    assert!(!log.exists(), "id failures never invoke the provider");
    assert!(publish_rows(&db).is_empty(), "id failures never journal");
}

#[test]
fn apply_preview_and_wrong_digest_write_nothing() {
    let _guard = lock();
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let log = dir.path().join("provider.log");
    let stub = write_stub(
        dir.path(),
        "stub.sh",
        &stub_provider(&log, "done", "healthy", 0),
    );
    let project = dir.path().join("p1");
    write_publish_project(&project, "p1", &stub);
    register_project(&db, &project);
    let _env = PublishEnv::set("fixture", None);

    // Preview without confirm -> 200 + path-free view + 64-hex digest.
    let preview = handle(
        &config,
        &db,
        &with_cookie(json_body(apply_request(&config, "p1"), "{}"), &token),
        Utc::now(),
    );
    assert_eq!(preview.status, 200, "preview status");
    let json = body_json(&preview);
    let digest = json["plan_digest"].as_str().unwrap();
    assert_eq!(digest.len(), 64, "digest length");
    assert!(json["confirmation"].is_object(), "gate described");
    assert!(json["preview"].is_object(), "preview shape");
    let raw = String::from_utf8_lossy(&preview.body);
    assert!(
        !raw.contains(&dir.path().to_string_lossy().to_string()),
        "preview leaked an absolute path"
    );

    // Wrong digest + confirm -> 409 mismatch + fresh digest + no write.
    let wrong = handle(
        &config,
        &db,
        &with_cookie(
            json_body(
                apply_request(&config, "p1"),
                r#"{"confirm":true,"plan_digest":"deadbeefdeadbeef"}"#,
            ),
            &token,
        ),
        Utc::now(),
    );
    assert_eq!(wrong.status, 409, "wrong digest status");
    assert_eq!(body_json(&wrong)["error"]["code"], "admin-digest-mismatch");
    assert!(body_json(&wrong)["plan_digest"].is_string(), "fresh digest");

    assert!(
        !log.exists(),
        "preview/mismatch must not invoke the provider"
    );
    assert!(
        publish_rows(&db).is_empty(),
        "preview/mismatch must not journal"
    );
}

#[test]
fn confirmed_apply_invokes_the_provider_once_and_journals() {
    let _guard = lock();
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let log = dir.path().join("provider.log");
    let stub = write_stub(
        dir.path(),
        "stub.sh",
        &stub_provider(&log, "done", "healthy", 0),
    );
    let project = dir.path().join("p1");
    write_publish_project(&project, "p1", &stub);
    register_project(&db, &project);
    let _env = PublishEnv::set("fixture", None);
    let head = git_head(&project);

    // Preview, then confirm with the same digest.
    let preview = handle(
        &config,
        &db,
        &with_cookie(json_body(apply_request(&config, "p1"), "{}"), &token),
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
                apply_request(&config, "p1"),
                &format!(r#"{{"confirm":true,"plan_digest":"{digest}"}}"#),
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
    assert_eq!(json["provider"], "fixture");
    assert_eq!(json["operation"], "publish");
    assert_eq!(json["status"], "done");
    assert_eq!(json["health"], "healthy");
    assert!(
        json["healthy"].as_bool().unwrap_or(false),
        "healthy: {json}"
    );
    assert_eq!(json["revision"], head);
    assert_eq!(json["build_status"], "succeeded");
    assert_eq!(json["run_status"], "succeeded");
    assert_eq!(json["container_identity"], "forge-stub-deadbeefcafe");

    // No absolute path or provider executable leaked.
    let raw = String::from_utf8_lossy(&applied.body);
    assert!(
        !raw.contains(&dir.path().to_string_lossy().to_string()),
        "apply response leaked an absolute path: {raw}"
    );
    assert!(
        !raw.contains(&stub.to_string_lossy().to_string()),
        "apply response leaked the provider executable"
    );

    // The provider was invoked exactly once with the server-resolved request.
    assert!(log.exists(), "confirmed apply must invoke the provider");
    let recorded = fs::read_to_string(&log).unwrap();
    assert_eq!(
        recorded
            .matches("\"contract\":\"forge-publish-provider/0.1.0\"")
            .count(),
        1,
        "exactly one request logged: {recorded}"
    );
    assert!(recorded.contains(&format!("\"revision\":\"{head}\"")));
    assert!(recorded.contains("\"operation\":\"publish\""));

    // One `publish` row carries the provider-reported state and phase columns.
    let rows = publish_rows(&db);
    assert_eq!(rows.len(), 1, "exactly one publish row");
    assert_eq!(rows[0].state, "done");
    assert_eq!(rows[0].revision.as_deref(), Some(head.as_str()));
    assert_eq!(rows[0].build_status.as_deref(), Some("succeeded"));
    assert_eq!(rows[0].run_status.as_deref(), Some("succeeded"));
    assert_eq!(
        rows[0].container_identity.as_deref(),
        Some("forge-stub-deadbeefcafe")
    );
}

#[test]
fn provider_failure_is_reported_honestly_never_as_success() {
    let _guard = lock();
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let log = dir.path().join("provider.log");
    // A provider that emits a valid envelope but exits non-zero.
    let stub = write_stub(
        dir.path(),
        "stub-fail.sh",
        &stub_provider(&log, "failed", "unhealthy", 1),
    );
    let project = dir.path().join("p1");
    write_publish_project(&project, "p1", &stub);
    register_project(&db, &project);
    let _env = PublishEnv::set("fixture", None);

    let preview = handle(
        &config,
        &db,
        &with_cookie(json_body(apply_request(&config, "p1"), "{}"), &token),
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
                apply_request(&config, "p1"),
                &format!(r#"{{"confirm":true,"plan_digest":"{digest}"}}"#),
            ),
            &token,
        ),
        Utc::now(),
    );
    assert_eq!(applied.status, 503, "provider failure is a typed 503");
    let json = body_json(&applied);
    assert_eq!(json["error"]["code"], "publish-provider-unavailable");
    assert!(
        json.get("healthy").is_none() && json.get("status").is_none(),
        "a failed provider must not report success fields: {json}"
    );
    let raw = String::from_utf8_lossy(&applied.body);
    assert!(
        !raw.contains(&dir.path().to_string_lossy().to_string()),
        "failure response leaked an absolute path: {raw}"
    );

    // The failure is journaled, never a fake success.
    let rows = publish_rows(&db);
    assert_eq!(rows.len(), 1, "failure must still be journaled");
    assert_eq!(rows[0].state, "failed");
}

#[test]
fn unhealthy_provider_response_is_not_success() {
    let _guard = lock();
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let log = dir.path().join("provider.log");
    // Exits zero but reports an unhealthy terminal state: the route must report
    // the provider's real status with `healthy:false`, never a success.
    let stub = write_stub(
        dir.path(),
        "stub-unhealthy.sh",
        &stub_provider(&log, "failed", "unhealthy", 0),
    );
    let project = dir.path().join("p1");
    write_publish_project(&project, "p1", &stub);
    register_project(&db, &project);
    let _env = PublishEnv::set("fixture", None);

    let preview = handle(
        &config,
        &db,
        &with_cookie(json_body(apply_request(&config, "p1"), "{}"), &token),
        Utc::now(),
    );
    let digest = body_json(&preview)["plan_digest"]
        .as_str()
        .unwrap()
        .to_string();

    let applied = handle(
        &config,
        &db,
        &with_cookie(
            json_body(
                apply_request(&config, "p1"),
                &format!(r#"{{"confirm":true,"plan_digest":"{digest}"}}"#),
            ),
            &token,
        ),
        Utc::now(),
    );
    assert_eq!(applied.status, 202, "valid but unhealthy response is 202");
    let json = body_json(&applied);
    assert_eq!(json["status"], "failed");
    assert_eq!(json["health"], "unhealthy");
    assert!(!json["healthy"].as_bool().unwrap_or(true), "healthy=false");

    let rows = publish_rows(&db);
    assert_eq!(rows.len(), 1, "unhealthy run is journaled");
    assert_eq!(rows[0].state, "failed", "journal carries the real state");
}

#[test]
fn catalog_reports_publish_as_web_with_an_empty_execution_block() {
    let _guard = lock();
    let rows = command_catalog::rows();
    let publish = rows
        .iter()
        .find(|row| row.id == "publish")
        .expect("missing catalog row publish");
    assert_eq!(publish.availability, "web", "publish should be web");
    assert_eq!(
        publish.route.as_deref(),
        Some("POST /v1/admin/projects/{id}/publish"),
        "publish route"
    );
    let execution = publish
        .execution
        .as_ref()
        .expect("publish must carry an execution block");
    assert_eq!(execution.route, "POST /v1/admin/projects/{id}/publish");
    assert_eq!(execution.method, "POST");
    assert!(execution.confirm_required, "confirm required");
    assert!(execution.digest_bound, "digest bound");
    assert_eq!(execution.risk, "remote_write");
    assert!(
        execution.parameters.is_empty(),
        "every publish input is server-resolved, so no browser parameters"
    );
}
