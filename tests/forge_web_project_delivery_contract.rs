//! Staged project-delivery execution contract (`forge-web-project-delivery`).
//!
//! Pins the five session-gated project-delivery admin routes —
//! `GET /v1/admin/projects/{id}/delivery/status`,
//! `POST .../delivery/preflight`, `POST .../delivery/stage`,
//! `POST .../delivery/promote` and `POST .../delivery/hermora-retry` — to
//! the established admin security boundary: anonymous requests are refused
//! before Core, mutation bodies must be JSON, hostile ids never echo, and
//! previews/mismatched digests never invoke a provider or write a row. Each
//! confirmed mutation delegates to the unchanged `delivery::handlers` verb
//! used by the CLI, preserving evidence gating, health gating, idempotency
//! and honest failure reporting.
//!
//! The OpenPanel provider and Hermora adapter are hermetic executable stubs
//! staged at run time. They speak their versioned stdin/stdout envelopes and
//! log the requests they receive. No network, provider daemon or credential
//! value is involved.

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

/// `FORGE_HERMORA_BIN` and `FORGE_PUBLISH_PROVIDER_CONFIG` are process-global
/// settings read by delivery Core inside the route call. Every test in this
/// binary serializes on one lock so parallel tests cannot observe each
/// other's adapter selection.
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

fn json_body(mut req: ApiRequest, body: &Value) -> ApiRequest {
    req.headers
        .insert("content-type".to_string(), "application/json".to_string());
    req.body = body.to_string().into_bytes();
    req
}

fn status_path(id: &str) -> String {
    format!("/v1/admin/projects/{id}/delivery/status")
}

fn verb_path(id: &str, verb: &str) -> String {
    format!("/v1/admin/projects/{id}/delivery/{verb}")
}

fn login_token(config: &ApiConfig, db: &Path) -> String {
    forge::identity::global::setup(db, "operator@example.test", "a-long-test-password").unwrap();
    let login = json_body(
        request("POST", "/v1/admin/session", &config.frontend_origin),
        &json!({"email": "operator@example.test", "password": "a-long-test-password"}),
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
    serde_json::from_slice(&response.body).expect("delivery admin routes are JSON-only")
}

fn post(
    config: &ApiConfig,
    db: &Path,
    token: &str,
    path: &str,
    body: &Value,
) -> forge::api::ApiResponse {
    handle(
        config,
        db,
        &with_cookie(
            json_body(request("POST", path, &config.frontend_origin), body),
            token,
        ),
        Utc::now(),
    )
}

/// Save and restore process-global delivery adapter environment around one
/// test. The provider configuration path selects the project-local stub
/// config; the Hermora variable selects the stub adapter binary.
struct DeliveryEnv {
    provider_config: Option<OsString>,
    hermora_bin: Option<OsString>,
}

impl DeliveryEnv {
    fn set(provider_config: Option<&Path>, hermora_bin: Option<&Path>) -> Self {
        let guard = Self {
            provider_config: std::env::var_os("FORGE_PUBLISH_PROVIDER_CONFIG"),
            hermora_bin: std::env::var_os("FORGE_HERMORA_BIN"),
        };
        match provider_config {
            Some(path) => std::env::set_var("FORGE_PUBLISH_PROVIDER_CONFIG", path),
            None => std::env::remove_var("FORGE_PUBLISH_PROVIDER_CONFIG"),
        }
        match hermora_bin {
            Some(path) => std::env::set_var("FORGE_HERMORA_BIN", path),
            None => std::env::remove_var("FORGE_HERMORA_BIN"),
        }
        guard
    }
}

impl Drop for DeliveryEnv {
    fn drop(&mut self) {
        match &self.provider_config {
            Some(value) => std::env::set_var("FORGE_PUBLISH_PROVIDER_CONFIG", value),
            None => std::env::remove_var("FORGE_PUBLISH_PROVIDER_CONFIG"),
        }
        match &self.hermora_bin {
            Some(value) => std::env::set_var("FORGE_HERMORA_BIN", value),
            None => std::env::remove_var("FORGE_HERMORA_BIN"),
        }
    }
}

fn write_delivery_project(dir: &Path, id: &str, provider_command: &Path) {
    fs::create_dir_all(dir).unwrap();
    fs::write(
        dir.join("forge.yaml"),
        format!(
            "schema: 1\nproject:\n  id: {id}\n  name: Test {id}\n  profile: rust-web\n  maturity: L2\nruntime:\n  language: rust\n"
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
    let forge_dir = dir.join(".forge");
    fs::create_dir_all(&forge_dir).unwrap();
    fs::write(
        forge_dir.join("providers.yaml"),
        format!(
            "providers:\n  - id: openpanel\n    command: \"{}\"\n",
            provider_command.display()
        ),
    )
    .unwrap();
}

fn run_git(dir: &Path, args: &[&str]) {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(dir);
    for arg in args {
        cmd.arg(arg);
    }
    let out = cmd.output().expect("git");
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
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

/// A hermetic OpenPanel provider stub. It logs the JSON request it receives,
/// then emits a valid provider response with the requested terminal status
/// values. It contacts nothing.
fn stub_provider(log: &Path, status: &str, run_status: &str, exit_code: i32) -> String {
    let response = json!({
        "contract": "forge-publish-provider/0.1.0",
        "provider": "openpanel",
        "operation_id": "delivery-stub",
        "status": status,
        "health": "healthy",
        "evidence": ["delivery stub round-tripped the request"],
        "recovery": [],
        "build_status": "succeeded",
        "run_status": run_status,
        "container_identity": "forge-stub-0123456789ab",
    });
    let body = serde_json::to_string(&response).unwrap();
    format!(
        "#!/bin/sh\nlog=\"{log}\"\ncat > \"${{log}}.in\"\ncat \"${{log}}.in\" >> \"$log\"\nprintf '%s\\n' '{body}'\nexit {exit_code}\n",
        log = log.display()
    )
}

/// A hermetic Hermora adapter stub. It logs the JSON request it receives and
/// answers `connected` without republishing or using any secret value.
fn stub_hermora(log: &Path) -> String {
    format!(
        "#!/bin/sh\nlog=\"{log}\"\ncat > \"${{log}}.in\"\ncat \"${{log}}.in\" >> \"$log\"\nprintf '%s\\n' '{{\"contract\":\"forge-delivery-hermora/0.1.0\",\"operation\":\"register\",\"status\":\"connected\",\"site_id\":\"site_stub\",\"environment_url\":\"https://delivery.example.test/app\"}}'\n",
        log = log.display()
    )
}

fn delivery_rows(db: &Path, project: &str) -> Vec<OperationEntry> {
    let registry = Registry::open(db).unwrap();
    let mut rows = Vec::new();
    for op_id in 1..256 {
        if let Ok(Some(entry)) = registry.operation(op_id) {
            if entry.project_id == project && entry.kind.starts_with("delivery.") {
                rows.push(entry);
            }
        }
    }
    rows
}

fn provider_calls(log: &Path) -> usize {
    fs::read_to_string(log)
        .map(|text| {
            text.matches("\"contract\":\"forge-publish-provider/0.1.0\"")
                .count()
        })
        .unwrap_or(0)
}

fn hermora_calls(log: &Path) -> usize {
    fs::read_to_string(log)
        .map(|text| {
            text.matches("\"contract\":\"forge-delivery-hermora/0.1.0\"")
                .count()
        })
        .unwrap_or(0)
}

fn assert_path_free(response: &forge::api::ApiResponse, root: &Path, adapter: &Path) {
    let raw = String::from_utf8_lossy(&response.body);
    assert!(
        !raw.contains(&root.to_string_lossy().to_string()),
        "response leaked the fixture root: {raw}"
    );
    assert!(
        !raw.contains(&adapter.to_string_lossy().to_string()),
        "response leaked the adapter executable: {raw}"
    );
}

fn preview_digest(config: &ApiConfig, db: &Path, token: &str, path: &str, body: &Value) -> String {
    let preview = post(config, db, token, path, body);
    assert_eq!(preview.status, 200, "preview status: {preview:?}");
    let json = body_json(&preview);
    let digest = json["plan_digest"].as_str().unwrap().to_string();
    assert_eq!(digest.len(), 64, "digest length");
    assert!(digest.chars().all(|c| c.is_ascii_hexdigit()));
    digest
}

fn confirmed(
    config: &ApiConfig,
    db: &Path,
    token: &str,
    path: &str,
    body: &Value,
) -> forge::api::ApiResponse {
    let digest = preview_digest(config, db, token, path, body);
    let mut confirmed = body.clone();
    confirmed["confirm"] = json!(true);
    confirmed["plan_digest"] = json!(digest);
    post(config, db, token, path, &confirmed)
}

struct Fixture {
    _dir: tempfile::TempDir,
    db: PathBuf,
    provider_log: PathBuf,
    hermora_log: PathBuf,
    hermora: PathBuf,
    revision: String,
    token: String,
    config: ApiConfig,
}

fn fixture(provider_status: &str, provider_run: &str, provider_exit: i32) -> Fixture {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let provider_log = dir.path().join("provider.log");
    let hermora_log = dir.path().join("hermora.log");
    let provider = write_stub(
        dir.path(),
        "openpanel-stub.sh",
        &stub_provider(&provider_log, provider_status, provider_run, provider_exit),
    );
    let hermora = write_stub(dir.path(), "hermora-stub.sh", &stub_hermora(&hermora_log));
    let project = dir.path().join("delivery-web");
    write_delivery_project(&project, "delivery-web", &provider);
    register_project(&db, &project);
    let revision = git_head(&project);
    Fixture {
        _dir: dir,
        db,
        provider_log,
        hermora_log,
        hermora,
        revision,
        token,
        config,
    }
}

#[test]
fn anonymous_and_non_json_are_refused_before_any_delivery_call() {
    let _guard = lock();
    let fx = fixture("succeeded", "succeeded", 0);
    let _env = DeliveryEnv::set(None, Some(&fx.hermora));

    let anonymous_status = handle(
        &fx.config,
        &fx.db,
        &request(
            "GET",
            &status_path("delivery-web"),
            &fx.config.frontend_origin,
        ),
        Utc::now(),
    );
    assert_eq!(anonymous_status.status, 401, "anonymous status refused");

    // Anonymous JSON POSTs are unauthorized before any Core call.
    for path in [
        verb_path("delivery-web", "preflight"),
        verb_path("delivery-web", "stage"),
        verb_path("delivery-web", "promote"),
        verb_path("delivery-web", "hermora-retry"),
    ] {
        let anonymous = handle(
            &fx.config,
            &fx.db,
            &json_body(
                request("POST", &path, &fx.config.frontend_origin),
                &json!({}),
            ),
            Utc::now(),
        );
        assert_eq!(anonymous.status, 401, "anonymous {path} refused");
    }

    // A session POST without JSON is refused before the session gate.
    let bad_type = handle(
        &fx.config,
        &fx.db,
        &with_cookie(
            request(
                "POST",
                &verb_path("delivery-web", "preflight"),
                &fx.config.frontend_origin,
            ),
            &fx.token,
        ),
        Utc::now(),
    );
    assert_eq!(bad_type.status, 415, "non-JSON refused");

    assert_eq!(provider_calls(&fx.provider_log), 0);
    assert_eq!(hermora_calls(&fx.hermora_log), 0);
    assert!(delivery_rows(&fx.db, "delivery-web").is_empty());
}

#[test]
fn status_is_side_effect_free_and_path_free() {
    let _guard = lock();
    let fx = fixture("succeeded", "succeeded", 0);
    let _env = DeliveryEnv::set(None, Some(&fx.hermora));

    let response = handle(
        &fx.config,
        &fx.db,
        &with_cookie(
            request(
                "GET",
                &status_path("delivery-web"),
                &fx.config.frontend_origin,
            ),
            &fx.token,
        ),
        Utc::now(),
    );
    assert_eq!(response.status, 200);
    let json = body_json(&response);
    assert_eq!(json["delivery_status"]["phase"], "draft");
    assert_eq!(json["delivery_status"]["revision"], fx.revision);
    assert_eq!(json["next"]["action"], "delivery-preflight");
    assert_path_free(&response, fx._dir.path(), &fx.hermora);
    assert_eq!(provider_calls(&fx.provider_log), 0);
    assert_eq!(hermora_calls(&fx.hermora_log), 0);
    assert!(delivery_rows(&fx.db, "delivery-web").is_empty());
}

#[test]
fn hostile_id_is_refused_and_unmanaged_id_is_not_found() {
    let _guard = lock();
    let fx = fixture("succeeded", "succeeded", 0);
    let _env = DeliveryEnv::set(None, Some(&fx.hermora));

    let hostile_status = handle(
        &fx.config,
        &fx.db,
        &with_cookie(
            request(
                "GET",
                &status_path("..%2F..%2Fetc"),
                &fx.config.frontend_origin,
            ),
            &fx.token,
        ),
        Utc::now(),
    );
    assert_eq!(hostile_status.status, 400, "hostile status refused");
    assert_eq!(
        body_json(&hostile_status)["error"]["code"],
        "admin-invalid-project-id"
    );
    assert!(
        !String::from_utf8_lossy(&hostile_status.body).contains(".."),
        "hostile id echoed"
    );

    let hostile_preflight = post(
        &fx.config,
        &fx.db,
        &fx.token,
        &verb_path("..%2F..%2Fetc", "preflight"),
        &json!({}),
    );
    assert_eq!(hostile_preflight.status, 400, "hostile mutation refused");
    assert_eq!(
        body_json(&hostile_preflight)["error"]["code"],
        "admin-invalid-project-id"
    );
    assert!(
        !String::from_utf8_lossy(&hostile_preflight.body).contains(".."),
        "hostile id echoed"
    );

    let unmanaged = handle(
        &fx.config,
        &fx.db,
        &with_cookie(
            request(
                "GET",
                &status_path("ghost-delivery"),
                &fx.config.frontend_origin,
            ),
            &fx.token,
        ),
        Utc::now(),
    );
    assert_eq!(unmanaged.status, 404);
    assert_eq!(
        body_json(&unmanaged)["error"]["code"],
        "admin-project-unmanaged"
    );
    assert_eq!(provider_calls(&fx.provider_log), 0);
    assert!(delivery_rows(&fx.db, "delivery-web").is_empty());
}

#[test]
fn missing_provider_is_a_typed_unavailable_without_a_path_echo() {
    let _guard = lock();
    let fx = fixture("succeeded", "succeeded", 0);
    let missing = fx._dir.path().join("missing-providers.yaml");
    let _env = DeliveryEnv::set(Some(&missing), Some(&fx.hermora));

    let response = confirmed(
        &fx.config,
        &fx.db,
        &fx.token,
        &verb_path("delivery-web", "preflight"),
        &json!({}),
    );
    assert_eq!(response.status, 503, "missing provider unavailable");
    assert_eq!(
        body_json(&response)["error"]["code"],
        "delivery-unavailable"
    );
    let raw = String::from_utf8_lossy(&response.body);
    assert!(
        !raw.contains("missing-providers"),
        "config path echoed: {raw}"
    );
    assert_path_free(&response, fx._dir.path(), &fx.hermora);
    assert_eq!(provider_calls(&fx.provider_log), 0);
}

#[test]
fn missing_prerequisites_are_conflicts_without_provider_calls() {
    let _guard = lock();
    let fx = fixture("succeeded", "succeeded", 0);
    let _env = DeliveryEnv::set(None, Some(&fx.hermora));

    let stage = post(
        &fx.config,
        &fx.db,
        &fx.token,
        &verb_path("delivery-web", "stage"),
        &json!({"confirm_operation_id": "90210"}),
    );
    assert_eq!(stage.status, 200, "stage preview status");
    let stage_digest = body_json(&stage)["plan_digest"]
        .as_str()
        .unwrap()
        .to_string();
    let stage = post(
        &fx.config,
        &fx.db,
        &fx.token,
        &verb_path("delivery-web", "stage"),
        &json!({"confirm": true, "plan_digest": stage_digest, "confirm_operation_id": "90210"}),
    );
    assert_eq!(
        stage.status, 400,
        "stage without a recorded preflight is invalid"
    );
    assert_eq!(body_json(&stage)["error"]["code"], "delivery-invalid");

    let promote = confirmed(
        &fx.config,
        &fx.db,
        &fx.token,
        &verb_path("delivery-web", "promote"),
        &json!({"confirm_revision": fx.revision}),
    );
    assert_eq!(promote.status, 409, "promote without stage conflicts");
    assert_eq!(body_json(&promote)["error"]["code"], "delivery-conflict");

    let hermora = confirmed(
        &fx.config,
        &fx.db,
        &fx.token,
        &verb_path("delivery-web", "hermora-retry"),
        &json!({
            "deployment_url": "https://delivery.example.test/app",
            "secret_ref": "env:DELIVERY_HERMORA_TOKEN",
        }),
    );
    assert_eq!(hermora.status, 409, "hermora without promote conflicts");
    assert_eq!(body_json(&hermora)["error"]["code"], "delivery-conflict");
    assert_eq!(provider_calls(&fx.provider_log), 0);
    assert_eq!(hermora_calls(&fx.hermora_log), 0);
    assert!(delivery_rows(&fx.db, "delivery-web").is_empty());
}

#[test]
fn previews_and_mismatched_digests_write_nothing() {
    let _guard = lock();
    let fx = fixture("succeeded", "succeeded", 0);
    let _env = DeliveryEnv::set(None, Some(&fx.hermora));
    let bodies = [
        (verb_path("delivery-web", "preflight"), json!({})),
        (
            verb_path("delivery-web", "stage"),
            json!({"confirm_operation_id": "27"}),
        ),
        (
            verb_path("delivery-web", "promote"),
            json!({"confirm_revision": fx.revision}),
        ),
        (
            verb_path("delivery-web", "hermora-retry"),
            json!({
                "deployment_url": "https://delivery.example.test/app",
                "secret_ref": "env:DELIVERY_HERMORA_TOKEN",
            }),
        ),
    ];

    for (path, body) in bodies {
        let preview = post(&fx.config, &fx.db, &fx.token, &path, &body);
        assert_eq!(preview.status, 200, "preview {path}");
        let json = body_json(&preview);
        assert!(json["preview"].is_object(), "preview shape {path}");
        if path.ends_with("hermora-retry") {
            let raw = String::from_utf8_lossy(&preview.body);
            assert!(
                !raw.contains("https://delivery.example.test/app"),
                "Hermora URL echoed in preview: {raw}"
            );
            assert!(
                !raw.contains("env:DELIVERY_HERMORA_TOKEN"),
                "secret reference echoed in preview: {raw}"
            );
        }
        let mut wrong = body.clone();
        wrong["confirm"] = json!(true);
        wrong["plan_digest"] = json!("f".repeat(64));
        let refused = post(&fx.config, &fx.db, &fx.token, &path, &wrong);
        assert_eq!(refused.status, 409, "mismatch {path}");
        assert_eq!(
            body_json(&refused)["error"]["code"],
            "admin-digest-mismatch"
        );
    }

    assert_eq!(provider_calls(&fx.provider_log), 0);
    assert_eq!(hermora_calls(&fx.hermora_log), 0);
    assert!(delivery_rows(&fx.db, "delivery-web").is_empty());
}

#[test]
fn malformed_confirmations_are_refused_without_echo_or_write() {
    let _guard = lock();
    let fx = fixture("succeeded", "succeeded", 0);
    let _env = DeliveryEnv::set(None, Some(&fx.hermora));
    let cases = [
        (
            verb_path("delivery-web", "stage"),
            json!({"confirm_operation_id": "12x"}),
        ),
        (
            verb_path("delivery-web", "stage"),
            json!({"confirm_operation_id": "-12"}),
        ),
        (
            verb_path("delivery-web", "promote"),
            json!({"confirm_revision": "short"}),
        ),
        (
            verb_path("delivery-web", "hermora-retry"),
            json!({
                "deployment_url": "https://delivery.example.test/app",
                "secret_ref": "env:ghp_0123456789abcdefghij",
            }),
        ),
        (
            verb_path("delivery-web", "hermora-retry"),
            json!({
                "deployment_url": "https://user:pass@delivery.example.test/app",
                "secret_ref": "env:DELIVERY_HERMORA_TOKEN",
            }),
        ),
    ];

    for (path, body) in cases {
        let response = post(&fx.config, &fx.db, &fx.token, &path, &body);
        assert_eq!(response.status, 400, "malformed {path}: {response:?}");
        assert_eq!(body_json(&response)["error"]["code"], "delivery-invalid");
        let raw = String::from_utf8_lossy(&response.body);
        assert!(!raw.contains("ghp_"), "credential echoed: {raw}");
        assert!(!raw.contains("user:pass"), "URL credential echoed: {raw}");
    }
    assert_eq!(provider_calls(&fx.provider_log), 0);
    assert_eq!(hermora_calls(&fx.hermora_log), 0);
    assert!(delivery_rows(&fx.db, "delivery-web").is_empty());
}

#[test]
fn confirmed_staged_run_reaches_connected_and_reuses_idempotent_rows() {
    let _guard = lock();
    let fx = fixture("succeeded", "succeeded", 0);
    let _env = DeliveryEnv::set(None, Some(&fx.hermora));

    let preflight = confirmed(
        &fx.config,
        &fx.db,
        &fx.token,
        &verb_path("delivery-web", "preflight"),
        &json!({}),
    );
    assert_eq!(preflight.status, 202, "preflight: {preflight:?}");
    let preflight_json = body_json(&preflight);
    let preflight_op = preflight_json["operation_id"].as_i64().unwrap();
    assert_eq!(
        preflight_json["delivery"]["phase"],
        "awaiting-stage-confirmation"
    );

    let stage_path = verb_path("delivery-web", "stage");
    let stage = confirmed(
        &fx.config,
        &fx.db,
        &fx.token,
        &stage_path,
        &json!({"confirm_operation_id": preflight_op.to_string()}),
    );
    assert_eq!(stage.status, 202, "stage: {stage:?}");
    let stage_json = body_json(&stage);
    let stage_op = stage_json["operation_id"].as_i64().unwrap();
    assert_eq!(
        stage_json["delivery"]["phase"],
        "awaiting-production-approval"
    );

    // An exact stage replay reuses the same journal row and does not invoke
    // the provider again.
    let replay = confirmed(
        &fx.config,
        &fx.db,
        &fx.token,
        &stage_path,
        &json!({"confirm_operation_id": preflight_op.to_string()}),
    );
    assert_eq!(replay.status, 202, "stage replay: {replay:?}");
    assert_eq!(body_json(&replay)["operation_id"], stage_op);
    assert_eq!(provider_calls(&fx.provider_log), 2);

    let promote = confirmed(
        &fx.config,
        &fx.db,
        &fx.token,
        &verb_path("delivery-web", "promote"),
        &json!({"confirm_revision": fx.revision}),
    );
    assert_eq!(promote.status, 202, "promote: {promote:?}");
    assert_eq!(body_json(&promote)["delivery"]["phase"], "healthy");

    let hermora = confirmed(
        &fx.config,
        &fx.db,
        &fx.token,
        &verb_path("delivery-web", "hermora-retry"),
        &json!({
            "deployment_url": "https://delivery.example.test/app",
            "secret_ref": "env:DELIVERY_HERMORA_TOKEN",
        }),
    );
    assert_eq!(hermora.status, 202, "hermora: {hermora:?}");
    let hermora_json = body_json(&hermora);
    assert_eq!(hermora_json["delivery"]["phase"], "hermora-connected");
    assert_eq!(hermora_json["delivery"]["hermora"]["site_id"], "site_stub");
    let raw = String::from_utf8_lossy(&hermora.body);
    assert!(
        !raw.contains("https://delivery.example.test/app"),
        "deployment URL echoed: {raw}"
    );
    assert!(
        !raw.contains("env:DELIVERY_HERMORA_TOKEN"),
        "secret reference echoed: {raw}"
    );
    assert_path_free(&hermora, fx._dir.path(), &fx.hermora);

    assert_eq!(provider_calls(&fx.provider_log), 3);
    assert_eq!(hermora_calls(&fx.hermora_log), 1);
    let rows = delivery_rows(&fx.db, "delivery-web");
    assert_eq!(rows.len(), 4, "one row per verb: {rows:?}");
    assert!(rows.iter().all(|row| row.state == "done"));
    for kind in ["delivery.preflight", "delivery.stage", "delivery.promote"] {
        let row = rows
            .iter()
            .find(|row| row.kind == kind)
            .unwrap_or_else(|| panic!("missing {kind} row: {rows:?}"));
        assert_eq!(row.revision.as_deref(), Some(fx.revision.as_str()));
    }
}

#[test]
fn provider_failure_is_a_typed_unavailable_and_journals_failed() {
    let _guard = lock();
    let fx = fixture("succeeded", "succeeded", 1);
    let _env = DeliveryEnv::set(None, Some(&fx.hermora));

    let preflight = confirmed(
        &fx.config,
        &fx.db,
        &fx.token,
        &verb_path("delivery-web", "preflight"),
        &json!({}),
    );
    assert_eq!(preflight.status, 503, "provider failure: {preflight:?}");
    assert_eq!(
        body_json(&preflight)["error"]["code"],
        "delivery-unavailable"
    );
    assert_path_free(&preflight, fx._dir.path(), &fx.hermora);
    assert_eq!(provider_calls(&fx.provider_log), 1);
    let rows = delivery_rows(&fx.db, "delivery-web");
    assert_eq!(rows.len(), 1, "failure journals one row");
    assert_eq!(rows[0].state, "failed");
}

#[test]
fn unhealthy_stage_blocks_promotion_without_a_second_provider_call() {
    let _guard = lock();
    let fx = fixture("succeeded", "failed", 0);
    let _env = DeliveryEnv::set(None, Some(&fx.hermora));

    let preflight = confirmed(
        &fx.config,
        &fx.db,
        &fx.token,
        &verb_path("delivery-web", "preflight"),
        &json!({}),
    );
    assert_eq!(preflight.status, 202);
    let preflight_op = body_json(&preflight)["operation_id"].as_i64().unwrap();
    let stage = confirmed(
        &fx.config,
        &fx.db,
        &fx.token,
        &verb_path("delivery-web", "stage"),
        &json!({"confirm_operation_id": preflight_op.to_string()}),
    );
    assert_eq!(stage.status, 202);
    assert_eq!(body_json(&stage)["delivery"]["phase"], "stage-failed");

    let promote = confirmed(
        &fx.config,
        &fx.db,
        &fx.token,
        &verb_path("delivery-web", "promote"),
        &json!({"confirm_revision": fx.revision}),
    );
    assert_eq!(promote.status, 409, "unhealthy stage blocks promote");
    assert_eq!(body_json(&promote)["error"]["code"], "delivery-conflict");
    assert_eq!(provider_calls(&fx.provider_log), 2);
    assert!(delivery_rows(&fx.db, "delivery-web")
        .iter()
        .all(|row| row.kind != "delivery.promote"));
}

#[test]
fn missing_hermora_adapter_preserves_the_healthy_deployment() {
    let _guard = lock();
    let fx = fixture("succeeded", "succeeded", 0);
    let _env = DeliveryEnv::set(None, None);

    let preflight = confirmed(
        &fx.config,
        &fx.db,
        &fx.token,
        &verb_path("delivery-web", "preflight"),
        &json!({}),
    );
    assert_eq!(preflight.status, 202);
    let preflight_op = body_json(&preflight)["operation_id"].as_i64().unwrap();
    let stage = confirmed(
        &fx.config,
        &fx.db,
        &fx.token,
        &verb_path("delivery-web", "stage"),
        &json!({"confirm_operation_id": preflight_op.to_string()}),
    );
    assert_eq!(stage.status, 202);
    let promote = confirmed(
        &fx.config,
        &fx.db,
        &fx.token,
        &verb_path("delivery-web", "promote"),
        &json!({"confirm_revision": fx.revision}),
    );
    assert_eq!(promote.status, 202);
    assert_eq!(body_json(&promote)["delivery"]["phase"], "healthy");

    let hermora = confirmed(
        &fx.config,
        &fx.db,
        &fx.token,
        &verb_path("delivery-web", "hermora-retry"),
        &json!({
            "deployment_url": "https://delivery.example.test/app",
            "secret_ref": "env:DELIVERY_HERMORA_TOKEN",
        }),
    );
    assert_eq!(hermora.status, 503, "missing adapter unavailable");
    assert_eq!(body_json(&hermora)["error"]["code"], "delivery-unavailable");
    let rows = delivery_rows(&fx.db, "delivery-web");
    assert!(rows
        .iter()
        .any(|row| row.kind == "delivery.promote" && row.state == "done"));
    assert!(rows
        .iter()
        .any(|row| row.kind == "delivery.hermora" && row.state == "failed"));
}

#[test]
fn catalog_reports_delivery_as_web_with_executable_blocks() {
    let _guard = lock();
    let rows = command_catalog::rows();
    let row = |id: &str| {
        rows.iter()
            .find(|row| row.id == id)
            .unwrap_or_else(|| panic!("missing catalog row {id}"))
    };
    let status = row("delivery.status");
    assert_eq!(status.availability, "web");
    assert_eq!(
        status.route.as_deref(),
        Some("GET /v1/admin/projects/{id}/delivery/status")
    );
    assert!(status.execution.is_none(), "status is read-only");

    for (id, route, parameters) in [
        (
            "delivery.preflight",
            "POST /v1/admin/projects/{id}/delivery/preflight",
            Vec::new(),
        ),
        (
            "delivery.stage",
            "POST /v1/admin/projects/{id}/delivery/stage",
            vec![("confirm_operation_id", "string", true)],
        ),
        (
            "delivery.promote",
            "POST /v1/admin/projects/{id}/delivery/promote",
            vec![("confirm_revision", "string", true)],
        ),
        (
            "delivery.hermora-retry",
            "POST /v1/admin/projects/{id}/delivery/hermora-retry",
            vec![
                ("deployment_url", "string", true),
                ("secret_ref", "string", true),
            ],
        ),
    ] {
        let row = row(id);
        assert_eq!(row.availability, "web", "{id} availability");
        assert_eq!(row.route.as_deref(), Some(route), "{id} route");
        let execution = row.execution.as_ref().expect("execution block");
        assert_eq!(execution.route, route, "{id} execution route");
        assert_eq!(execution.method, "POST", "{id} method");
        assert!(execution.confirm_required, "{id} confirm");
        assert!(execution.digest_bound, "{id} digest");
        assert_eq!(execution.risk, row.risk, "{id} risk");
        let actual: Vec<(&str, &str, bool)> = execution
            .parameters
            .iter()
            .map(|parameter| (parameter.name.as_str(), parameter.kind, parameter.required))
            .collect();
        assert_eq!(actual, parameters, "{id} parameters");
    }
}
