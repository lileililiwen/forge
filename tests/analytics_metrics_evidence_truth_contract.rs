//! CLI contract for `analytics-metrics-evidence-truth`.
//!
//! One rule judges every metric: a metric states a value only when
//! Forge observed it. A metric whose source was not gathered is
//! `unavailable`, carries no snapshot and no value, and names the
//! missing evidence.
//!
//! - the eight doctor-sourced metrics are `unavailable` with a note
//!   when no doctor summary was gathered;
//! - no doctor-sourced metric carries a source or an observation
//!   timestamp: the report never invents a reading;
//! - the registry-count metric is still `present` with the real count,
//!   and a measured metric still carries its source and window;
//! - the report is not complete and the journal row is `partial` with
//!   the unavailable count in its detail;
//! - the assurance browser's metrics route reports the same gap and
//!   still writes nothing.

use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use chrono::Utc;
use forge::api::{handle, ApiConfig, ApiRequest, ApiResponse};
use serde_json::Value;

/// The metrics whose evidence comes from the doctor inventory. Forge
/// gathers no doctor summary today, so none of them may carry a
/// reading.
const DOCTOR_SOURCED_METRICS: [&str; 8] = [
    "quality-healthy",
    "quality-warnings",
    "quality-failures",
    "agents-running",
    "specs-queued",
    "deployment-running",
    "deployment-offline",
    "deployment-pending",
];

/// The sources only a gathered doctor summary can produce.
const DOCTOR_SOURCES: [&str; 4] = ["doctor", "agent-sessions", "spec-registry", "deploy-state"];

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

fn clean_cmd() -> Command {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY")
        .env_remove("HTTP_PROXY")
        .env_remove("HTTPS_PROXY")
        .env_remove("ALL_PROXY")
        .env_remove("OPENAI_API_KEY")
        .env_remove("ANTHROPIC_API_KEY")
        .env_remove("FORGE_DEPLOYER_BIN")
        .env_remove("FORGE_DRIFTWATCH_BIN")
        .env_remove("FORGE_DOCS_TRANSLATOR_BIN")
        .env_remove("FORGE_PACKAGE_BIN")
        .env_remove("FORGE_NOTES_BIN")
        .env_remove("FORGE_ANALYTICS_BIN");
    cmd
}

fn run(db: &Path, args: &[&str]) -> std::process::Output {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    for a in args {
        cmd.arg(a);
    }
    cmd.output().expect("run forge")
}

fn run_json(db: &Path, args: &[&str]) -> Value {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    cmd.arg("--format").arg("json");
    for a in args {
        cmd.arg(a);
    }
    let out = cmd.output().expect("run forge json");
    serde_json::from_slice(&out.stdout).unwrap_or_else(|err| {
        panic!(
            "invalid json: {err}; stdout={} stderr={}",
            lossy(&out.stdout),
            lossy(&out.stderr)
        )
    })
}

/// The `analytics` journal rows, oldest first.
fn analytics_journal(db: &Path) -> Vec<(String, String, String)> {
    let conn = rusqlite::Connection::open(db).expect("open db");
    let mut stmt = conn
        .prepare(
            "SELECT project_id, state, detail FROM operations WHERE kind = 'analytics' ORDER BY op_id",
        )
        .expect("prepare");
    let mut rows = stmt.query([]).expect("query");
    let mut out = Vec::new();
    while let Some(row) = rows.next().expect("row") {
        out.push((
            row.get::<_, String>(0).expect("project_id"),
            row.get::<_, String>(1).expect("state"),
            row.get::<_, String>(2).expect("detail"),
        ));
    }
    out
}

/// Stage a project whose `analytics:` block declares one repository
/// provider with the given adapter command. The adapter answers, so
/// the repository metrics are measured and the report holds real
/// readings next to the doctor metrics that have none.
fn write_analytics_project(dir: &Path, id: &str, adapter_command: &Path) {
    fs::create_dir_all(dir).unwrap();
    let text = format!(
        "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: rust-web\n  maturity: L1\n\
         runtime:\n  language: rust\nanalytics:\n  enabled: true\n  default_window_days: 7\n  \
         repository:\n    - provider: github-analytics\n      enabled: true\n      \
         project_ref: owner/repo\n      adapter_command: {}\n",
        adapter_command.display()
    );
    fs::write(dir.join("forge.yaml"), text).unwrap();
}

/// Stage an adapter fixture that reports the manifest's project
/// reference and one measured repository reading.
fn write_script(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    let mut file = fs::File::create(&path).unwrap();
    file.write_all(b"#!/usr/bin/env bash\n").unwrap();
    file.write_all(body.as_bytes()).unwrap();
    let mut perms = fs::metadata(&path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&path, perms).unwrap();
    path
}

/// The aggregate of one metric id.
fn aggregate<'a>(report: &'a Value, metric_id: &str) -> &'a Value {
    report["aggregates"]
        .as_array()
        .expect("aggregates")
        .iter()
        .find(|a| a["metric_id"] == metric_id)
        .unwrap_or_else(|| panic!("the report carries no `{metric_id}` metric"))
}

/// Every doctor-sourced metric is an explicit gap: `unavailable`, no
/// snapshot, no value, and a note naming the missing evidence.
fn assert_doctor_metrics_are_unavailable(report: &Value) {
    for metric_id in DOCTOR_SOURCED_METRICS {
        let aggregate = aggregate(report, metric_id);
        assert_eq!(
            aggregate["state"], "unavailable",
            "{metric_id} must be unavailable: {aggregate}"
        );
        assert!(
            aggregate["current"].is_null(),
            "{metric_id} must carry no value: {aggregate}"
        );
        assert!(
            aggregate["snapshots"].as_array().unwrap().is_empty(),
            "{metric_id} must carry no snapshot: {aggregate}"
        );
        assert!(
            aggregate["windows"].as_array().unwrap().is_empty(),
            "{metric_id} must carry no window: {aggregate}"
        );
        let note = aggregate["note"].as_str().unwrap_or_default();
        assert!(
            !note.is_empty() && note.contains("no doctor summary was gathered"),
            "{metric_id} must name the missing evidence: {note}"
        );
        assert!(
            !aggregate.to_string().contains("observed_at"),
            "{metric_id} must carry no observation timestamp: {aggregate}"
        );
    }
}

/// No metric in the report claims a doctor source: the report never
/// invents a reading.
fn assert_no_metric_claims_a_doctor_source(report: &Value) {
    for aggregate in report["aggregates"].as_array().unwrap() {
        for snapshot in aggregate["snapshots"].as_array().unwrap() {
            let source = snapshot["source"].as_str().unwrap_or_default();
            assert!(
                !DOCTOR_SOURCES.contains(&source),
                "{} claims the doctor source `{source}`: {snapshot}",
                aggregate["metric_id"]
            );
        }
    }
}

#[test]
fn a_metrics_run_reports_the_unmeasured_doctor_metrics_as_unavailable() {
    // The doctor never ran, so the eight doctor-sourced metrics carry no
    // reading at all: they are `unavailable` with a note, the report is
    // not complete, and the journal says how many metrics are missing.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    let scripts = tmp.path().join("scripts");
    fs::create_dir_all(&scripts).unwrap();
    let adapter = write_script(
        &scripts,
        "good.sh",
        "printf '{\"project_ref\":\"owner/repo\",\"evidence\":[\"stars=42\",\"growth=3\"],\"note\":\"ok\"}\\n'\n",
    );
    write_analytics_project(&proj, "ana-truth", &adapter);
    let _ = run(&db, &["register", proj.to_str().unwrap()]);

    let out = run(&db, &["analytics", "metrics", proj.to_str().unwrap()]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "a metrics run must exit 0; stderr={}",
        lossy(&out.stderr)
    );
    let json = run_json(
        &db,
        &[
            "analytics",
            "metrics",
            proj.to_str().unwrap(),
            "--format",
            "json",
        ],
    );
    let report = &json["metrics"];
    assert_doctor_metrics_are_unavailable(report);
    assert_no_metric_claims_a_doctor_source(report);
    // The registry count is measured, so it is still present.
    let projects = aggregate(report, "projects");
    assert_eq!(projects["state"], "present");
    assert_eq!(projects["current"], 1);
    assert_eq!(projects["snapshots"][0]["source"], "registry");
    // A measured metric still carries its source and its window.
    let stars = aggregate(report, "repository-stars");
    assert_eq!(stars["state"], "present");
    assert_eq!(stars["current"], 42);
    assert_eq!(stars["snapshots"][0]["source"], "github-analytics");
    // Eight metrics were not measured, so the report is not complete.
    assert_eq!(report["complete"], false);

    let rows = analytics_journal(&db);
    assert!(!rows.is_empty(), "the metrics run wrote no journal row");
    let (project_id, state, detail) = rows.last().expect("journal row");
    assert_eq!(project_id, "ana-truth");
    assert_eq!(state, "partial");
    assert!(
        detail.contains("unavailable=8"),
        "the journal detail must name the unavailable count: {detail}"
    );
    assert!(detail.contains("complete=false"), "detail={detail}");
}

#[test]
fn the_assurance_browser_reports_the_same_gap() {
    // The browser aggregates with no doctor summary too, so it reports
    // the same explicit gap. The route still probes nothing and writes
    // nothing.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    let scripts = tmp.path().join("scripts");
    fs::create_dir_all(&scripts).unwrap();
    let adapter = write_script(
        &scripts,
        "good.sh",
        "printf '{\"project_ref\":\"owner/repo\",\"evidence\":[\"stars=42\",\"growth=3\"],\"note\":\"ok\"}\\n'\n",
    );
    write_analytics_project(&proj, "ana-browser", &adapter);
    {
        let mut registry = forge::registry::Registry::open(&db).unwrap();
        registry.register(&proj, None).unwrap();
    }
    let config = ApiConfig::default();
    let token = login_token(&config, &db);
    let response = get(
        &config,
        &db,
        &token,
        "/v1/admin/projects/ana-browser/analytics/metrics",
        None,
    );
    assert_eq!(response.status, 200);
    let body: Value = serde_json::from_slice(&response.body).expect("JSON body");
    let report = &body["metrics"];
    assert_doctor_metrics_are_unavailable(report);
    assert_no_metric_claims_a_doctor_source(report);
    assert_eq!(aggregate(report, "projects")["state"], "present");
    assert_eq!(report["complete"], false);
    assert!(
        analytics_journal(&db).is_empty(),
        "the browser's metrics route must not journal"
    );
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

fn get(config: &ApiConfig, db: &Path, token: &str, path: &str, query: Option<&str>) -> ApiResponse {
    let mut req = request("GET", path, &config.frontend_origin);
    if let Some(q) = query {
        req.query = Some(q.to_string());
    }
    handle(config, db, &with_cookie(req, token), Utc::now())
}

fn login_token(config: &ApiConfig, db: &Path) -> String {
    forge::identity::global::setup(db, "operator@example.test", "a-long-test-password").unwrap();
    let mut login = request("POST", "/v1/admin/session", &config.frontend_origin);
    login
        .headers
        .insert("content-type".to_string(), "application/json".to_string());
    login.body = br#"{"email":"operator@example.test","password":"a-long-test-password"}"#.to_vec();
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
