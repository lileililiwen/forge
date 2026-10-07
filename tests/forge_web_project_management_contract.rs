//! Project-management execution contract (`forge-web-project-management`).
//!
//! Pins the three typed, session-gated creation/registration routes —
//! `POST /v1/admin/projects/new` (`forge new`),
//! `POST /v1/admin/projects/import` (`forge import`) and
//! `POST /v1/admin/projects/register` (`forge register`) — to the same security
//! boundary the shipped lifecycle routes honour, plus the server-side-root rule
//! that lets a browser name a project without ever naming a filesystem path:
//!
//! - an anonymous JSON request is refused `401` before any Core call;
//! - a session request without a JSON body is refused `415`;
//! - a path-bearing / non-kebab `project` is a typed `400` that never echoes the
//!   offending input and never runs a Core operation;
//! - an unset `FORGE_ADMIN_PROJECTS_ROOT` is a typed `409 admin-prerequisite`
//!   that never echoes the root;
//! - a preview (`confirm` absent) returns a 64-hex `plan_digest` and a path-free
//!   view while writing nothing;
//! - a confirmed request with a mismatched digest is refused `409` with a
//!   refreshed digest and no write;
//! - only a confirmed matching digest delegates to the same in-process Core
//!   function the CLI runs (`generate`, `adopt_import`, `Registry::register`),
//!   journals the operation and returns a path-free `2xx`;
//! - a Core failure is reported as a typed non-`2xx` and journaled, never as a
//!   fake success;
//! - no response body ever carries the configured root, the resolved
//!   destination, a credential or the browser's raw input.
//!
//! `FORGE_ADMIN_PROJECTS_ROOT` is process-global, so every test that drives the
//! routes serializes on one lock; each call sets the variable for its own
//! request and restores the prior value.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::sync::Mutex;

use chrono::Utc;
use forge::api::{command_catalog, handle, ApiConfig, ApiRequest};
use forge::registry::Registry;
use tempfile::tempdir;

const ADMIN_ROOT: &str = "FORGE_ADMIN_PROJECTS_ROOT";

/// `FORGE_ADMIN_PROJECTS_ROOT` is process-global, so every test serializes.
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

/// Run one request with `FORGE_ADMIN_PROJECTS_ROOT` set to `root` (or unset when
/// `None`), restoring the prior value afterwards. Every route request in this
/// file goes through here so the process-global setting cannot leak between
/// serialized tests.
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

fn raw(response: &forge::api::ApiResponse) -> String {
    String::from_utf8_lossy(&response.body).to_string()
}

/// Every `operations` row currently in the registry.
fn operation_rows(db: &Path) -> Vec<(String, String)> {
    Registry::open(db)
        .unwrap()
        .journal_entries()
        .unwrap()
        .into_iter()
        .map(|entry| (entry.kind, entry.state))
        .collect()
}

/// A valid schema-1 rust-web manifest directory.
fn write_manifest_project(dir: &Path, id: &str) {
    fs::create_dir_all(dir).unwrap();
    fs::write(
        dir.join("forge.yaml"),
        format!(
            "schema: 1\nproject:\n  id: {id}\n  name: Test {id}\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\nfeatures: {{}}\n"
        ),
    )
    .unwrap();
}

/// A recognizable rust-web source directory with no manifest yet, so
/// `adopt_import` writes one.
fn write_importable(dir: &Path) {
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"adoptable\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    fs::write(dir.join("src").join("main.rs"), "fn main() {}\n").unwrap();
}

#[test]
fn anonymous_and_non_json_are_refused_before_any_core_call() {
    let _guard = lock();
    let temp = tempdir().unwrap();
    let db = temp.path().join("registry.db");
    let root = temp.path().join("projects");
    fs::create_dir_all(&root).unwrap();
    let config = ApiConfig::default();
    let token = login_token(&config, &db);

    for (path, body) in [
        (
            "/v1/admin/projects/new",
            r#"{"project":"p1","profile":"rust-web"}"#,
        ),
        ("/v1/admin/projects/import", r#"{"project":"p1"}"#),
        ("/v1/admin/projects/register", r#"{"project":"p1"}"#),
    ] {
        // Anonymous JSON -> 401.
        let anon = handle_with_root(
            &config,
            &db,
            &json_body(request("POST", path, &config.frontend_origin), body),
            Some(&root),
        );
        assert_eq!(anon.status, 401, "anonymous {path}");

        // Session but a non-JSON body -> 415 before the handler.
        let bad_ct = handle_with_root(
            &config,
            &db,
            &with_cookie(request("POST", path, &config.frontend_origin), &token),
            Some(&root),
        );
        assert_eq!(bad_ct.status, 415, "non-json {path}");
    }

    // Nothing ran: no project directory, no registry row, no journal entry.
    assert!(
        !root.join("p1").exists(),
        "refused requests must not create a directory"
    );
    assert!(
        operation_rows(&db).is_empty(),
        "refused requests must not journal"
    );
}

#[test]
fn hostile_project_name_is_refused_without_echo_or_write() {
    let _guard = lock();
    let temp = tempdir().unwrap();
    let db = temp.path().join("registry.db");
    let root = temp.path().join("projects");
    fs::create_dir_all(&root).unwrap();
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let before = operation_rows(&db);

    for hostile in [
        "..%2F..%2Fetc",
        "../escape",
        "BadName",
        "with/slash",
        "trail-",
    ] {
        let response = handle_with_root(
            &config,
            &db,
            &with_cookie(
                json_body(
                    request(
                        "POST",
                        "/v1/admin/projects/register",
                        &config.frontend_origin,
                    ),
                    &format!(r#"{{"project":"{hostile}"}}"#),
                ),
                &token,
            ),
            Some(&root),
        );
        assert_eq!(response.status, 400, "hostile {hostile}");
        assert_eq!(
            body_json(&response)["error"]["code"],
            "admin-invalid-project-name",
            "hostile {hostile}"
        );
        assert!(
            !raw(&response).contains(hostile),
            "the offending input `{hostile}` must never be echoed"
        );
    }
    assert_eq!(operation_rows(&db), before, "no Core operation may run");
}

#[test]
fn unset_root_is_a_typed_prerequisite_that_never_echoes_the_value() {
    let _guard = lock();
    let temp = tempdir().unwrap();
    let db = temp.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);

    let response = handle_with_root(
        &config,
        &db,
        &with_cookie(
            json_body(
                request(
                    "POST",
                    "/v1/admin/projects/register",
                    &config.frontend_origin,
                ),
                r#"{"project":"p1"}"#,
            ),
            &token,
        ),
        None,
    );
    assert_eq!(response.status, 409);
    let body = body_json(&response);
    assert_eq!(body["error"]["code"], "admin-prerequisite");
    let text = raw(&response);
    assert!(
        !text.contains(&temp.path().to_string_lossy().to_string()),
        "the response must not name a filesystem location"
    );
    assert!(
        !text.contains("/home/") && !text.contains("/tmp/"),
        "no absolute path may leak"
    );
    assert!(operation_rows(&db).is_empty());
}

#[test]
fn preview_returns_a_path_free_digest_without_writing_and_wrong_digest_is_refused() {
    let _guard = lock();
    let temp = tempdir().unwrap();
    let db = temp.path().join("registry.db");
    let root = temp.path().join("projects");
    let project = root.join("p1");
    write_manifest_project(&project, "p1");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let before_ops = operation_rows(&db);

    let preview = handle_with_root(
        &config,
        &db,
        &with_cookie(
            json_body(
                request(
                    "POST",
                    "/v1/admin/projects/register",
                    &config.frontend_origin,
                ),
                r#"{"project":"p1"}"#,
            ),
            &token,
        ),
        Some(&root),
    );
    assert_eq!(preview.status, 200);
    let body = body_json(&preview);
    let digest = body["plan_digest"].as_str().unwrap();
    assert_eq!(digest.len(), 64);
    assert!(digest.chars().all(|c| c.is_ascii_hexdigit()));
    assert_eq!(body["preview"]["action"], "project-register");
    assert_eq!(body["preview"]["manifest"]["id"], "p1");
    assert!(body["confirmation"].is_object());
    assert!(
        !raw(&preview).contains(&root.to_string_lossy().to_string()),
        "the preview must not carry the root"
    );
    assert_eq!(operation_rows(&db), before_ops, "preview must not write");

    // Confirm with a wrong digest -> 409 + fresh digest, still no write.
    let wrong = handle_with_root(
        &config,
        &db,
        &with_cookie(
            json_body(
                request(
                    "POST",
                    "/v1/admin/projects/register",
                    &config.frontend_origin,
                ),
                r#"{"project":"p1","confirm":true,"plan_digest":"deadbeef"}"#,
            ),
            &token,
        ),
        Some(&root),
    );
    assert_eq!(wrong.status, 409);
    let wrong_body = body_json(&wrong);
    assert_eq!(wrong_body["error"]["code"], "admin-digest-mismatch");
    assert_eq!(
        wrong_body["plan_digest"].as_str().unwrap().len(),
        64,
        "a refreshed digest is returned"
    );
    assert_eq!(
        operation_rows(&db),
        before_ops,
        "wrong digest must not write"
    );
}

#[test]
fn confirmed_register_adopts_a_manifest_directory_and_journals_it() {
    let _guard = lock();
    let temp = tempdir().unwrap();
    let db = temp.path().join("registry.db");
    let root = temp.path().join("projects");
    let project = root.join("p1");
    write_manifest_project(&project, "p1");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);

    let preview = handle_with_root(
        &config,
        &db,
        &with_cookie(
            json_body(
                request(
                    "POST",
                    "/v1/admin/projects/register",
                    &config.frontend_origin,
                ),
                r#"{"project":"p1"}"#,
            ),
            &token,
        ),
        Some(&root),
    );
    let digest = body_json(&preview)["plan_digest"]
        .as_str()
        .unwrap()
        .to_string();

    let applied = handle_with_root(
        &config,
        &db,
        &with_cookie(
            json_body(
                request(
                    "POST",
                    "/v1/admin/projects/register",
                    &config.frontend_origin,
                ),
                &format!(r#"{{"project":"p1","confirm":true,"plan_digest":"{digest}"}}"#),
            ),
            &token,
        ),
        Some(&root),
    );
    assert_eq!(applied.status, 202);
    let body = body_json(&applied);
    assert_eq!(body["registered"]["id"], "p1");
    assert_eq!(body["project_id"], "p1");
    assert!(body["operation_id"].as_i64().is_some());
    assert!(
        !raw(&applied).contains(&root.to_string_lossy().to_string()),
        "the response must not carry the root or destination"
    );
    // The project is registered and the operation is journaled as
    // `admin.project.register`.
    assert!(Registry::open(&db).unwrap().inspect("p1").is_ok());
    assert!(
        operation_rows(&db)
            .iter()
            .any(|(kind, state)| kind == "admin.project.register" && state == "done"),
        "the confirmed registration must journal a done admin.project.register row"
    );
}

#[test]
fn confirmed_import_writes_a_manifest_and_registers_without_a_path_leak() {
    let _guard = lock();
    let temp = tempdir().unwrap();
    let db = temp.path().join("registry.db");
    let root = temp.path().join("projects");
    let project = root.join("adoptable");
    write_importable(&project);
    let config = ApiConfig::default();
    let token = login_token(&config, &db);

    let request_body = r#"{"project":"adoptable","profile":"rust-web","id":"adopt-me"}"#;
    let preview = handle_with_root(
        &config,
        &db,
        &with_cookie(
            json_body(
                request("POST", "/v1/admin/projects/import", &config.frontend_origin),
                request_body,
            ),
            &token,
        ),
        Some(&root),
    );
    assert_eq!(preview.status, 200);
    assert_eq!(body_json(&preview)["preview"]["action"], "project-import");
    let digest = body_json(&preview)["plan_digest"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(
        !project.join("forge.yaml").exists(),
        "preview must not write"
    );

    let applied = handle_with_root(
        &config,
        &db,
        &with_cookie(
            json_body(
                request("POST", "/v1/admin/projects/import", &config.frontend_origin),
                &format!(
                    r#"{{"project":"adoptable","profile":"rust-web","id":"adopt-me","confirm":true,"plan_digest":"{digest}"}}"#
                ),
            ),
            &token,
        ),
        Some(&root),
    );
    assert_eq!(applied.status, 202);
    let body = body_json(&applied);
    assert_eq!(body["imported"]["id"], "adopt-me");
    assert!(
        project.join("forge.yaml").is_file(),
        "confirmed import writes the minimal manifest"
    );
    assert!(Registry::open(&db).unwrap().inspect("adopt-me").is_ok());
    assert!(
        !raw(&applied).contains(&root.to_string_lossy().to_string())
            && !raw(&applied).contains(&project.to_string_lossy().to_string()),
        "no absolute path may leak"
    );
}

#[test]
fn confirmed_new_creates_and_registers_a_project_without_a_path_leak() {
    let _guard = lock();
    let temp = tempdir().unwrap();
    let db = temp.path().join("registry.db");
    let root = temp.path().join("projects");
    fs::create_dir_all(&root).unwrap();
    let config = ApiConfig::default();
    let token = login_token(&config, &db);

    let preview = handle_with_root(
        &config,
        &db,
        &with_cookie(
            json_body(
                request("POST", "/v1/admin/projects/new", &config.frontend_origin),
                r#"{"project":"fresh-proj","profile":"rust-web"}"#,
            ),
            &token,
        ),
        Some(&root),
    );
    assert_eq!(preview.status, 200, "preview: {}", raw(&preview));
    assert_eq!(body_json(&preview)["preview"]["action"], "project-new");
    let digest = body_json(&preview)["plan_digest"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(!root.join("fresh-proj").exists(), "preview must not write");

    let applied = handle_with_root(
        &config,
        &db,
        &with_cookie(
            json_body(
                request("POST", "/v1/admin/projects/new", &config.frontend_origin),
                &format!(
                    r#"{{"project":"fresh-proj","profile":"rust-web","confirm":true,"plan_digest":"{digest}"}}"#
                ),
            ),
            &token,
        ),
        Some(&root),
    );
    assert_eq!(applied.status, 202, "applied: {}", raw(&applied));
    let body = body_json(&applied);
    assert_eq!(body["created"]["id"], "fresh-proj");
    assert_eq!(body["created"]["profile"], "rust-web");
    assert!(
        root.join("fresh-proj").join("forge.yaml").is_file(),
        "confirmed new writes the project tree"
    );
    assert!(Registry::open(&db).unwrap().inspect("fresh-proj").is_ok());
    assert!(
        !raw(&applied).contains(&root.to_string_lossy().to_string()),
        "no absolute path may leak"
    );
}

#[test]
fn core_failure_is_reported_honestly_and_journaled() {
    let _guard = lock();
    let temp = tempdir().unwrap();
    let db = temp.path().join("registry.db");
    let root = temp.path().join("projects");
    let project = root.join("empty-dir");
    fs::create_dir_all(&project).unwrap();
    let config = ApiConfig::default();
    let token = login_token(&config, &db);

    // A directory with no manifest previews to a typed Core error (and never a
    // digest), so the operator cannot confirm an action that cannot succeed.
    let preview = handle_with_root(
        &config,
        &db,
        &with_cookie(
            json_body(
                request(
                    "POST",
                    "/v1/admin/projects/register",
                    &config.frontend_origin,
                ),
                r#"{"project":"empty-dir"}"#,
            ),
            &token,
        ),
        Some(&root),
    );
    assert_ne!(
        preview.status, 200,
        "no digest for an unresolvable register"
    );
    assert!(body_json(&preview)["error"]["code"].is_string());

    // Confirmed anyway: refused before any write, never reported as success.
    let applied = handle_with_root(
        &config,
        &db,
        &with_cookie(
            json_body(
                request(
                    "POST",
                    "/v1/admin/projects/register",
                    &config.frontend_origin,
                ),
                r#"{"project":"empty-dir","confirm":true,"plan_digest":"does-not-matter"}"#,
            ),
            &token,
        ),
        Some(&root),
    );
    assert_ne!(applied.status, 202, "a failing register is not a success");
    assert!(!raw(&applied).contains("\"registered\""));
    assert!(
        !operation_rows(&db)
            .iter()
            .any(|(kind, state)| kind == "admin.project.register" && state == "done"),
        "no successful registration row may be written"
    );

    // A failure *after* a successful preview must be journaled as `failed`. Make
    // the `new` destination a nonempty directory between preview and confirm so
    // the Core `generate` refuses; the shared operation boundary records it.
    let blocked = root.join("blocked");
    let preview = handle_with_root(
        &config,
        &db,
        &with_cookie(
            json_body(
                request("POST", "/v1/admin/projects/new", &config.frontend_origin),
                r#"{"project":"blocked","profile":"rust-web"}"#,
            ),
            &token,
        ),
        Some(&root),
    );
    assert_eq!(preview.status, 200, "blocked preview: {}", raw(&preview));
    let digest = body_json(&preview)["plan_digest"]
        .as_str()
        .unwrap()
        .to_string();
    fs::create_dir_all(&blocked).unwrap();
    fs::write(blocked.join("occupied.txt"), "x").unwrap();

    let applied = handle_with_root(
        &config,
        &db,
        &with_cookie(
            json_body(
                request("POST", "/v1/admin/projects/new", &config.frontend_origin),
                &format!(
                    r#"{{"project":"blocked","profile":"rust-web","confirm":true,"plan_digest":"{digest}"}}"#
                ),
            ),
            &token,
        ),
        Some(&root),
    );
    assert_ne!(applied.status, 202, "a refused generation is not a success");
    assert!(
        !raw(&applied).contains(&blocked.to_string_lossy().to_string()),
        "the failure must not leak the destination path"
    );
    assert!(
        operation_rows(&db)
            .iter()
            .any(|(kind, state)| kind == "admin.project.new" && state == "failed"),
        "the failed generation must be journaled as failed"
    );
}

#[test]
fn catalog_reports_the_creation_commands_as_executable_web_rows() {
    let rows = command_catalog::rows();
    let expected = [
        (
            "new",
            "POST /v1/admin/projects/new",
            vec![
                ("project", "string", true),
                ("profile", "string", true),
                ("name", "string", false),
                ("features", "string_array", false),
            ],
        ),
        (
            "import",
            "POST /v1/admin/projects/import",
            vec![
                ("project", "string", true),
                ("profile", "string", false),
                ("id", "string", false),
            ],
        ),
        (
            "register",
            "POST /v1/admin/projects/register",
            vec![("project", "string", true)],
        ),
    ];
    for (id, route, parameters) in expected {
        let row = rows
            .iter()
            .find(|row| row.id == id)
            .unwrap_or_else(|| panic!("missing catalog row {id}"));
        assert_eq!(row.availability, "web", "{id} should be web");
        assert_eq!(row.route, Some(route), "{id} route");
        assert!(row.reason.is_none(), "{id} must carry no CLI-only reason");
        let execution = row
            .execution
            .as_ref()
            .unwrap_or_else(|| panic!("{id} must carry an execution block"));
        assert_eq!(execution.route, route, "{id} execution route");
        assert_eq!(execution.method, "POST", "{id} method");
        assert!(
            execution.confirm_required && execution.digest_bound,
            "{id} gate"
        );
        let actual: Vec<(&str, &str, bool)> = execution
            .parameters
            .iter()
            .map(|p| (p.name.as_str(), p.kind, p.required))
            .collect();
        assert_eq!(actual, parameters, "{id} typed parameters");
    }
}
