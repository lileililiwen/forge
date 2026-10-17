use std::fs;
use std::process::{Command, Output};

use tempfile::TempDir;

fn invoke_gate(project: &TempDir) -> Output {
    invoke_gate_options(project, "", false)
}

fn invoke_gate_with_path(project: &TempDir, path: &str) -> Output {
    invoke_gate_options(project, path, false)
}

fn invoke_gate_options(project: &TempDir, path: &str, dry_run: bool) -> Output {
    let registry = project.path().join("registry.db");
    let mut command = Command::new(env!("CARGO_BIN_EXE_forge"));
    command.args([
        "gate",
        project.path().to_str().expect("UTF-8 fixture path"),
        "--format",
        "json",
    ]);
    if dry_run {
        command.arg("--dry-run");
    }
    command
        .env("FORGE_REGISTRY", registry)
        .env("FORGE_GATE_BIN", "/does/not/exist/driftwatchdog")
        .env("PATH", path)
        .output()
        .expect("Forge gate process starts")
}

fn project() -> TempDir {
    let project = tempfile::tempdir().expect("temporary project");
    fs::write(project.path().join("forge.yaml"), "schema: 1\n").expect("write Forge manifest");
    fs::write(
        project.path().join(".project.json"),
        r#"{"schema_version":1,"id":"forge-native-fixture"}"#,
    )
    .expect("write project identity");
    fs::create_dir(project.path().join("src")).expect("create source root");
    fs::write(project.path().join("src/main.rs"), "fn main() {}\n").expect("write source file");
    project
}

#[test]
fn manifest_free_project_runs_forge_baseline_without_external_runtime() {
    let project = project();

    let output = invoke_gate(&project);

    assert!(
        output.status.success(),
        "status: {:?}\nstdout: {}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let document: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("gate stdout is one JSON document");
    assert_eq!(document["gate"]["runtime"], "forge");
    assert!(document["gate"]["checks"]
        .as_array()
        .expect("check array")
        .iter()
        .any(|check| check["id"] == "source-file-size" && check["state"] == "pass"));
    assert!(!project.path().join(".driftwatch").exists());
    assert!(project
        .path()
        .join(".forge/gate/forge-native-fixture/evidence.json")
        .is_file());
}

#[test]
fn manifest_free_project_blocks_on_source_file_over_baseline_limit() {
    let project = project();
    fs::write(
        project.path().join("src/main.rs"),
        (0..=1000).map(|_| "line\n").collect::<String>(),
    )
    .expect("write oversized source file");

    let output = invoke_gate(&project);

    assert_eq!(output.status.code(), Some(1));
    let document: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("blocked gate stdout is one JSON document");
    assert_eq!(document["gate"]["runtime"], "forge");
    let check = document["gate"]["checks"]
        .as_array()
        .expect("check array")
        .iter()
        .find(|check| check["id"] == "source-file-size")
        .expect("baseline check exists");
    assert_eq!(check["state"], "fail");
    assert!(check.to_string().contains("src/main.rs"), "check: {check}");
}

#[test]
fn project_extension_runs_alongside_unweakenable_baseline() {
    let project = project();
    fs::write(
        project.path().join("gate.toml"),
        r#"version = 1
profile = "minimal"
[source_size]
max_lines = 5000
[[checks]]
id = "source-file-size"
required = false
enabled = false
[[checks]]
id = "project-smoke"
command = "exit 0"
"#,
    )
    .expect("write extension manifest");
    fs::write(
        project.path().join("src/main.rs"),
        (0..=1000).map(|_| "line\n").collect::<String>(),
    )
    .expect("write oversized source file");

    let output = invoke_gate_with_path(&project, "/bin:/usr/bin");
    let document: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("gate stdout is one JSON document");
    let checks = document["gate"]["checks"].as_array().expect("check array");
    assert_eq!(document["gate"]["runtime"], "forge");
    assert!(checks
        .iter()
        .any(|check| { check["id"] == "source-file-size" && check["state"] == "fail" }));
    assert!(
        checks
            .iter()
            .any(|check| { check["id"] == "project-smoke" && check["state"] == "pass" }),
        "checks: {checks:?}"
    );
}

#[test]
fn dry_run_resolves_baseline_without_creating_gate_evidence_or_driftwatch_state() {
    let project = project();

    let output = invoke_gate_options(&project, "", true);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let document: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("dry-run stdout is one JSON document");
    assert!(document["gate"]["rehearsal"].is_object());
    assert!(!project.path().join(".driftwatch").exists());
    assert!(!project.path().join(".forge").exists());
}
