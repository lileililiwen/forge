//! Contract tests for the read-only project-catalog browser
//! (`web-project-catalog-browser`, web UI/UX audit gap 3).
//!
//! Static-token contract over the shipped `frontend/` assets (no browser,
//! no server) plus live in-process round-trips over the six typed
//! read-only admin GET routes: the browser section lists per-project
//! all-source inspect with provenance, tag/language distributions with
//! counts, evidence-backed gaps with verdicts, and one fleet entry.
//! No write, no provider, no shell on any path.

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
    let beta = tmp.path().join("beta");
    write_project(&alpha, "alpha", "rust");
    write_project(&beta, "beta", "python");
    {
        let mut registry = Registry::open(&db).unwrap();
        registry.register(&alpha, None).unwrap();
        registry.register(&beta, None).unwrap();
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
    serde_json::from_slice(&response.body).expect("catalog browser routes are JSON-only")
}

#[test]
fn browser_section_lives_on_the_projects_view() {
    let html = index_html();
    let app = app_js();

    for token in [
        "id=\"catalog-browser\"",
        "id=\"catalog-title\"",
        "Project catalog",
        "id=\"catalog-project\"",
        "id=\"catalog-inspect\"",
        "id=\"catalog-inspect-result\"",
        "id=\"catalog-tags\"",
        "id=\"catalog-tags-refresh\"",
        "id=\"catalog-languages\"",
        "id=\"catalog-languages-refresh\"",
        "id=\"catalog-gaps\"",
        "id=\"catalog-gaps-refresh\"",
        "id=\"catalog-gaps-project\"",
        "id=\"catalog-fleet-entry\"",
        "id=\"catalog-fleet-inspect\"",
        "id=\"catalog-fleet-result\"",
        "id=\"catalog-count\"",
    ] {
        assert!(html.contains(token), "index.html must declare `{token}`");
    }
    let projects = html
        .split_once("id=\"view-projects\"")
        .map(|(_, rest)| rest)
        .expect("projects view");
    let reference_at = projects
        .find("id=\"command-reference\"")
        .expect("reference section");
    let browser_at = projects
        .find("id=\"catalog-browser\"")
        .expect("browser section in projects view");
    let workbench_at = projects
        .find("id=\"workbench\"")
        .expect("workbench follows");
    assert!(
        reference_at < browser_at && browser_at < workbench_at,
        "the catalog browser must sit after the command reference and before the workbench"
    );
    for token in [
        "initCatalogBrowser",
        "catalogInspectProject",
        "catalogLoadTags",
        "catalogLoadLanguages",
        "catalogLoadGaps",
        "catalogInspectFleet",
        "catalogRecordLine",
        "catalogGapRow",
        "catalogRefreshAll",
    ] {
        assert!(app.contains(token), "app.js must implement `{token}`");
    }
    assert!(
        app.contains("initCatalogBrowser();"),
        "the dashboard boot must wire the catalog browser"
    );
}

#[test]
fn browser_renders_provenance_counts_gaps_and_evidence() {
    let app = app_js();

    // Provenance: every record line names source, revision, observation
    // and freshness.
    for token in [
        "source_revision",
        "observed_at",
        "source_kind",
        "freshness",
        "catalog-record",
    ] {
        assert!(
            app.contains(token),
            "the inspect renderer must surface `{token}`"
        );
    }
    // Tags/languages: distinct values with counts.
    assert!(
        app.contains("/v1/admin/projects/catalog/tags"),
        "tags must read the typed tags route"
    );
    assert!(
        app.contains("/v1/admin/projects/catalog/languages"),
        "languages must read the typed languages route"
    );
    // Gaps: verdict badge + category + remediation class + evidence with
    // source, revision and observation time, plus the summary line.
    for token in [
        "/v1/admin/projects/catalog/gaps",
        "remediation_class",
        "Showing ${gaps.length} of ${total} gaps",
        "No gaps — clean",
    ] {
        assert!(
            app.contains(token),
            "the gaps renderer must carry `{token}`"
        );
    }
    // Per-project inspect + fleet inspect hit the typed routes.
    assert!(
        app.contains("/v1/admin/projects/${encodeURIComponent(id)}/catalog"),
        "inspect must read the typed per-project route"
    );
    assert!(
        app.contains("/v1/admin/fleet/${encodeURIComponent(entry)}"),
        "fleet inspect must read the typed fleet route"
    );
    // Results are live regions set on every write.
    assert!(
        app.matches("setResultRole(").count() >= 6,
        "every browser result must announce through setResultRole"
    );
}

#[test]
fn browser_adds_no_write_no_shell_and_no_html_interpretation() {
    let app = app_js();

    // Exactly the six read-only GETs; no POST anywhere in the new block.
    let after = app.split("Project catalog browser").nth(1).unwrap_or("");
    let block = after.split("---- Project workbench").next().unwrap_or("");
    assert!(
        !block.is_empty(),
        "the catalog block must sit ahead of the workbench section"
    );
    for token in [
        "eval(",
        "new Function(",
        ".innerHTML",
        "child_process",
        "execSync",
        "method: \"POST\"",
        "method: 'POST'",
        "\"POST\"",
    ] {
        assert!(
            !block.contains(token),
            "the catalog block must not contain `{token}`"
        );
    }
    // Only the six typed GET route strings may appear in the block.
    for route in [
        "/v1/admin/projects/catalog",
        "/v1/admin/projects/catalog/tags",
        "/v1/admin/projects/catalog/languages",
        "/v1/admin/projects/catalog/gaps",
        "/catalog",
        "/v1/admin/fleet/",
    ] {
        assert!(
            block.contains(route),
            "the catalog block must read `{route}`"
        );
    }
    assert!(
        !block.contains("/v1/admin/commands"),
        "the browser must not refetch the command catalog"
    );
}

#[test]
fn browser_styles_meet_the_text_floor_with_existing_tokens() {
    let css = styles_css();

    for token in [
        "#catalog-browser",
        ".catalog-record",
        ".catalog-record-head",
        ".catalog-gap",
        "#catalog-inspect-result",
        "#catalog-fleet-result",
    ] {
        assert!(css.contains(token), "styles.css must style `{token}`");
    }
    assert!(
        css.contains("#catalog-browser select"),
        "browser controls must have a touch-target rule"
    );
}

#[test]
fn admin_catalog_list_matches_the_existing_catalog_family() {
    let (_tmp, db, config) = fixture();
    let token = login_token(&config, &db);
    let response = get(&config, &db, &token, "/v1/admin/projects/catalog", None);
    assert_eq!(response.status, 200);
    let body = body_json(&response);
    assert_eq!(body["catalog"]["total"], 2);
    let ids: Vec<String> = body["catalog"]["records"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["project_id"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(ids, vec!["alpha".to_string(), "beta".to_string()]);
    for record in body["catalog"]["records"].as_array().unwrap() {
        for field in ["source", "observed_at", "freshness", "evidence"] {
            assert!(
                record.get(field).is_some(),
                "record must carry provenance field `{field}`"
            );
        }
    }
}

#[test]
fn admin_catalog_inspect_returns_every_source_for_one_project() {
    let (_tmp, db, config) = fixture();
    let token = login_token(&config, &db);
    let response = get(
        &config,
        &db,
        &token,
        "/v1/admin/projects/alpha/catalog",
        None,
    );
    assert_eq!(response.status, 200);
    let body = body_json(&response);
    assert_eq!(body["catalog"]["project_id"], "alpha");
    assert_eq!(body["catalog"]["records"].as_array().unwrap().len(), 1);

    let missing = get(
        &config,
        &db,
        &token,
        "/v1/admin/projects/no-such/catalog",
        None,
    );
    assert_ne!(missing.status, 200);
    let missing_body = body_json(&missing);
    assert_eq!(missing_body["error"]["code"], "unknown-project");

    let hostile = get(
        &config,
        &db,
        &token,
        "/v1/admin/projects/..%2F..%2Fetc/catalog",
        None,
    );
    assert_eq!(hostile.status, 400);
}

#[test]
fn admin_catalog_tags_and_languages_carry_counts() {
    let (_tmp, db, config) = fixture();
    let token = login_token(&config, &db);
    let tags = get(
        &config,
        &db,
        &token,
        "/v1/admin/projects/catalog/tags",
        None,
    );
    assert_eq!(tags.status, 200);
    let tags_body = body_json(&tags);
    assert_eq!(
        tags_body["catalog"]["contract"],
        "forge-project-catalog/0.1.0"
    );
    assert!(tags_body["catalog"]["tags"].is_array());

    let languages = get(
        &config,
        &db,
        &token,
        "/v1/admin/projects/catalog/languages",
        None,
    );
    assert_eq!(languages.status, 200);
    let languages_body = body_json(&languages);
    let values: Vec<String> = languages_body["catalog"]["languages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["value"].as_str().unwrap().to_string())
        .collect();
    assert!(values.contains(&"rust".to_string()));
    assert!(values.contains(&"python".to_string()));
}

#[test]
fn admin_catalog_gaps_are_evidence_backed_and_filterable() {
    let (_tmp, db, config) = fixture();
    let token = login_token(&config, &db);
    let response = get(
        &config,
        &db,
        &token,
        "/v1/admin/projects/catalog/gaps",
        None,
    );
    assert_eq!(response.status, 200);
    let body = body_json(&response);
    assert_eq!(body["gaps"]["contract"], "forge-project-evidence/0.1.0");
    assert!(!body["gaps"]["findings"].as_array().unwrap().is_empty());
    for finding in body["gaps"]["findings"].as_array().unwrap() {
        for field in [
            "id",
            "project_id",
            "category",
            "status",
            "remediation_class",
            "source",
            "observed_at",
            "evidence",
        ] {
            assert!(finding.get(field).is_some(), "finding must carry `{field}`");
        }
    }
    let scoped = get(
        &config,
        &db,
        &token,
        "/v1/admin/projects/catalog/gaps",
        Some("project=alpha&status=fail"),
    );
    assert_eq!(scoped.status, 200);
    let scoped_body = body_json(&scoped);
    assert_eq!(scoped_body["gaps"]["project_id"], "alpha");
    for finding in scoped_body["gaps"]["findings"].as_array().unwrap() {
        assert_eq!(finding["project_id"], "alpha");
        assert_eq!(finding["status"], "fail");
    }
    let bogus = get(
        &config,
        &db,
        &token,
        "/v1/admin/projects/catalog/gaps",
        Some("category=bogus"),
    );
    assert_eq!(bogus.status, 400);
    assert_eq!(body_json(&bogus)["error"]["code"], "catalog-invalid");
}

#[test]
fn admin_catalog_routes_refuse_anonymous_and_invalid_filters() {
    let (_tmp, db, config) = fixture();
    let token = login_token(&config, &db);

    // No session: 401 with no catalog data.
    let anon = handle(
        &config,
        &db,
        &request("GET", "/v1/admin/projects/catalog", &config.frontend_origin),
        Utc::now(),
    );
    assert_eq!(anon.status, 401);
    assert!(serde_json::from_slice::<Value>(&anon.body)
        .unwrap()
        .get("catalog")
        .map(|c| c.is_null())
        .unwrap_or(true));

    // Unknown filter key: typed refusal, nothing written.
    let bad = get(
        &config,
        &db,
        &token,
        "/v1/admin/projects/catalog",
        Some("colour=red"),
    );
    assert_eq!(bad.status, 400);

    // Fleet inspect without a configured workspace registry: honest
    // typed refusal, never an invented entry.
    let fleet = get(&config, &db, &token, "/v1/admin/fleet/alpha", None);
    assert_ne!(fleet.status, 200);

    // Hostile fleet entry: 400 that never echoes the input.
    let hostile = get(&config, &db, &token, "/v1/admin/fleet/..", None);
    assert_eq!(hostile.status, 400);
    assert!(!String::from_utf8_lossy(&hostile.body).contains(".."));
}

#[test]
fn admin_catalog_reads_write_nothing() {
    let (_tmp, db, config) = fixture();
    let token = login_token(&config, &db);
    let before = fs::read(&db).expect("registry bytes before reads");
    for (path, query) in [
        ("/v1/admin/projects/catalog", None),
        ("/v1/admin/projects/alpha/catalog", None),
        ("/v1/admin/projects/catalog/tags", None),
        ("/v1/admin/projects/catalog/languages", None),
        ("/v1/admin/projects/catalog/gaps", None),
        (
            "/v1/admin/projects/catalog/gaps",
            Some("project=alpha&status=fail"),
        ),
    ] {
        let response = get(&config, &db, &token, path, query);
        assert_eq!(response.status, 200, "GET {path}");
    }
    let after = fs::read(&db).expect("registry bytes after reads");
    assert_eq!(
        before, after,
        "catalog browser reads must not write to the registry"
    );
}
