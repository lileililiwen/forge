use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("forge.yaml"),
        "schema: 1\nproject:\n  id: cli-demo\n  name: CLI Demo\n  profile: rust-web\n",
    )
    .unwrap();
    dir
}

#[test]
fn governance_local_status_is_available_without_external_tools() {
    let dir = project();
    let output = Command::new(forge_bin())
        .arg("--format")
        .arg("json")
        .arg("governance")
        .arg("status")
        .arg(dir.path())
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["observation"]["provider"], "local");
    assert_eq!(value["observation"]["status"], "pass");
}

#[test]
fn governance_use_external_preserves_manifest_and_reports_unavailable_adapter() {
    let dir = project();
    let manifest = fs::read(dir.path().join("forge.yaml")).unwrap();
    let use_output = Command::new(forge_bin())
        .arg("governance")
        .arg("use")
        .arg("workspace-governance")
        .arg(dir.path())
        .arg("--adapter")
        .arg("missing-governance-adapter")
        .output()
        .unwrap();
    assert_eq!(use_output.status.code(), Some(0));
    assert_eq!(fs::read(dir.path().join("forge.yaml")).unwrap(), manifest);

    let status_output = Command::new(forge_bin())
        .arg("--format")
        .arg("json")
        .arg("governance")
        .arg("status")
        .arg(dir.path())
        .output()
        .unwrap();
    assert_eq!(status_output.status.code(), Some(0));
    let value: Value = serde_json::from_slice(&status_output.stdout).unwrap();
    assert_eq!(value["observation"]["provider"], "workspace-governance");
    assert_eq!(value["observation"]["status"], "unavailable");
}

#[test]
fn governance_observation_is_available_through_mcp() {
    let dir = project();
    let request = forge::mcp::McpRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(Value::from(1)),
        method: "run_governance".to_string(),
        params: serde_json::json!({"path": dir.path()}),
    };
    let value = forge::mcp::dispatch(None, &request).unwrap();
    assert_eq!(value["observation"]["provider"], "local");
    assert_eq!(value["observation"]["status"], "pass");
}
