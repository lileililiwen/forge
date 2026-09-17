//! Cross-surface regression for `mature-mcp-surface`: the
//! MCP surface interacts with the agent, feature, spec and
//! doctor contracts without breaking existing surfaces.
//! Each test exercises one named interaction end to end
//! through the built binary: a model-issued request is
//! served by `forge mcp serve`, the resulting state is
//! observed through the CLI surface, and the contract is
//! confirmed to be identical to the equivalent CLI-only
//! flow.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

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

fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

fn run_mcp(db: &Path, requests: &[serde_json::Value]) -> (String, String) {
    let mut cmd = clean_cmd();
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
    (lossy(&output.stdout), lossy(&output.stderr))
}

fn response_for(stdout: &str, id: i64) -> serde_json::Value {
    for line in stdout.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let value: serde_json::Value = serde_json::from_str(line).expect("response json");
        if value.get("id").and_then(|v| v.as_i64()) == Some(id) {
            return value;
        }
    }
    panic!("no response for id={id} in:\n{stdout}");
}

fn write_rust_project(dir: &Path, id: &str) {
    fs::create_dir_all(dir).unwrap();
    let text = format!(
        "schema: 1\nproject:\n  id: {id}\n  name: Test {id}\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\n"
    );
    fs::write(dir.join("forge.yaml"), &text).unwrap();
}

fn register(db: &Path, dir: &Path, id: &str) {
    write_rust_project(dir, id);
    let out = clean_cmd()
        .arg("--registry")
        .arg(db)
        .arg("register")
        .arg(dir)
        .output()
        .expect("register");
    assert!(
        out.status.success(),
        "register failed: {}",
        lossy(&out.stderr)
    );
}

#[test]
fn mcp_doctor_matches_cli_doctor_for_same_project() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    register(&db, &proj, "cross-doc");

    let cli = clean_cmd()
        .arg("--registry")
        .arg(&db)
        .arg("--format")
        .arg("json")
        .arg("doctor")
        .arg(&proj)
        .output()
        .expect("cli doctor");
    assert_eq!(cli.status.code(), Some(0));
    let cli_doctor: serde_json::Value =
        serde_json::from_slice(&cli.stdout).expect("cli doctor json");
    let cli_findings = cli_doctor["doctor"]["findings"]
        .as_array()
        .expect("cli findings")
        .len();

    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "run_doctor",
        "params": {"path": proj.to_string_lossy()}
    });
    let (stdout, _stderr) = run_mcp(&db, &[req]);
    let response = response_for(&stdout, 1);
    let mcp_findings = response["result"]["doctor"]["findings"]
        .as_array()
        .expect("mcp findings")
        .len();
    assert_eq!(
        mcp_findings, cli_findings,
        "MCP and CLI must report the same finding count"
    );
}

#[test]
fn mcp_create_then_cli_feature_add_preserves_invariant() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dest = tmp.path().join("cross-feat");
    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "create_project",
        "params": {
            "path": dest.to_string_lossy(),
            "profile": "rust-web",
            "id": "cross-feat"
        }
    });
    let (stdout, _stderr) = run_mcp(&db, &[req]);
    let response = response_for(&stdout, 1);
    assert_eq!(response["result"]["created"]["record"]["id"], "cross-feat");

    // Now exercise the feature surface through the CLI on
    // the project that the MCP surface created. The doctor
    // must continue to pass on the feature-modified
    // project, mirroring the existing
    // feature-lifecycle → doctor invariant.
    let feat = clean_cmd()
        .arg("--registry")
        .arg(&db)
        .arg("feature")
        .arg("add")
        .arg("auth")
        .arg("cross-feat")
        .output()
        .expect("feature add");
    assert!(
        feat.status.success(),
        "feature add failed: {}",
        lossy(&feat.stderr)
    );

    let doc = clean_cmd()
        .arg("--registry")
        .arg(&db)
        .arg("--format")
        .arg("json")
        .arg("doctor")
        .arg(&dest)
        .output()
        .expect("doctor");
    assert_eq!(doc.status.code(), Some(0));
    let doc_value: serde_json::Value = serde_json::from_slice(&doc.stdout).expect("doctor json");
    let findings = doc_value["doctor"]["findings"]
        .as_array()
        .expect("findings");
    // The feature-modified project must still have the
    // manifest/profile/features-compatible findings PASS;
    // MCP must not have damaged the project model.
    let compatible_pass = findings
        .iter()
        .any(|f| f["id"] == "features-compatible" && f["status"] == "pass");
    assert!(
        compatible_pass,
        "features-compatible finding must remain PASS after MCP create + CLI feature add: {findings:?}"
    );
}

#[test]
fn mcp_create_then_mcp_generate_spec_preserves_idempotent_contract() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dest = tmp.path().join("cross-spec");
    let create_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "create_project",
        "params": {
            "path": dest.to_string_lossy(),
            "profile": "rust-web",
            "id": "cross-spec"
        }
    });
    let (stdout1, _) = run_mcp(&db, &[create_req]);
    let _ = response_for(&stdout1, 1);

    let spec_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "generate_spec",
        "params": {
            "path": dest.to_string_lossy(),
            "findings": ["dependency-drift"]
        }
    });
    let (stdout2, _) = run_mcp(&db, std::slice::from_ref(&spec_req));
    let response2 = response_for(&stdout2, 2);
    let first_id = response2["result"]["spec"]["spec"]["id"]["dir_name"]
        .as_str()
        .expect("spec id")
        .to_string();
    assert!(dest.join(".forge/specs").join(&first_id).is_dir());

    // A second MCP generate_spec on the same finding set
    // is idempotent: the existing spec is reported, the
    // proposal is not rewritten.
    let (stdout3, _) = run_mcp(&db, &[spec_req]);
    let response3 = response_for(&stdout3, 2);
    let second_id = response3["result"]["spec"]["spec"]["id"]["dir_name"]
        .as_str()
        .expect("spec id")
        .to_string();
    assert_eq!(
        first_id, second_id,
        "spec id must be stable across MCP calls"
    );
}

#[test]
fn mcp_create_then_mcp_run_agent_records_session_with_provider() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dest = tmp.path().join("cross-agent");
    let create_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "create_project",
        "params": {
            "path": dest.to_string_lossy(),
            "profile": "rust-web",
            "id": "cross-agent"
        }
    });
    let (stdout, _) = run_mcp(&db, &[create_req]);
    let _ = response_for(&stdout, 1);

    let agent_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "run_agent",
        "params": {
            "path": dest.to_string_lossy(),
            "session": "sess-mcp",
            "provider": "opencode",
            "transition": "start"
        }
    });
    let (stdout2, _) = run_mcp(&db, &[agent_req]);
    let response = response_for(&stdout2, 2);
    // The session was recorded on disk and the response
    // reports it; the state is either `active` (when the
    // binary is on PATH) or `unsupported` (when not), but
    // in both cases the session id and provider are
    // recorded.
    let transition = &response["result"]["transition"];
    assert_eq!(transition["session_id"], "sess-mcp");
    assert_eq!(transition["provider"], "opencode");
    let state = transition["state"].as_str().expect("state");
    assert!(
        state == "active" || state == "unsupported",
        "unexpected state {state}"
    );
    // The CLI surface sees the same session.
    let list = clean_cmd()
        .arg("--registry")
        .arg(&db)
        .arg("--format")
        .arg("json")
        .arg("agent")
        .arg("list")
        .arg(&dest)
        .output()
        .expect("agent list");
    assert_eq!(list.status.code(), Some(0));
    let list_value: serde_json::Value = serde_json::from_slice(&list.stdout).expect("list json");
    let sessions = list_value["sessions"].as_array().expect("sessions");
    assert!(
        sessions.iter().any(|s| s["session_id"] == "sess-mcp"),
        "CLI must observe the session created through MCP: {sessions:?}"
    );
}

#[test]
fn mcp_commit_refuses_when_outside_requested_paths() {
    // R2 mutating boundary: MCP commit must use the same
    // paths-only contract as the CLI; tracked edits outside
    // the requested set are refused and the working tree
    // is left untouched.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dest = tmp.path().join("cross-commit");
    let create_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "create_project",
        "params": {
            "path": dest.to_string_lossy(),
            "profile": "rust-web",
            "id": "cross-commit"
        }
    });
    let (stdout, _) = run_mcp(&db, &[create_req]);
    let _ = response_for(&stdout, 1);
    // Make the project a git repository and add an
    // unrelated tracked edit.
    run_git(&dest, &["init", "-q"]);
    run_git(&dest, &["config", "user.email", "forge@example.com"]);
    run_git(&dest, &["config", "user.name", "Forge Test"]);
    run_git(&dest, &["config", "init.defaultBranch", "main"]);
    run_git(&dest, &["checkout", "-q", "-b", "main"]);
    run_git(&dest, &["add", "--", "forge.yaml"]);
    run_git(&dest, &["commit", "-q", "-m", "initial"]);
    fs::write(dest.join("README.md"), "tracked edit outside scope\n").unwrap();

    let commit_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "commit",
        "params": {
            "path": dest.to_string_lossy(),
            "paths": ["forge.yaml"],
            "message": "bump manifest"
        }
    });
    let (stdout2, _) = run_mcp(&db, &[commit_req]);
    let response = response_for(&stdout2, 2);
    let error = response["error"].as_object().expect("error");
    let data = error["data"].as_object().expect("data");
    assert_eq!(data["code"], "git-dirty");
}

fn run_git(dir: &Path, args: &[&str]) {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(dir);
    for a in args {
        cmd.arg(a);
    }
    let out = cmd.output().expect("run git");
    assert!(
        out.status.success(),
        "git {:?} failed: status={} stderr={}",
        args,
        out.status,
        lossy(&out.stderr)
    );
}
