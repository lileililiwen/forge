//! Portfolio controls contract (`forge-web-portfolio-controls`).
//!
//! Pins the security boundary and honest behavior of the typed,
//! session-gated portfolio routes under `/v1/admin/portfolio`: anonymous
//! requests get `401` with no portfolio data, hostile origins get `403`
//! (including the deep-path CORS preflights), shell-metacharacter and
//! cross-path ids are refused as typed `400`s whose body never echoes the
//! input, unmanaged/observed-only projects get an honest `404`, Forge-owned
//! metadata mutations succeed and touch only owned fields, source-owned
//! evidence edits are refused leaving the snapshot unchanged, no provider or
//! readiness probe runs on page load (`not_run`), missing/low evidence is
//! never converted into a healthy result, and no response ever serializes an
//! absolute filesystem path.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use forge::api::{handle, ApiConfig, ApiRequest, ApiResponse};
use forge::portfolio::EvidenceStatus;
use forge::registry::{Registry, SnapshotWrite};
use serde_json::Value;
use tempfile::tempdir;

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

fn post(config: &ApiConfig, db: &Path, token: &str, path: &str, body: &[u8]) -> ApiResponse {
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

fn body_json(response: &ApiResponse) -> Value {
    serde_json::from_slice(&response.body).expect("portfolio routes are JSON-only")
}

fn leaks_path(response: &ApiResponse, path: &Path) -> bool {
    let text = String::from_utf8_lossy(&response.body).to_string();
    let candidates = [
        path.display().to_string(),
        path.canonicalize()
            .map(|p| p.display().to_string())
            .unwrap_or_default(),
    ];
    candidates
        .iter()
        .filter(|c| !c.is_empty())
        .any(|candidate| text.contains(candidate))
}

#[test]
fn anonymous_requests_are_refused_before_any_portfolio_data_leaks() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    forge::identity::global::setup(&db, "operator@example.test", "a-long-test-password").unwrap();
    let proj = dir.path().join("proj");
    write_project(&proj, "anon-proj");
    register_project(&db, &proj);

    let cases = [
        ("GET", "/v1/admin/portfolio"),
        ("GET", "/v1/admin/portfolio/evidence"),
        ("GET", "/v1/admin/portfolio/anon-proj"),
        ("GET", "/v1/admin/portfolio/anon-proj/evidence"),
    ];
    for (method, path) in cases {
        let response = handle(
            &config,
            &db,
            &request(method, path, &config.frontend_origin),
            Utc::now(),
        );
        assert_eq!(response.status, 401, "{method} {path}");
        let text = String::from_utf8_lossy(&response.body);
        assert!(
            !text.contains("projects") && !text.contains("sections"),
            "401 body leaked portfolio data: {text}"
        );
    }
    // A metadata write is refused before the Core boundary is reached.
    let write = handle(
        &config,
        &db,
        &json_body(
            request(
                "POST",
                "/v1/admin/portfolio/anon-proj/tags",
                &config.frontend_origin,
            ),
            br#"{"name":"alpha"}"#,
        ),
        Utc::now(),
    );
    assert_eq!(write.status, 401);
}

#[test]
fn hostile_origins_are_refused_including_the_deep_path_preflights() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);

    let hostile = "https://attacker.example";
    let list = handle(
        &config,
        &db,
        &with_cookie(request("GET", "/v1/admin/portfolio", hostile), &token),
        Utc::now(),
    );
    assert_eq!(list.status, 403);
    let write = handle(
        &config,
        &db,
        &with_cookie(
            json_body(
                request("POST", "/v1/admin/portfolio/anything/tags", hostile),
                br#"{"name":"x"}"#,
            ),
            &token,
        ),
        Utc::now(),
    );
    assert_eq!(write.status, 403);

    for path in [
        "/v1/admin/portfolio",
        "/v1/admin/portfolio/evidence",
        "/v1/admin/portfolio/anything",
        "/v1/admin/portfolio/anything/tags",
    ] {
        let preflight = handle(&config, &db, &request("OPTIONS", path, hostile), Utc::now());
        assert_eq!(preflight.status, 403, "OPTIONS {path}");
    }
    // The same preflight from the configured origin is allowed.
    let ok = handle(
        &config,
        &db,
        &request(
            "OPTIONS",
            "/v1/admin/portfolio/anything/tags",
            &config.frontend_origin,
        ),
        Utc::now(),
    );
    assert_eq!(ok.status, 204);
}

#[test]
fn shell_metacharacter_and_cross_path_ids_are_refused_without_echo_or_effect() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let proj = dir.path().join("proj");
    write_project(&proj, "safe-proj");
    register_project(&db, &proj);

    for id in [
        "safe-proj; rm -rf /",
        "$(whoami)",
        "evil|curl evil.test",
        "back`tick`",
        "UPPER-$(id)",
    ] {
        // Path segments cannot contain '/', so these reach the id validator.
        let response = get(&config, &db, &token, &format!("/v1/admin/portfolio/{id}"));
        assert_eq!(response.status, 400, "id {id:?} must be a typed refusal");
        let body = String::from_utf8_lossy(&response.body).to_string();
        assert!(body.contains("not a valid identifier"), "{body}");
        for probe in ["rm", "$(", "|", "`", "whoami"] {
            assert!(!body.contains(probe), "the refused input leaked: {body}");
        }
        assert!(
            !body.contains(&proj.display().to_string()),
            "absolute path echoed: {body}"
        );
    }

    for path in [
        "/v1/admin/portfolio/../../etc/passwd",
        "/v1/admin/portfolio/%2e%2e/secret/evidence",
    ] {
        let response = get(&config, &db, &token, path);
        assert!(
            response.status == 400 || response.status == 404,
            "traversal {path} must be refused, got {}",
            response.status
        );
    }
}

#[test]
fn unmanaged_and_observed_only_ids_get_the_honest_404_reason() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);

    let response = get(&config, &db, &token, "/v1/admin/portfolio/ghost-project");
    assert_eq!(response.status, 404);
    let body = body_json(&response);
    assert_eq!(body["error"]["code"], "portfolio-unmanaged-project");
    assert_eq!(body["effect"], "none");
    let reason = body["error"]["message"].as_str().unwrap();
    assert!(reason.contains("not managed by this Forge registry"));
    assert!(!reason.contains("ghost-project"), "the id is not echoed");
}

#[test]
fn empty_registry_reports_honest_empty_and_not_run_states() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);

    let list = body_json(&get(&config, &db, &token, "/v1/admin/portfolio"));
    assert_eq!(list["contract"], "forge-web-portfolio-controls/0.1.0");
    assert_eq!(list["projects"].as_array().unwrap().len(), 0);
    assert_eq!(list["summary"]["projects"], 0);

    let evidence = body_json(&get(&config, &db, &token, "/v1/admin/portfolio/evidence"));
    let sections = &evidence["sections"];
    assert_eq!(evidence["live_probes"], false);
    // Provider matrix is non-live: every row is `not-run`, never a pass.
    assert_eq!(sections["provider"]["live"], false);
    for row in sections["provider"]["rows"].as_array().unwrap() {
        assert_eq!(
            row["status"].as_str().unwrap(),
            "not-run",
            "no provider is probed on page load"
        );
    }
    // Readiness never runs the native matrix here.
    assert_eq!(sections["readiness"]["state"], "not_run");
    // No registered projects means the interest verdict is honest, not faked.
    assert_eq!(sections["interest"]["state"], "no_projects");
}

#[test]
fn forge_owned_metadata_mutations_are_recorded_and_read_back() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let proj = dir.path().join("proj");
    write_project(&proj, "pproj");
    register_project(&db, &proj);

    // Add a tag.
    let tag = post(
        &config,
        &db,
        &token,
        "/v1/admin/portfolio/pproj/tags",
        br#"{"name":"priority"}"#,
    );
    assert_eq!(tag.status, 200);
    let tag_body = body_json(&tag);
    assert_eq!(tag_body["effect"], "forge-owned-write");
    assert_eq!(tag_body["actor"], "global-admin");
    assert_eq!(tag_body["result"]["tag"]["name"], "priority");

    // Record a review (satisfies the "Record portfolio review" scenario).
    let review = post(
        &config,
        &db,
        &token,
        "/v1/admin/portfolio/pproj/reviews",
        br#"{"confidence":"high","lifecycle":"operational","next_action":"publish kit"}"#,
    );
    assert_eq!(review.status, 200);
    assert_eq!(body_json(&review)["effect"], "forge-owned-write");

    // Link a goal.
    let goal = post(
        &config,
        &db,
        &token,
        "/v1/admin/portfolio/pproj/goals",
        br#"{"title":"Reach L3","status":"planned","description":"upgrade tooling"}"#,
    );
    assert_eq!(goal.status, 200);
    assert_eq!(body_json(&goal)["result"]["goal"]["title"], "Reach L3");

    // The metadata now reads back in both the fleet list and detail.
    let list = body_json(&get(&config, &db, &token, "/v1/admin/portfolio"));
    let row = list["projects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["project_id"] == "pproj")
        .expect("pproj row");
    assert!(row["tags"]
        .as_array()
        .unwrap()
        .contains(&Value::String("priority".to_string())));
    assert_eq!(row["confidence"], "high");
    assert_eq!(row["lifecycle"], "operational");

    let detail_res = get(&config, &db, &token, "/v1/admin/portfolio/pproj");
    let detail = body_json(&detail_res);
    assert_eq!(detail["contract"], "forge-web-portfolio-controls/0.1.0");
    assert_eq!(detail["management"], "managed");
    let portfolio = &detail["portfolio"];
    assert_eq!(portfolio["profile"]["confidence"], "high");
    assert!(!portfolio["reviews"].as_array().unwrap().is_empty());
    assert!(!portfolio["goals"].as_array().unwrap().is_empty());

    // No registered absolute path ever appears.
    assert!(
        !leaks_path(&detail_res, &proj),
        "detail leaked the registered path"
    );
    assert!(!leaks_path(
        &get(&config, &db, &token, "/v1/admin/portfolio"),
        &proj
    ));
}

#[test]
fn source_owned_evidence_edits_are_refused_and_the_snapshot_is_preserved() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let proj = dir.path().join("proj");
    write_project(&proj, "pproj");
    register_project(&db, &proj);

    // Read the (currently empty) source-owned evidence.
    let before = body_json(&get(
        &config,
        &db,
        &token,
        "/v1/admin/portfolio/pproj/evidence",
    ));
    assert_eq!(before["evidence"].as_array().unwrap().len(), 0);

    // Attempting to edit/import evidence from the browser is refused.
    let attempt = post(
        &config,
        &db,
        &token,
        "/v1/admin/portfolio/pproj/evidence",
        br#"{"source":"gh","revision":"1","status":"observed"}"#,
    );
    assert_eq!(attempt.status, 403);
    let attempt_body = body_json(&attempt);
    assert_eq!(attempt_body["error"]["code"], "portfolio-source-owned");
    assert_eq!(attempt_body["effect"], "none");

    // The stored snapshot is provably unchanged (still zero rows).
    let after = body_json(&get(
        &config,
        &db,
        &token,
        "/v1/admin/portfolio/pproj/evidence",
    ));
    assert_eq!(after["evidence"].as_array().unwrap().len(), 0);

    // An unknown read kind is an honest 404, never a fabricated resource.
    let unknown = get(&config, &db, &token, "/v1/admin/portfolio/pproj/mystery");
    assert_eq!(unknown.status, 404);
    assert_eq!(
        body_json(&unknown)["error"]["code"],
        "portfolio-route-not-found"
    );
}

fn seed_snapshot(
    db: &Path,
    project_id: &str,
    source: &str,
    status: EvidenceStatus,
    stale_after: Option<&str>,
) {
    let registry = Registry::open(db).unwrap();
    registry
        .portfolio_import_snapshot(
            project_id,
            &SnapshotWrite {
                source_system: source.to_string(),
                source_revision: format!("{source}-rev"),
                observed_at: "2020-01-01T00:00:00Z".to_string(),
                status,
                stale_after: stale_after.map(|value| value.to_string()),
                evidence_json: "{}".to_string(),
            },
        )
        .unwrap();
}

#[test]
fn evidence_snapshots_render_their_honest_effective_state_and_retain_others() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let proj = dir.path().join("proj");
    write_project(&proj, "pproj");
    register_project(&db, &proj);

    // Observed but past its freshness bound renders `stale`; an unreachable
    // source renders `unavailable`; a source that answered garbage renders
    // `invalid`; a fresh observation stays `observed`. Each uses a distinct
    // source_system so the newest-per-source read model returns all four,
    // proving a stale/unavailable/malformed source never hides or downgrades
    // the other retained results and nothing is upgraded to healthy.
    seed_snapshot(
        &db,
        "pproj",
        "gh",
        EvidenceStatus::Observed,
        Some("2020-06-01T00:00:00Z"),
    );
    seed_snapshot(&db, "pproj", "gl", EvidenceStatus::Unavailable, None);
    seed_snapshot(&db, "pproj", "sr", EvidenceStatus::Invalid, None);
    seed_snapshot(
        &db,
        "pproj",
        "ci",
        EvidenceStatus::Observed,
        Some("2999-01-01T00:00:00Z"),
    );

    let response = get(&config, &db, &token, "/v1/admin/portfolio/pproj/evidence");
    let evidence = body_json(&response);
    let rows = evidence["evidence"].as_array().unwrap();
    let by_source: BTreeMap<&str, &Value> = rows
        .iter()
        .map(|row| (row["source_system"].as_str().unwrap(), row))
        .collect();
    assert_eq!(by_source.len(), 4, "every source result is retained");
    assert_eq!(by_source["gh"]["status"], "stale");
    assert!(
        by_source["gh"]["observed_at"].is_string(),
        "the observation time is shown"
    );
    assert_eq!(by_source["gl"]["status"], "unavailable");
    assert_eq!(by_source["sr"]["status"], "invalid");
    assert_eq!(by_source["ci"]["status"], "observed");
    for row in rows {
        assert_eq!(
            row["editable"], false,
            "source-owned evidence is never editable"
        );
    }
    // None of this serializes a filesystem path.
    assert!(!leaks_path(
        &get(&config, &db, &token, "/v1/admin/portfolio/pproj/evidence"),
        &proj
    ));
}

#[test]
fn evidence_view_surfaces_missing_evidence_as_withheld_not_healthy() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let proj = dir.path().join("proj");
    write_project(&proj, "pproj");
    register_project(&db, &proj);

    // Stale/unavailable and other sources retain their honest state; missing
    // interest evidence is withheld rather than reported as ready.
    let evidence = body_json(&get(&config, &db, &token, "/v1/admin/portfolio/evidence"));
    let sections = &evidence["sections"];
    assert_eq!(sections["interest"]["state"], "evaluated");
    assert_eq!(sections["interest"]["threshold"], 100);
    let verdicts = sections["interest"]["verdicts"].as_array().unwrap();
    let pproj = verdicts
        .iter()
        .find(|v| v["project_id"] == "pproj")
        .expect("pproj verdict present");
    assert_eq!(pproj["withheld"], true, "no evidence never reads as ready");
    assert_eq!(pproj["readiness"], "not-ready");
    assert!(
        !pproj["reasons"].as_array().unwrap().is_empty(),
        "the withholding reason is named"
    );
    // The gap report synthesizes source state instead of hiding it.
    assert!(sections["gaps"].get("total").is_some());
    // No path leaks from the aggregated evidence.
    assert!(!leaks_path(
        &get(&config, &db, &token, "/v1/admin/portfolio/evidence"),
        &proj
    ));
}

#[test]
fn evidence_view_reports_each_source_state_independently() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let proj = dir.path().join("proj");
    write_project(&proj, "pproj");
    register_project(&db, &proj);

    let evidence = body_json(&get(&config, &db, &token, "/v1/admin/portfolio/evidence"));
    let sections = &evidence["sections"];
    // Governance: local always enabled, workspace-governance unconfigured by
    // default (no adapter contacted), each carrying its real state.
    let governance = sections["governance"]["providers"]
        .as_array()
        .unwrap()
        .clone();
    let names: Vec<&str> = governance
        .iter()
        .map(|row| row["provider"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"local"));
    assert!(names.contains(&"workspace-governance"));
    // Analytics support is a catalog read; no provider is marked live.
    for row in sections["analytics"]["providers"].as_array().unwrap() {
        assert_eq!(row["live"], false);
        assert!(["supported", "planned"].contains(&row["support"].as_str().unwrap()));
    }
    // Fleet/inventory section reports its own availability state.
    assert!(sections["fleet"]["state"].is_string());
    // Catalog section carries its own contract and freshness observation.
    assert!(sections["catalog"]["observed_at"].is_string());
}

#[test]
fn frontend_portfolio_is_standalone_json_only_and_shell_free() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let app = fs::read_to_string(root.join("frontend/app.js")).unwrap();
    let index = fs::read_to_string(root.join("frontend/index.html")).unwrap();

    assert!(
        index.contains("id=\"portfolio\""),
        "portfolio section exists"
    );
    assert!(
        index.contains("id=\"portfolio-record-review\""),
        "typed metadata mutation control exists"
    );
    assert!(
        app.contains("/v1/admin/portfolio"),
        "typed list/evidence endpoints"
    );
    assert!(
        app.contains("/evidence"),
        "typed cross-project evidence endpoint"
    );
    assert!(
        app.contains("/tags") && app.contains("/reviews"),
        "typed metadata write endpoints"
    );
    // The portfolio region renders structured state only.
    let start = app
        .find("// ---- Portfolio controls")
        .expect("portfolio section marker");
    let end = app
        .find("async function dashboardPage")
        .expect("dashboard function");
    let region = &app[start..end];
    for forbidden in [
        "eval(",
        "new Function",
        "child_process",
        "innerHTML",
        "sh -c",
    ] {
        assert!(
            !region.contains(forbidden),
            "unsafe sink in the portfolio frontend: {forbidden}"
        );
    }
    assert!(!index.contains("<script>"), "no inline script on the page");
}
