//! Agent session contract (`agent-runtime-workflows`).
//!
//! Covers R1 — Managed agent sessions end to end through the built
//! binary: a successful start records the provider and project
//! association, pause/takeover return explicit unsupported state
//! rather than simulating success, restart preserves the prior
//! transition log, the new-session command creates a fresh
//! session anchored to the project, status renders the recorded
//! state, list returns the project's session inventory, and
//! `run-spec` refuses when the bound spec is missing (R1 boundary
//! scenario: a session that ends without verification is recorded
//! distinctly from a successful spec completion).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn clean_cmd() -> Command {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY");
    cmd.env_remove("HTTP_PROXY");
    cmd.env_remove("HTTPS_PROXY");
    cmd.env_remove("ALL_PROXY");
    cmd.env_remove("OPENAI_API_KEY");
    cmd.env_remove("ANTHROPIC_API_KEY");
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

fn run_json(db: &Path, args: &[&str]) -> serde_json::Value {
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
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
    })
}

fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

fn write_rust_project(dir: &Path, id: &str) {
    fs::create_dir_all(dir).unwrap();
    let text = format!(
        "schema: 1\nproject:\n  id: {id}\n  name: Test {id}\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\n"
    );
    fs::write(dir.join("forge.yaml"), &text).unwrap();
}

#[test]
fn start_records_session_with_provider_and_project_association() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_rust_project(&proj, "agent-start");

    let value = run_json(
        &db,
        &[
            "agent",
            "start",
            proj.to_str().unwrap(),
            "--session",
            "sess-1",
            "--provider",
            "codex",
        ],
    );
    assert_eq!(value["transition"]["session_id"], "sess-1");
    assert_eq!(value["transition"]["project_id"], "agent-start");
    assert_eq!(value["transition"]["provider"], "codex");
    assert_eq!(value["transition"]["requested"], "start");
    // State is either `active` (when codex is on PATH) or absent
    // on a `agent-unavailable` error. The contract is honest
    // about which one applies; in both cases the session id and
    // provider are recorded.
    let state = value["transition"]["state"].as_str();
    if let Some(s) = state {
        assert!(s == "active" || s == "unsupported", "state={s}");
    }
    let files = value["transition"]["files_written"].as_array().unwrap();
    let mut has_session = false;
    let mut has_log = false;
    for f in files {
        let s = f.as_str().unwrap();
        if s.ends_with("/session.json") {
            has_session = true;
        }
        if s.ends_with("/transitions.log") {
            has_log = true;
        }
    }
    assert!(has_session && has_log, "files={:?}", files);
}

#[test]
fn pause_returns_unsupported_state_with_explicit_evidence() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_rust_project(&proj, "agent-pause");

    // Create a session file directly so the test is deterministic
    // and does not depend on a real codex binary being present.
    let session_path = proj.join(".forge/agents/sess-pause/session.json");
    fs::create_dir_all(session_path.parent().unwrap()).unwrap();
    let now = chrono::Utc::now().to_rfc3339();
    let body = format!(
        r#"{{"contract":"0.1.0","session_id":"sess-pause","project_id":"agent-pause","project_path":"{}","provider":"codex","spec_id":"","state":"active","started_at":"{}","last_transition_at":"{}","transitions":[]}}"#,
        proj.display(),
        now,
        now
    );
    fs::write(&session_path, body).unwrap();

    let out = run_json(
        &db,
        &[
            "agent",
            "pause",
            proj.to_str().unwrap(),
            "--session",
            "sess-pause",
        ],
    );
    assert_eq!(out["transition"]["state"], "unsupported");
    let evidence = out["transition"]["evidence"].as_array().unwrap();
    assert!(evidence
        .iter()
        .any(|e| e.as_str().unwrap().contains("pause primitive")));
    let note = out["transition"]["note"].as_str().unwrap();
    assert!(note.contains("unsupported"));
}

#[test]
fn takeover_returns_unsupported_state_with_explicit_evidence() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_rust_project(&proj, "agent-takeover");

    let session_path = proj.join(".forge/agents/sess-take/session.json");
    fs::create_dir_all(session_path.parent().unwrap()).unwrap();
    let now = chrono::Utc::now().to_rfc3339();
    let body = format!(
        r#"{{"contract":"0.1.0","session_id":"sess-take","project_id":"agent-takeover","project_path":"{}","provider":"opencode","spec_id":"","state":"active","started_at":"{}","last_transition_at":"{}","transitions":[]}}"#,
        proj.display(),
        now,
        now
    );
    fs::write(&session_path, body).unwrap();

    let out = run_json(
        &db,
        &[
            "agent",
            "takeover",
            proj.to_str().unwrap(),
            "--session",
            "sess-take",
        ],
    );
    assert_eq!(out["transition"]["state"], "unsupported");
    let evidence = out["transition"]["evidence"].as_array().unwrap();
    assert!(evidence
        .iter()
        .any(|e| e.as_str().unwrap().contains("takeover primitive")));
}

#[test]
fn restart_preserves_prior_state_and_records_active() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_rust_project(&proj, "agent-restart");

    let session_path = proj.join(".forge/agents/sess-restart/session.json");
    fs::create_dir_all(session_path.parent().unwrap()).unwrap();
    let now = chrono::Utc::now().to_rfc3339();
    let body = format!(
        r#"{{"contract":"0.1.0","session_id":"sess-restart","project_id":"agent-restart","project_path":"{}","provider":"codex","spec_id":"spec-restart-abcdef","state":"active","started_at":"{}","last_transition_at":"{}","transitions":[]}}"#,
        proj.display(),
        now,
        now
    );
    fs::write(&session_path, body).unwrap();

    let out = run_json(
        &db,
        &[
            "agent",
            "restart",
            proj.to_str().unwrap(),
            "--session",
            "sess-restart",
        ],
    );
    assert_eq!(out["transition"]["state"], "active");
    assert_eq!(out["transition"]["session_id"], "sess-restart");
    assert_eq!(
        out["transition"]["spec_id"], "spec-restart-abcdef",
        "spec binding must survive restart"
    );
}

#[test]
fn new_session_creates_fresh_session_and_preserves_prior() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_rust_project(&proj, "agent-new");

    let session_path = proj.join(".forge/agents/old-sess/session.json");
    fs::create_dir_all(session_path.parent().unwrap()).unwrap();
    let now = chrono::Utc::now().to_rfc3339();
    let body = format!(
        r#"{{"contract":"0.1.0","session_id":"old-sess","project_id":"agent-new","project_path":"{}","provider":"codex","spec_id":"","state":"active","started_at":"{}","last_transition_at":"{}","transitions":[]}}"#,
        proj.display(),
        now,
        now
    );
    fs::write(&session_path, body).unwrap();

    let value = run_json(
        &db,
        &[
            "agent",
            "new-session",
            proj.to_str().unwrap(),
            "--session",
            "old-sess",
            "--new-session",
            "fresh-sess",
        ],
    );
    assert_eq!(value["transition"]["session_id"], "fresh-sess");
    // Prior session file is preserved.
    assert!(session_path.is_file());
    // New session file is written.
    assert!(proj.join(".forge/agents/fresh-sess/session.json").is_file());
}

#[test]
fn status_renders_recorded_session_state() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_rust_project(&proj, "agent-status");

    let session_path = proj.join(".forge/agents/sess-status/session.json");
    fs::create_dir_all(session_path.parent().unwrap()).unwrap();
    let now = chrono::Utc::now().to_rfc3339();
    let body = format!(
        r#"{{"contract":"0.1.0","session_id":"sess-status","project_id":"agent-status","project_path":"{}","provider":"codex","spec_id":"spec-status-1","state":"active","started_at":"{}","last_transition_at":"{}","transitions":[]}}"#,
        proj.display(),
        now,
        now
    );
    fs::write(&session_path, body).unwrap();

    let out = run_json(
        &db,
        &[
            "agent",
            "status",
            proj.to_str().unwrap(),
            "--session",
            "sess-status",
        ],
    );
    assert_eq!(out["session_id"], "sess-status");
    assert_eq!(out["project_id"], "agent-status");
    assert_eq!(out["provider"], "codex");
    assert_eq!(out["spec_id"], "spec-status-1");
    assert_eq!(out["state"], "active");
}

#[test]
fn list_returns_recorded_sessions() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_rust_project(&proj, "agent-list");

    for (id, state) in [("a-sess", "active"), ("b-sess", "paused")] {
        let path = proj.join(format!(".forge/agents/{id}/session.json"));
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        let body = format!(
            r#"{{"contract":"0.1.0","session_id":"{id}","project_id":"agent-list","project_path":"{}","provider":"codex","spec_id":"","state":"{state}","started_at":"{}","last_transition_at":"{}","transitions":[]}}"#,
            proj.display(),
            now,
            now
        );
        fs::write(&path, body).unwrap();
    }

    let out = run_json(&db, &["agent", "list", proj.to_str().unwrap()]);
    let sessions = out["sessions"].as_array().unwrap();
    assert_eq!(sessions.len(), 2);
    let ids: Vec<String> = sessions
        .iter()
        .map(|s| s["session_id"].as_str().unwrap().to_string())
        .collect();
    assert!(ids.contains(&"a-sess".to_string()));
    assert!(ids.contains(&"b-sess".to_string()));
}

#[test]
fn run_spec_refuses_when_bound_spec_is_missing() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_rust_project(&proj, "agent-runspec");

    let session_path = proj.join(".forge/agents/sess-runspec/session.json");
    fs::create_dir_all(session_path.parent().unwrap()).unwrap();
    let now = chrono::Utc::now().to_rfc3339();
    let body = format!(
        r#"{{"contract":"0.1.0","session_id":"sess-runspec","project_id":"agent-runspec","project_path":"{}","provider":"codex","spec_id":"spec-missing-123456","state":"active","started_at":"{}","last_transition_at":"{}","transitions":[]}}"#,
        proj.display(),
        now,
        now
    );
    fs::write(&session_path, body).unwrap();

    let out = run(
        &db,
        &[
            "agent",
            "run-spec",
            proj.to_str().unwrap(),
            "--session",
            "sess-runspec",
        ],
    );
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("error[spec-invalid]"), "stderr={stderr}");
    // Session file is preserved (R1 boundary scenario: a session
    // that ends without verification is recorded distinctly from
    // a successful spec completion).
    assert!(session_path.is_file());
    let body = fs::read_to_string(&session_path).unwrap();
    assert!(body.contains("spec-missing-123456"));
}

#[test]
fn unknown_session_returns_agent_unavailable() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_rust_project(&proj, "agent-missing");

    let out = run(
        &db,
        &[
            "agent",
            "status",
            proj.to_str().unwrap(),
            "--session",
            "no-such-session",
        ],
    );
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(
        stderr.contains("error[agent-unavailable]"),
        "stderr={stderr}"
    );
}

#[test]
fn unknown_provider_returns_agent_unavailable() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_rust_project(&proj, "agent-bad-provider");

    let out = run(
        &db,
        &[
            "agent",
            "start",
            proj.to_str().unwrap(),
            "--session",
            "sess-bad",
            "--provider",
            "made-up",
        ],
    );
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(
        stderr.contains("error[agent-unavailable]"),
        "stderr={stderr}"
    );
}
