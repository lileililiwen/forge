//! Contract tests for the normalized, provenance-bearing web fleet read
//! model served by `GET /v1/admin/projects` (`forge-web-project-fleet`).
//!
//! These drive the real authenticated JSON handler so the aggregation,
//! source states, conflict semantics, path redaction and the always-present
//! Forge-self record are proven end to end. External sources are exercised
//! through the explicit environment configuration path only; nothing is
//! scanned and no absolute filesystem path may reach a browser response.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use chrono::Utc;
use forge::api::{handle, ApiConfig, ApiRequest};
use forge::registry::Registry;
use serde_json::{json, Value};
use tempfile::TempDir;

/// The fleet aggregator resolves external sources from process-global
/// environment variables, so every test in this binary serializes on one
/// lock to avoid racing another test's environment with this one's reads.
static SERIAL: Mutex<()> = Mutex::new(());

const SHA: &str = "0123456789abcdef0123456789abcdef01234567";
const EMAIL: &str = "operator@example.test";
const PASSWORD: &str = "a-long-test-password";

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

/// Lock the serialization guard, tolerating a poisoned lock so a prior
/// panic does not cascade into unrelated failures.
fn lock() -> std::sync::MutexGuard<'static, ()> {
    match SERIAL.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// Put the aggregator back into the fully unconfigured baseline so a test
/// never inherits another test's source selection.
fn clear_source_env() {
    std::env::remove_var("FORGE_INVENTORY_SOURCE");
    std::env::remove_var("FORGE_WORKSPACE_REGISTRY");
    std::env::remove_var("FORGE_SELF_ID");
    std::env::remove_var("FORGE_FLEET_MAX_AGE_SECONDS");
}

fn setup_admin(db: &Path) {
    forge::identity::global::setup(db, EMAIL, PASSWORD).unwrap();
}

fn login_token(config: &ApiConfig, db: &Path) -> String {
    let mut login = request("POST", "/v1/admin/session", &config.frontend_origin);
    login
        .headers
        .insert("content-type".to_string(), "application/json".to_string());
    login.body = json!({ "email": EMAIL, "password": PASSWORD })
        .to_string()
        .into_bytes();
    let response = handle(config, db, &login, Utc::now());
    assert_eq!(response.status, 200, "login must succeed");
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

/// Register one minimal project (id == `id`) into the given registry db.
fn register_project(db: &Path, parent: &Path, id: &str) {
    let dir = parent.join(id);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("forge.yaml"),
        format!(
            "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: rust-web\n  maturity: L1\n  target_maturity: L2\n"
        ),
    )
    .unwrap();
    let mut registry = Registry::open(db).unwrap();
    registry.register(&dir, None).unwrap();
}

/// Write a portable inventory document with the given project objects.
fn write_inventory(dir: &Path, projects: Vec<Value>) -> PathBuf {
    let path = dir.join("inventory.json");
    let document = json!({
        "contract": "forge-project-inventory/0.1.0",
        "provider": "local",
        "generated_at": Utc::now().to_rfc3339(),
        "projects": projects,
    });
    std::fs::write(&path, serde_json::to_vec(&document).unwrap()).unwrap();
    path
}

fn inventory_project(id: &str) -> Value {
    json!({
        "id": id,
        "repository": format!("https://example.invalid/{id}.git"),
        "revision": SHA,
        "profile": "rust-product",
        "runtime": "library",
    })
}

/// Write a workspace fleet registry (`projects.json`) with the given entries.
fn write_fleet_registry(root: &Path, entries: Vec<Value>) -> PathBuf {
    let path = root.join("projects.json");
    let document = json!({
        "schema_version": 1,
        "workspace_root": null,
        "discovery": { "mode": "explicit", "unregistered_policy": "report" },
        "projects": entries,
    });
    std::fs::write(&path, serde_json::to_vec(&document).unwrap()).unwrap();
    path
}

fn fleet_entry(id: &str) -> Value {
    json!({ "id": id, "path": id, "profile": "typescript-monorepo", "lifecycle": "active" })
}

/// Fetch the fleet envelope for an authenticated session, asserting 200 and
/// returning the parsed body plus the raw text (for path-leak assertions).
fn fetch_fleet(config: &ApiConfig, db: &Path) -> (Value, String) {
    let token = login_token(config, db);
    let mut req = request("GET", "/v1/admin/projects", &config.frontend_origin);
    req.cookies.insert("forge_admin_session".to_string(), token);
    let response = handle(config, db, &req, Utc::now());
    assert_eq!(response.status, 200, "authenticated fleet read");
    let text = String::from_utf8(response.body.clone()).unwrap();
    (serde_json::from_str(&text).unwrap(), text)
}

fn source<'a>(envelope: &'a Value, id: &str) -> Option<&'a Value> {
    envelope["sources"]
        .as_array()?
        .iter()
        .find(|entry| entry["id"] == json!(id))
}

fn rows_for<'a>(envelope: &'a Value, identity: &str) -> Vec<&'a Value> {
    envelope["projects"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["identity"] == json!(identity))
        .collect()
}

#[test]
fn anonymous_fleet_request_is_denied_without_any_data() {
    let _guard = lock();
    clear_source_env();
    let dir = TempDir::new().unwrap();
    let db = dir.path().join("registry.db");
    setup_admin(&db);
    let config = ApiConfig::default();

    let anon = request("GET", "/v1/admin/projects", &config.frontend_origin);
    let response = handle(&config, &db, &anon, Utc::now());
    assert_eq!(response.status, 401);
    let body: Value = serde_json::from_slice(&response.body).unwrap();
    assert!(
        body.get("projects").is_none(),
        "no fleet rows leak to anonymous"
    );
    assert!(body.get("sources").is_none(), "no source details leak");
}

#[test]
fn fleet_always_includes_the_self_record_on_an_empty_registry() {
    let _guard = lock();
    clear_source_env();
    let dir = TempDir::new().unwrap();
    let db = dir.path().join("registry.db");
    setup_admin(&db);
    let config = ApiConfig::default();

    let (envelope, _) = fetch_fleet(&config, &db);
    let projects = envelope["projects"].as_array().unwrap();
    assert_eq!(
        projects.len(),
        1,
        "empty registry still yields the self row"
    );
    assert_eq!(envelope["summary"]["self_present"], json!(true));
    assert_eq!(envelope["summary"]["registered"], json!(0));
    let self_row = &projects[0];
    assert_eq!(self_row["is_self"], json!(true));
    assert_eq!(self_row["source"], json!("self"));
    assert_eq!(self_row["management"], json!("self"));
    assert!(
        self_row["capabilities"].as_array().unwrap().is_empty(),
        "unregistered self exposes no operation capability"
    );
    // Optional external sources are honestly reported as unconfigured.
    assert_eq!(
        source(&envelope, "inventory").unwrap()["status"],
        json!("unconfigured")
    );
    assert_eq!(
        source(&envelope, "fleet").unwrap()["status"],
        json!("unconfigured")
    );
}

#[test]
fn self_record_merges_with_a_registered_project_of_the_same_id() {
    let _guard = lock();
    clear_source_env();
    let dir = TempDir::new().unwrap();
    let db = dir.path().join("registry.db");
    setup_admin(&db);
    register_project(&db, dir.path(), "forge");
    let config = ApiConfig::default();

    let (envelope, _) = fetch_fleet(&config, &db);
    let forges = rows_for(&envelope, "forge");
    assert_eq!(forges.len(), 1, "registered Forge is not duplicated");
    let row = forges[0];
    assert_eq!(row["is_self"], json!(true));
    assert_eq!(row["management"], json!("self"));
    assert!(
        row["capabilities"]
            .as_array()
            .unwrap()
            .contains(&json!("inspect")),
        "a registered self row is inspectable"
    );
    assert_eq!(envelope["summary"]["registered"], json!(1));
}

#[test]
fn registry_rows_are_labelled_managed_and_self_stays_distinct() {
    let _guard = lock();
    clear_source_env();
    let dir = TempDir::new().unwrap();
    let db = dir.path().join("registry.db");
    setup_admin(&db);
    register_project(&db, dir.path(), "alpha");
    let config = ApiConfig::default();

    let (envelope, _) = fetch_fleet(&config, &db);
    let projects = envelope["projects"].as_array().unwrap();
    assert_eq!(projects.len(), 2, "self row plus one registered project");
    let self_rows = rows_for(&envelope, "forge");
    assert_eq!(self_rows.len(), 1);
    assert_eq!(self_rows[0]["is_self"], json!(true));
    let alpha = &rows_for(&envelope, "alpha")[0];
    assert_eq!(alpha["management"], json!("managed"));
    assert_eq!(alpha["source"], json!("registry"));
    assert!(alpha["capabilities"]
        .as_array()
        .unwrap()
        .contains(&json!("inspect")));
    assert_eq!(envelope["summary"]["registered"], json!(1));
}

#[test]
fn inventory_source_contributes_observed_rows_only() {
    let _guard = lock();
    clear_source_env();
    let dir = TempDir::new().unwrap();
    let db = dir.path().join("registry.db");
    setup_admin(&db);
    let inventory = write_inventory(dir.path(), vec![inventory_project("tool-beta")]);
    std::env::set_var("FORGE_INVENTORY_SOURCE", &inventory);
    let config = ApiConfig::default();

    let (envelope, _) = fetch_fleet(&config, &db);
    let tool = rows_for(&envelope, "tool-beta");
    assert_eq!(tool.len(), 1);
    assert_eq!(tool[0]["source"], json!("inventory"));
    assert_eq!(tool[0]["management"], json!("observed"));
    assert!(
        tool[0]["capabilities"].as_array().unwrap().is_empty(),
        "observed rows expose no Forge operation"
    );
    let inventory_source = source(&envelope, "inventory").unwrap();
    assert_eq!(inventory_source["status"], json!("available"));
    assert_eq!(inventory_source["count"], json!(1));
    clear_source_env();
}

#[test]
fn workspace_fleet_source_contributes_observed_rows() {
    let _guard = lock();
    clear_source_env();
    let dir = TempDir::new().unwrap();
    let db = dir.path().join("registry.db");
    setup_admin(&db);
    let registry = write_fleet_registry(dir.path(), vec![fleet_entry("gamma")]);
    std::env::set_var("FORGE_WORKSPACE_REGISTRY", &registry);
    let config = ApiConfig::default();

    let (envelope, _) = fetch_fleet(&config, &db);
    let gamma = rows_for(&envelope, "gamma");
    assert_eq!(gamma.len(), 1);
    assert_eq!(gamma[0]["source"], json!("fleet"));
    assert_eq!(gamma[0]["management"], json!("observed"));
    assert_eq!(
        source(&envelope, "fleet").unwrap()["status"],
        json!("available")
    );
    clear_source_env();
}

#[test]
fn stale_inventory_source_is_marked_stale_not_healthy() {
    let _guard = lock();
    clear_source_env();
    let dir = TempDir::new().unwrap();
    let db = dir.path().join("registry.db");
    setup_admin(&db);
    // generated_at is years old and the default window is one day, so the
    // source must classify as stale and its rows carry the stale freshness.
    let path = dir.path().join("inventory.json");
    let document = json!({
        "contract": "forge-project-inventory/0.1.0",
        "provider": "local",
        "generated_at": "2000-01-01T00:00:00Z",
        "projects": [inventory_project("ancient")],
    });
    std::fs::write(&path, serde_json::to_vec(&document).unwrap()).unwrap();
    std::env::set_var("FORGE_INVENTORY_SOURCE", &path);
    let config = ApiConfig::default();

    let (envelope, _) = fetch_fleet(&config, &db);
    assert_eq!(
        source(&envelope, "inventory").unwrap()["status"],
        json!("stale")
    );
    let ancient = &rows_for(&envelope, "ancient")[0];
    assert_eq!(ancient["freshness"], json!("stale"));
    clear_source_env();
}

#[test]
fn malformed_inventory_entry_is_named_not_dropped() {
    let _guard = lock();
    clear_source_env();
    let dir = TempDir::new().unwrap();
    let db = dir.path().join("registry.db");
    setup_admin(&db);
    let mut bad = inventory_project("broken");
    bad["revision"] = json!("not-a-sha");
    let inventory = write_inventory(dir.path(), vec![inventory_project("healthy"), bad]);
    std::env::set_var("FORGE_INVENTORY_SOURCE", &inventory);
    let config = ApiConfig::default();

    let (envelope, _) = fetch_fleet(&config, &db);
    // The valid entry still reports; the malformed one is surfaced by name.
    assert_eq!(rows_for(&envelope, "healthy").len(), 1);
    assert!(
        rows_for(&envelope, "broken").is_empty(),
        "malformed row is not emitted"
    );
    let inventory_source = source(&envelope, "inventory").unwrap();
    let malformed = inventory_source["malformed"].as_array().unwrap();
    assert_eq!(malformed.len(), 1);
    assert_eq!(malformed[0]["name"], json!("broken"));
    clear_source_env();
}

#[test]
fn configured_but_unreadable_source_is_unavailable_without_a_path() {
    let _guard = lock();
    clear_source_env();
    let dir = TempDir::new().unwrap();
    let db = dir.path().join("registry.db");
    setup_admin(&db);
    // A configured source that cannot be read (nonexistent file) is
    // unavailable; the underlying error text names an absolute path, so it
    // must never reach the browser.
    let missing = dir.path().join("does-not-exist.json");
    std::env::set_var("FORGE_INVENTORY_SOURCE", &missing);
    let config = ApiConfig::default();

    let (envelope, text) = fetch_fleet(&config, &db);
    let inventory_source = source(&envelope, "inventory").unwrap();
    assert_eq!(inventory_source["status"], json!("unavailable"));
    assert!(
        inventory_source["reason"].is_string(),
        "a safe reason is present"
    );
    // Neither the missing path nor any absolute filesystem path is serialized.
    assert!(
        !text.contains(missing.to_str().unwrap()),
        "path must not leak: {text}"
    );
    assert!(
        !text.contains(dir.path().to_str().unwrap()),
        "temp root must not leak"
    );
    // The other (self) source remains visible despite one failed source.
    assert_eq!(envelope["summary"]["self_present"], json!(true));
    clear_source_env();
}

#[test]
fn identity_conflict_across_sources_is_retained_and_links_disabled() {
    let _guard = lock();
    clear_source_env();
    let dir = TempDir::new().unwrap();
    let db = dir.path().join("registry.db");
    setup_admin(&db);
    register_project(&db, dir.path(), "shared");
    let inventory = write_inventory(dir.path(), vec![inventory_project("shared")]);
    std::env::set_var("FORGE_INVENTORY_SOURCE", &inventory);
    let config = ApiConfig::default();

    let (envelope, _) = fetch_fleet(&config, &db);
    let shared = rows_for(&envelope, "shared");
    assert_eq!(
        shared.len(),
        2,
        "both source records are retained, never collapsed"
    );
    assert!(shared.iter().all(|row| row["conflict"] == json!(true)));
    assert!(
        shared
            .iter()
            .all(|row| row["capabilities"].as_array().unwrap().is_empty()),
        "ambiguous records cannot initiate mutations"
    );
    clear_source_env();
}

#[test]
fn no_absolute_path_is_serialized_with_all_sources_active() {
    let _guard = lock();
    clear_source_env();
    let dir = TempDir::new().unwrap();
    let db = dir.path().join("registry.db");
    setup_admin(&db);
    register_project(&db, dir.path(), "local-one");
    let inventory = write_inventory(dir.path(), vec![inventory_project("inv-one")]);
    let fleet_root = dir.path().join("ws");
    std::fs::create_dir_all(&fleet_root).unwrap();
    let registry = write_fleet_registry(&fleet_root, vec![fleet_entry("fleet-one")]);
    std::env::set_var("FORGE_INVENTORY_SOURCE", &inventory);
    std::env::set_var("FORGE_WORKSPACE_REGISTRY", &registry);
    let config = ApiConfig::default();

    let (envelope, text) = fetch_fleet(&config, &db);
    let temp_root = dir.path().to_str().unwrap();
    assert!(
        !text.contains(temp_root),
        "absolute temp path leaked: {text}"
    );
    assert!(!text.contains("/home/"), "home path leaked");
    // Every declared row from each source is present exactly once.
    assert_eq!(rows_for(&envelope, "local-one").len(), 1);
    assert_eq!(rows_for(&envelope, "inv-one").len(), 1);
    assert_eq!(rows_for(&envelope, "fleet-one").len(), 1);
    assert_eq!(
        rows_for(&envelope, "forge").len(),
        1,
        "self present exactly once"
    );
    clear_source_env();
}
