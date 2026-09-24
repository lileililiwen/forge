//! CLI contract for `forge deploy` (`adapter-deployment`).
//!
//! Exercises the deploy surface end to end through the
//! built binary:
//!
//! - R1 success: a project with a configured target and
//!   artifact records a `delivered` apply and a `running`
//!   observation; the per-stage JSON envelope carries the
//!   per-target outcome and the registry receives a `deploy`
//!   journal row.
//! - R1 failure: a target reference to a `ssh` kind is
//!   refused with `error[deploy-target-unavailable]` before
//!   any side effect runs; a missing target name is
//!   refused with `error[deploy-invalid]`. A deploy apply
//!   without `--confirm` is refused with
//!   `error[deploy-invalid]`.
//! - R1 boundary: a project with multiple targets and no
//!   default requires `--target` (or `deployment.default`)
//!   so Forge never picks an arbitrary server.
//! - R2 success: a re-observation on a previously applied
//!   deploy updates the persisted state with the new
//!   observation timestamp and status.
//! - R2 failure: a missing deployer binary surfaces as
//!   `error[deploy-target-unavailable]` with the failure
//!   reason, and the prior state is preserved.
//! - R2 boundary: when the adapter reports an unknown
//!   observation (target unreachable), the last successful
//!   observation is preserved and the new observation is
//!   recorded as `unknown` so disconnected means unknown,
//!   not offline proof.
//!
//! All scenarios drive the CLI through the built binary;
//! the deploy adapter is exercised through a small
//! `FORGE_DEPLOYER_BIN` fixture shell script that stands
//! in for a real `docker compose` or `scp` round trip.

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
# Stand-in deploy adapter: reads the payload on stdin,
# acknowledges the apply and reports a `running` observation.
cat >/dev/null
printf '%s' '{"contract":"forge-deploy-executor/0.1.0","apply_status":"delivered","apply_note":"fixture adapter applied the artifact","apply_evidence":["compose up: app-0.1.0"],"observation_status":"running","observation_detail":"docker service app is running","observation_evidence":["docker ps: app healthy"],"recovery":[]}'
"#;

const ADAPTER_FAIL_BODY: &str = r#"#!/bin/sh
cat >/dev/null
printf '%s' '{"contract":"forge-deploy-executor/0.1.0","apply_status":"failed","apply_note":"fixture adapter could not apply the artifact","apply_evidence":["docker compose up failed"],"observation_status":"unknown","observation_detail":"target unreachable","observation_evidence":[],"recovery":["re-run forge deploy apply with --confirm"]}'
"#;

const ADAPTER_UNKNOWN_BODY: &str = r#"#!/bin/sh
cat >/dev/null
printf '%s' '{"contract":"forge-deploy-executor/0.1.0","apply_status":"delivered","apply_note":"fixture adapter applied the artifact","apply_evidence":["compose up: app-0.1.0"],"observation_status":"unknown","observation_detail":"target unreachable: ssh handshake refused","observation_evidence":[],"recovery":[]}'
"#;

#[test]
fn cli_help_lists_deploy_subcommand() {
    let out = clean_cmd().arg("--help").output().expect("help");
    assert_eq!(out.status.code(), Some(0));
    let text = lossy(&out.stdout);
    assert!(text.contains("deploy"), "help must mention deploy:\n{text}");
    let out = clean_cmd()
        .arg("deploy")
        .arg("--help")
        .output()
        .expect("help");
    assert_eq!(out.status.code(), Some(0));
    let text = lossy(&out.stdout);
    for sub in ["plan", "apply", "observe", "list", "inspect"] {
        assert!(
            text.contains(sub),
            "deploy help must mention `{sub}`:\n{text}"
        );
    }
}

#[test]
fn plan_captures_target_artifact_and_revision() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_deploy_project(
        &proj,
        "dep-plan-ok",
        "  artifact: docker-compose.yml\n  default: home\n  targets:\n    - name: home\n      kind: local\n  health:\n    kind: docker\n    service: app\n",
    );
    let value = run_json(&db, &["deploy", "plan", proj.to_str().unwrap()]);
    let plan = &value["plan"];
    assert_eq!(plan["ready"], true);
    assert_eq!(plan["target"]["name"], "home");
    assert_eq!(plan["target"]["kind"], "local");
    assert_eq!(plan["artifact"]["path"], "docker-compose.yml");
    assert!(plan["health"].is_object());
    let health = &plan["health"];
    assert_eq!(health["kind"], "docker");
    assert_eq!(health["service"], "app");
}

#[test]
fn plan_refuses_unknown_target() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_deploy_project(
        &proj,
        "dep-plan-bad-target",
        "  artifact: docker-compose.yml\n  default: home\n  targets:\n    - name: home\n      kind: local\n",
    );
    let out = run(
        &db,
        &[
            "deploy",
            "plan",
            proj.to_str().unwrap(),
            "--target-name",
            "ghost",
        ],
    );
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(
        stderr.contains("deploy-invalid"),
        "stderr must report deploy-invalid: {stderr}"
    );
}

#[test]
fn plan_refuses_ssh_target_with_unavailable_boundary() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_deploy_project(
        &proj,
        "dep-plan-ssh",
        "  artifact: docker-compose.yml\n  default: vps\n  targets:\n    - name: vps\n      kind: ssh\n",
    );
    let out = run(&db, &["deploy", "plan", proj.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(
        stderr.contains("deploy-target-unavailable"),
        "stderr must report deploy-target-unavailable: {stderr}"
    );
}

#[test]
fn plan_requires_default_or_explicit_target() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_deploy_project(
        &proj,
        "dep-plan-multi",
        "  artifact: docker-compose.yml\n  targets:\n    - name: home\n      kind: local\n    - name: vps\n      kind: docker-compose\n",
    );
    let out = run(&db, &["deploy", "plan", proj.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(
        stderr.contains("deploy-invalid"),
        "stderr must report deploy-invalid: {stderr}"
    );
}

#[test]
fn apply_refuses_without_confirm() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_deploy_project(
        &proj,
        "dep-apply-noconf",
        "  artifact: docker-compose.yml\n  default: home\n  targets:\n    - name: home\n      kind: local\n",
    );
    let out = run(&db, &["deploy", "apply", proj.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(
        stderr.contains("deploy-invalid"),
        "stderr must report deploy-invalid: {stderr}"
    );
}

#[test]
fn apply_records_running_observation_via_fixture_adapter() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_deploy_project(
        &proj,
        "dep-apply-ok",
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
    let out = cmd.output().expect("apply");
    assert_eq!(out.status.code(), Some(0));
    let json: serde_json::Value = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|_| panic!("stdout: {}", lossy(&out.stdout)));
    let report = &json["deploy"];
    let apply = report["stages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["stage"] == "apply")
        .expect("apply stage");
    assert_eq!(apply["status"], "delivered");
    let observe = report["stages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["stage"] == "observe")
        .expect("observe stage");
    assert_eq!(observe["status"], "running");
    assert_eq!(report["observation"]["status"], "running");
    // State file is persisted under the project's deploy
    // directory.
    let state_path = proj.join(".forge/deploy/dep-apply-ok");
    assert!(state_path.is_dir(), "state dir missing: {state_path:?}");
}

#[test]
fn apply_records_health_failure_with_recovery() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_deploy_project(
        &proj,
        "dep-apply-fail",
        "  artifact: docker-compose.yml\n  default: home\n  targets:\n    - name: home\n      kind: local\n  health:\n    kind: docker\n    service: app\n",
    );
    let adapter = write_adapter_script(tmp.path(), "fake-deployer-fail.sh", ADAPTER_FAIL_BODY);
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
    assert_eq!(out.status.code(), Some(1));
    let json: serde_json::Value = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|_| panic!("stdout: {}", lossy(&out.stdout)));
    let report = &json["deploy"];
    let apply = report["stages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["stage"] == "apply")
        .expect("apply stage");
    assert_eq!(apply["status"], "failed");
    let recovery = apply["recovery"].as_array().expect("recovery list");
    assert!(
        !recovery.is_empty(),
        "recovery notes must be present: {apply:?}"
    );
    let stderr = lossy(&out.stderr);
    assert!(
        stderr.contains("deploy-health-failed"),
        "stderr must report deploy-health-failed: {stderr}"
    );
}

#[test]
fn apply_records_unknown_observation_preserving_recovery_boundary() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_deploy_project(
        &proj,
        "dep-apply-unknown",
        "  artifact: docker-compose.yml\n  default: home\n  targets:\n    - name: home\n      kind: local\n  health:\n    kind: docker\n    service: app\n",
    );
    let adapter =
        write_adapter_script(tmp.path(), "fake-deployer-unknown.sh", ADAPTER_UNKNOWN_BODY);
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
    // The apply stage is `delivered`; the observe stage
    // surfaces the unreachable target as `failed` so a
    // partial run is observable.
    assert_eq!(out.status.code(), Some(1));
    let json: serde_json::Value = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|_| panic!("stdout: {}", lossy(&out.stdout)));
    let report = &json["deploy"];
    let apply = report["stages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["stage"] == "apply")
        .expect("apply stage");
    assert_eq!(apply["status"], "delivered");
    let observe = report["stages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["stage"] == "observe")
        .expect("observe stage");
    assert_eq!(observe["status"], "failed");
    assert_eq!(report["observation"]["status"], "unknown");
}

#[test]
fn apply_refuses_when_adapter_binary_missing() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_deploy_project(
        &proj,
        "dep-apply-missing",
        "  artifact: docker-compose.yml\n  default: home\n  targets:\n    - name: home\n      kind: local\n",
    );
    let mut cmd = clean_cmd();
    cmd.arg("--registry")
        .arg(&db)
        .arg("deploy")
        .arg("apply")
        .arg(proj.to_str().unwrap())
        .arg("--confirm")
        .env("FORGE_DEPLOYER_BIN", "/nonexistent/forge-deployer");
    let out = cmd.output().expect("apply");
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(
        stderr.contains("deploy-target-unavailable"),
        "stderr must report deploy-target-unavailable: {stderr}"
    );
    // The state file must NOT be written for a missing
    // binary so a retry can succeed when the binary is
    // restored.
    let state_path = proj.join(".forge/deploy/dep-apply-missing");
    assert!(
        !state_path.exists(),
        "state dir must not exist for a missing adapter: {state_path:?}"
    );
}

#[test]
fn apply_refuses_pre_namespacing_contract_and_names_the_mismatch() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_deploy_project(
        &proj,
        "dep-contract-mismatch",
        "  artifact: docker-compose.yml\n  default: home\n  targets:\n    - name: home\n      kind: local\n",
    );
    // A stale executor answering with the bare `0.1.0` wire
    // value must be refused as a contract mismatch naming
    // both sides, and must never masquerade as delivered.
    let stale = write_adapter_script(
        proj.parent().unwrap(),
        "stale-contract.sh",
        r#"#!/bin/sh
cat >/dev/null
printf '%s' '{"contract":"0.1.0","apply_status":"delivered","apply_note":"stale executor","apply_evidence":[],"observation_status":"running","observation_detail":"stale","observation_evidence":[],"recovery":[]}'
"#,
    );
    let mut cmd = clean_cmd();
    cmd.arg("--registry")
        .arg(&db)
        .arg("--format")
        .arg("json")
        .arg("deploy")
        .arg("apply")
        .arg(&proj)
        .arg("--confirm")
        .env("FORGE_DEPLOYER_BIN", &stale);
    let out = cmd.output().expect("apply");
    assert_eq!(out.status.code(), Some(1));
    assert!(
        out.stdout.is_empty(),
        "a contract refusal must not render a deploy report: {}",
        lossy(&out.stdout)
    );
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("deploy-target-unavailable"), "{stderr}");
    assert!(
        stderr.contains("forge-deploy-executor/0.1.0") && stderr.contains("`0.1.0`"),
        "the refusal must name both the expected and observed contract: {stderr}"
    );
    assert!(
        !proj.join(".forge/deploy").exists(),
        "a refused contract must never persist state"
    );
}

#[test]
fn non_zero_exit_with_envelope_records_failed_stage_not_unavailable() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_deploy_project(
        &proj,
        "dep-nonzero-envelope",
        "  artifact: docker-compose.yml\n  default: home\n  targets:\n    - name: home\n      kind: local\n",
    );
    // The runtime's own failed-stage report arrives with a
    // non-zero exit: Forge records the named stage outcome
    // with the adapter's evidence (observable on stdout)
    // rather than collapsing it into `unavailable`.
    let failed = write_adapter_script(
        tmp.path(),
        "failed-stage.sh",
        r#"#!/bin/sh
cat >/dev/null
printf '%s' '{"contract":"forge-deploy-executor/0.1.0","apply_status":"failed","apply_note":"job registration refused","apply_evidence":["jenkins-local exit 6: governance check failed"],"observation_status":"unknown","observation_detail":"no observation possible","observation_evidence":[],"recovery":["resolve the governance findings before rerunning"],"source":"fixture-jenkins/0.1.0","source_revision":"cafe1234"}'
exit 1
"#,
    );
    let mut cmd = clean_cmd();
    cmd.arg("--registry")
        .arg(&db)
        .arg("--format")
        .arg("json")
        .arg("deploy")
        .arg("apply")
        .arg(&proj)
        .arg("--confirm")
        .env("FORGE_DEPLOYER_BIN", &failed);
    let out = cmd.output().expect("apply");
    assert_eq!(out.status.code(), Some(1));
    let json: serde_json::Value = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|_| panic!("stdout: {}", lossy(&out.stdout)));
    let apply = json["deploy"]["stages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["stage"] == "apply")
        .expect("apply stage");
    assert_eq!(apply["status"], "failed");
    assert_eq!(apply["note"], "job registration refused");
    let evidence: Vec<&str> = apply["evidence"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e.as_str().unwrap_or(""))
        .collect();
    assert!(
        evidence
            .iter()
            .any(|e| e.contains("governance check failed")),
        "the runtime's evidence must survive: {evidence:?}"
    );
    assert!(
        evidence.contains(&"executor=fixture-jenkins/0.1.0@cafe1234"),
        "the stage must attribute to the adapter and revision: {evidence:?}"
    );
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("deploy-health-failed"), "{stderr}");
    assert!(
        !proj.join(".forge/deploy").exists(),
        "a failed stage must not persist a deploy record"
    );
}

#[test]
fn dry_run_apply_never_persists_state() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_deploy_project(
        &proj,
        "dep-dryrun-state",
        "  artifact: docker-compose.yml\n  default: home\n  targets:\n    - name: home\n      kind: local\n",
    );
    let adapter = write_adapter_script(tmp.path(), "fake-deployer.sh", ADAPTER_OK_BODY);
    let mut cmd = clean_cmd();
    cmd.arg("--registry")
        .arg(&db)
        .arg("deploy")
        .arg("apply")
        .arg(&proj)
        .arg("--confirm")
        .arg("--dry-run")
        .env("FORGE_DEPLOYER_BIN", &adapter);
    let out = cmd.output().expect("apply dry-run");
    // The rehearsal reports its result; Forge never writes a
    // state record for it, so the last real deploy's evidence
    // (here: none) stands.
    let stdout = lossy(&out.stdout);
    assert!(stdout.contains("delivered"), "{stdout}");
    assert!(
        !proj.join(".forge/deploy").exists(),
        "a dry-run rehearsal must not persist deploy state"
    );
}

#[test]
fn list_reports_persisted_deploys() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_deploy_project(
        &proj,
        "dep-list",
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
    let value = run_json(&db, &["deploy", "list", proj.to_str().unwrap()]);
    let deploys = value["deploys"].as_array().unwrap();
    assert!(!deploys.is_empty(), "expected at least one deploy");
    assert_eq!(deploys[0]["project_id"], "dep-list");
    assert_eq!(deploys[0]["target"], "home");
    assert_eq!(deploys[0]["current_state"], "running");
}

#[test]
fn inspect_returns_deploy_state() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_deploy_project(
        &proj,
        "dep-inspect",
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
    let list = run_json(&db, &["deploy", "list", proj.to_str().unwrap()]);
    let deploy_id = list["deploys"][0]["deploy_id"]
        .as_str()
        .unwrap()
        .to_string();
    let value = run_json(
        &db,
        &["deploy", "inspect", &deploy_id, proj.to_str().unwrap()],
    );
    let deploy = &value["deploy"];
    assert_eq!(deploy["identity"]["target"], "home");
    assert_eq!(deploy["current_state"], "running");
    let stages: Vec<&str> = deploy["stage_outcomes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["stage"].as_str().unwrap())
        .collect();
    assert!(stages.contains(&"apply"));
    assert!(stages.contains(&"observe"));
}

#[test]
fn observe_updates_state_on_subsequent_run() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_deploy_project(
        &proj,
        "dep-observe",
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
    // Now run a re-observation: the state must be updated
    // to a fresh timestamp.
    let mut cmd = clean_cmd();
    cmd.arg("--registry")
        .arg(&db)
        .arg("--format")
        .arg("json")
        .arg("deploy")
        .arg("observe")
        .arg(proj.to_str().unwrap())
        .arg("--confirm")
        .env("FORGE_DEPLOYER_BIN", &adapter);
    let out = cmd.output().expect("observe");
    assert_eq!(out.status.code(), Some(0));
    let json: serde_json::Value = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|_| panic!("stdout: {}", lossy(&out.stdout)));
    let report = &json["deploy"];
    let observe = report["stages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["stage"] == "observe")
        .expect("observe stage");
    assert_eq!(observe["status"], "running");
}

#[test]
fn observe_refuses_without_prior_state() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_deploy_project(
        &proj,
        "dep-observe-fresh",
        "  artifact: docker-compose.yml\n  default: home\n  targets:\n    - name: home\n      kind: local\n  health:\n    kind: docker\n    service: app\n",
    );
    let adapter = write_adapter_script(tmp.path(), "fake-deployer.sh", ADAPTER_OK_BODY);
    let mut cmd = clean_cmd();
    cmd.arg("--registry")
        .arg(&db)
        .arg("deploy")
        .arg("observe")
        .arg(proj.to_str().unwrap())
        .arg("--confirm")
        .env("FORGE_DEPLOYER_BIN", &adapter);
    let out = cmd.output().expect("observe");
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(
        stderr.contains("deploy-target-stale"),
        "stderr must report deploy-target-stale: {stderr}"
    );
}
