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

/// Cross-surface regression for `workspace-governance-adapter-consumption`:
/// a preset-selected provider whose adapter is broken must never render
/// healthy through any read surface, and the standalone default must stay
/// independent of the workspace root.
fn registered_project(db: &std::path::Path, dest: &std::path::Path, id: &str) {
    let out = Command::new(forge_bin())
        .arg("--registry")
        .arg(db)
        .args([
            "new",
            "--profile",
            "rust-web",
            "--id",
            id,
            dest.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(0),
        "new failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn stage_executable_fixture(root: &std::path::Path, name: &str) -> std::path::PathBuf {
    let candidate = root.join("workspace-governance/scripts/forge_governance_adapter.py");
    std::fs::create_dir_all(candidate.parent().unwrap()).unwrap();
    std::fs::copy(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/governance-audit")
            .join(name),
        &candidate,
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&candidate, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    candidate
}

fn cli_json(db: &std::path::Path, args: &[&str]) -> serde_json::Value {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_WORKSPACE_ROOT")
        .arg("--registry")
        .arg(db)
        .arg("--format")
        .arg("json");
    for arg in args {
        cmd.arg(arg);
    }
    let out = cmd.output().unwrap();
    assert!(
        out.status.success(),
        "exit {:?}; stderr {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("json")
}

#[test]
fn broken_preset_adapter_never_renders_healthy_in_checker_or_portal() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let project = tmp.path().join("gov-cross-preset");
    registered_project(&db, &project, "gov-cross-preset");
    let workspace = tempfile::tempdir().unwrap();
    let candidate = stage_executable_fixture(workspace.path(), "error-findings.sh");

    // Select through the packaged preset, then break the checkout.
    let selected = cli_json(
        &db,
        &[
            "governance",
            "use",
            "workspace-governance",
            project.to_str().unwrap(),
            "--workspace-root",
            workspace.path().to_str().unwrap(),
        ],
    );
    assert_eq!(selected["provider"], "workspace-governance");
    std::fs::remove_file(&candidate).unwrap();

    // The checker plane surfaces the gap as a warning and never an
    // implicit pass; exit stays 0 because findings are evidence.
    let check = cli_json(&db, &["check", project.to_str().unwrap()]);
    let alerts = check["alerts"].as_array().unwrap();
    let governance_alert = alerts
        .iter()
        .find(|alert| alert["source"] == "governance")
        .expect("a broken governance plane must alert");
    assert_eq!(governance_alert["severity"], "warning");
    assert_eq!(
        governance_alert["symbol"],
        "governance/workspace-governance"
    );

    // The portal settings entry is `unavailable`, never `ok`.
    let view = cli_json(&db, &["portal", "view", "settings", "gov-cross-preset"]);
    let entries = view["view"]["entries"].as_array().unwrap();
    let settings = entries
        .iter()
        .find(|entry| entry["id"] == "gov-cross-preset:settings")
        .expect("the registered project renders a settings entry");
    assert_eq!(settings["status"], "unavailable");
    assert_eq!(
        settings["attributes"]["governance_provider"],
        "workspace-governance"
    );

    // And with a live-but-failing adapter the surfaces keep the mapping
    // honest: fail is fail everywhere, not pass.
    stage_executable_fixture(workspace.path(), "error-findings.sh");
    let selected_again = cli_json(
        &db,
        &[
            "governance",
            "use",
            "workspace-governance",
            project.to_str().unwrap(),
            "--workspace-root",
            workspace.path().to_str().unwrap(),
        ],
    );
    assert_eq!(selected_again["provider"], "workspace-governance");
    let check = cli_json(&db, &["check", project.to_str().unwrap()]);
    let alerts = check["alerts"].as_array().unwrap();
    let governance_alert = alerts
        .iter()
        .find(|alert| alert["source"] == "governance")
        .expect("a failing governance plane must alert");
    assert_eq!(governance_alert["severity"], "error");
    let view = cli_json(&db, &["portal", "view", "settings", "gov-cross-preset"]);
    let settings = view["view"]["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["id"] == "gov-cross-preset:settings")
        .unwrap();
    assert_eq!(settings["status"], "fail");
}

#[test]
fn unconfigured_governance_never_requests_a_workspace_root() {
    // Standalone default re-verified: no config, no env, no sibling.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let project = tmp.path().join("gov-cross-local");
    registered_project(&db, &project, "gov-cross-local");

    let status = cli_json(&db, &["governance", "status", project.to_str().unwrap()]);
    assert_eq!(status["observation"]["provider"], "local");
    assert_eq!(status["observation"]["status"], "pass");
    let list = cli_json(&db, &["governance", "list", project.to_str().unwrap()]);
    let providers = list["providers"].as_array().unwrap();
    assert_eq!(providers.len(), 1);
    assert_eq!(providers[0]["provider"], "local");
    assert!(providers[0]["adapter"].is_null());
}

#[test]
fn mcp_transport_shares_the_preset_resolved_observation() {
    // Read-plane parity: the MCP transport consumes the normalized
    // observation the CLI selected, whatever the adapter said.
    let dir = project();
    let workspace = tempfile::tempdir().unwrap();
    stage_executable_fixture(workspace.path(), "adoption-gap.sh");
    let candidate = workspace
        .path()
        .join("workspace-governance/scripts/forge_governance_adapter.py");
    forge::governance::save_provider_selection(
        dir.path(),
        "workspace-governance",
        Some(candidate.to_str().unwrap()),
        true,
        10_000,
    )
    .unwrap();

    let request = forge::mcp::McpRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(serde_json::Value::from(1)),
        method: "run_governance".to_string(),
        params: serde_json::json!({"path": dir.path()}),
    };
    let value = forge::mcp::dispatch(None, &request).unwrap();
    assert_eq!(value["observation"]["provider"], "workspace-governance");
    assert_eq!(value["observation"]["status"], "blocked");
}
