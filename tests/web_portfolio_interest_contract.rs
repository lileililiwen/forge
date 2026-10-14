//! Web portal interest-reads contract (`forge-web-portfolio-interest`).
//!
//! Pins the read-only interest surface the portal panel exposes for the
//! selected project, hitting the real admin route through the in-process
//! `forge::api::handle`:
//!
//! - the three reads (`list`, `show`, `trend`) surface for a managed
//!   project and carry the interest contract version;
//! - a stale window reads `stale` exactly as the CLI reports it, and is
//!   never upgraded to `current` (the recorded-never-synthesized
//!   invariant);
//! - an unmanaged id is refused honestly, leaving the registry untouched;
//! - the reads are read-only: no project is created or modified.
//!
//! The static frontend contract (cards present, handlers wired, no banned
//! sinks, no write/import button) is checked separately. The 401/403/415
//! boundary is already covered by
//! `forge_web_portfolio_controls_contract.rs` and is intentionally
//! skipped here.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use forge::api::{handle, ApiConfig, ApiRequest, ApiResponse};
use forge::portfolio::interest::{Coverage, InterestWrite, MetricValue, PrivacyMode};
use forge::registry::Registry;
use serde_json::Value;
use tempfile::tempdir;

const CONTRACT: &str = "forge-portfolio-interest/0.1.0";

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

fn with_query(mut req: ApiRequest, query: &str) -> ApiRequest {
    req.query = Some(query.to_string());
    req
}

fn login_token(config: &ApiConfig, db: &Path) -> String {
    forge::identity::global::setup(db, "operator@example.test", "a-long-test-password").unwrap();
    let mut login = request("POST", "/v1/admin/session", &config.frontend_origin);
    login = with_json(
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

fn with_json(mut req: ApiRequest, body: &[u8]) -> ApiRequest {
    req.headers
        .insert("content-type".to_string(), "application/json".to_string());
    req.body = body.to_vec();
    req
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

fn register_project(db: &Path, dir: &Path) {
    let mut registry = Registry::open(db).unwrap();
    registry.register(dir, None).unwrap();
}

fn get(config: &ApiConfig, db: &Path, token: &str, path: &str) -> ApiResponse {
    handle(
        config,
        db,
        &with_cookie(request("GET", path, &config.frontend_origin), token),
        Utc::now(),
    )
}

fn get_query(config: &ApiConfig, db: &Path, token: &str, path: &str, query: &str) -> ApiResponse {
    handle(
        config,
        db,
        &with_query(
            with_cookie(request("GET", path, &config.frontend_origin), token),
            query,
        ),
        Utc::now(),
    )
}

fn body_json(response: &ApiResponse) -> Value {
    serde_json::from_slice(&response.body).expect("interest routes are JSON-only")
}

fn seed_snapshot(db: &Path, write: &InterestWrite) {
    let registry = Registry::open(db).unwrap();
    registry
        .interest_insert_snapshot(write, "ops-admin", "2020-01-01T00:00:00Z")
        .expect("seed snapshot");
}

fn snapshot(
    project: &str,
    source: &str,
    revision: &str,
    start: &str,
    end: &str,
    value: u64,
) -> InterestWrite {
    InterestWrite {
        project_id: project.to_string(),
        source: source.to_string(),
        source_revision: revision.to_string(),
        window_start: start.to_string(),
        window_end: end.to_string(),
        privacy_mode: PrivacyMode::ExactCount,
        coverage: Coverage::Complete,
        replaces_source_revision: None,
        metrics: vec![MetricValue {
            metric: "unique_visitors".to_string(),
            value,
        }],
    }
}

#[test]
fn the_three_interest_reads_surface_and_carry_the_contract() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let proj = dir.path().join("proj");
    write_project(&proj, "ip-proj");
    register_project(&db, &proj);

    // One current window so the reads have something to report.
    seed_snapshot(
        &db,
        &snapshot(
            "ip-proj",
            "github-analytics",
            "wk-36",
            "2026-09-01T00:00:00Z",
            "2026-09-08T00:00:00Z",
            120,
        ),
    );

    let list = body_json(&get(
        &config,
        &db,
        &token,
        "/v1/admin/portfolio/ip-proj/interest/list",
    ));
    assert_eq!(list["contract"], CONTRACT);
    assert_eq!(list["project_id"], "ip-proj");
    assert_eq!(list["measured"], true);
    assert_eq!(list["snapshots"].as_array().unwrap().len(), 1);

    let show = body_json(&get(
        &config,
        &db,
        &token,
        "/v1/admin/portfolio/ip-proj/interest/show",
    ));
    assert_eq!(show["contract"], CONTRACT);
    assert_eq!(show["project_id"], "ip-proj");
    assert_eq!(show["projection"]["snapshots"].as_array().unwrap().len(), 1);

    let trend = body_json(&get_query(
        &config,
        &db,
        &token,
        "/v1/admin/portfolio/ip-proj/interest/trend",
        "metric=unique_visitors&limit=12",
    ));
    assert_eq!(trend["contract"], CONTRACT);
    assert_eq!(trend["project_id"], "ip-proj");
    let points = &trend["interest"]["trend"]["points"];
    assert_eq!(points.as_array().unwrap().len(), 1);
    assert_eq!(trend["interest"]["trend"]["metric"], "unique_visitors");
}

#[test]
fn a_stale_window_is_reported_stale_never_current() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let proj = dir.path().join("proj");
    write_project(&proj, "ip-proj");
    register_project(&db, &proj);

    // A window that ended long before the default 14-day stale bound.
    seed_snapshot(
        &db,
        &snapshot(
            "ip-proj",
            "github-analytics",
            "wk-01",
            "2020-01-01T00:00:00Z",
            "2020-01-08T00:00:00Z",
            120,
        ),
    );

    let list = body_json(&get(
        &config,
        &db,
        &token,
        "/v1/admin/portfolio/ip-proj/interest/list",
    ));
    assert_eq!(list["stale_label"], "stale");
    assert_eq!(list["stale_snapshots"], 1);
    let snap = &list["snapshots"].as_array().unwrap()[0];
    assert_eq!(snap["freshness"], "stale");
    // Mutation guard: a stale window must never be advertised as current.
    assert_ne!(snap["freshness"], "current");

    let show = body_json(&get(
        &config,
        &db,
        &token,
        "/v1/admin/portfolio/ip-proj/interest/show",
    ));
    assert_eq!(show["stale_label"], "stale");

    let trend = body_json(&get_query(
        &config,
        &db,
        &token,
        "/v1/admin/portfolio/ip-proj/interest/trend",
        "metric=unique_visitors&limit=12",
    ));
    let tpoint = &trend["interest"]["trend"]["points"].as_array().unwrap()[0];
    assert_eq!(tpoint["freshness"], "stale");
    assert_ne!(tpoint["freshness"], "current");
}

#[test]
fn an_unmanaged_id_is_refused_and_changes_no_state() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);

    for path in [
        "/v1/admin/portfolio/ghost-project/interest/list",
        "/v1/admin/portfolio/ghost-project/interest/show",
        "/v1/admin/portfolio/ghost-project/interest/trend?metric=unique_visitors",
    ] {
        let response = get(&config, &db, &token, path);
        assert_eq!(response.status, 404, "GET {path}");
        let body = body_json(&response);
        assert_eq!(body["error"]["code"], "portfolio-unmanaged-project");
        assert_eq!(body["effect"], "none");
        let reason = body["error"]["message"].as_str().unwrap();
        assert!(!reason.contains("ghost-project"), "the id is not echoed");
    }
    // No project was created by the refused reads.
    let list = body_json(&get(&config, &db, &token, "/v1/admin/portfolio"));
    assert_eq!(list["projects"].as_array().unwrap().len(), 0);
}

#[test]
fn interest_reads_are_read_only_and_touch_no_rows() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let proj = dir.path().join("proj");
    write_project(&proj, "ip-proj");
    register_project(&db, &proj);
    seed_snapshot(
        &db,
        &snapshot(
            "ip-proj",
            "github-analytics",
            "wk-36",
            "2026-09-01T00:00:00Z",
            "2026-09-08T00:00:00Z",
            120,
        ),
    );

    let before = Registry::open(&db)
        .unwrap()
        .interest_snapshots("ip-proj", 100)
        .unwrap()
        .len();

    for path in [
        "/v1/admin/portfolio/ip-proj/interest/list",
        "/v1/admin/portfolio/ip-proj/interest/show",
        "/v1/admin/portfolio/ip-proj/interest/trend?metric=unique_visitors&limit=12",
    ] {
        let response = get(&config, &db, &token, path);
        assert_eq!(response.status, 200, "GET {path}");
        assert_ne!(
            body_json(&response)["effect"],
            serde_json::json!("forge-owned-write"),
            "a read carries no write effect"
        );
    }

    let after = Registry::open(&db)
        .unwrap()
        .interest_snapshots("ip-proj", 100)
        .unwrap()
        .len();
    assert_eq!(before, after, "the reads mutated no snapshot rows");
}

#[test]
fn frontend_interest_card_is_wired_without_banned_sinks_or_write_button() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let app = fs::read_to_string(root.join("frontend/app.js")).unwrap();
    let index = fs::read_to_string(root.join("frontend/index.html")).unwrap();

    // The interest card exists with the three read buttons and result
    // regions, but no import/write button.
    for marker in [
        "id=\"portfolio-interest\"",
        "id=\"portfolio-interest-list\"",
        "id=\"portfolio-interest-show\"",
        "id=\"portfolio-interest-trend\"",
        "id=\"portfolio-interest-list-result\"",
        "id=\"portfolio-interest-show-result\"",
        "id=\"portfolio-interest-trend-result\"",
        "id=\"portfolio-interest-metric\"",
        "id=\"portfolio-interest-limit\"",
    ] {
        assert!(index.contains(marker), "interest markup missing: {marker}");
    }
    assert!(
        !index.contains("portfolio-interest-import"),
        "the interest card must not expose an import/write button"
    );

    // The handlers are wired to the buttons and call the admin reads.
    for marker in [
        "portfolioInterestList",
        "portfolioInterestShow",
        "portfolioInterestTrend",
        "interest/list",
        "interest/show",
        "interest/trend",
        "portfolio-interest-metric",
    ] {
        assert!(
            app.contains(marker),
            "interest handler wiring missing: {marker}"
        );
    }
    // The metric select is populated from the documented labels.
    assert!(
        app.contains("unique_visitors")
            && app.contains("outbound_cta_clicks")
            && app.contains("paid_interest_events"),
        "the metric select must be populated from the documented labels"
    );

    // No banned sink anywhere in the portfolio interest wiring.
    for forbidden in [
        "eval(",
        "new Function",
        "innerHTML",
        "child_process",
        "sh -c",
    ] {
        assert!(
            !app.contains(forbidden),
            "unsafe sink in the portfolio frontend: {forbidden}"
        );
    }
    assert!(!index.contains("<script>"), "no inline script on the page");
}
