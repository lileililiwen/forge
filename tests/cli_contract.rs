//! R3 — Independent command contract: help, version, structured errors
//! and empty-registry behavior work without network, AI config or GUI.

use std::path::PathBuf;
use std::process::Command;

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

/// Clean environment: no network proxy, no AI config, no registry leak.
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

#[test]
fn help_lists_supported_surface_without_network_or_ai() {
    let out = clean_cmd().arg("--help").output().expect("run forge");
    assert_eq!(out.status.code(), Some(0));
    let text = String::from_utf8_lossy(&out.stdout);
    for cmd in ["list", "inspect", "register", "agent", "test", "commit", "push"] {
        assert!(text.contains(cmd), "help must mention {cmd}:\n{text}");
    }
}

#[test]
fn version_reports_without_services() {
    let out = clean_cmd().arg("--version").output().expect("run forge");
    assert_eq!(out.status.code(), Some(0));
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("forge"), "unexpected version output: {text}");
}

#[test]
fn unknown_subcommand_fails_without_side_effects() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("registry.db");
    let out = clean_cmd()
        .arg("--registry")
        .arg(&db)
        .arg("frobnicate")
        .output()
        .expect("run forge");
    assert_ne!(out.status.code(), Some(0));
    assert_eq!(out.status.code(), Some(2));
    assert!(
        !db.exists(),
        "usage failure must not create a registry database"
    );
}

#[test]
fn empty_registry_lists_empty_collection() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("registry.db");

    let out = clean_cmd()
        .arg("--registry")
        .arg(&db)
        .arg("list")
        .output()
        .expect("run forge");
    assert_eq!(out.status.code(), Some(0));
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("No projects registered"), "{text}");

    let out = clean_cmd()
        .arg("--registry")
        .arg(&db)
        .arg("--format")
        .arg("json")
        .arg("list")
        .output()
        .expect("run forge");
    assert_eq!(out.status.code(), Some(0));
    let value: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("list must emit JSON");
    assert_eq!(value, serde_json::json!({"projects": []}));
}

#[test]
fn unknown_project_returns_structured_nonzero_error() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("registry.db");

    let out = clean_cmd()
        .arg("--registry")
        .arg(&db)
        .arg("inspect")
        .arg("no-such-project")
        .output()
        .expect("run forge");
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("error[unknown-project]"), "{stderr}");

    let out = clean_cmd()
        .arg("--registry")
        .arg(&db)
        .arg("--format")
        .arg("json")
        .arg("inspect")
        .arg("no-such-project")
        .output()
        .expect("run forge");
    assert_eq!(out.status.code(), Some(1));
    let value: serde_json::Value = serde_json::from_slice(&out.stderr).expect("error must be JSON");
    assert_eq!(value["error"]["code"], "unknown-project");
}
