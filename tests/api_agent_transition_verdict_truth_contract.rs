//! HTTP contract for the agent journal state.
//!
//! One rule decides the journal state: a transition the runtime refused
//! or lost is recorded as the state it reported, and a transition the
//! runtime carried out is recorded as `done`. The command and the MCP
//! tool read that rule; the HTTP route must read it too.
//!
//! - the HTTP route journals the state the runtime reported;
//! - a refused transition is never journalled as a completed action;
//! - the route and the command agree on the state they record.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use forge::api::{handle, ApiConfig, ApiRequest, ApiResponse};
use forge::registry::Registry;

/// The bearer session the route authorizes against.
const SESSION_ID: &str = "a1b2c3d4e5f60718";

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

fn json_body(mut req: ApiRequest, body: &[u8]) -> ApiRequest {
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
            "schema: 1\nproject:\n  id: {id}\n  name: Test {id}\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\nfeatures: {{}}\n"
        ),
    )
    .unwrap();
}

fn write_session(dir: &Path, session_id: &str, project_id: &str) {
    let session_dir = dir.join(".forge/agents").join(session_id);
    fs::create_dir_all(&session_dir).unwrap();
    let now = chrono::Utc::now().to_rfc3339();
    fs::write(
        session_dir.join("session.json"),
        format!(
            r#"{{"contract":"0.1.0","session_id":"{session_id}","project_id":"{project_id}","project_path":"{}","provider":"codex","spec_id":"","state":"active","started_at":"{now}","last_transition_at":"{now}","transitions":[]}}"#,
            dir.display()
        ),
    )
    .unwrap();
}

fn fixture() -> (tempfile::TempDir, PathBuf, ApiConfig, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let project = tmp.path().join("proj");
    write_project(&project, "api-agent");
    {
        let mut registry = Registry::open(&db).unwrap();
        registry.register(&project, None).unwrap();
    }
    write_session(&project, "s1", "api-agent");
    seed_identity_session(&project, "api-agent", SESSION_ID);
    let config = ApiConfig::default();
    (tmp, db, config, project)
}

/// An operator session with the `admin:access` permission the route
/// demands, seeded through the same file shape the Core persists.
fn seed_identity_session(project: &Path, project_id: &str, session_id: &str) {
    let now = Utc::now();
    let session = forge::identity::AdminSession {
        session_id: session_id.to_string(),
        project_id: project_id.to_string(),
        provider: "okta".to_string(),
        subject: "user-1".to_string(),
        issued_at: now,
        expires_at: now + chrono::Duration::seconds(3600),
        permissions: vec!["admin:access".to_string()],
        state: forge::identity::SessionState::Active,
        note: None,
    };
    forge::identity::save_session(project, project_id, &session).unwrap();
}

/// The journal rows of one kind, oldest first.
fn journal_rows(db: &Path, kind: &str) -> Vec<(String, String)> {
    Registry::open(db)
        .unwrap()
        .journal_entries()
        .unwrap()
        .iter()
        .filter(|e| e.kind == kind)
        .map(|e| (e.state.clone(), e.detail.clone().unwrap_or_default()))
        .collect()
}

fn transition(config: &ApiConfig, db: &Path, token: &str, body: &[u8]) -> (u16, serde_json::Value) {
    let mut req = request(
        "POST",
        "/v1/projects/api-agent/agents",
        &config.frontend_origin,
    );
    req.bearer_token = Some(token.to_string());
    req = json_body(req, body);
    let response: ApiResponse = handle(config, db, &req, Utc::now());
    let value = serde_json::from_slice(&response.body).unwrap_or_else(|err| {
        panic!(
            "invalid json: {err}; body={}",
            String::from_utf8_lossy(&response.body)
        )
    });
    (response.status, value)
}

#[test]
fn the_http_route_journals_the_state_the_runtime_reported() {
    // The bundled provider exposes no backing runtime in this build:
    // the transition is refused and the state the runtime reported is
    // `unsupported`.
    let (_tmp, db, config, _project) = fixture();
    let token = SESSION_ID;

    let (status, value) = transition(
        &config,
        &db,
        token,
        br#"{"session":"s1","transition":"start","provider":"codex"}"#,
    );
    assert_eq!(status, 202, "the route answers with its outcome: {value}");
    assert_eq!(value["agent"]["transition"]["state"], "unsupported");

    let rows = journal_rows(&db, "api");
    assert_eq!(rows.len(), 1, "one transition row per transition: {rows:?}");
    let (state, detail) = &rows[0];
    assert_eq!(
        state, "unsupported",
        "the route journals the state the runtime reported; detail={detail}"
    );
    assert_ne!(
        state, "done",
        "a refused transition is never a completed action; detail={detail}"
    );
    assert!(
        detail.contains("-> `unsupported`"),
        "the detail names the state it reached: {detail}"
    );
}

#[test]
fn the_route_and_the_command_agree_on_the_state_they_record() {
    // One rule, every surface: the command and the route record the
    // same state for the same refused transition.
    let (_tmp, db, config, _project) = fixture();
    let token = SESSION_ID;
    let (status, _value) = transition(
        &config,
        &db,
        token,
        br#"{"session":"s1","transition":"start","provider":"codex"}"#,
    );
    assert_eq!(status, 202);

    let rows = journal_rows(&db, "api");
    assert_eq!(rows.len(), 1, "one transition row per transition: {rows:?}");

    // The command, driven against the same registry.
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_forge"))
        .env_remove("FORGE_REGISTRY")
        .arg("--registry")
        .arg(&db)
        .arg("--format")
        .arg("json")
        .arg("agent")
        .arg("start")
        .arg(_project.to_str().unwrap())
        .arg("--session")
        .arg("s1")
        .arg("--provider")
        .arg("codex")
        .output()
        .expect("run forge agent start");
    assert_eq!(
        out.status.code(),
        Some(0),
        "the command answers with its honest state; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );

    let command_rows: Vec<_> = journal_rows(&db, "agent");
    assert_eq!(
        command_rows.len(),
        1,
        "one row per transition: {command_rows:?}"
    );
    assert_eq!(
        command_rows[0].0, rows[0].0,
        "the command and the route read the same rule"
    );
    assert_eq!(command_rows[0].0, "unsupported");
}

#[test]
fn the_route_writes_a_transition_row_and_an_operation_row() {
    // Two rows, two facts: the transition's row names the transition's
    // state, and the operation's row says the request was handled. The
    // requirement names both, so a reader never has to guess which is
    // which.
    let (_tmp, db, config, _project) = fixture();
    let token = SESSION_ID;
    let (status, _value) = transition(
        &config,
        &db,
        token,
        br#"{"session":"s1","transition":"start","provider":"codex"}"#,
    );
    assert_eq!(status, 202);

    let transition_rows = journal_rows(&db, "api");
    assert_eq!(transition_rows.len(), 1, "{transition_rows:?}");
    assert_eq!(transition_rows[0].0, "unsupported");
    assert!(transition_rows[0].1.contains("-> `unsupported`"));

    let operation_rows = journal_rows(&db, "api.agent");
    assert_eq!(operation_rows.len(), 1, "{operation_rows:?}");
    assert_eq!(
        operation_rows[0].0, "done",
        "the operation row records that the request was handled"
    );
    assert!(
        operation_rows[0].1.contains("api.agent completed"),
        "the operation row names the request it completed: {}",
        operation_rows[0].1
    );
}
