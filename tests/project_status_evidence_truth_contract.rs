//! Project status evidence-truth contract (`project-status-evidence-truth`).
//!
//! The project status projection must name the evidence behind its own
//! verdict. Four claims are pinned here, all driven through the public
//! surface — the session-gated route and `forge::doctor::run_doctor`:
//!
//! 1. **The state partition.** A doctor report that carries an unavailable
//!    finding is projected as `issues` — never `healthy`, never `stale`.
//!    A check that could not run is missing evidence, and missing
//!    evidence is a problem the projection has to show.
//! 2. **The summary.** The doctor sub-check's summary names the failing,
//!    the unavailable and the unmet counts apart, and it never reports
//!    "no problems" while its state reports `issues`.
//! 3. **The agreement.** The doctor sub-check and the check sub-check
//!    count the same unavailable findings: the checker maps an
//!    unavailable finding to a warning alert, and the projection's
//!    summary names that same count.
//! 4. **The doctor's truth.** The projection's partition is the inverse
//!    of the doctor's own `healthy` field: a report the doctor does not
//!    call healthy is never projected as `healthy`.
//!
//! The projection lives behind a private module, so the contract is read
//! through the route the operator calls. The doctor report the projection
//! reads is the one the public doctor produces for the same directory, so
//! the two claims are compared over the same evidence.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use chrono::Utc;
use forge::api::{handle, ApiConfig, ApiRequest};
use forge::checker::{build_document, now_rfc3339, AlertSeverity, DEFAULT_MAX_ALERTS};
use forge::doctor::{run_doctor, DoctorReport, FindingStatus};
use forge::registry::Registry;
use tempfile::tempdir;

const PROJECT_CONTRACT: &str = "forge-project-status/0.1.0";

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

/// Seed the global administrator and return a live session token.
fn login_token(config: &ApiConfig, db: &Path) -> String {
    forge::identity::global::setup(db, "operator@example.test", "a-long-test-password").unwrap();
    let login = json_body(
        request("POST", "/v1/admin/session", &config.frontend_origin),
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

fn body_json(response: &forge::api::ApiResponse) -> serde_json::Value {
    serde_json::from_slice(&response.body).expect("status routes are JSON-only")
}

fn register_project(db: &Path, dir: &Path) {
    let mut registry = Registry::open(db).unwrap();
    registry.register(dir, None).unwrap();
}

/// The manifest of a rust-web project at L1.
fn manifest(id: &str) -> String {
    format!(
        "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\n"
    )
}

/// A rust-web project at L1 that is **not** a git repository, so the
/// repository inspector cannot run and the doctor reports one unavailable
/// finding. Every other check passes, so the report's only problem is
/// missing evidence.
fn write_unavailable_project(dir: &Path, id: &str) {
    fs::create_dir_all(dir).unwrap();
    fs::write(dir.join("forge.yaml"), manifest(id)).unwrap();
    fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"unavailable\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    fs::write(dir.join("README.md"), "# Unavailable project\n").unwrap();
}

/// The number of applicable findings with the given status.
fn count_of(report: &DoctorReport, status: FindingStatus) -> usize {
    report
        .findings
        .iter()
        .filter(|finding| finding.applicable && finding.status == status)
        .count()
}

/// Read one sub-check of a status response.
fn sub_check<'a>(body: &'a serde_json::Value, id: &str) -> &'a serde_json::Value {
    body["checks"]
        .as_array()
        .expect("checks array")
        .iter()
        .find(|check| check["id"] == id)
        .unwrap_or_else(|| panic!("the {id} sub-check must be present"))
}

fn summary_of(check: &serde_json::Value) -> &str {
    check["summary"].as_str().expect("a summary")
}

/// The count a summary names for one kind of problem, e.g.
/// `unavailable finding(s)`.
fn count_named(summary: &str, label: &str) -> usize {
    summary
        .split(", ")
        .find_map(|part| part.strip_suffix(label))
        .and_then(|digits| digits.trim().parse::<usize>().ok())
        .unwrap_or_else(|| panic!("the summary {summary:?} must name the {label} count"))
}

#[test]
fn an_unavailable_check_is_issues_and_its_summary_names_the_unavailable_count() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let proj = dir.path().join("proj");
    write_unavailable_project(&proj, "unavailable-status");
    register_project(&db, &proj);

    let report = run_doctor(&proj, None, None, None).expect("doctor report");
    let unavailable = count_of(&report, FindingStatus::Unavailable);
    assert!(
        unavailable >= 1,
        "the fixture must carry an unavailable finding: {report:?}"
    );

    let response = handle(
        &config,
        &db,
        &with_cookie(
            request(
                "GET",
                "/v1/admin/projects/unavailable-status/status",
                &config.frontend_origin,
            ),
            &token,
        ),
        Utc::now(),
    );
    assert_eq!(response.status, 200);
    let body = body_json(&response);
    assert_eq!(body["contract"], PROJECT_CONTRACT);

    let doctor = sub_check(&body, "doctor");
    assert_eq!(
        doctor["state"], "issues",
        "a check that could not run is missing evidence, and missing evidence is an issue"
    );
    assert_ne!(doctor["state"], "healthy");
    assert_ne!(doctor["state"], "stale");
    assert_ne!(doctor["state"], "unavailable");

    let summary = summary_of(doctor);
    assert_eq!(
        count_named(summary, "unavailable finding(s)"),
        unavailable,
        "the summary must name the unavailable findings it counted: {summary}"
    );
    assert!(
        summary.contains("failing finding(s)"),
        "the summary must name the failing count: {summary}"
    );
    assert!(
        summary.contains("unmet control(s)"),
        "the summary must name the unmet count: {summary}"
    );
    assert!(
        !summary.contains("0 failing finding(s), 0 unmet control(s)"),
        "a summary must never report no problems while its state reports issues: {summary}"
    );
}

#[test]
fn the_doctor_sub_check_and_the_check_sub_check_count_the_same_unavailable_findings() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let proj = dir.path().join("proj");
    write_unavailable_project(&proj, "agreement-status");
    register_project(&db, &proj);

    let report = run_doctor(&proj, None, None, None).expect("doctor report");
    let unavailable = count_of(&report, FindingStatus::Unavailable);
    assert!(
        unavailable >= 1,
        "the fixture must carry an unavailable finding"
    );

    // The checker assembly the route reuses: the unavailable findings are
    // warning alerts there.
    let governance = forge::governance::inspect(&proj);
    let document = build_document(
        &proj,
        &report,
        &governance,
        &now_rfc3339(),
        DEFAULT_MAX_ALERTS,
    );
    let errors = document
        .alerts
        .iter()
        .filter(|alert| alert.severity == AlertSeverity::Error)
        .count();
    let warnings = document
        .alerts
        .iter()
        .filter(|alert| alert.severity == AlertSeverity::Warning)
        .count();
    for finding in report
        .findings
        .iter()
        .filter(|finding| finding.applicable && finding.status == FindingStatus::Unavailable)
    {
        let symbol = format!("doctor/{}", finding.id);
        assert!(
            document
                .alerts
                .iter()
                .any(|alert| alert.symbol == symbol && alert.severity == AlertSeverity::Warning),
            "the checker must count the unavailable finding {symbol} as a warning"
        );
    }

    let response = handle(
        &config,
        &db,
        &with_cookie(
            request(
                "GET",
                "/v1/admin/projects/agreement-status/status",
                &config.frontend_origin,
            ),
            &token,
        ),
        Utc::now(),
    );
    assert_eq!(response.status, 200);
    let body = body_json(&response);

    let doctor = sub_check(&body, "doctor");
    let doctor_summary = summary_of(doctor);
    assert_eq!(
        count_named(doctor_summary, "unavailable finding(s)"),
        unavailable,
        "the doctor sub-check must count the unavailable findings: {doctor_summary}"
    );

    let check = sub_check(&body, "check");
    let check_summary = summary_of(check);
    assert_eq!(
        check_summary,
        format!("{errors} error(s), {warnings} warning(s)"),
        "the check sub-check must count the same report the doctor sub-check read"
    );
    assert!(
        count_named(check_summary, "warning(s)") >= unavailable,
        "the check sub-check must count the unavailable findings: {check_summary}"
    );
}

#[test]
fn the_projection_is_the_inverse_of_the_doctors_own_health_field() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let proj = dir.path().join("proj");
    write_unavailable_project(&proj, "truth-status");
    register_project(&db, &proj);

    let report = run_doctor(&proj, None, None, None).expect("doctor report");
    assert!(
        !report.healthy,
        "the fixture must not be healthy in the doctor's own words: {report:?}"
    );

    let response = handle(
        &config,
        &db,
        &with_cookie(
            request(
                "GET",
                "/v1/admin/projects/truth-status/status",
                &config.frontend_origin,
            ),
            &token,
        ),
        Utc::now(),
    );
    assert_eq!(response.status, 200);
    let body = body_json(&response);

    let doctor = sub_check(&body, "doctor");
    assert_ne!(
        doctor["state"], "healthy",
        "a report the doctor does not call healthy is never projected as healthy"
    );
    assert_eq!(body["state"], "issues");
}

#[test]
fn a_stale_observation_with_an_unavailable_finding_is_issues_and_names_the_staleness() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let proj = dir.path().join("proj");
    write_unavailable_project(&proj, "stale-status");
    register_project(&db, &proj);

    // The registry observation now predates the manifest, so the doctor
    // report is stale — and still carries the unavailable finding.
    fs::write(proj.join("forge.yaml"), manifest("stale-status")).unwrap();

    let response = handle(
        &config,
        &db,
        &with_cookie(
            request(
                "GET",
                "/v1/admin/projects/stale-status/status",
                &config.frontend_origin,
            ),
            &token,
        ),
        Utc::now(),
    );
    assert_eq!(response.status, 200);
    let body = body_json(&response);

    let doctor = sub_check(&body, "doctor");
    assert_eq!(
        doctor["state"], "issues",
        "a stale observation never hides a finding the doctor could not gather"
    );
    let summary = summary_of(doctor);
    assert!(
        summary.contains("unavailable finding(s)"),
        "the summary must name the unavailable count: {summary}"
    );
    assert!(
        summary.contains("stale"),
        "the summary must name the staleness: {summary}"
    );
}

#[test]
fn the_fleet_summary_counts_the_states_the_projection_produces() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let proj = dir.path().join("proj");
    write_unavailable_project(&proj, "fleet-status");
    register_project(&db, &proj);

    let response = handle(
        &config,
        &db,
        &with_cookie(
            request("GET", "/v1/admin/status", &config.frontend_origin),
            &token,
        ),
        Utc::now(),
    );
    assert_eq!(response.status, 200);
    let body = body_json(&response);
    assert_eq!(body["contract"], PROJECT_CONTRACT);
    assert_eq!(body["total"], 1);

    let counts = &body["counts"];
    let sum = counts["healthy"].as_u64().unwrap()
        + counts["issues"].as_u64().unwrap()
        + counts["stale"].as_u64().unwrap()
        + counts["unavailable"].as_u64().unwrap();
    assert_eq!(
        sum,
        body["total"].as_u64().unwrap(),
        "counts must sum to total"
    );
    assert_eq!(counts["issues"], 1);
}
