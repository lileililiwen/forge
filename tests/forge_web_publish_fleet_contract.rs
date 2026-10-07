//! Contract tests for the fleet's local publish-history projection
//! (`forge-web-publish-fleet`): the most recent `publish` operation per
//! project from the local registry `operations` journal, projected into the
//! authenticated `GET /v1/admin/projects` fleet.
//!
//! The projection reads process-global environment variables, so every test
//! in this binary serializes on one lock (mirroring the fleet contract) to
//! keep one test's configuration from racing another's reads.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Mutex;

use chrono::Utc;
use forge::api::{handle, ApiConfig, ApiRequest};
use forge::registry::{PublishPhaseEvidence, Registry};
use serde_json::{json, Value};
use tempfile::TempDir;

static SERIAL: Mutex<()> = Mutex::new(());

const EMAIL: &str = "operator@example.test";
const PASSWORD: &str = "a-long-test-password";
const SHA: &str = "0123456789abcdef0123456789abcdef01234567";

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

fn lock() -> std::sync::MutexGuard<'static, ()> {
    match SERIAL.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// Remove every source-selection variable so a test never inherits another
/// test's configuration.
fn clear_source_env() {
    std::env::remove_var("FORGE_INVENTORY_SOURCE");
    std::env::remove_var("FORGE_WORKSPACE_REGISTRY");
    std::env::remove_var("FORGE_SELF_ID");
    std::env::remove_var("FORGE_FLEET_MAX_AGE_SECONDS");
    std::env::remove_var("FORGE_PUBLISH_HISTORY");
    std::env::remove_var("FORGE_PUBLISH_HISTORY_LIMIT");
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

/// Append one publish journal row with the additive phase evidence. Each call
/// gets a higher `op_id`, so the last call is the most recent.
fn record_publish(db: &Path, project_id: &str, state: &str, detail: &str) {
    let registry = Registry::open(db).unwrap();
    registry
        .record_publish_phase(
            project_id,
            state,
            PublishPhaseEvidence::new()
                .revision(SHA)
                .build_status("succeeded")
                .run_status("running")
                .container_identity(&format!("forge-{project_id}-0123456789ab")),
            Some(detail),
        )
        .unwrap();
}

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
fn empty_publish_journal_reports_an_available_empty_source() {
    let _guard = lock();
    clear_source_env();
    let dir = TempDir::new().unwrap();
    let db = dir.path().join("registry.db");
    setup_admin(&db);
    let config = ApiConfig::default();

    let (envelope, _) = fetch_fleet(&config, &db);
    assert_eq!(
        envelope["projects"].as_array().unwrap().len(),
        1,
        "an empty journal still yields only the self row"
    );
    let published = source(&envelope, "published").unwrap();
    assert_eq!(published["status"], json!("available"));
    assert_eq!(published["count"], json!(0));
    assert_eq!(envelope["summary"]["by_source"]["published"], json!(0));
    clear_source_env();
}

#[test]
fn published_only_project_appears_as_an_observed_row() {
    let _guard = lock();
    clear_source_env();
    let dir = TempDir::new().unwrap();
    let db = dir.path().join("registry.db");
    setup_admin(&db);
    record_publish(&db, "shipped", "done", "fleet stages=4 healthy=true");
    let config = ApiConfig::default();

    let (envelope, _) = fetch_fleet(&config, &db);
    let row = &rows_for(&envelope, "shipped")[0];
    assert_eq!(row["source"], json!("published"));
    assert_eq!(row["management"], json!("observed"));
    assert_eq!(row["state"], json!("done"));
    assert!(
        row["capabilities"].as_array().unwrap().is_empty(),
        "an observed published row exposes no Forge operation"
    );
    assert_eq!(row["publish"]["state"], json!("done"));
    assert_eq!(row["publish"]["healthy"], json!(true));
    assert_eq!(row["publish"]["stages"], json!(4));
    assert_eq!(row["publish"]["revision"], json!(SHA));
    assert_eq!(source(&envelope, "published").unwrap()["count"], json!(1));
    assert_eq!(envelope["summary"]["by_source"]["published"], json!(1));
    clear_source_env();
}

#[test]
fn registered_and_published_project_stays_managed_and_merged() {
    let _guard = lock();
    clear_source_env();
    let dir = TempDir::new().unwrap();
    let db = dir.path().join("registry.db");
    setup_admin(&db);
    register_project(&db, dir.path(), "alpha");
    record_publish(
        &db,
        "alpha",
        "done",
        "publish all summary: healthy=true stages=4",
    );
    let config = ApiConfig::default();

    let (envelope, _) = fetch_fleet(&config, &db);
    let alpha_rows = rows_for(&envelope, "alpha");
    assert_eq!(
        alpha_rows.len(),
        1,
        "the published id is merged, not duplicated"
    );
    let alpha = alpha_rows[0];
    assert_eq!(alpha["source"], json!("registry"));
    assert_eq!(alpha["management"], json!("managed"));
    assert_eq!(alpha["conflict"], json!(false));
    assert!(
        alpha["capabilities"]
            .as_array()
            .unwrap()
            .contains(&json!("inspect")),
        "a merged managed row keeps its inspect capability"
    );
    assert_eq!(alpha["publish"]["state"], json!("done"));
    assert_eq!(alpha["publish"]["healthy"], json!(true));
    clear_source_env();
}

#[test]
fn only_the_most_recent_publish_is_projected() {
    let _guard = lock();
    clear_source_env();
    let dir = TempDir::new().unwrap();
    let db = dir.path().join("registry.db");
    setup_admin(&db);
    record_publish(
        &db,
        "cycler",
        "failed",
        "publish sync via jenkins: sync failed (exit 23)",
    );
    record_publish(&db, "cycler", "done", "fleet stages=4 healthy=true");
    let config = ApiConfig::default();

    let (envelope, _) = fetch_fleet(&config, &db);
    let rows = rows_for(&envelope, "cycler");
    assert_eq!(rows.len(), 1, "only the most recent publish is projected");
    assert_eq!(rows[0]["state"], json!("done"));
    assert_eq!(rows[0]["publish"]["state"], json!("done"));
    assert_eq!(source(&envelope, "published").unwrap()["count"], json!(1));
    clear_source_env();
}

#[test]
fn publish_history_can_be_disabled() {
    let _guard = lock();
    clear_source_env();
    let dir = TempDir::new().unwrap();
    let db = dir.path().join("registry.db");
    setup_admin(&db);
    record_publish(&db, "shipped", "done", "fleet stages=4 healthy=true");
    std::env::set_var("FORGE_PUBLISH_HISTORY", "0");
    let config = ApiConfig::default();

    let (envelope, _) = fetch_fleet(&config, &db);
    assert!(
        rows_for(&envelope, "shipped").is_empty(),
        "a disabled projection contributes no rows"
    );
    let published = source(&envelope, "published").unwrap();
    assert_eq!(published["status"], json!("unconfigured"));
    assert!(published["reason"].is_string(), "a safe reason is present");
    assert_eq!(envelope["summary"]["self_present"], json!(true));
    clear_source_env();
}

#[test]
fn publish_detail_paths_are_redacted() {
    let _guard = lock();
    clear_source_env();
    let dir = TempDir::new().unwrap();
    let db = dir.path().join("registry.db");
    setup_admin(&db);
    let leak = dir.path().join("secret-project");
    record_publish(
        &db,
        "shipped",
        "failed",
        &format!("publish sync via jenkins: rsync {} failed", leak.display()),
    );
    let config = ApiConfig::default();

    let (envelope, text) = fetch_fleet(&config, &db);
    let row = &rows_for(&envelope, "shipped")[0];
    let detail = row["publish"]["detail"].as_str().unwrap();
    assert!(
        detail.contains("[local path]"),
        "the absolute path is replaced: {detail}"
    );
    assert!(!text.contains(leak.to_str().unwrap()), "path must not leak");
    assert!(
        !text.contains(dir.path().to_str().unwrap()),
        "temp root must not leak"
    );
    assert!(!text.contains("/home/"), "home path leaked: {text}");
    clear_source_env();
}

#[test]
fn anonymous_publish_fleet_request_is_denied_without_any_data() {
    let _guard = lock();
    clear_source_env();
    let dir = TempDir::new().unwrap();
    let db = dir.path().join("registry.db");
    setup_admin(&db);
    record_publish(&db, "shipped", "done", "fleet stages=4 healthy=true");
    let config = ApiConfig::default();

    let anon = request("GET", "/v1/admin/projects", &config.frontend_origin);
    let response = handle(&config, &db, &anon, Utc::now());
    assert_eq!(response.status, 401);
    let body: Value = serde_json::from_slice(&response.body).unwrap();
    assert!(body.get("projects").is_none(), "no rows leak to anonymous");
    assert!(body.get("sources").is_none(), "no source details leak");
    clear_source_env();
}
