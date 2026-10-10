//! Contract tests for the read-only assurance browser
//! (`web-assurance-browser`, web UI/UX audit gap 7).
//!
//! Static-token contract over the shipped `frontend/` assets (no browser,
//! no server) plus live in-process round-trips over the seventeen typed
//! read-only admin GET routes: spec list/inspect/route, remediate
//! scan/diff, describe/classify list/show, contract list/inspect/emit,
//! governance list/status/inspect, analytics metrics, studio preview
//! read. No write, no provider probe, no adapter run, no shell, no
//! journal row, no summary write, no observation persist on any path.

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

fn project_dir(tmp: &tempfile::TempDir, db: &Path, id: &str) -> PathBuf {
    let registry = Registry::open(db).unwrap();
    let record = registry.inspect(id).unwrap();
    assert!(record.path.starts_with(tmp.path().to_string_lossy().as_ref()));
    PathBuf::from(&record.path)
}

fn get(config: &ApiConfig, db: &Path, token: &str, path: &str, query: Option<&str>) -> ApiResponse {
    let mut req = request("GET", path, &config.frontend_origin);
    if let Some(q) = query {
        req = with_query(req, q);
    }
    handle(config, db, &with_cookie(req, token), Utc::now())
}

fn body_json(response: &ApiResponse) -> Value {
    serde_json::from_slice(&response.body).expect("assurance routes are JSON-only")
}

fn error_code(response: &ApiResponse) -> String {
    body_json(response)
        .pointer("/error/code")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

fn journal_len(db: &Path) -> usize {
    Registry::open(db)
        .unwrap()
        .recent_operations(10_000)
        .unwrap()
        .len()
}

#[test]
fn browser_section_lives_on_the_projects_view() {
    let html = index_html();
    let app = app_js();
    let css = styles_css();
    for id in [
        "assurance-browser",
        "assurance-project",
        "assurance-registry",
        "assurance-search",
        "assurance-list",
        "assurance-count",
        "assurance-list-result",
        "assurance-inspect-id",
        "assurance-inspect",
        "assurance-inspect-result",
        "assurance-finding",
        "assurance-pack",
        "assurance-route",
        "assurance-scan",
        "assurance-diff",
        "assurance-routed-result",
        "assurance-family",
        "assurance-emit",
        "assurance-window",
        "assurance-metrics",
        "assurance-governance",
        "assurance-preview",
        "assurance-extra-result",
        "assurance-cli-only",
        "assurance-error-summary",
        "assurance-refresh",
    ] {
        assert!(html.contains(id), "projects view is missing #{id}");
    }
    for fragment in [
        "/v1/admin/contracts",
        "/specs",
        "/spec/route",
        "/remediate/scan",
        "/remediate/diff",
        "/describe/proposals",
        "/classify/proposals",
        "/contracts/emit",
        "/governance/status",
        "/analytics/metrics",
        "/studio/preview",
        "initAssuranceBrowser",
    ] {
        assert!(app.contains(fragment), "app.js never calls {fragment}");
    }
    assert!(css.contains("#assurance-browser"), "styles lack the assurance block");
    let start = app
        .find("Assurance browser")
        .expect("assurance block ships");
    let end = app[start..]
        .find("Project workbench")
        .map(|i| start + i)
        .unwrap_or(app.len());
    assert!(
        !app[start..end].contains("innerHTML"),
        "assurance block renders textContent-only"
    );
    let assurance_js: String = app
        .split("// ---- Assurance browser")
        .nth(1)
        .unwrap_or("")
        .split("// ---- Project workbench")
        .next()
        .unwrap_or("")
        .to_string();
    for banned in ["eval(", "Function(", "child_process", "spawn(", "POST"] {
        assert!(
            !assurance_js.contains(banned),
            "assurance section must not contain `{banned}`"
        );
    }
}

#[test]
fn anonymous_assurance_reads_are_rejected() {
    let (_tmp, db, config) = fixture();
    for (path, query) in [
        ("/v1/admin/contracts", None),
        ("/v1/admin/projects/alpha/specs", None),
        ("/v1/admin/projects/alpha/remediate/scan", None),
        ("/v1/admin/projects/alpha/governance/status", None),
        ("/v1/admin/projects/alpha/analytics/metrics", None),
        ("/v1/admin/projects/alpha/studio/preview", None),
    ] {
        let mut req = request("GET", path, &config.frontend_origin);
        if let Some(q) = query {
            req = with_query(req, q);
        }
        let response = handle(&config, &db, &req, Utc::now());
        assert_eq!(response.status, 401, "anon {path} must be 401");
    }
}

#[test]
fn contract_list_and_inspect_match_the_cli() {
    let (_tmp, db, config) = fixture();
    let token = login_token(&config, &db);
    let list = get(&config, &db, &token, "/v1/admin/contracts", None);
    assert_eq!(list.status, 200);
    let body = body_json(&list);
    assert!(body.pointer("/contracts").and_then(Value::as_array).is_some());
    assert!(body.pointer("/manifest").is_some());
    let cli_count = forge::contract::CONTRACTS.len();
    assert_eq!(
        body.pointer("/contracts").unwrap().as_array().unwrap().len(),
        cli_count
    );
    let inspect = get(
        &config,
        &db,
        &token,
        "/v1/admin/contracts/platform.gate-result",
        None,
    );
    assert_eq!(inspect.status, 200);
    assert!(body_json(&inspect).pointer("/schema").is_some());
    let unknown = get(&config, &db, &token, "/v1/admin/contracts/no-such", None);
    assert_eq!(unknown.status, 400);
    assert!(!String::from_utf8_lossy(&unknown.body).contains("no-such"));
}

#[test]
fn spec_list_inspect_and_route_are_read_only() {
    let (_tmp, db, config) = fixture();
    let token = login_token(&config, &db);
    let before = journal_len(&db);
    let list = get(&config, &db, &token, "/v1/admin/projects/alpha/specs", None);
    assert_eq!(list.status, 200);
    assert_eq!(
        body_json(&list).pointer("/specs").unwrap().as_array().unwrap().len(),
        0
    );
    let unknown = get(
        &config,
        &db,
        &token,
        "/v1/admin/projects/alpha/specs/alpha-deadbeef",
        None,
    );
    assert_eq!(unknown.status, 404);
    let route = get(
        &config,
        &db,
        &token,
        "/v1/admin/projects/alpha/spec/route",
        Some("finding=gaps.ci.alpha.ci"),
    );
    assert_eq!(route.status, 200);
    assert!(body_json(&route).pointer("/route").is_some());
    let missing = get(
        &config,
        &db,
        &token,
        "/v1/admin/projects/alpha/spec/route",
        None,
    );
    assert_eq!(missing.status, 400);
    assert_eq!(journal_len(&db), before, "spec reads must not journal");
}

#[test]
fn remediate_scan_and_diff_rebuild_server_side() {
    let (_tmp, db, config) = fixture();
    let token = login_token(&config, &db);
    let scan = get(
        &config,
        &db,
        &token,
        "/v1/admin/projects/alpha/remediate/scan",
        None,
    );
    assert_eq!(scan.status, 200);
    let findings = body_json(&scan)
        .pointer("/scan/findings")
        .unwrap()
        .as_array()
        .unwrap()
        .len();
    assert!(findings >= 1, "a fresh fixture misses the CI asset");
    let packless = get(
        &config,
        &db,
        &token,
        "/v1/admin/projects/alpha/remediate/diff",
        Some("finding=gaps.ci.alpha.ci"),
    );
    assert_eq!(packless.status, 400, "diff without a pack is a typed refusal");
    assert_eq!(error_code(&packless), "remediation-invalid");
    let packs = forge::standard::all_packs();
    assert!(!packs.is_empty());
    let selector = format!("{}@{}", packs[0].id, packs[0].version);
    let diff = get(
        &config,
        &db,
        &token,
        "/v1/admin/projects/alpha/remediate/diff",
        Some(&format!("finding=gaps.ci.alpha.ci&pack={selector}")),
    );
    assert_eq!(diff.status, 200);
    assert!(body_json(&diff).pointer("/diff").unwrap().is_array());
}

#[test]
fn describe_and_classify_lists_and_shows_agree() {
    let (_tmp, db, config) = fixture();
    let token = login_token(&config, &db);
    for list_path in [
        "/v1/admin/projects/alpha/describe/proposals",
        "/v1/admin/projects/alpha/classify/proposals",
    ] {
        let list = get(&config, &db, &token, list_path, None);
        assert_eq!(list.status, 200, "{list_path} must be 200");
        assert_eq!(
            body_json(&list).pointer("/proposals").unwrap().as_array().unwrap().len(),
            0
        );
    }
    let unknown = get(
        &config,
        &db,
        &token,
        "/v1/admin/projects/alpha/describe/proposals/description-deadbeef",
        None,
    );
    assert_eq!(unknown.status, 404);
    let malformed = get(
        &config,
        &db,
        &token,
        "/v1/admin/projects/alpha/classify/proposals/not-an-id",
        None,
    );
    assert!(matches!(malformed.status, 400 | 404));
}

#[test]
fn contract_emit_is_read_only_with_honest_gaps() {
    let (_tmp, db, config) = fixture();
    let token = login_token(&config, &db);
    let readiness = get(
        &config,
        &db,
        &token,
        "/v1/admin/projects/alpha/contracts/emit",
        Some("family=platform.readiness"),
    );
    assert_eq!(readiness.status, 200);
    let readiness_body = body_json(&readiness);
    assert!(
        readiness_body.pointer("/envelope").is_some()
            || readiness_body.pointer("/unavailable").unwrap_or(&Value::Bool(false))
                == &Value::Bool(true),
        "readiness emit must be an envelope or an honest gap"
    );
    let job = get(
        &config,
        &db,
        &token,
        "/v1/admin/projects/alpha/contracts/emit",
        Some("family=platform.job-outcome"),
    );
    assert_eq!(job.status, 200);
    assert_eq!(
        body_json(&job).pointer("/unavailable").unwrap(),
        &Value::Bool(true)
    );
    let unknown = get(
        &config,
        &db,
        &token,
        "/v1/admin/projects/alpha/contracts/emit",
        Some("family=no-such"),
    );
    assert_eq!(unknown.status, 400);
    let missing = get(
        &config,
        &db,
        &token,
        "/v1/admin/projects/alpha/contracts/emit",
        None,
    );
    assert_eq!(missing.status, 400);
}

#[test]
fn governance_reads_never_run_or_persist() {
    let (tmp, db, config) = fixture();
    let token = login_token(&config, &db);
    let before = journal_len(&db);
    let list = get(&config, &db, &token, "/v1/admin/projects/alpha/governance", None);
    assert_eq!(list.status, 200);
    assert!(
        body_json(&list).pointer("/providers").unwrap().as_array().unwrap().len() >= 1
    );
    let status = get(
        &config,
        &db,
        &token,
        "/v1/admin/projects/alpha/governance/status",
        None,
    );
    assert_eq!(status.status, 200);
    assert!(body_json(&status).pointer("/observation").is_some());
    let inspect = get(
        &config,
        &db,
        &token,
        "/v1/admin/projects/alpha/governance/inspect",
        None,
    );
    assert_eq!(inspect.status, 200);
    assert_eq!(journal_len(&db), before, "governance reads must not journal");
    let dir = project_dir(&tmp, &db, "alpha");
    fs::create_dir_all(dir.join(".forge")).unwrap();
    fs::write(
        dir.join(".forge/providers.yaml"),
        "provider:\n  provider: acme-governance\n  enabled: true\n  protocol_version: \"0.1.0\"\n  timeout_ms: 1000\n",
    )
    .unwrap();
    let blocked = get(
        &config,
        &db,
        &token,
        "/v1/admin/projects/alpha/governance/status",
        None,
    );
    assert_eq!(blocked.status, 200);
    let body = body_json(&blocked);
    assert_eq!(body.pointer("/unavailable").unwrap(), &Value::Bool(true));
    assert!(
        body.pointer("/reason")
            .and_then(Value::as_str)
            .unwrap()
            .contains("acme-governance")
    );
    assert_eq!(journal_len(&db), before, "external governance must not persist");
}

#[test]
fn analytics_metrics_probe_nothing_and_write_nothing() {
    let (tmp, db, config) = fixture();
    let token = login_token(&config, &db);
    let before = journal_len(&db);
    let metrics = get(
        &config,
        &db,
        &token,
        "/v1/admin/projects/alpha/analytics/metrics",
        None,
    );
    assert_eq!(metrics.status, 200);
    assert!(body_json(&metrics).pointer("/metrics/aggregates").is_some());
    let windowed = get(
        &config,
        &db,
        &token,
        "/v1/admin/projects/alpha/analytics/metrics",
        Some("window_days=30"),
    );
    assert_eq!(windowed.status, 200);
    for bad in ["window_days=9999", "window_days=abc"] {
        let refused = get(
            &config,
            &db,
            &token,
            "/v1/admin/projects/alpha/analytics/metrics",
            Some(bad),
        );
        assert_eq!(refused.status, 400, "{bad} must be a typed refusal");
    }
    assert_eq!(journal_len(&db), before, "metrics must not journal");
    let dir = project_dir(&tmp, &db, "alpha");
    assert!(
        !dir.join(".forge").exists(),
        "metrics must not write a summary file"
    );
}

#[test]
fn studio_preview_reads_without_spawning() {
    let (_tmp, db, config) = fixture();
    let token = login_token(&config, &db);
    let before = journal_len(&db);
    let preview = get(
        &config,
        &db,
        &token,
        "/v1/admin/projects/alpha/studio/preview",
        None,
    );
    assert_eq!(preview.status, 200);
    let body = body_json(&preview);
    assert_eq!(
        body.pointer("/preview/state").and_then(Value::as_str).unwrap(),
        "none"
    );
    assert_eq!(journal_len(&db), before, "preview reads must not journal");
}

#[test]
fn hostile_and_unknown_ids_stay_typed() {
    let (_tmp, db, config) = fixture();
    let token = login_token(&config, &db);
    let hostile = get(&config, &db, &token, "/v1/admin/projects/../specs", None);
    assert!(matches!(hostile.status, 400 | 404));
    if hostile.status == 400 {
        assert!(!String::from_utf8_lossy(&hostile.body).contains(".."));
    }
    let unmanaged = get(
        &config,
        &db,
        &token,
        "/v1/admin/projects/no-such-project/specs",
        None,
    );
    assert_eq!(unmanaged.status, 404);
    let hostile_spec = get(
        &config,
        &db,
        &token,
        "/v1/admin/projects/alpha/specs/..%2F..",
        None,
    );
    assert!(matches!(hostile_spec.status, 400 | 404));
}

#[test]
fn assurance_catalog_rows_are_web_with_typed_routes() {
    let (_tmp, db, config) = fixture();
    let token = login_token(&config, &db);
    let mut req = request("GET", "/v1/admin/commands", &config.frontend_origin);
    req = with_cookie(req, &token);
    let response = handle(&config, &db, &req, Utc::now());
    assert_eq!(response.status, 200);
    let body: Value = serde_json::from_slice(&response.body).unwrap();
    let commands = body.pointer("/commands").unwrap().as_array().unwrap();
    let web: BTreeMap<String, String> = commands
        .iter()
        .filter(|row| row.pointer("/availability").and_then(Value::as_str) == Some("web"))
        .map(|row| {
            (
                row.pointer("/id").and_then(Value::as_str).unwrap().to_string(),
                row.pointer("/route").and_then(Value::as_str).unwrap().to_string(),
            )
        })
        .collect();
    for (id, route) in [
        ("spec.list", "GET /v1/admin/projects/{id}/specs"),
        ("spec.inspect", "GET /v1/admin/projects/{id}/specs/{spec}"),
        ("spec.route", "GET /v1/admin/projects/{id}/spec/route"),
        ("remediate.scan", "GET /v1/admin/projects/{id}/remediate/scan"),
        ("remediate.diff", "GET /v1/admin/projects/{id}/remediate/diff"),
        ("describe.list", "GET /v1/admin/projects/{id}/describe/proposals"),
        (
            "describe.show",
            "GET /v1/admin/projects/{id}/describe/proposals/{proposal}",
        ),
        ("classify.list", "GET /v1/admin/projects/{id}/classify/proposals"),
        (
            "classify.show",
            "GET /v1/admin/projects/{id}/classify/proposals/{proposal}",
        ),
        ("contract.list", "GET /v1/admin/contracts"),
        ("contract.inspect", "GET /v1/admin/contracts/{family}"),
        ("contract.emit", "GET /v1/admin/projects/{id}/contracts/emit"),
        ("governance.list", "GET /v1/admin/projects/{id}/governance"),
        (
            "governance.status",
            "GET /v1/admin/projects/{id}/governance/status",
        ),
        (
            "governance.inspect",
            "GET /v1/admin/projects/{id}/governance/inspect",
        ),
        ("analytics.metrics", "GET /v1/admin/projects/{id}/analytics/metrics"),
        ("studio.preview", "GET /v1/admin/projects/{id}/studio/preview"),
    ] {
        assert_eq!(web.get(id).map(String::as_str), Some(route), "{id} must be web at {route}");
    }
    let neben: BTreeMap<String, String> = commands
        .iter()
        .map(|row| {
            (
                row.pointer("/id").and_then(Value::as_str).unwrap().to_string(),
                row.pointer("/availability").and_then(Value::as_str).unwrap().to_string(),
            )
        })
        .collect();
    for (id, availability) in [
        ("spec.generate", "web"),
        ("spec.apply", "web"),
        ("remediate.plan", "web"),
        ("remediate.apply", "web"),
        ("describe.suggest", "not_yet_web"),
        ("describe.approve", "not_yet_web"),
        ("describe.reject", "not_yet_web"),
        ("classify.suggest", "not_yet_web"),
        ("classify.derive", "not_yet_web"),
        ("classify.apply", "web"),
        ("classify.approve", "web"),
        ("classify.reject", "web"),
        ("contract.validate", "cli_only"),
        ("governance.use", "not_yet_web"),
        ("analytics.inspect", "provider_required"),
        ("studio.spec", "web"),
        ("studio.refine", "web"),
    ] {
        assert_eq!(
            neben.get(id).map(String::as_str),
            Some(availability),
            "{id} must stay {availability}"
        );
    }
}
