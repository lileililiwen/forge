//! CLI contract for `deploy-plan-verdict-truth`.
//!
//! One rule names what a deploy run did: a plan the shared delivery rule
//! allows, a plan it blocks, a deploy that applied the artifact, a deploy
//! that applied nothing and a deploy whose stage did not complete. A plan
//! deploys nothing, so it is never recorded as a completed deployment, and
//! every journal row of the deploy lane names the run in its detail.
//!
//! - a ready plan journals `planned` and names the run;
//! - a blocked plan stays `blocked`;
//! - a delivered deploy still journals `done`;
//! - a rehearsal still journals `planned`.
//!
//! The journal row is read from the registry's own SQLite database, the
//! way the other deploy contract tests read it.

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

fn run(db: &Path, args: &[&str], adapter: &Path) -> std::process::Output {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    for a in args {
        cmd.arg(a);
    }
    cmd.env("FORGE_DEPLOYER_BIN", adapter);
    cmd.output().expect("run forge")
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
        "git {args:?} failed: {}",
        lossy(&out.stderr)
    );
}

/// A project with one local deploy target and a committed artifact.
fn write_deploy_project(dir: &Path, id: &str) {
    fs::create_dir_all(dir).unwrap();
    let text = format!(
        "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\ndeployment:\n  artifact: docker-compose.yml\n  default: home\n  targets:\n    - name: home\n      kind: local\n"
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
    run_git(
        dir,
        &["add", "--", "forge.yaml", "README.md", "docker-compose.yml"],
    );
    run_git(dir, &["commit", "-q", "-m", "initial"]);
}

/// The project's committed HEAD.
fn head_revision(dir: &Path) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .arg("rev-parse")
        .arg("HEAD")
        .output()
        .expect("git rev-parse");
    assert!(out.status.success(), "git rev-parse failed");
    lossy(&out.stdout).trim().to_string()
}

/// The Gate evidence that satisfies the shared delivery rule.
fn write_gate_evidence(dir: &Path, id: &str, revision: &str) {
    let evidence = serde_json::json!({
        "contract": forge::gate::GATE_CONTRACT_VERSION,
        "project_id": id,
        "runtime": "forge",
        "profile": "full",
        "revision": revision,
        "aggregate": "passed",
        "checks": [],
        "observed_at": "2026-10-24T00:00:00Z",
        "dry_run": false
    });
    let path = dir.join(".forge/gate").join(id).join("evidence.json");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, serde_json::to_vec(&evidence).unwrap()).unwrap();
}

/// The deploy project with the Gate evidence that satisfies the shared
/// delivery rule, so the plan is ready.
fn write_gated_deploy_project(dir: &Path, id: &str) {
    write_deploy_project(dir, id);
    write_gate_evidence(dir, id, &head_revision(dir));
}

fn write_adapter_script(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, body).unwrap();
    let mut perms = fs::metadata(&path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&path, perms).unwrap();
    path
}

/// A stand-in deploy adapter that acknowledges the apply and reports a
/// `running` observation.
const ADAPTER_OK_BODY: &str = r#"#!/bin/sh
cat >/dev/null
printf '%s' '{"contract":"forge-deploy-executor/0.1.0","apply_status":"delivered","apply_note":"fixture adapter applied the artifact","apply_evidence":["compose up: app-0.1.0"],"observation_status":"running","observation_detail":"docker service app is running","observation_evidence":["docker ps: app healthy"],"recovery":[]}'
"#;

/// The `deploy` journal rows, oldest first.
fn deploy_journal(db: &Path) -> Vec<(String, String, String)> {
    let conn = rusqlite::Connection::open(db).expect("open db");
    let mut stmt = conn
        .prepare(
            "SELECT project_id, state, detail FROM operations WHERE kind = 'deploy' ORDER BY op_id",
        )
        .expect("prepare");
    let mut rows = stmt.query([]).expect("query");
    let mut out = Vec::new();
    while let Some(row) = rows.next().expect("row") {
        out.push((
            row.get::<_, String>(0).expect("project_id"),
            row.get::<_, String>(1).expect("state"),
            row.get::<_, String>(2).expect("detail"),
        ));
    }
    out
}

#[test]
fn a_ready_plan_is_planned_and_journals_a_planned_row() {
    // A plan deploys nothing: no adapter runs, no deploy state is
    // written. The journal row must not claim a completed deployment.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_gated_deploy_project(&proj, "dep-plan");
    let adapter = write_adapter_script(tmp.path(), "fake-deployer.sh", ADAPTER_OK_BODY);

    let out = run(&db, &["deploy", "plan", proj.to_str().unwrap()], &adapter);
    assert_eq!(
        out.status.code(),
        Some(0),
        "a ready plan exits 0; stderr={}",
        lossy(&out.stderr)
    );
    let stdout = lossy(&out.stdout);
    assert!(
        stdout.contains("deploy plan ready"),
        "the summary says what the run did: {stdout}"
    );

    let rows = deploy_journal(&db);
    assert!(!rows.is_empty(), "the plan wrote no journal row");
    let (project_id, state, detail) = rows.last().expect("journal row");
    assert_eq!(project_id, "dep-plan");
    assert_eq!(
        state, "planned",
        "a plan is not a completed deployment; detail={detail}"
    );
    assert!(
        detail.contains("verdict=planned"),
        "the detail names the run: {detail}"
    );
    assert!(
        detail.contains("ready=true"),
        "the detail keeps the readiness: {detail}"
    );

    assert!(
        !proj.join(".forge/deploy").exists(),
        "a plan must not persist deploy state"
    );
}

#[test]
fn a_blocked_plan_stays_blocked() {
    // Without the Gate evidence the shared delivery rule refuses the
    // revision, so the plan is blocked and the row says so.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_deploy_project(&proj, "dep-plan-blocked");
    let adapter = write_adapter_script(tmp.path(), "fake-deployer.sh", ADAPTER_OK_BODY);

    let out = run(&db, &["deploy", "plan", proj.to_str().unwrap()], &adapter);
    assert_eq!(
        out.status.code(),
        Some(0),
        "a blocked plan still reports its plan; stderr={}",
        lossy(&out.stderr)
    );
    let stdout = lossy(&out.stdout);
    assert!(
        stdout.contains("deploy plan not ready"),
        "the summary names the refusal: {stdout}"
    );

    let rows = deploy_journal(&db);
    let (_, state, detail) = rows.last().expect("journal row");
    assert_eq!(state, "blocked", "detail={detail}");
    assert!(
        detail.contains("verdict=blocked"),
        "the detail names the run: {detail}"
    );
    assert!(
        detail.contains("ready=false"),
        "the detail keeps the readiness: {detail}"
    );
}

#[test]
fn a_delivered_deploy_still_journals_a_done_row() {
    // The applied deploy is unchanged: it still journals `done`, and the
    // detail names the run as well.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_gated_deploy_project(&proj, "dep-delivered");
    let adapter = write_adapter_script(tmp.path(), "fake-deployer.sh", ADAPTER_OK_BODY);

    let out = run(
        &db,
        &["deploy", "apply", proj.to_str().unwrap(), "--confirm"],
        &adapter,
    );
    assert_eq!(
        out.status.code(),
        Some(0),
        "a delivered deploy exits 0; stderr={}",
        lossy(&out.stderr)
    );

    let rows = deploy_journal(&db);
    let (_, state, detail) = rows.last().expect("journal row");
    assert_eq!(state, "done", "detail={detail}");
    assert!(
        detail.contains("verdict=delivered"),
        "the detail names the run: {detail}"
    );
}

#[test]
fn a_rehearsal_still_journals_a_planned_row() {
    // A rehearsal deploys nothing, so it is a planned run too.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_gated_deploy_project(&proj, "dep-rehearsal");
    let adapter = write_adapter_script(tmp.path(), "fake-deployer.sh", ADAPTER_OK_BODY);

    let out = run(
        &db,
        &[
            "deploy",
            "apply",
            proj.to_str().unwrap(),
            "--confirm",
            "--dry-run",
        ],
        &adapter,
    );
    assert_eq!(
        out.status.code(),
        Some(0),
        "a rehearsal exits 0; stderr={}",
        lossy(&out.stderr)
    );

    let rows = deploy_journal(&db);
    let (_, state, detail) = rows.last().expect("journal row");
    assert_eq!(state, "planned", "detail={detail}");
    assert!(
        detail.contains("verdict=planned"),
        "the detail names the run: {detail}"
    );
}
