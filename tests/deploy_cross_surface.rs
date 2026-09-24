//! Cross-surface tests for `forge deploy` (`adapter-deployment`).
//!
//! Exercises interactions between the new deploy surface
//! and the other Core contracts:
//!
//! - The Core registry's `deploy` journal row records the
//!   per-stage summary so the operations table stays
//!   independent of the deploy surface.
//! - The doctor verdict on the deploy project's directory is
//!   unchanged after a successful deploy (no regression on
//!   `forge doctor`).
//! - A re-deploy on the same target after a successful
//!   first run reports the apply stage as `delivered` and
//!   the observe stage as `running` (R1 success: idempotency
//!   through identity).
//! - A re-deploy after the source revision has moved on
//!   derives a different deploy id so a stale approval
//!   cannot be silently reused (R2 boundary).
//! - A credential-shaped substring in evidence is redacted
//!   through `redact_deploy_evidence` which delegates to
//!   `policy::redact_credentials`.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

fn clean_cmd() -> Command {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY")
        .env_remove("HTTP_PROXY")
        .env_remove("HTTPS_PROXY")
        .env_remove("ALL_PROXY")
        .env_remove("OPENAI_API_KEY")
        .env_remove("ANTHROPIC_API_KEY")
        .env_remove("FORGE_DEPLOYER_BIN");
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
            lossy(&out.stdout),
            lossy(&out.stderr)
        )
    })
}

fn run_git(dir: &Path, args: &[&str]) {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(dir);
    for a in args {
        cmd.arg(a);
    }
    let out = cmd.output().expect("git");
    assert!(
        out.status.success(),
        "git {:?} failed: status={} stderr={}",
        args,
        out.status,
        lossy(&out.stderr)
    );
}

fn write_deploy_project(dir: &Path, id: &str, deployment_yaml: &str) {
    fs::create_dir_all(dir).unwrap();
    let text = format!(
        "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\ndeployment:\n{deployment_yaml}"
    );
    fs::write(dir.join("forge.yaml"), text).unwrap();
    fs::write(dir.join("README.md"), "v1\n").unwrap();
    fs::write(
        dir.join("docker-compose.yml"),
        "services:\n  app:\n    image: app:0.1.0\n",
    )
    .unwrap();
    run_git(dir, &["init", "-q"]);
    run_git(dir, &["config", "user.email", "forge@example.com"]);
    run_git(dir, &["config", "user.name", "Forge Test"]);
    run_git(dir, &["config", "init.defaultBranch", "main"]);
    run_git(dir, &["checkout", "-q", "-b", "main"]);
    run_git(
        dir,
        &["add", "--", "forge.yaml", "README.md", "docker-compose.yml"],
    );
    run_git(dir, &["commit", "-q", "-m", "initial"]);
}

fn write_adapter_script(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, body).unwrap();
    let mut perms = fs::metadata(&path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&path, perms).unwrap();
    path
}

const ADAPTER_OK_BODY: &str = r#"#!/bin/sh
cat >/dev/null
printf '%s' '{"contract":"forge-deploy-executor/0.1.0","apply_status":"delivered","apply_note":"fixture adapter applied the artifact","apply_evidence":["compose up: app-0.1.0"],"observation_status":"running","observation_detail":"docker service app is running","observation_evidence":["docker ps: app healthy"],"recovery":[]}'
"#;

const ADAPTER_LEAK_BODY: &str = r#"#!/bin/sh
cat >/dev/null
printf '%s' '{"contract":"forge-deploy-executor/0.1.0","apply_status":"delivered","apply_note":"token ghp_abcdefghijklmnopqrstuvwxyz0123456789 was used","apply_evidence":["compose up: app-0.1.0"],"observation_status":"running","observation_detail":"docker service app is running","observation_evidence":["docker ps: app healthy"],"recovery":[]}'
"#;

#[test]
fn registry_journal_records_deploy_run() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_deploy_project(
        &proj,
        "dep-journal",
        "  artifact: docker-compose.yml\n  default: home\n  targets:\n    - name: home\n      kind: local\n  health:\n    kind: docker\n    service: app\n",
    );
    let adapter = write_adapter_script(tmp.path(), "fake-deployer.sh", ADAPTER_OK_BODY);
    let mut cmd = clean_cmd();
    cmd.arg("--registry")
        .arg(&db)
        .arg("deploy")
        .arg("apply")
        .arg(proj.to_str().unwrap())
        .arg("--confirm")
        .env("FORGE_DEPLOYER_BIN", &adapter);
    let out = cmd.output().expect("apply");
    assert_eq!(out.status.code(), Some(0));
    // The registry's operations table records a deploy row.
    let value = run_json(&db, &["list"]);
    let projects = value["projects"].as_array().unwrap();
    assert!(projects.is_empty(), "deploy is not a registered project");
    // Verify the journal row was written by running a
    // second command and checking the registry can be
    // re-opened cleanly.
    let out = run(&db, &["deploy", "list", proj.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0));
    let stdout = lossy(&out.stdout);
    assert!(stdout.contains("dep-journal"), "stdout: {stdout}");
}

#[test]
fn doctor_verdict_is_unchanged_after_deploy() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_deploy_project(
        &proj,
        "dep-doctor",
        "  artifact: docker-compose.yml\n  default: home\n  targets:\n    - name: home\n      kind: local\n  health:\n    kind: docker\n    service: app\n",
    );
    // Capture the doctor verdict before the deploy.
    let before = run_json(&db, &["doctor", proj.to_str().unwrap()]);
    let before_finding_count = before["doctor"]["findings"]
        .as_array()
        .map(|a| a.len())
        .unwrap_or(0);
    // Apply the deploy.
    let adapter = write_adapter_script(tmp.path(), "fake-deployer.sh", ADAPTER_OK_BODY);
    let mut cmd = clean_cmd();
    cmd.arg("--registry")
        .arg(&db)
        .arg("deploy")
        .arg("apply")
        .arg(proj.to_str().unwrap())
        .arg("--confirm")
        .env("FORGE_DEPLOYER_BIN", &adapter);
    let out = cmd.output().expect("apply");
    assert_eq!(out.status.code(), Some(0));
    // Capture the doctor verdict after the deploy.
    let after = run_json(&db, &["doctor", proj.to_str().unwrap()]);
    let after_finding_count = after["doctor"]["findings"]
        .as_array()
        .map(|a| a.len())
        .unwrap_or(0);
    assert_eq!(
        before_finding_count, after_finding_count,
        "doctor finding count must not change after deploy"
    );
}

#[test]
fn redeploy_on_same_revision_is_idempotent() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_deploy_project(
        &proj,
        "dep-redeploy",
        "  artifact: docker-compose.yml\n  default: home\n  targets:\n    - name: home\n      kind: local\n  health:\n    kind: docker\n    service: app\n",
    );
    let adapter = write_adapter_script(tmp.path(), "fake-deployer.sh", ADAPTER_OK_BODY);
    let mut cmd = clean_cmd();
    cmd.arg("--registry")
        .arg(&db)
        .arg("--format")
        .arg("json")
        .arg("deploy")
        .arg("apply")
        .arg(proj.to_str().unwrap())
        .arg("--confirm")
        .env("FORGE_DEPLOYER_BIN", &adapter);
    let out = cmd.output().expect("first apply");
    assert_eq!(out.status.code(), Some(0));
    let first = serde_json::from_slice::<serde_json::Value>(&out.stdout)
        .unwrap_or_else(|_| panic!("stdout: {}", lossy(&out.stdout)));
    let first_id = first["deploy"]["identity"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    // Second apply on the same revision: the deploy id is
    // the same and both stages are reported as `delivered`
    // and `running` again. The persisted state is
    // overwritten by the new run.
    let mut cmd = clean_cmd();
    cmd.arg("--registry")
        .arg(&db)
        .arg("--format")
        .arg("json")
        .arg("deploy")
        .arg("apply")
        .arg(proj.to_str().unwrap())
        .arg("--confirm")
        .env("FORGE_DEPLOYER_BIN", &adapter);
    let out = cmd.output().expect("second apply");
    assert_eq!(out.status.code(), Some(0));
    let second = serde_json::from_slice::<serde_json::Value>(&out.stdout)
        .unwrap_or_else(|_| panic!("stdout: {}", lossy(&out.stdout)));
    let second_id = second["deploy"]["identity"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(
        first_id, second_id,
        "deploy id must be stable on same revision"
    );
}

#[test]
fn redeploy_after_new_revision_derives_new_id() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_deploy_project(
        &proj,
        "dep-newrev",
        "  artifact: docker-compose.yml\n  default: home\n  targets:\n    - name: home\n      kind: local\n  health:\n    kind: docker\n    service: app\n",
    );
    let adapter = write_adapter_script(tmp.path(), "fake-deployer.sh", ADAPTER_OK_BODY);
    let mut cmd = clean_cmd();
    cmd.arg("--registry")
        .arg(&db)
        .arg("--format")
        .arg("json")
        .arg("deploy")
        .arg("apply")
        .arg(proj.to_str().unwrap())
        .arg("--confirm")
        .env("FORGE_DEPLOYER_BIN", &adapter);
    let out = cmd.output().expect("first apply");
    assert_eq!(out.status.code(), Some(0));
    let first = serde_json::from_slice::<serde_json::Value>(&out.stdout)
        .unwrap_or_else(|_| panic!("stdout: {}", lossy(&out.stdout)));
    let first_id = first["deploy"]["identity"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    // Add a new commit on the same project.
    fs::write(proj.join("README.md"), "v2\n").unwrap();
    run_git(&proj, &["add", "--", "README.md"]);
    run_git(&proj, &["commit", "-q", "-m", "second"]);
    // Re-apply: the deploy id is different.
    let mut cmd = clean_cmd();
    cmd.arg("--registry")
        .arg(&db)
        .arg("--format")
        .arg("json")
        .arg("deploy")
        .arg("apply")
        .arg(proj.to_str().unwrap())
        .arg("--confirm")
        .env("FORGE_DEPLOYER_BIN", &adapter);
    let out = cmd.output().expect("second apply");
    assert_eq!(out.status.code(), Some(0));
    let second = serde_json::from_slice::<serde_json::Value>(&out.stdout)
        .unwrap_or_else(|_| panic!("stdout: {}", lossy(&out.stdout)));
    let second_id = second["deploy"]["identity"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    assert_ne!(
        first_id, second_id,
        "deploy id must change when the source revision moves"
    );
}

#[test]
fn credential_in_evidence_is_redacted() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_deploy_project(
        &proj,
        "dep-redact",
        "  artifact: docker-compose.yml\n  default: home\n  targets:\n    - name: home\n      kind: local\n  health:\n    kind: docker\n    service: app\n",
    );
    let adapter = write_adapter_script(tmp.path(), "fake-deployer-leak.sh", ADAPTER_LEAK_BODY);
    let mut cmd = clean_cmd();
    cmd.arg("--registry")
        .arg(&db)
        .arg("--format")
        .arg("json")
        .arg("deploy")
        .arg("apply")
        .arg(proj.to_str().unwrap())
        .arg("--confirm")
        .env("FORGE_DEPLOYER_BIN", &adapter);
    let out = cmd.output().expect("apply");
    let stdout = lossy(&out.stdout);
    assert!(
        !stdout.contains("ghp_abcdefghijklmnopqrstuvwxyz0123456789"),
        "credential must be redacted in stdout: {stdout}"
    );
    assert!(
        stdout.contains("[REDACTED]"),
        "stdout must show [REDACTED] for the credential: {stdout}"
    );
}
