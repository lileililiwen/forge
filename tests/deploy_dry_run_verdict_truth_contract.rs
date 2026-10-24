//! CLI contract for `deploy-dry-run-verdict-truth`.
//!
//! One rule judges a deploy run: a run that deployed nothing planned,
//! a run that deployed and completed every stage delivered, and a run
//! whose stage did not complete failed. The report carries the
//! verdict, the note says what the run did, the journal row follows
//! the verdict and the exit code follows it too.
//!
//! - a rehearsal is `planned`, is not healthy, says that nothing was
//!   deployed, writes no deploy state, journals `planned` and exits 0;
//! - a completed deploy is `delivered`, is healthy, journals `done` and
//!   exits 0;
//! - a failed deploy is `failed`, is not healthy, journals `failed` and
//!   exits 1 with the report on stdout.

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

fn run_json(db: &Path, args: &[&str], adapter: &Path) -> serde_json::Value {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    cmd.arg("--format").arg("json");
    for a in args {
        cmd.arg(a);
    }
    cmd.env("FORGE_DEPLOYER_BIN", adapter);
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
    run_git(dir, &["checkout", "-q", "-b", "main"]);
    run_git(
        dir,
        &["add", "--", "forge.yaml", "README.md", "docker-compose.yml"],
    );
    run_git(dir, &["commit", "-q", "-m", "initial"]);
}

/// The project's committed HEAD. The shared delivery rule binds the
/// Gate evidence to the revision that is about to be delivered.
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
        "observed_at": "2026-10-18T00:00:00Z",
        "dry_run": false
    });
    let path = dir.join(".forge/gate").join(id).join("evidence.json");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, serde_json::to_vec(&evidence).unwrap()).unwrap();
}

/// The deploy project with the Gate evidence that satisfies the shared
/// delivery rule, so a real deploy is not refused before the adapter
/// runs.
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

/// A stand-in deploy adapter that could not apply the artifact.
const ADAPTER_FAIL_BODY: &str = r#"#!/bin/sh
cat >/dev/null
printf '%s' '{"contract":"forge-deploy-executor/0.1.0","apply_status":"failed","apply_note":"fixture adapter could not apply the artifact","apply_evidence":["docker compose up failed"],"observation_status":"unknown","observation_detail":"target unreachable","observation_evidence":[],"recovery":["re-run forge deploy apply with --confirm"]}'
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
fn a_rehearsal_is_planned_and_journals_a_planned_row() {
    // Nothing is deployed: the adapter rehearsed, so its stages are the
    // adapter's rehearsal output and not evidence of a delivery.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_deploy_project(&proj, "dep-rehearsal");
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
        "a rehearsal is a successful rehearsal; stderr={}",
        lossy(&out.stderr)
    );
    let stdout = lossy(&out.stdout);
    assert!(
        stdout.contains("nothing was deployed"),
        "the note says what the run did: {stdout}"
    );
    assert!(
        !stdout.contains("deploy applied to target"),
        "a rehearsal must not claim it applied: {stdout}"
    );

    let json = run_json(
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
    let report = &json["deploy"];
    assert_eq!(report["verdict"], "planned", "{report}");
    assert_eq!(report["healthy"], false, "{report}");
    assert_eq!(report["dry_run"], true, "{report}");

    let rows = deploy_journal(&db);
    assert!(!rows.is_empty(), "the rehearsal wrote no journal row");
    let (project_id, state, detail) = rows.last().expect("journal row");
    assert_eq!(project_id, "dep-rehearsal");
    assert_eq!(state, "planned");
    assert!(
        detail.contains("verdict=planned"),
        "the detail names the verdict: {detail}"
    );
    assert!(
        detail.contains("healthy=false"),
        "the detail names the health: {detail}"
    );

    assert!(
        !proj.join(".forge/deploy").exists(),
        "a rehearsal must not persist deploy state"
    );
}

#[test]
fn a_completed_deploy_is_delivered_and_journals_a_done_row() {
    // A real deploy writes its state and says so.
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
    let stdout = lossy(&out.stdout);
    assert!(
        stdout.contains("deploy applied to target"),
        "the note says what the run did: {stdout}"
    );

    let json = run_json(
        &db,
        &["deploy", "apply", proj.to_str().unwrap(), "--confirm"],
        &adapter,
    );
    let report = &json["deploy"];
    assert_eq!(report["verdict"], "delivered", "{report}");
    assert_eq!(report["healthy"], true, "{report}");

    let rows = deploy_journal(&db);
    let (_, state, detail) = rows.last().expect("journal row");
    assert_eq!(state, "done");
    assert!(detail.contains("verdict=delivered"), "{detail}");
    assert!(
        proj.join(".forge/deploy").exists(),
        "a real deploy persists its state"
    );
}

#[test]
fn a_failed_deploy_is_failed_and_journals_a_failed_row() {
    // The adapter could not apply the artifact: the run failed, the
    // journal says so and the command exits 1 with the report.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_gated_deploy_project(&proj, "dep-failed");
    let adapter = write_adapter_script(tmp.path(), "fake-deployer.sh", ADAPTER_FAIL_BODY);

    let out = run(
        &db,
        &["deploy", "apply", proj.to_str().unwrap(), "--confirm"],
        &adapter,
    );
    let stderr = lossy(&out.stderr);
    assert_eq!(
        out.status.code(),
        Some(1),
        "a failed deploy exits 1; stderr={stderr}"
    );
    let stdout = lossy(&out.stdout);
    assert!(
        stdout.contains("deploy did not complete cleanly"),
        "the report reaches stdout: {stdout}"
    );

    let rows = deploy_journal(&db);
    let (_, state, detail) = rows.last().expect("journal row");
    assert_eq!(state, "failed");
    assert!(detail.contains("verdict=failed"), "{detail}");
}
