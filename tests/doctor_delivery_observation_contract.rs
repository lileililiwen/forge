//! Contract tests for the doctor's delivery-state observation
//! (`lifecycle-observation-closure`).
//!
//! The doctor reads the delivery state the delivery lane recorded and
//! reports it as a `delivery-state` finding: a failed delivery fails the
//! doctor, a delivery in progress warns, a healthy delivery passes and a
//! project without a delivery is not applicable. Observing the delivery
//! writes nothing — the registry bytes and the journal row count are the
//! same before and after the run.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use forge::registry::{Registry, ReservationOutcome};
use serde_json::Value;

/// The revision the delivery lane recorded for the seeded deliveries.
const REVISION: &str = "0123456789abcdef0123456789abcdef01234567";

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn clean_cmd() -> Command {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY")
        .env_remove("HTTP_PROXY")
        .env_remove("HTTPS_PROXY")
        .env_remove("ALL_PROXY")
        .env_remove("OPENAI_API_KEY")
        .env_remove("ANTHROPIC_API_KEY")
        .env_remove("FORGE_DRIFTWATCH_BIN");
    cmd
}

fn lossy(bytes: &[u8]) -> std::borrow::Cow<'_, str> {
    String::from_utf8_lossy(bytes)
}

/// A fake DriftWatch adapter that reports one passing finding. The real
/// binary is not always installed, and an unavailable policy check never
/// lets a project be healthy — this keeps the verdict about the delivery.
fn write_fake_driftwatch(dir: &Path) -> PathBuf {
    let path = dir.join("fake-driftwatch.sh");
    fs::write(
        &path,
        "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then\n  echo \"driftwatch 0.1.0\"\n  exit 0\nfi\nprintf '%s' '{\"tool\":\"driftwatch\",\"tool_version\":\"0.1.0\",\"contract\":\"0.1.0\",\"source_revision\":null,\"findings\":[{\"id\":\"FAKE-001\",\"category\":\"security\",\"severity\":\"pass\",\"applicable\":true,\"message\":\"nothing to report\",\"evidence\":[\"nothing to report\"]}]}'\n",
    )
    .unwrap();
    let mut perms = fs::metadata(&path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&path, perms).unwrap();
    path
}

/// A project the doctor can call healthy: a valid manifest, a build
/// definition, documentation and a git repository with an `origin` remote.
fn write_project(dir: &Path, id: &str) {
    fs::create_dir_all(dir).unwrap();
    fs::write(
        dir.join("forge.yaml"),
        format!(
            "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: rust-web\n  maturity: L1\n  target_maturity: L1\nruntime:\n  language: rust\n"
        ),
    )
    .unwrap();
    fs::write(dir.join("Cargo.toml"), "[package]\nname = \"demo\"\n").unwrap();
    fs::write(dir.join("README.md"), "# demo\n").unwrap();
    let git = |args: &[&str]| {
        let out = Command::new("git")
            .args(args)
            .current_dir(dir)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_SYSTEM", "/dev/null")
            .output()
            .expect("run git");
        assert!(out.status.success(), "git {args:?} failed");
    };
    git(&["init", "--initial-branch=main"]);
    git(&["add", "-A"]);
    git(&[
        "-c",
        "user.name=Forge",
        "-c",
        "user.email=forge@example.invalid",
        "commit",
        "-m",
        "seed",
    ]);
    git(&[
        "remote",
        "add",
        "origin",
        "https://example.invalid/demo.git",
    ]);
}

/// Record one delivery lane operation the way the lane records it.
fn record_delivery(
    registry: &Registry,
    project_id: &str,
    kind: &str,
    state: &str,
    detail: &str,
    build: Option<&str>,
    run: Option<&str>,
) -> i64 {
    let key = format!("{kind}.{project_id}:seed");
    let op_id = match registry
        .reserve_idempotent_operation(kind, project_id, &key, "seed-fingerprint")
        .unwrap()
    {
        ReservationOutcome::Reserved { op_id } => op_id,
        ReservationOutcome::Reused { op_id } => op_id,
    };
    registry
        .finalize_operation(op_id, state, detail)
        .expect("finalize the delivery operation");
    registry
        .update_operation_phase(op_id, Some(REVISION), build, run, None)
        .expect("record the delivery phase");
    op_id
}

/// The `delivery-state` finding of one report.
fn delivery_finding(report: &Value) -> &Value {
    report["doctor"]["findings"]
        .as_array()
        .expect("findings array")
        .iter()
        .find(|f| f["id"] == "delivery-state")
        .expect("the report carries a delivery-state finding")
}

fn evidence_of(finding: &Value) -> String {
    finding["evidence"]
        .as_array()
        .expect("evidence array")
        .iter()
        .map(|e| e.as_str().unwrap_or_default().to_string())
        .collect::<Vec<String>>()
        .join(" ")
}

/// Run `forge doctor` for one project directory and return the JSON report.
fn run_doctor_json(db: &Path, project: &Path, driftwatch: &Path) -> Value {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    cmd.arg("--format").arg("json");
    cmd.arg("doctor").arg(project);
    cmd.env("FORGE_DRIFTWATCH_BIN", driftwatch);
    let out = cmd.output().expect("run forge doctor");
    assert!(
        out.status.success(),
        "forge doctor failed: {}",
        lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap_or_else(|err| {
        panic!(
            "forge doctor did not answer with JSON: {err}; stdout={}",
            lossy(&out.stdout)
        )
    })
}

fn journal_row_count(db: &Path) -> usize {
    let registry = Registry::open(db).expect("open the registry");
    registry.journal_entries().expect("read the journal").len()
}

#[test]
fn a_healthy_delivery_passes_and_leaves_the_project_healthy() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("demo");
    write_project(&project, "doctor-delivery-healthy");
    let driftwatch = write_fake_driftwatch(tmp.path());
    let db = tmp.path().join("registry.db");

    let mut registry = Registry::open(&db).unwrap();
    let record = registry.register(&project, None).unwrap();
    record_delivery(
        &registry,
        &record.id,
        "delivery.promote",
        "done",
        r#"{"evidence":["promoted"],"recovery":[]}"#,
        Some("succeeded"),
        Some("succeeded"),
    );
    drop(registry);

    let report = run_doctor_json(&db, &project, &driftwatch);
    let finding = delivery_finding(&report);
    assert_eq!(finding["status"], "pass", "{finding}");
    assert_eq!(finding["applicable"], true, "{finding}");
    assert_eq!(finding["remediation"], "manual", "{finding}");
    let evidence = evidence_of(finding);
    assert!(evidence.contains("phase=healthy"), "{evidence}");
    assert!(evidence.contains(REVISION), "{evidence}");
    assert!(
        report["doctor"]["healthy"] == true,
        "a healthy delivery keeps the project healthy: {report}"
    );
}

#[test]
fn a_failed_delivery_fails_the_doctor() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("demo");
    write_project(&project, "doctor-delivery-failed");
    let driftwatch = write_fake_driftwatch(tmp.path());
    let db = tmp.path().join("registry.db");

    let mut registry = Registry::open(&db).unwrap();
    let record = registry.register(&project, None).unwrap();
    record_delivery(
        &registry,
        &record.id,
        "delivery.stage",
        "failed",
        r#"{"evidence":["stage failed"],"recovery":[]}"#,
        Some("failed"),
        Some("failed"),
    );
    drop(registry);

    let report = run_doctor_json(&db, &project, &driftwatch);
    let finding = delivery_finding(&report);
    assert_eq!(finding["status"], "fail", "{finding}");
    assert_eq!(finding["applicable"], true, "{finding}");
    let evidence = evidence_of(finding);
    assert!(evidence.contains("phase=stage-failed"), "{evidence}");
    assert!(
        report["doctor"]["healthy"] == false,
        "a failed delivery denies health: {report}"
    );
    let blocking = report["doctor"]["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["status"] == "fail" || f["status"] == "unavailable")
        .count();
    assert!(
        blocking >= 1,
        "the failed delivery is a blocking finding: {report}"
    );
}

#[test]
fn a_delivery_in_progress_warns_without_denying_health() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("demo");
    write_project(&project, "doctor-delivery-progress");
    let driftwatch = write_fake_driftwatch(tmp.path());
    let db = tmp.path().join("registry.db");

    let mut registry = Registry::open(&db).unwrap();
    let record = registry.register(&project, None).unwrap();
    record_delivery(
        &registry,
        &record.id,
        "delivery.stage",
        "done",
        r#"{"evidence":["staged"],"recovery":[]}"#,
        Some("succeeded"),
        Some("succeeded"),
    );
    drop(registry);

    let report = run_doctor_json(&db, &project, &driftwatch);
    let finding = delivery_finding(&report);
    assert_eq!(finding["status"], "warn", "{finding}");
    assert_eq!(finding["applicable"], true, "{finding}");
    let evidence = evidence_of(finding);
    assert!(
        evidence.contains("phase=awaiting-production-approval"),
        "{evidence}"
    );
    assert!(
        report["doctor"]["healthy"] == true,
        "a delivery in progress is a fact, not a fault: {report}"
    );
}

#[test]
fn a_project_without_a_delivery_is_reported_as_not_applicable() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("demo");
    write_project(&project, "doctor-delivery-absent");
    let driftwatch = write_fake_driftwatch(tmp.path());
    let db = tmp.path().join("registry.db");

    let mut registry = Registry::open(&db).unwrap();
    registry.register(&project, None).unwrap();
    drop(registry);

    let report = run_doctor_json(&db, &project, &driftwatch);
    let finding = delivery_finding(&report);
    assert_eq!(finding["status"], "pass", "{finding}");
    assert_eq!(finding["applicable"], false, "{finding}");
    assert_eq!(finding["remediation"], "automatic", "{finding}");
    let evidence = evidence_of(finding);
    assert!(
        evidence.contains("no delivery has been recorded"),
        "{evidence}"
    );
    assert!(
        report["doctor"]["healthy"] == true,
        "a project without a delivery is not unhealthy: {report}"
    );
}

#[test]
fn observing_the_delivery_writes_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("demo");
    write_project(&project, "doctor-delivery-readonly");
    let driftwatch = write_fake_driftwatch(tmp.path());
    let db = tmp.path().join("registry.db");

    let mut registry = Registry::open(&db).unwrap();
    let record = registry.register(&project, None).unwrap();
    record_delivery(
        &registry,
        &record.id,
        "delivery.stage",
        "failed",
        r#"{"evidence":["stage failed"],"recovery":[]}"#,
        Some("failed"),
        Some("failed"),
    );
    drop(registry);

    let bytes_before = fs::read(&db).unwrap();
    let rows_before = journal_row_count(&db);

    let report = run_doctor_json(&db, &project, &driftwatch);
    assert_eq!(delivery_finding(&report)["status"], "fail");

    let bytes_after = fs::read(&db).unwrap();
    let rows_after = journal_row_count(&db);
    assert_eq!(rows_before, rows_after, "the doctor wrote no journal row");
    assert_eq!(
        bytes_before.len(),
        bytes_after.len(),
        "the doctor changed the registry size"
    );
    assert_eq!(
        bytes_before, bytes_after,
        "the doctor changed the registry bytes"
    );
}
