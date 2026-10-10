//! Contract tests for the read-only creation-catalog browser
//! (`web-creation-catalog-browser`, web UI/UX audit gap 6).
//!
//! Static-token contract over the shipped `frontend/` assets (no browser,
//! no server) plus live in-process round-trips over the twenty typed
//! read-only admin GET routes: the six pure creation catalogs with
//! list/inspect/resolve, structured-intent validation, and the
//! project-bound standard check/diff plus plan-receipt reads.
//! No write, no provider, no adapter, no toolchain probe, no shell,
//! no journal row on any path.

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

fn with_query(mut req: ApiRequest, query: &str) -> ApiRequest {
    req.query = Some(query.to_string());
    req
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

fn write_project(dir: &Path, id: &str, language: &str) {
    fs::create_dir_all(dir).unwrap();
    fs::write(
        dir.join("forge.yaml"),
        format!(
            "schema: 1\nproject:\n  id: {id}\n  name: Test {id}\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: {language}\n"
        ),
    )
    .unwrap();
}

fn fixture() -> (tempfile::TempDir, PathBuf, ApiConfig) {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let alpha = tmp.path().join("alpha");
    write_project(&alpha, "alpha", "rust");
    {
        let mut registry = Registry::open(&db).unwrap();
        registry.register(&alpha, None).unwrap();
    }
    let config = ApiConfig::default();
    (tmp, db, config)
}

fn get(config: &ApiConfig, db: &Path, token: &str, path: &str, query: Option<&str>) -> ApiResponse {
    let mut req = request("GET", path, &config.frontend_origin);
    if let Some(q) = query {
        req = with_query(req, q);
    }
    handle(config, db, &with_cookie(req, token), Utc::now())
}

fn body_json(response: &ApiResponse) -> Value {
    serde_json::from_slice(&response.body).expect("creation routes are JSON-only")
}

fn error_code(response: &ApiResponse) -> String {
    body_json(response)
        .pointer("/error/code")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

#[test]
fn browser_section_lives_on_the_projects_view() {
    let html = index_html();
    let app = app_js();

    for token in [
        "id=\"creation-catalog\"",
        "id=\"creation-title\"",
        "Creation catalog",
        "id=\"creation-registry\"",
        "id=\"creation-search\"",
        "id=\"creation-list\"",
        "id=\"creation-count\"",
        "id=\"creation-list-result\"",
        "id=\"creation-inspect-id\"",
        "id=\"creation-inspect\"",
        "id=\"creation-inspect-result\"",
        "id=\"creation-resolve-profile\"",
        "id=\"creation-resolve-ids\"",
        "id=\"creation-resolve\"",
        "id=\"creation-resolve-result\"",
        "id=\"creation-project\"",
        "id=\"creation-standard-check\"",
        "id=\"creation-standard-against\"",
        "id=\"creation-standard-diff\"",
        "id=\"creation-standard-result\"",
        "id=\"creation-intent-action\"",
        "id=\"creation-intent-profile\"",
        "id=\"creation-intent-validate\"",
        "id=\"creation-intent-plans\"",
        "id=\"creation-intent-result\"",
        "id=\"creation-cli-only\"",
        "id=\"creation-refresh\"",
        "id=\"creation-error-summary\"",
    ] {
        assert!(html.contains(token), "index.html lacks {token}");
    }
    for token in [
        "initCreationCatalog",
        "creationList",
        "creationInspect",
        "creationResolve",
        "creationStandardCheck",
        "creationStandardDiff",
        "creationIntentValidate",
        "creationIntentPlans",
        "creationRenderCliOnly",
        "/v1/admin/creation/",
        "/resolve?",
        "intents/validate?",
        "/standard/check",
        "/standard/diff?",
        "/intent/plans",
    ] {
        assert!(app.contains(token), "app.js lacks {token}");
    }
    for token in [
        "<option value=\"profiles\">",
        "<option value=\"features\">",
        "<option value=\"components\">",
        "<option value=\"ui-patterns\">",
        "<option value=\"standards\">",
        "<option value=\"procedures\">",
    ] {
        assert!(html.contains(token), "index.html lacks {token}");
    }
    // The browser never writes: no POST/PUT/DELETE against a creation
    // path, no shell, no path submission.
    assert!(
        !app.contains("POST /v1/admin/creation") && !app.contains("POST /v1/admin/projects/\"+"),
        "creation browser must stay read-only"
    );
    assert!(!app.contains("eval("), "creation browser uses no eval");
    let start = app
        .find("Creation catalog browser")
        .expect("creation block ships");
    let end = app[start..]
        .find("Project workbench")
        .map(|i| start + i)
        .unwrap_or(app.len());
    assert!(
        !app[start..end].contains("innerHTML"),
        "creation block renders textContent-only"
    );
    let css = styles_css();
    assert!(
        css.contains("#creation-catalog"),
        "styles cover #creation-catalog"
    );
}

#[test]
fn anonymous_creation_reads_are_rejected() {
    let (_tmp, db, config) = fixture();
    forge::identity::global::setup(&db, "op@example.test", "a-long-test-password").unwrap();
    let response = handle(
        &config,
        &db,
        &request(
            "GET",
            "/v1/admin/creation/profiles",
            &config.frontend_origin,
        ),
        Utc::now(),
    );
    assert_eq!(response.status, 401);
    assert_eq!(error_code(&response), "api-unauthorized");
}

#[test]
fn creation_lists_match_the_cli_catalogs() {
    let (_tmp, db, config) = fixture();
    let token = login_token(&config, &db);
    for (registry, contract) in [
        ("profiles", None),
        ("features", Some("0.1.0")),
        ("components", Some("0.1.0")),
        ("ui-patterns", Some("0.1.0")),
        ("standards", None),
        ("procedures", Some("0.1.0")),
    ] {
        let response = get(
            &config,
            &db,
            &token,
            &format!("/v1/admin/creation/{registry}"),
            None,
        );
        assert_eq!(response.status, 200, "{registry} list");
        let body = body_json(&response);
        assert_eq!(body["creation"]["registry"], registry);
        let entries = body["creation"]["entries"].as_array().unwrap();
        assert!(!entries.is_empty(), "{registry} catalog is empty");
        assert!(
            entries.iter().all(|e| e.get("id").is_some()),
            "{registry} entries carry ids"
        );
        if let Some(contract) = contract {
            assert_eq!(
                body["creation"]["contract"], contract,
                "{registry} contract"
            );
        }
        assert_eq!(body["contract"], forge::api::API_CONTRACT_VERSION);
    }
}

#[test]
fn creation_inspects_agree_with_the_cli() {
    let (_tmp, db, config) = fixture();
    let token = login_token(&config, &db);
    for (path, id) in [
        ("/v1/admin/creation/profiles/rust-web", "rust-web"),
        ("/v1/admin/creation/features/auth", "auth"),
        ("/v1/admin/creation/components/audit-action", "audit-action"),
        ("/v1/admin/creation/ui-patterns/login", "login"),
        (
            "/v1/admin/creation/procedures/create-project",
            "create-project",
        ),
    ] {
        let response = get(&config, &db, &token, path, None);
        assert_eq!(response.status, 200, "{path}");
        let body = body_json(&response);
        assert_eq!(body["creation"]["id"], id);
        assert_eq!(body["creation"]["entry"]["id"], id);
    }
    // The standard selector is `<pack>@<version>` from the live list.
    let list = get(&config, &db, &token, "/v1/admin/creation/standards", None);
    let entries = body_json(&list)["creation"]["entries"]
        .as_array()
        .unwrap()
        .clone();
    let first = &entries[0];
    let spec = format!(
        "{}@{}",
        first["id"].as_str().unwrap(),
        first["version"].as_str().unwrap()
    );
    let response = get(
        &config,
        &db,
        &token,
        &format!("/v1/admin/creation/standards/{spec}"),
        None,
    );
    assert_eq!(response.status, 200, "standard inspect {spec}");
    assert_eq!(body_json(&response)["creation"]["id"], spec);
}

#[test]
fn unknown_creation_entries_are_typed_refusals() {
    let (_tmp, db, config) = fixture();
    let token = login_token(&config, &db);
    for (path, code) in [
        ("/v1/admin/creation/profiles/no-such", "unknown-profile"),
        ("/v1/admin/creation/features/no-such", "unknown-feature"),
        ("/v1/admin/creation/components/no-such", "component-invalid"),
        (
            "/v1/admin/creation/ui-patterns/no-such",
            "ui-pattern-invalid",
        ),
        ("/v1/admin/creation/standards/no-such", "standard-invalid"),
        ("/v1/admin/creation/procedures/no-such", "procedure-invalid"),
    ] {
        let response = get(&config, &db, &token, path, None);
        assert_eq!(response.status, 400, "{path}");
        assert_eq!(error_code(&response), code, "{path}");
    }
}

#[test]
fn hostile_creation_ids_never_reach_core() {
    let (_tmp, db, config) = fixture();
    let token = login_token(&config, &db);
    for path in [
        // A `..` segment where the entry id goes: the gate refuses it.
        "/v1/admin/creation/profiles/..",
        // Percent-encoded traversal: the `%` gate refuses it.
        "/v1/admin/creation/features/%2e%2e",
        // A traversal that adds a segment matches no route at all.
        "/v1/admin/creation/profiles/../registry.db",
    ] {
        let response = get(&config, &db, &token, path, None);
        assert!(
            response.status == 400 || response.status == 404,
            "{path} never reaches Core"
        );
        if response.status == 400 {
            assert_eq!(error_code(&response), "admin-invalid-creation-id", "{path}");
        }
        assert!(
            !String::from_utf8_lossy(&response.body).contains("registry.db")
                && !String::from_utf8_lossy(&response.body).contains("%2e"),
            "hostile input is never echoed"
        );
    }
}

#[test]
fn creation_resolves_are_pure_and_reviewable() {
    let (_tmp, db, config) = fixture();
    let token = login_token(&config, &db);
    let before = fs::read(&db).unwrap();
    let cases = [
        (
            "/v1/admin/creation/profiles/resolve",
            "id=rust-web&feature=auth",
            "rust-web",
        ),
        (
            "/v1/admin/creation/features/resolve",
            "profile=rust-web&feature=auth",
            "rust-web",
        ),
        (
            "/v1/admin/creation/components/resolve",
            "profile=rust-web&component=audit-action",
            "rust-web",
        ),
        (
            "/v1/admin/creation/ui-patterns/resolve",
            "profile=react-web&pattern=login",
            "react-web",
        ),
    ];
    for (path, query, profile) in cases {
        let response = get(&config, &db, &token, path, Some(query));
        assert_eq!(response.status, 200, "{path}?{query}");
        let body = body_json(&response);
        let creation = &body["creation"];
        assert!(
            creation["profile"] == profile || creation["id"] == profile,
            "{path} echoes its subject"
        );
        assert!(creation.get("resolution").is_some(), "{path} resolves");
    }
    // A missing subject is a static refusal, never a guess.
    let response = get(
        &config,
        &db,
        &token,
        "/v1/admin/creation/features/resolve",
        Some("feature=auth"),
    );
    assert_eq!(response.status, 400);
    assert_eq!(error_code(&response), "admin-invalid-creation-query");
    // An unknown subject is the typed Core refusal.
    let response = get(
        &config,
        &db,
        &token,
        "/v1/admin/creation/features/resolve",
        Some("profile=rust-web&feature=no-such"),
    );
    assert_eq!(response.status, 400);
    assert_eq!(error_code(&response), "unknown-feature");
    assert_eq!(fs::read(&db).unwrap(), before, "resolves write nothing");
}

#[test]
fn intent_validate_is_journal_free() {
    let (_tmp, db, config) = fixture();
    let token = login_token(&config, &db);
    let before = fs::read(&db).unwrap();
    let response = get(
        &config,
        &db,
        &token,
        "/v1/admin/creation/intents/validate",
        Some("action=create_project&profile=rust-web&require=auth"),
    );
    assert_eq!(response.status, 200);
    let body = body_json(&response);
    assert!(body["creation"]["validated"].is_object());
    assert!(!body["creation"]["intent_hash"]
        .as_str()
        .unwrap_or("")
        .is_empty());
    assert_eq!(body["creation"]["contract"], "0.1.0");
    // Malformed constraints and unknown actions are typed refusals.
    let response = get(
        &config,
        &db,
        &token,
        "/v1/admin/creation/intents/validate",
        Some("action=create_project&profile=rust-web&constraint=naked"),
    );
    assert_eq!(response.status, 400);
    assert_eq!(error_code(&response), "intent-invalid");
    let response = get(
        &config,
        &db,
        &token,
        "/v1/admin/creation/intents/validate",
        Some("action=bogus&profile=rust-web"),
    );
    assert_eq!(response.status, 400);
    assert_eq!(error_code(&response), "intent-invalid");
    assert_eq!(fs::read(&db).unwrap(), before, "validate writes nothing");
}

#[test]
fn project_bound_reads_resolve_server_side() {
    let (tmp, db, config) = fixture();
    let token = login_token(&config, &db);
    let tmp_str = tmp.path().display().to_string();
    // Standard check on a project without a snapshot: honest Absent,
    // scrubbed of the absolute directory.
    let response = get(
        &config,
        &db,
        &token,
        "/v1/admin/projects/alpha/standard/check",
        None,
    );
    assert_eq!(response.status, 200);
    let body = body_json(&response);
    assert_eq!(body["creation"]["project_id"], "alpha");
    assert_eq!(body["creation"]["report"]["state"], "absent");
    assert!(
        !String::from_utf8_lossy(&response.body).contains(&tmp_str),
        "check report carries no absolute path"
    );
    // Diff without `against` is a static refusal.
    let response = get(
        &config,
        &db,
        &token,
        "/v1/admin/projects/alpha/standard/diff",
        None,
    );
    assert_eq!(response.status, 400);
    assert_eq!(error_code(&response), "admin-invalid-creation-query");
    // Diff without a snapshot is the typed Core refusal, scrubbed.
    let response = get(
        &config,
        &db,
        &token,
        "/v1/admin/projects/alpha/standard/diff",
        Some("against=baseline-service@1.1.0"),
    );
    assert_eq!(response.status, 400);
    assert_eq!(error_code(&response), "standard-invalid");
    assert!(
        !String::from_utf8_lossy(&response.body).contains(&tmp_str),
        "diff refusal carries no absolute path"
    );
    // Plan receipts: empty when nothing was persisted, path-free.
    let response = get(
        &config,
        &db,
        &token,
        "/v1/admin/projects/alpha/intent/plans",
        None,
    );
    assert_eq!(response.status, 200);
    let body = body_json(&response);
    assert_eq!(body["creation"]["project_id"], "alpha");
    assert_eq!(body["creation"]["plans"].as_array().unwrap().len(), 0);
    // Unknown projects are unmanaged; hostile ids never reach Core.
    let response = get(
        &config,
        &db,
        &token,
        "/v1/admin/projects/no-such/standard/check",
        None,
    );
    assert_eq!(response.status, 404);
    assert_eq!(error_code(&response), "admin-project-unmanaged");
}

#[test]
fn creation_catalog_rows_are_web_with_typed_routes() {
    let (_tmp, db, config) = fixture();
    let token = login_token(&config, &db);
    let mut fetch = request("GET", "/v1/admin/commands", &config.frontend_origin);
    fetch
        .cookies
        .insert("forge_admin_session".to_string(), token);
    let response = handle(&config, &db, &fetch, Utc::now());
    assert_eq!(response.status, 200);
    let body: Value = serde_json::from_slice(&response.body).unwrap();
    let rows = body["commands"].as_array().unwrap();
    let by_id: BTreeMap<&str, &Value> = rows
        .iter()
        .map(|row| (row["id"].as_str().unwrap(), row))
        .collect();
    for (id, route) in [
        ("profile.list", "GET /v1/admin/creation/profiles"),
        ("profile.inspect", "GET /v1/admin/creation/profiles/{id}"),
        ("profile.resolve", "GET /v1/admin/creation/profiles/resolve"),
        ("feature.list", "GET /v1/admin/creation/features"),
        ("feature.inspect", "GET /v1/admin/creation/features/{id}"),
        ("feature.resolve", "GET /v1/admin/creation/features/resolve"),
        ("component.list", "GET /v1/admin/creation/components"),
        (
            "component.inspect",
            "GET /v1/admin/creation/components/{id}",
        ),
        (
            "component.resolve",
            "GET /v1/admin/creation/components/resolve",
        ),
        ("ui-pattern.list", "GET /v1/admin/creation/ui-patterns"),
        (
            "ui-pattern.inspect",
            "GET /v1/admin/creation/ui-patterns/{id}",
        ),
        (
            "ui-pattern.resolve",
            "GET /v1/admin/creation/ui-patterns/resolve",
        ),
        ("standard.list", "GET /v1/admin/creation/standards"),
        ("standard.inspect", "GET /v1/admin/creation/standards/{id}"),
        (
            "standard.check",
            "GET /v1/admin/projects/{id}/standard/check",
        ),
        ("standard.diff", "GET /v1/admin/projects/{id}/standard/diff"),
        ("procedure.list", "GET /v1/admin/creation/procedures"),
        (
            "procedure.inspect",
            "GET /v1/admin/creation/procedures/{id}",
        ),
        ("intent.validate", "GET /v1/admin/creation/intents/validate"),
        ("intent.list", "GET /v1/admin/projects/{id}/intent/plans"),
    ] {
        let row = by_id.get(id).unwrap_or_else(|| panic!("row {id} ships"));
        assert_eq!(row["availability"], "web", "{id}");
        assert_eq!(row["route"], route, "{id}");
        assert_eq!(row["risk"], "read", "{id}");
    }
    // The writes and the native/file-bound verbs stay out of the web.
    for id in [
        "profile.preflight",
        "procedure.validate",
        "component.qualify",
        "ui-pattern.install",
        "standard.upgrade",
    ] {
        let row = by_id.get(id).unwrap_or_else(|| panic!("row {id} ships"));
        assert_ne!(row["availability"], "web", "{id} stays out of the web");
        assert!(
            !row["reason"].as_str().unwrap_or("").is_empty(),
            "{id} keeps its reason"
        );
    }
}
