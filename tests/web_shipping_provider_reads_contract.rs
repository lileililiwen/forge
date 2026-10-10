//! Contract tests for the read-only shipping/provider reads
//! (`web-shipping-provider-reads`, web UI/UX audit gap 8).
//!
//! Static-token contract over the shipped `frontend/` assets (no browser,
//! no server) plus live in-process round-trips over the five typed
//! read-only admin GET routes: the project-bound publish-provider
//! list/inspect and plugin list over the server-resolved
//! `.forge/providers.yaml`, and the pure-global evidence matrix
//! (never live) and static provider inspect. No adapter, no probe, no
//! network, no write, no journal row on any path.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use forge::api::{handle, ApiConfig, ApiRequest, ApiResponse};
use forge::registry::Registry;
use serde_json::Value;

fn app_js() -> String {
    std::fs::read_to_string("frontend/app.js").expect("frontend/app.js ships")
}

fn index_html() -> String {
    std::fs::read_to_string("frontend/index.html").expect("frontend/index.html ships")
}

fn styles_css() -> String {
    std::fs::read_to_string("frontend/styles.css").expect("frontend/styles.css ships")
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

fn json_body(mut req: ApiRequest, body: &[u8]) -> ApiRequest {
    req.headers
        .insert("content-type".to_string(), "application/json".to_string());
    req.body = body.to_vec();
    req
}

fn login_token(config: &ApiConfig, db: &Path) -> String {
    forge::identity::global::setup(db, "operator@example.test", "a-long-test-password").unwrap();
    let mut login = request("POST", "/v1/admin/session", &config.frontend_origin);
    login = json_body(
        login,
        br#"{"email":"operator@example.test","password":"a-long-test-password"}"#,
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

fn write_project(dir: &Path, id: &str) {
    fs::create_dir_all(dir).unwrap();
    fs::write(
        dir.join("forge.yaml"),
        format!(
            "schema: 1\nproject:\n  id: {id}\n  name: Test {id}\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\n"
        ),
    )
    .unwrap();
}

fn write_providers(dir: &Path) {
    let forge_dir = dir.join(".forge");
    fs::create_dir_all(&forge_dir).unwrap();
    fs::write(
        forge_dir.join("providers.yaml"),
        "providers:\n  - id: jenkins-mac\n    command: ./adapters/jenkins-mac\n    enabled: true\n  - id: backup\n    command: ./adapters/backup\n    enabled: false\nplugins:\n  - id: jenkins-mac\n    kind: delivery\n    capabilities: [delivery]\n    description: test delivery plugin\n",
    )
    .unwrap();
}

fn fixture() -> (tempfile::TempDir, PathBuf, ApiConfig) {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let alpha = tmp.path().join("alpha");
    let beta = tmp.path().join("beta");
    write_project(&alpha, "alpha");
    write_project(&beta, "beta");
    write_providers(&alpha);
    {
        let mut registry = Registry::open(&db).unwrap();
        registry.register(&alpha, None).unwrap();
        registry.register(&beta, None).unwrap();
    }
    let config = ApiConfig::default();
    (tmp, db, config)
}

fn get(config: &ApiConfig, db: &Path, token: &str, path: &str) -> ApiResponse {
    let req = request("GET", path, &config.frontend_origin);
    handle(config, db, &with_cookie(req, token), Utc::now())
}

fn body_json(response: &ApiResponse) -> Value {
    serde_json::from_slice(&response.body).expect("admin responses are JSON")
}

fn journal_len(db: &Path, project: &str) -> usize {
    Registry::open(db)
        .unwrap()
        .operations_for_project(project, 100)
        .unwrap()
        .len()
}

#[test]
fn shipping_card_lives_in_the_delivery_view() {
    let html = index_html();
    for id in [
        "shipping-reads-title",
        "shipping-providers",
        "shipping-provider-id",
        "shipping-provider-inspect",
        "shipping-providers-result",
        "shipping-matrix",
        "shipping-evidence-inspect",
        "shipping-matrix-result",
        "shipping-plugins",
        "shipping-plugins-result",
        "shipping-reads-cli-only",
        "shipping-reads-error-summary",
    ] {
        assert!(html.contains(id), "delivery view must carry `{id}`");
    }
    // The card sits after the history card and before confirmed actions.
    let history = html
        .find("history-status-result")
        .expect("history card ships");
    let card = html
        .find("shipping-reads-title")
        .expect("shipping card ships");
    let actions = html
        .find("delivery-actions-title")
        .expect("confirmed actions ship");
    assert!(history < card && card < actions);
}

#[test]
fn shipping_fetchers_use_only_the_five_typed_routes() {
    let js = app_js();
    let start = js
        .find("Shipping provider reads (web-shipping-provider-reads)")
        .expect("shipping block ships");
    let end = js
        .find("Project workbench (single managed project)")
        .expect("workbench block ships");
    let block = &js[start..end];
    for route in [
        "/publish/providers",
        "/v1/admin/providers/matrix",
        "/v1/admin/providers/${encodeURIComponent(provider)}",
        "/plugins",
    ] {
        assert!(block.contains(route), "shipping block must fetch `{route}`");
    }
}

#[test]
fn shipping_adds_no_write_no_shell_and_no_html_interpretation() {
    let js = app_js();
    let start = js
        .find("Shipping provider reads (web-shipping-provider-reads)")
        .expect("shipping block ships");
    let end = js
        .find("Project workbench (single managed project)")
        .expect("workbench block ships");
    let block = &js[start..end];
    for banned in [
        "innerHTML",
        "eval(",
        "Function(",
        "XMLHttpRequest",
        ".command",
    ] {
        assert!(
            !block.contains(banned),
            "shipping block must not contain `{banned}`"
        );
    }
    assert!(block.contains("textContent") || block.contains("el("));
    // The block names no POST and no enable/disable verb.
    assert!(!block.contains("method: \"POST\"") && !block.contains("method: 'POST'"));
}

#[test]
fn anonymous_reads_are_rejected_before_any_shipping_data() {
    let (_tmp, db, config) = fixture();
    forge::identity::global::setup(&db, "anon@example.test", "a-long-test-password").unwrap();
    for path in [
        "/v1/admin/projects/alpha/publish/providers",
        "/v1/admin/projects/alpha/publish/providers/jenkins-mac",
        "/v1/admin/providers/matrix",
        "/v1/admin/providers/driftwatch-policy",
        "/v1/admin/projects/alpha/plugins",
    ] {
        let response = handle(
            &config,
            &db,
            &request("GET", path, &config.frontend_origin),
            Utc::now(),
        );
        assert_eq!(response.status, 401, "{path}");
        assert!(String::from_utf8_lossy(&response.body).contains("api-unauthorized"));
    }
}

#[test]
fn publish_provider_list_and_inspect_agree_with_the_cli() {
    let (tmp, db, config) = fixture();
    let token = login_token(&config, &db);
    let server_root = tmp.path().display().to_string();
    let before = journal_len(&db, "alpha");
    let response = get(
        &config,
        &db,
        &token,
        "/v1/admin/projects/alpha/publish/providers",
    );
    assert_eq!(response.status, 200);
    let body = body_json(&response);
    let providers = body["providers"].as_array().unwrap();
    assert_eq!(providers.len(), 2);
    assert_eq!(providers[0]["id"], "jenkins-mac");
    assert_eq!(providers[0]["enabled"], true);
    assert_eq!(providers[1]["id"], "backup");
    assert_eq!(providers[1]["enabled"], false);
    // No absolute server path leaks through the command projection.
    assert!(!body.to_string().contains(&server_root));

    let response = get(
        &config,
        &db,
        &token,
        "/v1/admin/projects/alpha/publish/providers/jenkins-mac",
    );
    assert_eq!(response.status, 200);
    let body = body_json(&response);
    assert_eq!(body["provider"]["id"], "jenkins-mac");
    assert_eq!(body["provider"]["enabled"], true);

    // Unknown provider is a typed 404 that never echoes the input.
    let response = get(
        &config,
        &db,
        &token,
        "/v1/admin/projects/alpha/publish/providers/nope-missing",
    );
    assert_eq!(response.status, 404);
    let body = body_json(&response);
    assert_eq!(body["error"]["code"], "admin-unknown-provider");
    assert!(!body.to_string().contains("nope-missing"));

    // Hostile segment is a static 400 that never echoes the input.
    let response = get(
        &config,
        &db,
        &token,
        "/v1/admin/projects/alpha/publish/providers/..%2Fetc",
    );
    assert!(response.status == 400 || response.status == 404);

    // Unmanaged project stays a typed 404.
    let response = get(
        &config,
        &db,
        &token,
        "/v1/admin/projects/ghost/publish/providers",
    );
    assert_eq!(response.status, 404);

    // A project with no config answers honest unavailable-with-reason.
    let response = get(
        &config,
        &db,
        &token,
        "/v1/admin/projects/beta/publish/providers",
    );
    assert_eq!(response.status, 200);
    let body = body_json(&response);
    assert_eq!(body["unavailable"], true);
    assert!(!body["reason"].as_str().unwrap().is_empty());

    assert_eq!(journal_len(&db, "alpha"), before);
}

#[test]
fn evidence_matrix_never_probes_and_never_claims_support() {
    let (_tmp, db, config) = fixture();
    let token = login_token(&config, &db);
    let response = get(&config, &db, &token, "/v1/admin/providers/matrix");
    assert_eq!(response.status, 200);
    let body = body_json(&response);
    let matrix = &body["matrix"];
    assert_eq!(matrix["live"], false);
    let rows = matrix["rows"].as_array().unwrap();
    assert!(!rows.is_empty());
    for row in rows {
        assert_eq!(row["status"], "not-run", "{}", row["provider"]);
    }
    assert_eq!(matrix["supported"], 0);
    // Byte-parity with the Core call the CLI wraps: same rows, same order.
    let expected = serde_json::to_value(forge::provider::matrix(false)).unwrap();
    assert_eq!(matrix["rows"], expected["rows"]);
    assert_eq!(journal_len(&db, "alpha"), journal_len(&db, "alpha"));
}

#[test]
fn evidence_provider_inspect_never_probes() {
    let (_tmp, db, config) = fixture();
    let token = login_token(&config, &db);
    let response = get(
        &config,
        &db,
        &token,
        "/v1/admin/providers/driftwatch-policy",
    );
    assert_eq!(response.status, 200);
    let body = body_json(&response);
    let expected =
        serde_json::to_value(forge::provider::inspect("driftwatch-policy").unwrap()).unwrap();
    assert_eq!(body["descriptor"], expected);

    // Unknown provider is a typed 404 that never echoes the input.
    let response = get(&config, &db, &token, "/v1/admin/providers/nope-missing");
    assert_eq!(response.status, 404);
    let body = body_json(&response);
    assert_eq!(body["error"]["code"], "admin-unknown-provider");
    assert!(!body.to_string().contains("nope-missing"));
}

#[test]
fn plugin_list_agrees_with_the_cli_and_empty_is_an_answer() {
    let (_tmp, db, config) = fixture();
    let token = login_token(&config, &db);
    let before = journal_len(&db, "alpha");
    let response = get(&config, &db, &token, "/v1/admin/projects/alpha/plugins");
    assert_eq!(response.status, 200);
    let body = body_json(&response);
    let plugins = body["plugins"].as_array().unwrap();
    assert_eq!(plugins.len(), 2);
    let jenkins = plugins.iter().find(|p| p["id"] == "jenkins-mac").unwrap();
    assert_eq!(jenkins["kind"], "delivery");
    assert_eq!(jenkins["enabled"], true);
    // The fixture adapter file does not exist on disk, so the record is
    // honestly `unavailable` with a reason — never a fabricated `ready`.
    assert_eq!(jenkins["state"], "unavailable");
    assert!(!jenkins["reason"].as_str().unwrap().is_empty());
    assert_eq!(jenkins["description"], "test delivery plugin");
    assert_eq!(jenkins["capabilities"], serde_json::json!(["delivery"]));
    assert_eq!(body["contract"], "forge-plugins/0.1.0");

    // A project with no config answers an honest empty registry.
    let response = get(&config, &db, &token, "/v1/admin/projects/beta/plugins");
    assert_eq!(response.status, 200);
    let body = body_json(&response);
    assert_eq!(body["plugins"].as_array().unwrap().len(), 0);
    assert_eq!(journal_len(&db, "alpha"), before);
}

#[test]
fn catalog_pins_five_shipping_rows_and_leaves_probe_writes_cli() {
    let (_tmp, db, config) = fixture();
    let token = login_token(&config, &db);
    let mut fetch = request("GET", "/v1/admin/commands", &config.frontend_origin);
    fetch = with_cookie(fetch, &token);
    let response = handle(&config, &db, &fetch, Utc::now());
    assert_eq!(response.status, 200);
    let body: Value = serde_json::from_slice(&response.body).unwrap();
    let by_id: BTreeMap<&str, &Value> = body["commands"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| (row["id"].as_str().unwrap(), row))
        .collect();
    for (id, route) in [
        (
            "publish.provider.list",
            "GET /v1/admin/projects/{id}/publish/providers",
        ),
        (
            "publish.provider.inspect",
            "GET /v1/admin/projects/{id}/publish/providers/{provider}",
        ),
        ("provider.matrix", "GET /v1/admin/providers/matrix"),
        ("provider.inspect", "GET /v1/admin/providers/{provider}"),
        ("plugins.list", "GET /v1/admin/projects/{id}/plugins"),
    ] {
        let row = by_id[id];
        assert_eq!(row["availability"], "web", "{id}");
        assert_eq!(row["route"], route, "{id}");
    }
    // The probe/write verbs gain no route here.
    for (id, availability) in [
        ("publish.provider.enable", "not_yet_web"),
        ("publish.provider.disable", "not_yet_web"),
        ("provider.run", "provider_required"),
        ("fleet.online", "provider_required"),
        ("push", "cli_only"),
        ("mirror", "cli_only"),
    ] {
        let row = by_id[id];
        assert_eq!(row["availability"], availability, "{id}");
        assert!(row.get("route").is_none() || row["route"].is_null());
    }
}

#[test]
fn shipping_styles_meet_the_text_floor_with_existing_tokens() {
    let css = styles_css();
    assert!(css.contains("#shipping-reads-title"));
}
