use std::process::{Command, Output};

use serde_json::Value;
use tempfile::tempdir;

fn forge(args: &[&str], cwd: &std::path::Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_forge"))
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("Forge CLI starts")
}

#[test]
fn module_list_uses_executable_checkout_and_returns_sorted_modules() {
    let cwd = tempdir().expect("temporary caller directory");
    let output = forge(&["--format", "json", "module", "list"], cwd.path());
    assert!(
        output.status.success(),
        "module list failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let modules: Value = serde_json::from_slice(&output.stdout).expect("JSON module list");
    let rows = modules.as_array().expect("module list is an array");
    let ids = rows
        .iter()
        .map(|row| row["id"].as_str().expect("module id"))
        .collect::<Vec<_>>();
    assert!(
        ids.windows(2).all(|pair| pair[0] < pair[1]),
        "IDs are sorted: {ids:?}"
    );
    assert!(ids.contains(&"dotnet-platform-libs"));
    assert!(ids.contains(&"rust-platform-libs"));
    assert!(ids.contains(&"workspace-governance"));
}

#[test]
fn module_list_ndjson_emits_one_record_per_module() {
    let cwd = tempdir().expect("temporary caller directory");
    let output = forge(&["--format", "ndjson", "module", "list"], cwd.path());
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("NDJSON is UTF-8");
    let records = stdout
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).expect("one JSON record per line"))
        .collect::<Vec<_>>();
    assert_eq!(records.len(), 3);
    assert_eq!(records[0]["id"], "dotnet-platform-libs");
    assert_eq!(records[1]["id"], "rust-platform-libs");
    assert_eq!(records[2]["id"], "workspace-governance");
}

#[test]
fn module_verify_unknown_id_is_not_treated_as_a_path() {
    let cwd = tempdir().expect("temporary caller directory");
    let output = forge(
        &["--format", "json", "module", "verify", "missing-module"],
        cwd.path(),
    );
    assert!(!output.status.success());
    let error: Value = serde_json::from_slice(&output.stderr).expect("typed JSON error");
    assert_eq!(error["error"]["code"], "module-not-found");
}

#[test]
fn module_verify_dry_run_uses_gate_rehearsal_without_persisting_evidence() {
    let cwd = tempdir().expect("temporary caller directory");
    let evidence = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("modules/rust-platform-libs/.forge/gate/rust-platform-libs/evidence.json");
    let existed_before = evidence.exists();
    let output = forge(
        &[
            "module",
            "verify",
            "rust-platform-libs",
            "--dry-run",
            "--profile",
            "fast",
        ],
        cwd.path(),
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "module dry-run failed: {stdout}");
    assert!(stdout.contains("dry-run: gate plan rehearsal"), "{stdout}");
    assert!(stdout.contains("execution profile fast"), "{stdout}");
    assert_eq!(evidence.exists(), existed_before);
}
