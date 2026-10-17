//! Forge-native Gate CLI contract.
//!
//! These integration tests exercise Forge's bundled Gate engine and evidence
//! projection. The retired external-runtime document protocol is covered by
//! the in-tree DriftWatchdog crate; Forge itself owns the public verdict,
//! revision binding, evidence file, and operation journal.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use forge::registry::Registry;

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn project(tmp: &Path, id: &str) -> PathBuf {
    let project = tmp.join(id);
    fs::create_dir_all(project.join("src")).unwrap();
    fs::write(project.join("forge.yaml"), "schema: 1\n").unwrap();
    fs::write(
        project.join(".project.json"),
        format!(r#"{{"schema_version":1,"id":"{id}"}}"#),
    )
    .unwrap();
    fs::write(project.join("src/main.rs"), "fn main() {}\n").unwrap();
    git(&project, &["init", "-q"]);
    git(&project, &["config", "user.email", "forge@example.com"]);
    git(&project, &["config", "user.name", "Forge Test"]);
    git(&project, &["config", "commit.gpgsign", "false"]);
    git(&project, &["add", "-A"]);
    git(&project, &["commit", "-q", "-m", "initial"]);
    project
}

fn git(project: &Path, args: &[&str]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(project)
        .args(args)
        .output()
        .expect("git starts");
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn run_gate(registry: &Path, args: &[&str], legacy_runtime: Option<&str>) -> Output {
    let mut command = Command::new(forge_bin());
    command
        .env_remove("FORGE_REGISTRY")
        .env_remove("FORGE_WORKSPACE_ROOT")
        .env_remove("FORGE_WORKSPACE_REGISTRY")
        .env("FORGE_REGISTRY", registry)
        .env_remove("FORGE_GATE_BIN")
        .env_remove("FORGE_DRIFTWATCH_BIN")
        .arg("--format")
        .arg("json")
        .arg("gate");
    if let Some(path) = legacy_runtime {
        command.env("FORGE_GATE_BIN", path);
    }
    command.args(args).output().expect("Forge gate starts")
}

fn gate_json(output: &Output) -> serde_json::Value {
    assert!(
        !output.stdout.is_empty(),
        "gate produced no JSON; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("gate stdout is JSON")
}

fn evidence_path(project: &Path, id: &str) -> PathBuf {
    project.join(".forge/gate").join(id).join("evidence.json")
}

fn gate_journal(registry: &Path, project_id: &str) -> Vec<(String, String)> {
    Registry::open(registry)
        .unwrap()
        .operations_for_project(project_id, 50)
        .unwrap()
        .into_iter()
        .filter(|entry| entry.kind == "gate")
        .map(|entry| (entry.state, entry.detail.unwrap_or_default()))
        .collect()
}

#[test]
fn baseline_run_persists_forge_evidence_and_done_journal() {
    let tmp = tempfile::tempdir().unwrap();
    let registry = tmp.path().join("registry.db");
    let project = project(tmp.path(), "native-pass");

    let output = run_gate(&registry, &[project.to_str().unwrap()], None);
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let document = gate_json(&output);
    assert_eq!(document["gate"]["runtime"], "forge");
    assert_eq!(document["gate"]["aggregate"], "passed");
    assert_eq!(document["gate"]["checks"].as_array().unwrap().len(), 1);
    assert_eq!(document["gate"]["checks"][0]["id"], "source-file-size");
    assert!(evidence_path(&project, "native-pass").is_file());
    let rows = gate_journal(&registry, "native-pass");
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(rows[0].0, "done");
}

#[test]
fn required_extension_failure_blocks_and_persists_with_the_baseline() {
    let tmp = tempfile::tempdir().unwrap();
    let registry = tmp.path().join("registry.db");
    let project = project(tmp.path(), "native-fail");
    fs::write(
        project.join("gate.toml"),
        "version = 1\nprofile = \"minimal\"\n[[checks]]\nid = \"project-smoke\"\ncommand = \"exit 1\"\n",
    )
    .unwrap();

    let output = run_gate(&registry, &[project.to_str().unwrap()], None);
    assert_eq!(output.status.code(), Some(1));
    let document = gate_json(&output);
    assert_eq!(document["gate"]["runtime"], "forge");
    assert_eq!(document["gate"]["aggregate"], "blocked");
    let checks = document["gate"]["checks"].as_array().unwrap();
    assert!(checks
        .iter()
        .any(|row| { row["id"] == "source-file-size" && row["state"] == "pass" }));
    assert!(checks
        .iter()
        .any(|row| { row["id"] == "project-smoke" && row["state"] == "fail" }));
    assert!(evidence_path(&project, "native-fail").is_file());
    assert_eq!(gate_journal(&registry, "native-fail")[0].0, "blocked");
}

#[test]
fn dry_run_previews_the_native_plan_without_evidence_or_journaling() {
    let tmp = tempfile::tempdir().unwrap();
    let registry = tmp.path().join("registry.db");
    let project = project(tmp.path(), "native-preview");

    let output = run_gate(&registry, &["--dry-run", project.to_str().unwrap()], None);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let document = gate_json(&output);
    assert_eq!(document["dry_run"], true);
    assert_eq!(document["gate"]["persisted"], false);
    assert_eq!(document["gate"]["journaled"], false);
    assert!(document["gate"]["rehearsal"].is_object());
    assert!(!project.join(".forge").exists());
    assert!(gate_journal(&registry, "native-preview").is_empty());
}

#[test]
fn legacy_runtime_declarations_and_overrides_do_not_replace_forge() {
    let tmp = tempfile::tempdir().unwrap();
    let registry = tmp.path().join("registry.db");
    let project = project(tmp.path(), "native-legacy-runtime");
    fs::write(
        project.join(".project.json"),
        r#"{"schema_version":1,"id":"native-legacy-runtime","kind":"product","verification":{"command":"true","gate_runtime":"jenkins-gate"}}"#,
    )
    .unwrap();

    let output = run_gate(
        &registry,
        &[project.to_str().unwrap()],
        Some("/missing/legacy-gate"),
    );
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let document = gate_json(&output);
    assert_eq!(document["gate"]["runtime"], "forge");
    assert_eq!(document["gate"]["aggregate"], "passed");
    assert!(evidence_path(&project, "native-legacy-runtime").is_file());
}

#[test]
fn status_transitions_from_absent_to_fresh_then_stale() {
    let tmp = tempfile::tempdir().unwrap();
    let registry = tmp.path().join("registry.db");
    let project = project(tmp.path(), "native-freshness");

    let absent = run_gate(&registry, &["status", project.to_str().unwrap()], None);
    assert_eq!(absent.status.code(), Some(1));
    let absent_doc = gate_json(&absent);
    assert_eq!(absent_doc["gate"]["freshness"], "absent");
    assert!(gate_journal(&registry, "native-freshness").is_empty());

    let run = run_gate(&registry, &[project.to_str().unwrap()], None);
    assert!(run.status.success());
    let fresh = run_gate(&registry, &["status", project.to_str().unwrap()], None);
    assert!(fresh.status.success());
    assert_eq!(gate_json(&fresh)["gate"]["freshness"], "fresh");

    fs::write(project.join("README.md"), "changed\n").unwrap();
    git(&project, &["add", "-A"]);
    git(&project, &["commit", "-q", "-m", "second"]);
    let stale = run_gate(&registry, &["status", project.to_str().unwrap()], None);
    assert_eq!(stale.status.code(), Some(1));
    let stale_doc = gate_json(&stale);
    assert_eq!(stale_doc["gate"]["freshness"], "stale");
    assert_eq!(stale_doc["gate"]["aggregate"], "passed");
}

#[test]
fn invalid_timeout_and_unknown_target_refuse_before_journaling() {
    let tmp = tempfile::tempdir().unwrap();
    let registry = tmp.path().join("registry.db");
    let project = project(tmp.path(), "native-refusal");

    let timeout = run_gate(
        &registry,
        &["--timeout-secs", "0", project.to_str().unwrap()],
        None,
    );
    assert_eq!(timeout.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&timeout.stderr).contains("gate-invalid"));
    let unknown = run_gate(&registry, &["not-registered-native-target"], None);
    assert_eq!(unknown.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&unknown.stderr).contains("unknown-project"));
    assert!(gate_journal(&registry, "native-refusal").is_empty());
}
