//! CLI contract tests for `forge delivery rollback`.
//!
//! Each test drives the verb against a local provider stub and asserts one
//! observable outcome: a promoted healthy release is restored, an unproven
//! target is refused, a provider that does not declare `rollback` is refused
//! before it runs, and a failed rollback does not claim the release moved.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

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
        .env_remove("FORGE_PUBLISH_PROVIDER_CONFIG")
        .env_remove("FORGE_HERMORA_BIN");
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

fn run_json(db: &Path, args: &[&str]) -> Value {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    cmd.arg("--format").arg("json");
    for a in args {
        cmd.arg(a);
    }
    let out = cmd.output().expect("run forge json");
    assert!(
        out.status.success(),
        "forge failed: status={} stderr={}",
        out.status,
        lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap_or_else(|err| {
        panic!(
            "invalid json: {err}; stdout={} stderr={}",
            lossy(&out.stdout),
            lossy(&out.stderr)
        )
    })
}

fn run_with_fixtures(db: &Path, fixture_path: &Path, args: &[&str]) -> std::process::Output {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    let mut path = std::ffi::OsString::from(fixture_path);
    path.push(":");
    path.push(std::env::var_os("PATH").unwrap_or_default());
    cmd.env("PATH", path);
    for a in args {
        cmd.arg(a);
    }
    cmd.output().expect("run forge with fixtures")
}

fn write_project(dir: &Path, id: &str) {
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(
        dir.join("forge.yaml"),
        format!(
            "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: rust-web\n  maturity: L2\n  target_maturity: L3\nruntime:\n  language: rust\n"
        ),
    )
    .unwrap();
    std::fs::write(dir.join("README.md"), "v1\n").unwrap();
    git(dir, &["init", "-q", "--initial-branch=main"]);
    git(dir, &["add", "."]);
    git(dir, &["commit", "-q", "-m", "seed"]);
}

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args([
            "-c",
            "user.name=Forge",
            "-c",
            "user.email=forge@example.com",
        ])
        .args(args)
        .current_dir(dir)
        .output()
        .expect("git");
    assert!(out.status.success(), "git {args:?}: {}", lossy(&out.stderr));
}

fn write_gate_evidence(proj: &Path, id: &str, revision: &str) {
    let evidence = serde_json::json!({
        "contract": forge::gate::GATE_CONTRACT_VERSION,
        "project_id": id,
        "runtime": "forge",
        "profile": "full",
        "revision": revision,
        "aggregate": "passed",
        "checks": [],
        "observed_at": "2026-10-30T00:00:00Z",
        "dry_run": false
    });
    let path = proj.join(".forge/gate").join(id).join("evidence.json");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, serde_json::to_vec(&evidence).unwrap()).unwrap();
}

fn register_and_gate(db: &Path, proj: &Path, id: &str) -> String {
    let reg = run(db, &["register", proj.to_str().unwrap()]);
    assert!(reg.status.success(), "register: {}", lossy(&reg.stderr));
    let revision = run_json(db, &["inspect", id])["last_commit"]
        .as_str()
        .unwrap()
        .to_string();
    write_gate_evidence(proj, id, &revision);
    revision
}

fn commit_and_register(db: &Path, proj: &Path, id: &str, text: &str) -> String {
    std::fs::write(proj.join("README.md"), text).unwrap();
    git(proj, &["add", "README.md"]);
    git(proj, &["commit", "-q", "-m", "next"]);
    register_and_gate(db, proj, id)
}

/// Which operations the stub provider declares and how it answers
/// `rollback`.
#[derive(Clone, Copy)]
struct Provider {
    declares_rollback: bool,
    rollback_status: &'static str,
    rollback_health: &'static str,
}

const HEALTHY: Provider = Provider {
    declares_rollback: true,
    rollback_status: "succeeded",
    rollback_health: "healthy",
};

fn install_provider(dir: &Path, project_dir: &Path, provider: Provider) -> PathBuf {
    let provider_dir = dir.join("bin");
    std::fs::create_dir_all(&provider_dir).unwrap();
    let script = provider_dir.join("forge-openpanel-stub");
    let capabilities = if provider.declares_rollback {
        r#"["preflight","publish","rollback"]"#
    } else {
        r#"["preflight","publish"]"#
    };
    let body = format!(
        r#"#!/bin/sh
REQ=$(cat)
printf '%s\n' "$REQ" >> "$(dirname "$0")/requests.log"
case "$REQ" in
  *'"operation":"capabilities"'*)
    printf '{{"contract":"forge-publish-provider/0.1.0","provider":"openpanel","operation_id":"caps","status":"done","health":"unknown","capabilities":{capabilities}}}\n'
    ;;
  *'"operation":"rollback"'*)
    printf '{{"contract":"forge-publish-provider/0.1.0","provider":"openpanel","operation_id":"stub-op","status":"{status}","health":"{health}","evidence":["rollback stub"],"recovery":[],"build_status":"succeeded","run_status":"succeeded"}}\n'
    ;;
  *)
    QUEUE=$(printf '%s' "$REQ" | tr ',' '\n' | grep '"queue_id"' | cut -d'"' -f4)
    if [ -z "$QUEUE" ]; then QUEUE=delivery-default; fi
    printf '{{"contract":"forge-publish-provider/0.1.0","provider":"openpanel","operation_id":"stub-op","status":"succeeded","health":"healthy","evidence":["provider stub"],"recovery":[],"build_status":"succeeded","run_status":"succeeded","container_identity":"forge-stub-0123456789ab","queue_id":"%s"}}\n' "$QUEUE"
    ;;
esac
"#,
        status = provider.rollback_status,
        health = provider.rollback_health,
    );
    std::fs::write(&script, body).unwrap();
    let mut perms = std::fs::metadata(&script).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&script, perms).unwrap();
    let config_path = project_dir.join(".forge/providers.yaml");
    std::fs::create_dir_all(config_path.parent().unwrap()).unwrap();
    std::fs::write(
        &config_path,
        "providers:\n  - id: openpanel\n    command: forge-openpanel-stub\n",
    )
    .unwrap();
    provider_dir
}

/// Promote the revision the project is registered at right now.
fn promote_current(db: &Path, fixture: &Path, id: &str, revision: &str) {
    let preflight = run_with_fixtures(db, fixture, &["delivery", "preflight", id]);
    assert!(
        preflight.status.success(),
        "preflight: {}",
        lossy(&preflight.stderr)
    );
    let report = run_json(db, &["delivery", "status", id]);
    let op_id = report["preflight"]["op_id"].as_i64().unwrap().to_string();
    let stage = run_with_fixtures(
        db,
        fixture,
        &["delivery", "stage", id, "--confirm-operation-id", &op_id],
    );
    assert!(stage.status.success(), "stage: {}", lossy(&stage.stderr));
    let promote = run_with_fixtures(
        db,
        fixture,
        &["delivery", "promote", id, "--confirm-revision", revision],
    );
    assert!(
        promote.status.success(),
        "promote: {}",
        lossy(&promote.stderr)
    );
    let report = run_json(db, &["delivery", "status", id]);
    assert_eq!(report["phase"], "healthy");
}

fn journal_has(db: &Path, id: &str, kind: &str) -> bool {
    let registry = forge::registry::Registry::open(db).unwrap();
    registry
        .operations_for_project(id, 200)
        .unwrap()
        .iter()
        .any(|row| row.kind == kind)
}

fn rollback_rows(db: &Path, id: &str) -> Vec<forge::registry::OperationEntry> {
    let registry = forge::registry::Registry::open(db).unwrap();
    registry
        .operations_for_project(id, 200)
        .unwrap()
        .into_iter()
        .filter(|row| row.kind == "delivery.rollback")
        .collect()
}

fn last_request(fixture: &Path) -> Value {
    let text = std::fs::read_to_string(fixture.join("requests.log")).unwrap();
    let line = text.lines().last().expect("a logged request");
    serde_json::from_str(line).unwrap()
}

/// Two promoted revisions, `r1` then `r2`. Returns `(r1, r2)`.
fn two_promoted_revisions(db: &Path, proj: &Path, id: &str, fixture: &Path) -> (String, String) {
    write_project(proj, id);
    let r1 = register_and_gate(db, proj, id);
    promote_current(db, fixture, id, &r1);
    let r2 = commit_and_register(db, proj, id, "v2\n");
    promote_current(db, fixture, id, &r2);
    (r1, r2)
}

#[test]
fn rollback_to_a_promoted_release_restores_it() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    let id = "rollback-restore";
    let fixture = install_provider(&tmp.path().join("fx"), &proj, HEALTHY);
    let (r1, r2) = two_promoted_revisions(&db, &proj, id, &fixture);

    let out = run_with_fixtures(&db, &fixture, &["delivery", "rollback", id, "--to", &r1]);
    assert!(out.status.success(), "rollback: {}", lossy(&out.stderr));
    let report = run_json(&db, &["delivery", "status", id]);
    assert_eq!(report["phase"], "rolled-back");
    assert_eq!(report["rollback"]["state"], "done");
    assert_eq!(report["rollback"]["target_revision"], r1);

    let request = last_request(&fixture);
    assert_eq!(request["operation"], "rollback");
    assert_eq!(request["target_revision"], r1);
    assert_eq!(
        request["queue_id"].as_str().unwrap(),
        format!("delivery-rollback-{id}-{}", &r2[..12])
    );

    let rows = rollback_rows(&db, id);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].state, "done");
}

#[test]
fn rollback_to_an_unpromoted_revision_is_refused_before_the_provider() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    let id = "rollback-unpromoted";
    write_project(&proj, id);
    let revision = register_and_gate(&db, &proj, id);
    let fixture = install_provider(&tmp.path().join("fx"), &proj, HEALTHY);
    promote_current(&db, &fixture, id, &revision);
    let calls_before = std::fs::read_to_string(fixture.join("requests.log"))
        .unwrap()
        .lines()
        .count();

    let stranger = "ffffffffffffffffffffffffffffffffffffffff";
    let out = run_with_fixtures(
        &db,
        &fixture,
        &["delivery", "rollback", id, "--to", stranger],
    );
    assert!(!out.status.success(), "unpromoted target must be refused");
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("delivery-conflict"), "{stderr}");
    assert!(stderr.contains(stranger), "{stderr}");
    assert!(stderr.contains("no healthy promotion"), "{stderr}");
    assert!(
        rollback_rows(&db, id).is_empty(),
        "a refused rollback writes no row"
    );
    let calls_after = std::fs::read_to_string(fixture.join("requests.log"))
        .unwrap()
        .lines()
        .count();
    assert_eq!(calls_before, calls_after, "the provider must not run");
}

#[test]
fn the_registered_revision_is_not_a_rollback_target() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    let id = "rollback-current";
    write_project(&proj, id);
    let revision = register_and_gate(&db, &proj, id);
    let fixture = install_provider(&tmp.path().join("fx"), &proj, HEALTHY);
    promote_current(&db, &fixture, id, &revision);

    let out = run_with_fixtures(
        &db,
        &fixture,
        &["delivery", "rollback", id, "--to", &revision],
    );
    assert!(!out.status.success());
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("delivery-conflict"), "{stderr}");
    assert!(
        stderr.contains("already the registered revision"),
        "{stderr}"
    );
    assert!(rollback_rows(&db, id).is_empty());
}

#[test]
fn rollback_requires_exactly_one_selector() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    let id = "rollback-selector";
    write_project(&proj, id);
    let revision = register_and_gate(&db, &proj, id);
    let fixture = install_provider(&tmp.path().join("fx"), &proj, HEALTHY);
    promote_current(&db, &fixture, id, &revision);

    for args in [
        vec!["delivery", "rollback", id],
        vec![
            "delivery",
            "rollback",
            id,
            "--to",
            revision.as_str(),
            "--previous",
        ],
    ] {
        let out = run_with_fixtures(&db, &fixture, &args);
        assert!(!out.status.success(), "{args:?} must be refused");
        let stderr = lossy(&out.stderr);
        assert!(stderr.contains("delivery-invalid"), "{stderr}");
        assert!(stderr.contains("exactly one"), "{stderr}");
    }
}

#[test]
fn rollback_is_refused_when_the_provider_does_not_declare_it() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    let id = "rollback-undeclared";
    let fixture = install_provider(
        &tmp.path().join("fx"),
        &proj,
        Provider {
            declares_rollback: false,
            ..HEALTHY
        },
    );
    let (r1, _r2) = two_promoted_revisions(&db, &proj, id, &fixture);

    let out = run_with_fixtures(&db, &fixture, &["delivery", "rollback", id, "--to", &r1]);
    assert!(!out.status.success(), "undeclared rollback must be refused");
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("delivery-invalid"), "{stderr}");
    assert!(
        stderr.contains("does not declare the `rollback` capability"),
        "{stderr}"
    );
    assert!(stderr.contains("declared: preflight, publish"), "{stderr}");
    assert!(rollback_rows(&db, id).is_empty());
    assert_eq!(last_request(&fixture)["operation"], "capabilities");
}

#[test]
fn rollback_previous_resolves_the_newest_earlier_release() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    let id = "rollback-previous";
    let fixture = install_provider(&tmp.path().join("fx"), &proj, HEALTHY);
    let (r1, _r2) = two_promoted_revisions(&db, &proj, id, &fixture);

    let out = run_with_fixtures(&db, &fixture, &["delivery", "rollback", id, "--previous"]);
    assert!(out.status.success(), "rollback: {}", lossy(&out.stderr));
    let report = run_json(&db, &["delivery", "status", id]);
    assert_eq!(report["phase"], "rolled-back");
    assert_eq!(report["rollback"]["target_revision"], r1);
    assert_eq!(last_request(&fixture)["target_revision"], r1);
}

#[test]
fn rollback_previous_with_no_earlier_release_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    let id = "rollback-no-earlier";
    write_project(&proj, id);
    let revision = register_and_gate(&db, &proj, id);
    let fixture = install_provider(&tmp.path().join("fx"), &proj, HEALTHY);
    promote_current(&db, &fixture, id, &revision);

    let out = run_with_fixtures(&db, &fixture, &["delivery", "rollback", id, "--previous"]);
    assert!(!out.status.success());
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("delivery-conflict"), "{stderr}");
    assert!(stderr.contains("no earlier healthy release"), "{stderr}");
    assert!(rollback_rows(&db, id).is_empty());
}

#[test]
fn a_failed_rollback_does_not_claim_the_release_moved() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    let id = "rollback-failed";
    let failing = install_provider(
        &tmp.path().join("fx"),
        &proj,
        Provider {
            declares_rollback: true,
            rollback_status: "failed",
            rollback_health: "unhealthy",
        },
    );
    let (r1, _r2) = two_promoted_revisions(&db, &proj, id, &failing);

    let out = run_with_fixtures(&db, &failing, &["delivery", "rollback", id, "--to", &r1]);
    assert!(
        out.status.success(),
        "a provider-declared failure is recorded, not a transport error: {}",
        lossy(&out.stderr)
    );
    let rows = rollback_rows(&db, id);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].state, "failed");
    let report = run_json(&db, &["delivery", "status", id]);
    assert_eq!(report["phase"], "healthy");
    assert_eq!(report["rollback"]["state"], "failed");
}

#[test]
fn rollback_does_not_require_gate_evidence_for_the_current_revision() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    let id = "rollback-no-gate";
    let fixture = install_provider(&tmp.path().join("fx"), &proj, HEALTHY);
    let (r1, _r2) = two_promoted_revisions(&db, &proj, id, &fixture);
    std::fs::remove_dir_all(proj.join(".forge/gate")).unwrap();

    let out = run_with_fixtures(&db, &fixture, &["delivery", "rollback", id, "--to", &r1]);
    assert!(
        out.status.success(),
        "rollback must not consult the ship gate: {}",
        lossy(&out.stderr)
    );
    assert_eq!(
        run_json(&db, &["delivery", "status", id])["phase"],
        "rolled-back"
    );
}

#[test]
fn a_replay_reuses_the_row_and_a_different_target_reserves_a_new_one() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    let id = "rollback-idempotent";
    let fixture = install_provider(&tmp.path().join("fx"), &proj, HEALTHY);
    write_project(&proj, id);
    let r1 = register_and_gate(&db, &proj, id);
    promote_current(&db, &fixture, id, &r1);
    let r2 = commit_and_register(&db, &proj, id, "v2\n");
    promote_current(&db, &fixture, id, &r2);
    let r3 = commit_and_register(&db, &proj, id, "v3\n");
    promote_current(&db, &fixture, id, &r3);

    let first = run_with_fixtures(&db, &fixture, &["delivery", "rollback", id, "--to", &r1]);
    assert!(first.status.success(), "{}", lossy(&first.stderr));
    let first_op = run_json(&db, &["delivery", "status", id])["rollback"]["op_id"]
        .as_i64()
        .unwrap();

    let replay = run_with_fixtures(&db, &fixture, &["delivery", "rollback", id, "--to", &r1]);
    assert!(replay.status.success(), "{}", lossy(&replay.stderr));
    let replay_op = run_json(&db, &["delivery", "status", id])["rollback"]["op_id"]
        .as_i64()
        .unwrap();
    assert_eq!(first_op, replay_op, "a replay reuses the row");
    assert_eq!(rollback_rows(&db, id).len(), 1);

    let other = run_with_fixtures(&db, &fixture, &["delivery", "rollback", id, "--to", &r2]);
    assert!(other.status.success(), "{}", lossy(&other.stderr));
    assert_eq!(
        rollback_rows(&db, id).len(),
        2,
        "a new target is a new operation"
    );
    assert!(journal_has(&db, id, "delivery.rollback"));
}
