//! Cross-surface regression for `supervised-agent-adapters`: the
//! delegated runtime must not change what the other planes see.
//! MCP dispatches the same Core path, the portal stays a read-only
//! surface that never spawns the runtime, doctor verdicts are
//! byte-equivalent across a supervised round trip, and the journal
//! keeps its per-transport kinds.

use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn clean_cmd(envs: &[(&str, &str)]) -> Command {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY");
    cmd.env_remove("FORGE_ARIADEX_BIN");
    cmd.env_remove("FORGE_SISYPHUSFY_BIN");
    cmd.env_remove("STUB_DIR");
    cmd.env_remove("SISY_STUB_DIR");
    cmd.env_remove("SISY_OUTCOME");
    cmd.env_remove("SISY_EXIT");
    cmd.env_remove("HTTP_PROXY");
    cmd.env_remove("HTTPS_PROXY");
    cmd.env_remove("ALL_PROXY");
    for (key, value) in envs {
        cmd.env(key, value);
    }
    cmd
}

fn run(db: &Path, envs: &[(&str, &str)], args: &[&str]) -> std::process::Output {
    let mut cmd = clean_cmd(envs);
    cmd.arg("--registry").arg(db);
    for a in args {
        cmd.arg(a);
    }
    cmd.output().expect("run forge")
}

fn run_json(db: &Path, envs: &[(&str, &str)], args: &[&str]) -> serde_json::Value {
    let mut all: Vec<&str> = vec!["--format", "json"];
    all.extend_from_slice(args);
    let out = run(db, envs, &all);
    serde_json::from_slice(&out.stdout).unwrap_or_else(|err| {
        panic!(
            "invalid json: {err}; stdout={} stderr={}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
    })
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

fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

fn write_rust_project(dir: &Path, id: &str) {
    fs::create_dir_all(dir).unwrap();
    let text = format!(
        "schema: 1\nproject:\n  id: {id}\n  name: Test {id}\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\n"
    );
    fs::write(dir.join("forge.yaml"), text).unwrap();
}

fn write_script(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, format!("#!/usr/bin/env bash\nset -u\n{body}")).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    path
}

/// Minimal live-daemon ariadex stub for cross-surface checks.
fn ariadex_stub(dir: &Path) -> PathBuf {
    write_script(
        dir,
        "ariadex.sh",
        r#"
echo "$*" >> "${STUB_DIR:?}/log"
case "${1:-}" in
  --version) echo 'ariadex 0.1.0'; exit 0;;
  start|stop|pause|resume|takeover) echo 'ok'; exit 0;;
  status) printf '{"daemon": {"alive": true, "mode": "AUTO", "session": "cafe1234abcd"}}\n'; exit 0;;
  *) exit 2;;
esac
"#,
    )
}

fn sisyphusfy_stub(dir: &Path) -> PathBuf {
    write_script(
        dir,
        "sisyphusfy.sh",
        r#"
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
}

struct Harness {
    _tmp: tempfile::TempDir,
    db: PathBuf,
    proj: PathBuf,
    envs: Vec<(String, String)>,
    stub_dir: PathBuf,
}

impl Harness {
    fn new(id: &str) -> Self {
        let tmp = tempfile::tempdir().unwrap();
        let proj = tmp.path().join("proj");
        write_rust_project(&proj, id);
        let scripts = tmp.path().join("scripts");
        fs::create_dir_all(&scripts).unwrap();
        let stub_dir = tmp.path().join("stub");
        fs::create_dir_all(&stub_dir).unwrap();
        let ariadex = ariadex_stub(&scripts);
        let sisy = sisyphusfy_stub(&scripts);
        let stub_dir_display = stub_dir.display().to_string();
        Harness {
            db: tmp.path().join("registry.db"),
            stub_dir,
            proj,
            envs: vec![
                ("FORGE_ARIADEX_BIN".into(), ariadex.display().to_string()),
                ("STUB_DIR".into(), stub_dir_display),
                ("FORGE_SISYPHUSFY_BIN".into(), sisy.display().to_string()),
            ],
            _tmp: tmp,
        }
    }

    fn env_refs(&self) -> Vec<(&str, &str)> {
        self.envs
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect()
    }

    fn stub_log(&self) -> String {
        fs::read_to_string(self.stub_dir.join("log")).unwrap_or_default()
    }
}

#[test]
fn mcp_run_agent_delegates_ariadex_with_the_same_backing_record() {
    let h = Harness::new("xagent-mcp-ari");
    let requests = vec![serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "run_agent",
        "params": {
            "path": h.proj.to_str().unwrap(),
            "session": "sess-m",
            "provider": "ariadex",
            "transition": "start"
        }
    })];
    let stdout = run_mcp(&h.db, &h.env_refs(), &requests);
    let response = response_for(&stdout, 1);
    let transition = &response["result"]["transition"];
    assert_eq!(transition["state"], "active");
    assert_eq!(transition["provider"], "ariadex");
    assert!(response["result"]["error"].is_null());
    // The CLI reads the same record with its backing pointer.
    let value = run_json(
        &h.db,
        &h.env_refs(),
        &[
            "agent",
            "status",
            h.proj.to_str().unwrap(),
            "--session",
            "sess-m",
        ],
    );
    // The CLI live-probes the runtime; the stored record carries
    // the same backing the MCP path persisted.
    let session: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(h.proj.join(".forge/agents/sess-m/session.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(session["backing"]["runtime"], "ariadex");
    assert_eq!(session["backing"]["handle"], "cafe1234abcd");
    assert_eq!(value["backing"]["handle"], "cafe1234abcd");
    assert_eq!(value["live"]["state"], "active");
    // The journal keeps the MCP kind.
    let registry = forge::registry::Registry::open(&h.db).unwrap();
    let rows = registry
        .operations_for_project("xagent-mcp-ari", 10)
        .unwrap();
    assert!(rows.iter().any(|r| r.kind == "mcp" && r.state == "done"));
}

#[test]
fn mcp_run_spec_with_supervisor_journals_verdict_and_envelope() {
    let h = Harness::new("xagent-mcp-spec");
    let spec_id = "spec-x-1234567890ab";
    let dir = h.proj.join(".forge/specs").join(spec_id);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("manifest.json"), "{\"contract\":\"0.1.0\"}").unwrap();
    fs::write(dir.join("tasks.md"), "- [ ] x\n").unwrap();
    let sess_dir = h.proj.join(".forge/agents/sess-ms");
    fs::create_dir_all(&sess_dir).unwrap();
    let now = chrono::Utc::now().to_rfc3339();
    fs::write(
        sess_dir.join("session.json"),
        format!(
            r#"{{"contract":"0.1.0","session_id":"sess-ms","project_id":"xagent-mcp-spec","project_path":"{}","provider":"codex","spec_id":"{spec_id}","state":"active","started_at":"{now}","last_transition_at":"{now}","transitions":[]}}"#,
            h.proj.display()
        ),
    )
    .unwrap();
    let outcome = h.stub_dir.join("done.json");
    fs::write(
        &outcome,
        r#"{"stop_reason":"complete","iterations":1,"verification":{"status":"success","source":"configured"}}"#,
    )
    .unwrap();
    let mut envs = h.env_refs();
    let outcome_path = outcome.to_str().unwrap().to_string();
    envs.push(("SISY_OUTCOME", outcome_path.as_str()));
    envs.push(("SISY_EXIT", "0"));
    let stdout = run_mcp(
        &h.db,
        &envs,
        &[serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "run_agent",
            "params": {
                "path": h.proj.to_str().unwrap(),
                "session": "sess-ms",
                "provider": "sisyphusfy",
                "transition": "run_spec"
            }
        })],
    );
    let response = response_for(&stdout, 1);
    let transition = &response["result"]["transition"];
    assert_eq!(transition["verdict"], "done");
    assert_eq!(transition["spec_id"], spec_id);
    let registry = forge::registry::Registry::open(&h.db).unwrap();
    let rows = registry
        .operations_for_project("xagent-mcp-spec", 10)
        .unwrap();
    assert!(rows.iter().any(|r| r.kind == "mcp"
        && r.state == "done"
        && r.detail
            .as_deref()
            .unwrap_or_default()
            .contains("verdict `done`")));
}

#[test]
fn mcp_run_agent_rejects_sisyphusfy_as_a_session_provider() {
    let h = Harness::new("xagent-mcp-bad");
    let stdout = run_mcp(
        &h.db,
        &h.env_refs(),
        &[serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "run_agent",
            "params": {
                "path": h.proj.to_str().unwrap(),
                "session": "sess-bad",
                "provider": "sisyphusfy",
                "transition": "start"
            }
        })],
    );
    let response = response_for(&stdout, 1);
    assert!(response["error"].is_object());
    assert_eq!(response["error"]["code"], -32602);
    assert!(h.stub_log().is_empty(), "no runtime may spawn");
}

#[test]
fn mcp_tools_list_advertises_no_new_supervised_surface() {
    let h = Harness::new("xagent-mcp-tools");
    let stdout = run_mcp(
        &h.db,
        &h.env_refs(),
        &[serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"})],
    );
    let response = response_for(&stdout, 1);
    let names: Vec<&str> = response["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap_or_default())
        .collect();
    assert!(names.contains(&"run_agent"));
    for forbidden in ["ariadex", "sisyphusfy", "agent_status", "runtime_attach"] {
        assert!(
            !names.contains(&forbidden),
            "supervised surface must not grow the tool registry: {names:?}"
        );
    }
}

#[test]
fn portal_stays_read_only_and_never_spawns_the_runtime() {
    let h = Harness::new("xagent-portal");
    let started = run_json(
        &h.db,
        &h.env_refs(),
        &[
            "agent",
            "start",
            h.proj.to_str().unwrap(),
            "--session",
            "sess-pt",
            "--provider",
            "ariadex",
        ],
    );
    assert_eq!(started["transition"]["state"], "active");
    let log_lines_after_start = h.stub_log().lines().count();
    // Register + open the project dashboard: the agents section
    // counts the supervised session, and portal spawns nothing.
    run(
        &h.db,
        &h.env_refs(),
        &["register", h.proj.to_str().unwrap()],
    );
    let dashboard = run_json(
        &h.db,
        &h.env_refs(),
        &["portal", "dashboard", "xagent-portal"],
    );
    let sections = dashboard["dashboard"]["sections"].as_array().unwrap();
    let agents = sections
        .iter()
        .find(|s| s["section_id"].as_str() == Some("agents"))
        .expect("agents section");
    assert_eq!(agents["status"].as_str().unwrap(), "ok");
    assert!(agents["entries"]
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["label"]
            .as_str()
            .unwrap_or_default()
            .contains("agent sessions")));
    assert_eq!(
        h.stub_log().lines().count(),
        log_lines_after_start,
        "portal must not spawn the runtime"
    );
}

#[test]
fn doctor_verdict_is_byte_identical_across_a_supervised_round_trip() {
    let h = Harness::new("xagent-doctor");
    let before = run(
        &h.db,
        &h.env_refs(),
        &["--format", "json", "doctor", h.proj.to_str().unwrap()],
    );
    assert!(
        !before.stdout.is_empty(),
        "stderr={}",
        lossy(&before.stderr)
    );
    run(
        &h.db,
        &h.env_refs(),
        &[
            "agent",
            "start",
            h.proj.to_str().unwrap(),
            "--session",
            "sess-d",
            "--provider",
            "ariadex",
        ],
    );
    run(
        &h.db,
        &h.env_refs(),
        &[
            "agent",
            "pause",
            h.proj.to_str().unwrap(),
            "--session",
            "sess-d",
        ],
    );
    let after = run(
        &h.db,
        &h.env_refs(),
        &["--format", "json", "doctor", h.proj.to_str().unwrap()],
    );
    assert_eq!(before.stdout, after.stdout, "doctor verdict changed");
}

#[test]
fn agent_list_serializes_supervised_provider_for_every_surface() {
    let h = Harness::new("xagent-list");
    run(
        &h.db,
        &h.env_refs(),
        &[
            "agent",
            "start",
            h.proj.to_str().unwrap(),
            "--session",
            "sess-l",
            "--provider",
            "ariadex",
        ],
    );
    let value = run_json(
        &h.db,
        &h.env_refs(),
        &["agent", "list", h.proj.to_str().unwrap()],
    );
    let sessions = value["sessions"].as_array().unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0]["provider"], "ariadex");
    assert_eq!(sessions[0]["state"], "active");
}

#[test]
fn checker_surface_ignores_supervised_sessions() {
    let h = Harness::new("xagent-check");
    run(
        &h.db,
        &h.env_refs(),
        &[
            "agent",
            "start",
            h.proj.to_str().unwrap(),
            "--session",
            "sess-ck",
            "--provider",
            "ariadex",
        ],
    );
    // Register so the checker can resolve the target; `forge check`
    // must not surface a session or spawn the runtime.
    run(
        &h.db,
        &h.env_refs(),
        &["register", h.proj.to_str().unwrap()],
    );
    let before = h.stub_log().lines().count();
    let out = run(&h.db, &h.env_refs(), &["check", "xagent-check"]);
    let stdout = lossy(&out.stdout);
    assert!(stdout.contains("\"alerts\""), "stdout={stdout}");
    assert!(
        !stdout.contains("sess-ck"),
        "sessions never leak to the checker: {stdout}"
    );
    assert_eq!(
        h.stub_log().lines().count(),
        before,
        "checker must not spawn the runtime"
    );
}
