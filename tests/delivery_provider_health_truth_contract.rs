//! Contract test: the delivery projection reports the provider's own
//! health.
//!
//! The canonical capability says the status projection returns
//! `phase=healthy` "(or `degraded` if the provider's last `verify`
//! reports unhealthy)". The projection computed its phase from the
//! build and run statuses alone, so a provider that answered
//! `health: "degraded"` was projected as `healthy` — a phase that
//! contradicts the evidence the same document carries.
//!
//! The provider is stubbed through a local executable on a controlled
//! `PATH`, the same way `tests/delivery_contract.rs` stubs it, so the
//! workflow runs without an OpenPanel daemon.

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
        .env_remove("FORGE_DEPLOYER_BIN")
        .env_remove("FORGE_DRIFTWATCH_BIN")
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
    if !out.status.success() {
        panic!(
            "forge failed: status={} stderr={}",
            out.status,
            lossy(&out.stderr)
        );
    }
    serde_json::from_slice(&out.stdout).unwrap_or_else(|err| {
        panic!(
            "invalid json: {err}; stdout={} stderr={}",
            lossy(&out.stdout),
            lossy(&out.stderr)
        )
    })
}

/// Run `forge` with the fixture bins on `PATH` so the publish
/// provider resolves to the local stub.
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
    let text = format!(
        "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: rust-web\n  maturity: L2\n  target_maturity: L3\nruntime:\n  language: rust\n"
    );
    std::fs::write(dir.join("forge.yaml"), text).unwrap();
    std::fs::write(dir.join("README.md"), "v1\n").unwrap();
    let _ = std::process::Command::new("git")
        .args(["init", "--initial-branch=main"])
        .current_dir(dir)
        .output();
    let _ = std::process::Command::new("git")
        .args([
            "-c",
            "user.name=Forge",
            "-c",
            "user.email=forge@example.com",
            "add",
            ".",
        ])
        .current_dir(dir)
        .output();
    let _ = std::process::Command::new("git")
        .args([
            "-c",
            "user.name=Forge",
            "-c",
            "user.email=forge@example.com",
            "commit",
            "-m",
            "seed",
        ])
        .current_dir(dir)
        .output();
}

/// Stage a fake `forge-publish-provider/0.1.0` provider that answers
/// with the given `health`.
///
/// The preflight and the stage verbs always answer a successful build
/// and run, so the delivery workflow itself succeeds and the promote
/// guard is satisfied. The promote verb answers the given build and
/// run statuses, so a test can state a failed build or run without
/// failing the earlier verbs.
///
/// The provider is invoked with the request on stdin and without
/// arguments, so the stub reads the request to learn which verb it
/// answers: the request's queue id names the environment.
fn install_provider(
    dir: &Path,
    project_dir: &Path,
    health: &str,
    promote_build_status: &str,
    promote_run_status: &str,
) -> PathBuf {
    let provider_dir = dir.join("bin");
    std::fs::create_dir_all(&provider_dir).unwrap();
    let script = provider_dir.join("forge-openpanel-stub");
    let body = format!(
        "#!/bin/sh\nREQ=$(cat)\nQUEUE=$(printf '%s' \"$REQ\" | tr ',' '\\n' | grep '\"queue_id\"' | cut -d'\"' -f4)\nif [ -z \"$QUEUE\" ]; then QUEUE=delivery-default; fi\nBUILD=succeeded\nRUN=succeeded\ncase \"$QUEUE\" in\n  delivery-production-*) BUILD=\"{promote_build_status}\"; RUN=\"{promote_run_status}\" ;;\nesac\nprintf '{{\"contract\":\"forge-publish-provider/0.1.0\",\"provider\":\"openpanel\",\"operation_id\":\"stub-op\",\"status\":\"succeeded\",\"health\":\"{health}\",\"evidence\":[\"provider stub\"],\"recovery\":[],\"build_status\":\"%s\",\"run_status\":\"%s\",\"container_identity\":\"forge-stub-0123456789ab\",\"queue_id\":\"%s\"}}\\n' \"$BUILD\" \"$RUN\" \"$QUEUE\"\n"
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

fn seed_register(db: &Path, proj: &Path, id: &str) {
    let reg = run(db, &["register", proj.to_str().unwrap()]);
    assert!(reg.status.success(), "register: {}", lossy(&reg.stderr));
    let revision = registered_revision(db, id);
    write_gate_evidence(proj, id, &revision, "passed");
}

/// Write the Gate evidence the delivery lane requires before it will
/// preflight.
fn write_gate_evidence(proj: &Path, id: &str, revision: &str, aggregate: &str) {
    let evidence = serde_json::json!({
        "contract": forge::gate::GATE_CONTRACT_VERSION,
        "project_id": id,
        "runtime": "forge",
        "profile": "full",
        "revision": revision,
        "aggregate": aggregate,
        "checks": [],
        "observed_at": "2026-10-18T00:00:00Z",
        "dry_run": false
    });
    let path = proj.join(".forge/gate").join(id).join("evidence.json");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, evidence.to_string()).unwrap();
}

fn registered_revision(db: &Path, id: &str) -> String {
    let inspect = run_json(db, &["inspect", id]);
    inspect["last_commit"].as_str().unwrap().to_string()
}

/// Drive preflight → stage → promote against a provider that answers
/// the given `health`, and return the delivery status projection.
fn projected_phase_for(db: &Path, _proj: &Path, id: &str, fixture: &Path) -> Value {
    let preflight = run_with_fixtures(db, fixture, &["delivery", "preflight", id]);
    assert!(
        preflight.status.success(),
        "preflight: {}",
        lossy(&preflight.stderr)
    );
    let report = run_json(db, &["delivery", "status", id]);
    let preflight_op = report["preflight"]["op_id"].as_i64().expect("op_id");
    let stage = run_with_fixtures(
        db,
        fixture,
        &[
            "delivery",
            "stage",
            id,
            "--confirm-operation-id",
            &preflight_op.to_string(),
        ],
    );
    assert!(stage.status.success(), "stage: {}", lossy(&stage.stderr));
    let rev = registered_revision(db, id);
    let promote = run_with_fixtures(
        db,
        fixture,
        &["delivery", "promote", id, "--confirm-revision", &rev],
    );
    assert!(
        promote.status.success(),
        "promote: {}",
        lossy(&promote.stderr)
    );
    run_json(db, &["delivery", "status", id])
}

#[test]
fn a_provider_that_reports_unhealthy_is_projected_as_degraded() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    let id = "delivery-provider-degraded";
    write_project(&proj, id);
    seed_register(&db, &proj, id);
    let fixture = install_provider(
        &tmp.path().join("fx"),
        &proj,
        "degraded",
        "succeeded",
        "succeeded",
    );

    let report = projected_phase_for(&db, &proj, id, &fixture);

    let phase = report["phase"].as_str().expect("phase");
    assert_eq!(
        phase, "degraded",
        "a provider whose last verify reported `degraded` must not be projected as healthy; \
         the projection was {phase} and the full report was {report}"
    );
}

#[test]
fn a_provider_that_reports_healthy_is_still_projected_as_healthy() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    let id = "delivery-provider-healthy";
    write_project(&proj, id);
    seed_register(&db, &proj, id);
    let fixture = install_provider(
        &tmp.path().join("fx"),
        &proj,
        "healthy",
        "succeeded",
        "succeeded",
    );

    let report = projected_phase_for(&db, &proj, id, &fixture);

    let phase = report["phase"].as_str().expect("phase");
    assert_eq!(
        phase, "healthy",
        "a provider whose last verify reported `healthy` must still be projected as healthy; \
         the projection was {phase} and the full report was {report}"
    );
}

#[test]
fn a_provider_that_reports_no_health_keeps_the_existing_verdict() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    let id = "delivery-provider-no-health";
    write_project(&proj, id);
    seed_register(&db, &proj, id);
    // The adapter contract requires the `health` field, so a provider
    // that reports no health answers it empty. The absence of an
    // answer is not an answer of `healthy`, and it is not an answer of
    // `degraded` either: the projection keeps the verdict it gives a
    // successful build and run.
    let fixture = install_provider(&tmp.path().join("fx"), &proj, "", "succeeded", "succeeded");

    let report = projected_phase_for(&db, &proj, id, &fixture);

    let phase = report["phase"].as_str().expect("phase");
    assert_eq!(
        phase, "healthy",
        "a provider that reported no health must keep the existing verdict; \
         the projection was {phase} and the full report was {report}"
    );
}

#[test]
fn a_promote_whose_build_failed_is_projected_as_degraded() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    let id = "delivery-provider-failed-build";
    write_project(&proj, id);
    seed_register(&db, &proj, id);
    // The provider reports itself healthy, but its promote answered a
    // failed build. The phase was already `degraded` for a failed
    // build before this change, and it stays `degraded`.
    let fixture = install_provider(
        &tmp.path().join("fx"),
        &proj,
        "healthy",
        "failed",
        "succeeded",
    );

    let report = projected_phase_for(&db, &proj, id, &fixture);

    let phase = report["phase"].as_str().expect("phase");
    assert_eq!(
        phase, "degraded",
        "a promote whose build failed is degraded, unchanged; \
         the projection was {phase} and the full report was {report}"
    );
}
