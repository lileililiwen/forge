//! Contract test: the deploy status projection exposes the phase
//! evidence it has, and names a missing value instead of dropping it.
//!
//! The canonical capability says every entry exposes its `revision`,
//! `build_status`, `run_status` and `container_identity` "through both
//! the JSON document and the human renderer; missing values render as
//! `unknown`, never as a verified success". The projection dropped the
//! field entirely when the row carried no phase evidence, so a row
//! with none rendered exactly like a fully verified publish.
//!
//! The same rule is pinned on every surface that reads the row: the
//! CLI's human table, the CLI's JSON document and the API's
//! `GET /v1/admin/projects/{id}/deploy/status` route.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use chrono::Utc;
use forge::api::{handle, ApiConfig, ApiRequest};
use serde_json::Value;

/// The four phase evidence fields every projected entry exposes.
const EVIDENCE_FIELDS: [&str; 4] = [
    "revision",
    "build_status",
    "run_status",
    "container_identity",
];

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

fn clean_cmd() -> Command {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY")
        .env_remove("FORGE_PUBLISH_PROVIDER")
        .env_remove("FORGE_DEPLOYER_BIN")
        .env_remove("FORGE_HERMORA_BIN");
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
    if !out.status.success() {
        panic!(
            "forge failed: status={} stderr={}",
            out.status,
            lossy(&out.stderr)
        );
    }
    serde_json::from_slice(&out.stdout).unwrap_or_else(|err| {
        panic!(
            "invalid json: {err}; stdout={} stderr={}",
            lossy(&out.stdout),
            lossy(&out.stderr)
        )
    })
}

/// Record one publish row for `p1` that carries no phase evidence —
/// the shape a legacy row, or a provider that returned a terminal
/// response without phase statuses, persists.
fn record_row_without_phase_evidence(db: &Path) {
    let registry = forge::registry::Registry::open(db).expect("open registry");
    registry
        .record_operation("publish", "p1", "done", "shipped 1.2.3")
        .expect("record operation");
}

/// Record one publish row for `p1` that carries its own phase
/// evidence — the shape a provider that reported the build, the run
/// and the container identity persists.
fn record_row_with_phase_evidence(db: &Path) {
    let registry = forge::registry::Registry::open(db).expect("open registry");
    registry
        .record_publish_phase(
            "p1",
            "done",
            forge::registry::PublishPhaseEvidence::new()
                .revision("0123456789abcdef0123456789abcdef01234567")
                .container_identity("forge-p1-0123456789ab")
                .build_status("succeeded")
                .run_status("succeeded"),
            Some("provider=stub health=healthy"),
        )
        .expect("record publish phase");
}

/// The four projected phase evidence values of one entry, in a stable
/// order, so two surfaces can be compared field by field.
fn evidence_of(entry: &Value) -> Vec<String> {
    EVIDENCE_FIELDS
        .iter()
        .map(|field| entry[*field].as_str().unwrap_or_default().to_string())
        .collect()
}

#[test]
fn the_human_renderer_names_the_phase_evidence_it_does_not_have() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    record_row_without_phase_evidence(&db);

    let out = run(&db, &["deploy", "status", "--project", "p1"]);
    assert!(
        out.status.success(),
        "deploy status: {}",
        lossy(&out.stderr)
    );
    let stdout = lossy(&out.stdout);
    assert!(
        stdout.contains("build=unknown"),
        "a missing build status must render as `unknown`; the renderer printed: {stdout}"
    );
    assert!(
        stdout.contains("run=unknown"),
        "a missing run status must render as `unknown`; the renderer printed: {stdout}"
    );
    assert!(
        stdout.contains("revision=unknown"),
        "a missing revision must render as `unknown`; the renderer printed: {stdout}"
    );
    assert!(
        stdout.contains("container=unknown"),
        "a missing container identity must render as `unknown`; the renderer printed: {stdout}"
    );
}

#[test]
fn the_json_document_names_the_phase_evidence_it_does_not_have() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    record_row_without_phase_evidence(&db);

    let json = run_json(&db, &["deploy", "status", "--project", "p1"]);
    let entries = json["entries"].as_array().expect("entries array");
    assert_eq!(entries.len(), 1, "one row: {json}");
    let entry = &entries[0];
    for field in EVIDENCE_FIELDS {
        assert_eq!(
            entry[field], "unknown",
            "the entry must expose `{field}` as `unknown` when the row carries none; the entry was {entry}"
        );
    }
}

#[test]
fn a_row_with_phase_evidence_keeps_its_own_values() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    record_row_with_phase_evidence(&db);

    let json = run_json(&db, &["deploy", "status", "--project", "p1"]);
    let entries = json["entries"].as_array().expect("entries array");
    let entry = &entries[0];
    assert_eq!(entry["build_status"], "succeeded", "the entry was {entry}");
    assert_eq!(entry["run_status"], "succeeded", "the entry was {entry}");
    assert_eq!(
        entry["revision"], "0123456789abcdef0123456789abcdef01234567",
        "the entry was {entry}"
    );
    assert_eq!(
        entry["container_identity"], "forge-p1-0123456789ab",
        "the entry was {entry}"
    );

    let out = run(&db, &["deploy", "status", "--project", "p1"]);
    let stdout = lossy(&out.stdout);
    assert!(
        stdout.contains("build=succeeded") && stdout.contains("run=succeeded"),
        "the renderer must keep the values it has; it printed: {stdout}"
    );
    assert!(
        !stdout.contains("unknown"),
        "a row with phase evidence must not render `unknown`; it printed: {stdout}"
    );
}

/// One admin request, shaped like the ones the shipped history routes
/// are driven with.
fn api_request(method: &str, path: &str, origin: &str) -> ApiRequest {
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

fn api_json_body(mut req: ApiRequest, body: &[u8]) -> ApiRequest {
    req.headers
        .insert("content-type".to_string(), "application/json".to_string());
    req.body = body.to_vec();
    req
}

/// The session cookie the admin routes require, obtained through the
/// same login the shipped history contract uses.
fn api_login_token(config: &ApiConfig, db: &Path) -> String {
    forge::identity::global::setup(db, "operator@example.test", "a-long-test-password").unwrap();
    let mut login = api_request("POST", "/v1/admin/session", &config.frontend_origin);
    login = api_json_body(
        login,
        br#"{"email":"operator@example.test","password":"a-long-test-password"}"#,
    );
    let response = handle(config, db, &login, Utc::now());
    assert_eq!(response.status, 200, "the session login must succeed");
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

/// The API's deploy status document for `p1`.
fn api_deploy_status(config: &ApiConfig, db: &Path, token: &str) -> Value {
    let mut req = api_request(
        "GET",
        "/v1/admin/projects/p1/deploy/status",
        &config.frontend_origin,
    );
    req.cookies
        .insert("forge_admin_session".to_string(), token.to_string());
    let response = handle(config, db, &req, Utc::now());
    assert_eq!(response.status, 200, "the deploy status route must answer");
    serde_json::from_slice(&response.body).expect("the admin route is JSON-only")
}

/// A registry with one managed project `p1` and the session token the
/// admin routes need. The route answers only for a project the
/// registry manages.
fn api_fixture() -> (tempfile::TempDir, PathBuf, ApiConfig, String) {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let project = tmp.path().join("p1");
    fs::create_dir_all(&project).unwrap();
    fs::write(
        project.join("forge.yaml"),
        "schema: 1\nproject:\n  id: p1\n  name: Test p1\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\n",
    )
    .unwrap();
    {
        let mut registry = forge::registry::Registry::open(&db).expect("open registry");
        registry.register(&project, None).expect("register project");
    }
    let config = ApiConfig::default();
    let token = api_login_token(&config, &db);
    (tmp, db, config, token)
}

#[test]
fn the_api_route_names_the_phase_evidence_it_does_not_have() {
    let (_tmp, db, config, token) = api_fixture();
    record_row_without_phase_evidence(&db);

    let json = api_deploy_status(&config, &db, &token);
    let entries = json["entries"].as_array().expect("entries array");
    assert_eq!(entries.len(), 1, "one publish row: {json}");
    let entry = &entries[0];
    for field in EVIDENCE_FIELDS {
        assert_eq!(
            entry[field], "unknown",
            "the route must expose `{field}` as `unknown` when the row carries none; the entry was {entry}"
        );
    }

    // One surface, one rule: the CLI's JSON document projects the same
    // values for the same row.
    let cli = run_json(&db, &["deploy", "status", "--project", "p1"]);
    let cli_entries = cli["entries"].as_array().expect("entries array");
    assert_eq!(evidence_of(&cli_entries[0]), evidence_of(entry));
}

#[test]
fn the_api_route_keeps_the_row_own_phase_evidence() {
    let (_tmp, db, config, token) = api_fixture();
    record_row_with_phase_evidence(&db);

    let json = api_deploy_status(&config, &db, &token);
    let entries = json["entries"].as_array().expect("entries array");
    assert_eq!(entries.len(), 1, "one publish row: {json}");
    let entry = &entries[0];
    assert_eq!(entry["build_status"], "succeeded", "the entry was {entry}");
    assert_eq!(entry["run_status"], "succeeded", "the entry was {entry}");
    assert_eq!(
        entry["revision"], "0123456789abcdef0123456789abcdef01234567",
        "the entry was {entry}"
    );
    assert_eq!(
        entry["container_identity"], "forge-p1-0123456789ab",
        "the entry was {entry}"
    );

    // One surface, one rule: the CLI's JSON document projects the same
    // values for the same row.
    let cli = run_json(&db, &["deploy", "status", "--project", "p1"]);
    let cli_entries = cli["entries"].as_array().expect("entries array");
    assert_eq!(evidence_of(&cli_entries[0]), evidence_of(entry));
}
