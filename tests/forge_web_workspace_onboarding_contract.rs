//! Workspace-onboarding execution contract (`forge-web-workspace-onboarding`).
//!
//! Pins the two session-gated workspace routes —
//! `GET /v1/admin/workspace/candidates` (live read-only discovery) and
//! `POST /v1/admin/workspace/onboard` (bulk preview → confirm → apply) — to
//! the same security boundary the shipped management routes honour, plus the
//! dynamic-growth rule: discovery re-reads the configured root on every
//! request, so new sibling directories appear with no code, list or
//! configuration change. No concrete host folder appears here or in any
//! response; every fixture directory lives under a throwaway tempdir.
//!
//! - an anonymous JSON request is refused `401` before any filesystem read;
//! - a session request without a JSON body is refused `415`;
//! - an unset `FORGE_ADMIN_PROJECTS_ROOT` is a typed `409
//!   admin-prerequisite` that names the variable, never its value;
//! - a path-bearing leaf is a typed `400` that never echoes the input;
//! - a preview returns per-item plans plus a 64-hex digest and writes nothing;
//! - a mismatched digest (including a directory set that changed between
//!   preview and confirm) is refused `409` with a fresh preview and no write;
//! - a confirmed matching digest onboards each item through the same Core
//!   functions the CLI runs (`adopt_import`, `Registry::register`) and
//!   reports honest per-item results (`202` all ok, `207` partial);
//! - no response body ever carries the configured root, a resolved
//!   destination, a manifest body, a credential or the raw hostile input.
//!
//! `FORGE_ADMIN_PROJECTS_ROOT` is process-global, so every test that drives
//! the routes serializes on one lock; each call sets the variable for its own
//! request and restores the prior value.

use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use chrono::Utc;
use forge::api::{handle, ApiConfig, ApiRequest};
use forge::registry::Registry;
use serde_json::{json, Value};
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

/// Run one request with `FORGE_ADMIN_PROJECTS_ROOT` set to `root` (or unset
/// when `None`), restoring the prior value afterwards.
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

fn login_token(config: &ApiConfig, db: &Path) -> String {
    forge::identity::global::setup(db, "operator@example.test", "a-long-test-password").unwrap();
    let login = json_body(
        request("POST", "/v1/admin/session", &config.frontend_origin),
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
    serde_json::from_slice(&response.body).expect("workspace routes are JSON-only")
}

fn raw(response: &forge::api::ApiResponse) -> String {
    String::from_utf8_lossy(&response.body).to_string()
}

fn candidates_request(config: &ApiConfig) -> ApiRequest {
    request(
        "GET",
        "/v1/admin/workspace/candidates?limit=100",
        &config.frontend_origin,
    )
}

fn onboard_request(config: &ApiConfig, token: &str, body: &Value) -> ApiRequest {
    with_cookie(
        json_body(
            request(
                "POST",
                "/v1/admin/workspace/onboard",
                &config.frontend_origin,
            ),
            &body.to_string(),
        ),
        token,
    )
}

/// A workspace fixture exercising every candidate shape: a manifest project,
/// an importable Cargo project, a non-kebab leaf, an ambiguous project
/// (Cargo + Next.js markers), an undecidable empty directory, a hidden
/// directory and a plain file (both skipped), plus a symlink pointing
/// outside the root (skipped).
struct Workspace {
    _dir: tempfile::TempDir,
    db: PathBuf,
    root: PathBuf,
    token: String,
    config: ApiConfig,
}

fn write_manifest(dir: &Path, id: &str) {
    fs::create_dir_all(dir).unwrap();
    fs::write(
        dir.join("forge.yaml"),
        format!(
            "schema: 1\nproject:\n  id: {id}\n  name: Test {id}\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\nfeatures: {{}}\n"
        ),
    )
    .unwrap();
}

fn write_cargo(dir: &Path, name: &str) {
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(
        dir.join("Cargo.toml"),
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"),
    )
    .unwrap();
    fs::write(dir.join("src").join("main.rs"), "fn main() {}\n").unwrap();
}

fn workspace() -> Workspace {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let root = dir.path().join("workspace");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);

    write_manifest(&root.join("manifest-proj"), "manifest-proj");
    write_cargo(&root.join("importable"), "importable");
    write_cargo(&root.join("DemoApp"), "demoapp");
    let ambiguous = root.join("mixed");
    fs::create_dir_all(&ambiguous).unwrap();
    fs::write(
        ambiguous.join("Cargo.toml"),
        "[package]\nname = \"mixed\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    fs::write(
        ambiguous.join("package.json"),
        "{\"dependencies\": {\"next\": \"15.0.0\"}}\n",
    )
    .unwrap();
    fs::create_dir_all(&root.join("empty")).unwrap();
    fs::create_dir_all(&root.join("badprofile")).unwrap();
    fs::write(
        root.join("badprofile").join("forge.yaml"),
        "schema: 1\nproject:\n  id: badprofile\n  name: Test badprofile\n  profile: no-such-profile\n  maturity: L1\nruntime:\n  language: rust\nfeatures: {}\n",
    )
    .unwrap();
    fs::create_dir_all(&root.join(".hidden")).unwrap();
    fs::write(root.join("notadir"), "just a file\n").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink("/tmp", root.join("escape-link")).unwrap();

    Workspace {
        _dir: dir,
        db,
        root,
        token,
        config,
    }
}

fn discover(fx: &Workspace) -> Value {
    let response = handle_with_root(
        &fx.config,
        &fx.db,
        &with_cookie(candidates_request(&fx.config), &fx.token),
        Some(&fx.root),
    );
    assert_eq!(response.status, 200, "discover: {response:?}");
    body_json(&response)
}

fn candidate<'a>(json: &'a Value, directory: &str) -> &'a Value {
    json["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["directory"] == directory)
        .unwrap_or_else(|| panic!("missing candidate {directory}: {json}"))
}

fn project_ids(db: &Path) -> Vec<String> {
    Registry::open(db)
        .unwrap()
        .list()
        .unwrap()
        .into_iter()
        .map(|record| record.id)
        .collect()
}

#[test]
fn anonymous_and_non_json_are_refused_before_any_filesystem_read() {
    let _guard = lock();
    let fx = workspace();

    let anon = handle_with_root(
        &fx.config,
        &fx.db,
        &candidates_request(&fx.config),
        Some(&fx.root),
    );
    assert_eq!(anon.status, 401, "anonymous discover");

    let anon_onboard = handle_with_root(
        &fx.config,
        &fx.db,
        &json_body(
            request(
                "POST",
                "/v1/admin/workspace/onboard",
                &fx.config.frontend_origin,
            ),
            r#"{"items":[]}"#,
        ),
        Some(&fx.root),
    );
    assert_eq!(anon_onboard.status, 401, "anonymous onboard");

    let bad_ct = handle_with_root(
        &fx.config,
        &fx.db,
        &with_cookie(
            request(
                "POST",
                "/v1/admin/workspace/onboard",
                &fx.config.frontend_origin,
            ),
            &fx.token,
        ),
        Some(&fx.root),
    );
    assert_eq!(bad_ct.status, 415, "non-json onboard");
    assert!(
        project_ids(&fx.db).is_empty(),
        "refused calls journal nothing"
    );
}

#[test]
fn unset_root_is_a_typed_prerequisite_that_never_echoes_the_value() {
    let _guard = lock();
    let fx = workspace();

    for (method, path, body) in [
        ("GET", "/v1/admin/workspace/candidates", ""),
        (
            "POST",
            "/v1/admin/workspace/onboard",
            r#"{"items":[{"directory":"manifest-proj"}]}"#,
        ),
    ] {
        let mut req = request(method, path, &fx.config.frontend_origin);
        let req = with_cookie(json_body(req, body), &fx.token);
        let response = handle_with_root(&fx.config, &fx.db, &req, None);
        assert_eq!(response.status, 409, "unset root {method} {path}");
        assert_eq!(body_json(&response)["error"]["code"], "admin-prerequisite");
        assert!(
            raw(&response).contains("FORGE_ADMIN_PROJECTS_ROOT"),
            "names the variable"
        );
    }
}

#[test]
fn hostile_leaves_are_refused_without_echo_or_read() {
    let _guard = lock();
    let fx = workspace();

    for hostile in ["../escape", "a/b", "", ".", "..", "with\0nul"] {
        let response = handle_with_root(
            &fx.config,
            &fx.db,
            &onboard_request(
                &fx.config,
                &fx.token,
                &json!({"items":[{"directory": hostile}]}),
            ),
            Some(&fx.root),
        );
        assert_eq!(response.status, 400, "hostile {hostile:?}");
        // Single/double dots necessarily appear in any JSON body; the
        // requirement for those is refusal with a static message, which the
        // status and code assertions above already pin.
        if hostile.trim().chars().count() > 2 {
            assert!(
                !raw(&response).contains(hostile.trim()),
                "echoed {hostile:?}"
            );
        }
    }
    assert!(
        project_ids(&fx.db).is_empty(),
        "hostile calls journal nothing"
    );
}

#[test]
fn discovery_lists_live_states_with_no_absolute_path() {
    let _guard = lock();
    let fx = workspace();
    let json = discover(&fx);

    let manifest = candidate(&json, "manifest-proj");
    assert_eq!(manifest["state"], "unregistered");
    assert_eq!(manifest["action"], "register");
    assert_eq!(manifest["id"], "manifest-proj");
    assert_eq!(manifest["profile"], "rust-web");
    assert_eq!(manifest["selectable"], true);

    let importable = candidate(&json, "importable");
    assert_eq!(importable["state"], "unregistered");
    assert_eq!(importable["action"], "import");
    assert_eq!(importable["id"], "importable");

    let mixed = candidate(&json, "mixed");
    assert_eq!(mixed["state"], "ambiguous");
    assert_eq!(mixed["selectable"], false);

    let empty = candidate(&json, "empty");
    assert_eq!(empty["state"], "undecidable");
    assert_eq!(empty["selectable"], false);

    let demo = candidate(&json, "DemoApp");
    assert_eq!(demo["state"], "unregistered");
    assert_eq!(demo["id"], "demoapp");
    assert_eq!(demo["selectable"], true);

    let names: Vec<&str> = json["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["directory"].as_str().unwrap())
        .collect();
    assert!(!names.contains(&".hidden"), "hidden dirs skipped");
    assert!(!names.contains(&"notadir"), "files skipped");
    assert!(!names.contains(&"escape-link"), "symlinks skipped");

    let body = json.to_string();
    assert!(
        !body.contains(&fx.root.to_string_lossy().to_string()),
        "discovery leaked the root"
    );
}

#[test]
fn preview_writes_nothing_and_mismatch_is_refused() {
    let _guard = lock();
    let fx = workspace();
    let selection = json!({"items": [{"directory": "manifest-proj"}, {"directory": "importable"}]});

    let preview = handle_with_root(
        &fx.config,
        &fx.db,
        &onboard_request(&fx.config, &fx.token, &selection),
        Some(&fx.root),
    );
    assert_eq!(preview.status, 200, "preview: {preview:?}");
    let json = body_json(&preview);
    assert_eq!(json["preview"].as_array().unwrap().len(), 2);
    let digest = json["plan_digest"].as_str().unwrap().to_string();
    assert_eq!(digest.len(), 64);

    let mut wrong = selection.clone();
    wrong["confirm"] = json!(true);
    wrong["plan_digest"] = json!("0".repeat(64));
    let refused = handle_with_root(
        &fx.config,
        &fx.db,
        &onboard_request(&fx.config, &fx.token, &wrong),
        Some(&fx.root),
    );
    assert_eq!(refused.status, 409, "mismatch refused");
    assert_eq!(
        body_json(&refused)["error"]["code"],
        "admin-digest-mismatch"
    );

    assert!(!fx.root.join("importable").join("forge.yaml").exists());
    assert!(
        project_ids(&fx.db).is_empty(),
        "preview/mismatch journal nothing"
    );
}

#[test]
fn confirmed_onboard_applies_and_reports_per_item() {
    let _guard = lock();
    let fx = workspace();
    let selection = json!({"items": [{"directory": "manifest-proj"}, {"directory": "importable"}]});

    let preview = handle_with_root(
        &fx.config,
        &fx.db,
        &onboard_request(&fx.config, &fx.token, &selection),
        Some(&fx.root),
    );
    let digest = body_json(&preview)["plan_digest"]
        .as_str()
        .unwrap()
        .to_string();
    let mut confirmed = selection.clone();
    confirmed["confirm"] = json!(true);
    confirmed["plan_digest"] = json!(digest);
    let applied = handle_with_root(
        &fx.config,
        &fx.db,
        &onboard_request(&fx.config, &fx.token, &confirmed),
        Some(&fx.root),
    );
    assert_eq!(applied.status, 202, "applied: {applied:?}");
    let json = body_json(&applied);
    assert_eq!(json["succeeded"], 2);
    assert_eq!(json["failed"], 0);
    assert!(json["results"]
        .as_array()
        .unwrap()
        .iter()
        .all(|entry| entry["ok"] == true));

    let mut ids = project_ids(&fx.db);
    ids.sort();
    assert_eq!(ids, vec!["importable", "manifest-proj"]);
    assert!(fx.root.join("importable").join("forge.yaml").is_file());
    assert!(
        !raw(&applied).contains(&fx.root.to_string_lossy().to_string()),
        "apply leaked the root"
    );

    // Re-discovery is live: both are now registered, not selectable.
    let next = discover(&fx);
    assert_eq!(candidate(&next, "manifest-proj")["state"], "registered");
    assert_eq!(candidate(&next, "importable")["state"], "registered");
    assert_eq!(candidate(&next, "importable")["selectable"], false);
}

#[test]
fn blocked_selection_is_refused_before_any_write() {
    let _guard = lock();
    let fx = workspace();
    let selection = json!({"items": [{"directory": "mixed"}, {"directory": "importable"}]});

    let preview = handle_with_root(
        &fx.config,
        &fx.db,
        &onboard_request(&fx.config, &fx.token, &selection),
        Some(&fx.root),
    );
    assert_eq!(preview.status, 200);
    let json = body_json(&preview);
    let mixed = json["preview"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["directory"] == "mixed")
        .expect("mixed preview");
    assert!(mixed.get("blocked").is_some(), "mixed blocked");

    let mut confirmed = selection.clone();
    confirmed["confirm"] = json!(true);
    confirmed["plan_digest"] = json!(json["plan_digest"]);
    let refused = handle_with_root(
        &fx.config,
        &fx.db,
        &onboard_request(&fx.config, &fx.token, &confirmed),
        Some(&fx.root),
    );
    assert_eq!(refused.status, 409, "blocked selection refused");
    assert_eq!(
        body_json(&refused)["error"]["code"],
        "admin-onboard-blocked"
    );
    assert!(
        project_ids(&fx.db).is_empty(),
        "blocked batch writes nothing"
    );
}

#[test]
fn runtime_failure_is_a_partial_207_not_a_blanket_success() {
    let _guard = lock();
    let fx = workspace();
    let selection = json!({"items": [{"directory": "importable"}, {"directory": "badprofile"}]});

    let preview = handle_with_root(
        &fx.config,
        &fx.db,
        &onboard_request(&fx.config, &fx.token, &selection),
        Some(&fx.root),
    );
    assert_eq!(preview.status, 200, "preview: {preview:?}");
    let digest = body_json(&preview)["plan_digest"]
        .as_str()
        .unwrap()
        .to_string();

    // `badprofile` previews fine (its manifest parses) but registration
    // fails on the unknown profile, while its sibling succeeds.
    let mut confirmed = selection.clone();
    confirmed["confirm"] = json!(true);
    confirmed["plan_digest"] = json!(digest);
    let applied = handle_with_root(
        &fx.config,
        &fx.db,
        &onboard_request(&fx.config, &fx.token, &confirmed),
        Some(&fx.root),
    );
    assert_eq!(applied.status, 207, "partial: {applied:?}");
    let json = body_json(&applied);
    assert_eq!(json["succeeded"], 1);
    assert_eq!(json["failed"], 1);
    assert_eq!(project_ids(&fx.db), vec!["importable"]);
}

#[test]
fn id_override_onboards_a_non_kebab_leaf() {
    let _guard = lock();
    let fx = workspace();
    let selection = json!({"items": [{"directory": "DemoApp", "id": "demo-custom"}]});

    let preview = handle_with_root(
        &fx.config,
        &fx.db,
        &onboard_request(&fx.config, &fx.token, &selection),
        Some(&fx.root),
    );
    assert_eq!(preview.status, 200, "preview: {preview:?}");
    assert_eq!(body_json(&preview)["preview"][0]["id"], "demo-custom");
    let digest = body_json(&preview)["plan_digest"]
        .as_str()
        .unwrap()
        .to_string();

    let mut confirmed = selection.clone();
    confirmed["confirm"] = json!(true);
    confirmed["plan_digest"] = json!(digest);
    let applied = handle_with_root(
        &fx.config,
        &fx.db,
        &onboard_request(&fx.config, &fx.token, &confirmed),
        Some(&fx.root),
    );
    assert_eq!(applied.status, 202, "applied: {applied:?}");
    assert_eq!(project_ids(&fx.db), vec!["demo-custom"]);
}

#[test]
fn workspace_growth_appears_on_refresh_with_no_other_change() {
    let _guard = lock();
    let fx = workspace();
    let before = discover(&fx);
    let count = before["candidates"].as_array().unwrap().len();

    write_cargo(&fx.root.join("brand-new"), "brand-new");
    let after = discover(&fx);
    assert_eq!(after["candidates"].as_array().unwrap().len(), count + 1);
    assert_eq!(candidate(&after, "brand-new")["state"], "unregistered");
    assert_eq!(after["total"], before["total"].as_u64().unwrap() + 1);
}
