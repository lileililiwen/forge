//! CLI and MCP contract for `agent-transition-verdict-truth`.
//!
//! One rule decides the journal state: a run-spec records the
//! supervisor's verdict when the supervisor reported one, a transition
//! the runtime refused or lost is recorded as the state it reported,
//! and a transition the runtime carried out is recorded as `done`. A
//! refused transition is never `done`.
//!
//! - a refused transition is journalled as the state the runtime
//!   reported;
//! - a run-spec with no reported verdict says so;
//! - a verified supervisor run still journals `done`;
//! - the MCP tool reads the same rule.

use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

fn clean_cmd(envs: &[(&str, &str)]) -> Command {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY");
    cmd.env_remove("FORGE_ARIADEX_BIN");
    cmd.env_remove("FORGE_SISYPHUSFY_BIN");
    cmd.env_remove("SISY_OUTCOME");
    cmd.env_remove("SISY_EXIT");
    cmd.env_remove("HTTP_PROXY")
        .env_remove("HTTPS_PROXY")
        .env_remove("ALL_PROXY");
    for (key, value) in envs {
        cmd.env(key, value);
    }
    cmd
}

fn run_cli(db: &Path, envs: &[(&str, &str)], args: &[&str]) -> std::process::Output {
    let mut cmd = clean_cmd(envs);
    cmd.arg("--registry").arg(db);
    cmd.arg("--format").arg("json");
    for a in args {
        cmd.arg(a);
    }
    cmd.output().expect("run forge")
}

fn run_mcp(db: &Path, envs: &[(&str, &str)], requests: &[serde_json::Value]) -> String {
    let mut cmd = clean_cmd(envs);
    cmd.arg("--registry").arg(db);
    cmd.arg("mcp").arg("serve");
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    let mut child = cmd.spawn().expect("spawn forge mcp serve");
    {
        let stdin = child.stdin.as_mut().expect("stdin");
        for r in requests {
            let line = serde_json::to_string(r).expect("encode");
            stdin.write_all(line.as_bytes()).expect("write");
            stdin.write_all(b"\n").expect("newline");
        }
    }
    let output = child.wait_with_output().expect("wait");
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn response_for(stdout: &str, id: i64) -> serde_json::Value {
    for line in stdout.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let value: serde_json::Value = serde_json::from_str(line).expect("response json");
        if value["id"] == serde_json::json!(id) {
            return value;
        }
    }
    panic!("no response for id {id} in {stdout}");
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

fn write_spec(dir: &Path, spec_id: &str) {
    let spec_dir = dir.join(".forge/specs").join(spec_id);
    fs::create_dir_all(&spec_dir).unwrap();
    fs::write(spec_dir.join("manifest.json"), "{\"contract\":\"0.1.0\"}").unwrap();
    fs::write(spec_dir.join("tasks.md"), "- [ ] x\n").unwrap();
}

fn write_session(dir: &Path, session_id: &str, project_id: &str, spec_id: &str) {
    let session_dir = dir.join(".forge/agents").join(session_id);
    fs::create_dir_all(&session_dir).unwrap();
    let now = chrono::Utc::now().to_rfc3339();
    fs::write(
        session_dir.join("session.json"),
        format!(
            r#"{{"contract":"0.1.0","session_id":"{session_id}","project_id":"{project_id}","project_path":"{}","provider":"codex","spec_id":"{spec_id}","state":"active","started_at":"{now}","last_transition_at":"{now}","transitions":[]}}"#,
            dir.display()
        ),
    )
    .unwrap();
}

fn sisyphusfy_stub(dir: &Path) -> PathBuf {
    let path = dir.join("sisyphusfy.sh");
    fs::write(
        &path,
        r#"#!/usr/bin/env bash
set -u
case "${1:-}" in
  loop)
    cat "${SISY_OUTCOME:?}"
    exit "${SISY_EXIT:-0}";;
  *)
    echo "error: unknown $*" >&2
    exit 2;;
esac
"#,
    )
    .unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    path
}

/// The journal rows of one kind, oldest first.
fn journal_rows(db: &Path, kind: &str) -> Vec<(String, String)> {
    forge::registry::Registry::open(db)
        .unwrap()
        .journal_entries()
        .unwrap()
        .iter()
        .filter(|e| e.kind == kind)
        .map(|e| (e.state.clone(), e.detail.clone().unwrap_or_default()))
        .collect()
}

#[test]
fn a_refused_transition_is_journalled_as_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_project(&proj, "agent-refused");
    write_session(&proj, "s1", "agent-refused", "");

    // The bundled provider exposes no backing runtime in this build:
    // the transition is refused and the state is `unsupported`.
    let out = run_cli(
        &db,
        &[],
        &[
            "agent",
            "start",
            proj.to_str().unwrap(),
            "--session",
            "s1",
            "--provider",
            "codex",
        ],
    );
    assert_eq!(
        out.status.code(),
        Some(0),
        "the transition answers with its honest state; stderr={}",
        lossy(&out.stderr)
    );
    let value: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("the command answers JSON");
    assert_eq!(value["transition"]["state"], "unsupported");

    let rows = journal_rows(&db, "agent");
    assert_eq!(rows.len(), 1, "one row per transition");
    let (state, detail) = &rows[0];
    assert_eq!(
        state, "unsupported",
        "a refused transition is never `done`; detail={detail}"
    );
    assert!(detail.contains("-> `unsupported`"), "{detail}");
}

#[test]
fn a_run_spec_with_no_verdict_names_that_it_saw_none() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_project(&proj, "agent-spec");
    write_spec(&proj, "spec-x-1234567890ab");
    write_session(&proj, "s1", "agent-spec", "spec-x-1234567890ab");

    // No supervisor: the runtime ran no spec and reported no verdict.
    let out = run_cli(
        &db,
        &[],
        &[
            "agent",
            "run-spec",
            proj.to_str().unwrap(),
            "--session",
            "s1",
        ],
    );
    assert_eq!(
        out.status.code(),
        Some(0),
        "the run answers with its honest state; stderr={}",
        lossy(&out.stderr)
    );
    let value: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("the command answers JSON");
    assert_eq!(value["transition"]["state"], "unsupported");
    assert!(
        value["transition"].get("verdict").is_none(),
        "the envelope carries no verdict either: {value}"
    );

    let rows = journal_rows(&db, "agent");
    let (state, detail) = &rows[0];
    assert_eq!(state, "unsupported", "detail={detail}");
    assert!(
        detail.contains("(verdict `none reported`)"),
        "the detail says it saw no verdict: {detail}"
    );
    assert!(!detail.contains("verdict `done`"), "{detail}");
}

#[test]
fn a_verified_supervisor_run_still_journals_done() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    let scripts = tmp.path().join("scripts");
    fs::create_dir_all(&scripts).unwrap();
    write_project(&proj, "agent-supervised");
    write_spec(&proj, "spec-x-1234567890ab");
    write_session(&proj, "s1", "agent-supervised", "spec-x-1234567890ab");
    let outcome = tmp.path().join("done.json");
    fs::write(
        &outcome,
        r#"{"stop_reason":"complete","iterations":1,"verification":{"status":"success","source":"configured"}}"#,
    )
    .unwrap();
    let stub = sisyphusfy_stub(&scripts);
    let envs = [
        ("FORGE_SISYPHUSFY_BIN", stub.to_str().unwrap()),
        ("SISY_OUTCOME", outcome.to_str().unwrap()),
        ("SISY_EXIT", "0"),
    ];

    let out = run_cli(
        &db,
        &envs,
        &[
            "agent",
            "run-spec",
            proj.to_str().unwrap(),
            "--session",
            "s1",
            "--provider",
            "sisyphusfy",
        ],
    );
    assert_eq!(
        out.status.code(),
        Some(0),
        "the supervisor run answers; stderr={}",
        lossy(&out.stderr)
    );
    let value: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("the command answers JSON");
    assert_eq!(value["transition"]["verdict"], "done");

    let rows = journal_rows(&db, "agent");
    let (state, detail) = &rows[0];
    assert_eq!(
        state, "done",
        "a verified supervisor run is unchanged: {detail}"
    );
    assert!(detail.contains("(verdict `done`)"), "{detail}");
}

#[test]
fn the_mcp_tool_reads_the_same_rule() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_project(&proj, "agent-mcp");
    write_session(&proj, "s1", "agent-mcp", "");

    let stdout = run_mcp(
        &db,
        &[],
        &[serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "run_agent",
            "params": {
                "path": proj.to_str().unwrap(),
                "session": "s1",
                "provider": "codex",
                "transition": "start"
            }
        })],
    );
    let response = response_for(&stdout, 1);
    assert_eq!(response["result"]["transition"]["state"], "unsupported");

    let rows = journal_rows(&db, "mcp");
    assert_eq!(rows.len(), 1, "one row per transition");
    let (state, detail) = &rows[0];
    assert_eq!(
        state, "unsupported",
        "a refused transition is never `done`; detail={detail}"
    );
}
