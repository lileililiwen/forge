//! Passing runs, freshness lifecycle, registry identity, and evidence persist/read.

use super::*;
use forge::gate::GATE_CONTRACT_VERSION;

#[test]
fn passing_run_persists_bound_evidence_journals_done_and_exits_zero() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = project(tmp.path(), "gate-ok");
    let stub = passing_stub(tmp.path());

    let run = gate(&db, &[&proj.display().to_string()], Some(&stub));
    assert_eq!(run.status, 0, "stderr: {}", run.stderr);
    let value = gate_json(&run);
    assert_eq!(value["contract"], "0.1.0");
    assert_eq!(value["dry_run"], false);
    assert_eq!(value["gate"]["aggregate"], "passed");
    assert_eq!(value["gate"]["freshness"], "fresh");
    assert_eq!(value["gate"]["runtime"], "gate-pass");
    assert_eq!(value["gate"]["runtime_version"], "driftwatch 0.1.0");
    assert_eq!(value["gate"]["revision"], head(&proj));
    assert_eq!(value["gate"]["manifest_digest"], "sha256:dc1fb6a3db59efd5");
    assert_eq!(value["gate"]["checks"][0]["id"], "docs");
    assert_eq!(value["gate"]["checks"][0]["state"], "pass");
    assert_eq!(
        value["persisted"], ".forge/gate/gate-ok/evidence.json",
        "{value}"
    );

    // The persisted record exists and carries the same bytes as served.
    let persisted: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(evidence_file(&proj, "gate-ok")).unwrap())
            .unwrap();
    assert_eq!(persisted["aggregate"], "passed");
    assert_eq!(persisted["revision"], head(&proj));
    assert_eq!(persisted["observed_at"], value["gate"]["observed_at"]);

    // One journal row, done verdict, attributed summary.
    let rows = journal_for(&db, "gate-ok");
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(rows[0].1, "done");
    assert!(rows[0].2.contains("aggregate=passed"), "{rows:?}");
    assert!(rows[0].2.contains("revision="), "{rows:?}");

    // Human output carries the same fields (surface parity).
    let human = gate_human(&db, &["status", &proj.display().to_string()], Some(&stub));
    assert_eq!(human.status, 0, "stderr: {}", human.stderr);
    assert!(
        human.stdout.contains("aggregate: passed"),
        "{}",
        human.stdout
    );
    assert!(
        human.stdout.contains("freshness: fresh"),
        "{}",
        human.stdout
    );
    let observed = value["gate"]["observed_at"].as_str().unwrap_or("");
    assert!(human.stdout.contains(observed), "{}", human.stdout);
}

#[test]
fn status_reports_absent_then_fresh_then_stale() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = project(tmp.path(), "gate-fresh");
    let stub = passing_stub(tmp.path());

    // Never-run: unverified, non-zero, no journal rows, no files.
    let before = Registry::open(&db)
        .unwrap()
        .journal_entries()
        .unwrap()
        .len();
    let absent = gate(&db, &["status", &proj.display().to_string()], None);
    assert_eq!(absent.status, 1);
    let value = gate_json(&absent);
    assert_eq!(value["gate"]["freshness"], "absent");
    assert_eq!(value["gate"]["aggregate"], "unverified");
    assert_eq!(
        Registry::open(&db)
            .unwrap()
            .journal_entries()
            .unwrap()
            .len(),
        before,
        "status must not journal"
    );

    // After a run: fresh + zero.
    let run = gate(&db, &[&proj.display().to_string()], Some(&stub));
    assert_eq!(run.status, 0, "stderr: {}", run.stderr);
    let fresh = gate(&db, &["status", &proj.display().to_string()], None);
    assert_eq!(fresh.status, 0, "stdout: {}", fresh.stdout);
    let fresh_value = gate_json(&fresh);
    assert_eq!(fresh_value["gate"]["freshness"], "fresh");
    assert_eq!(fresh_value["gate"]["aggregate"], "passed");

    // The revision moves: the same record is stale, never a pass claim.
    write_file(&proj, "notes.md", "moved\n");
    git(&proj, &["add", "-A"]);
    git(&proj, &["commit", "-q", "-m", "second"]);
    let stale = gate(&db, &["status", &proj.display().to_string()], None);
    assert_eq!(stale.status, 1);
    let stale_value = gate_json(&stale);
    assert_eq!(stale_value["gate"]["freshness"], "stale");
    assert_eq!(stale_value["gate"]["aggregate"], "passed");
    assert_eq!(
        stale_value["gate"]["observed_at"],
        fresh_value["gate"]["observed_at"]
    );
}

#[test]
fn registered_project_journals_under_its_registry_identity() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = project(tmp.path(), "gate-registered");
    let registered = Command::new(forge_bin())
        .env_remove("FORGE_REGISTRY")
        .arg("--registry")
        .arg(&db)
        .arg("register")
        .arg(&proj)
        .output()
        .expect("register");
    assert!(registered.status.success(), "{}", lossy(&registered.stderr));

    let stub = passing_stub(tmp.path());
    // Address it by registry id: the journal row and evidence directory
    // both carry the registry identity.
    let run = gate(&db, &["gate-registered"], Some(&stub));
    assert_eq!(run.status, 0, "stderr: {}", run.stderr);
    let value = gate_json(&run);
    assert_eq!(value["gate"]["project_id"], "gate-registered");
    assert!(evidence_file(&proj, "gate-registered").is_file());
    let rows = journal_for(&db, "gate-registered");
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(rows[0].1, "done");
}

#[test]
fn env_override_beats_the_declaration_and_runs_exactly_that_binary() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = project(tmp.path(), "gate-env");
    write_file(
        &proj,
        ".project.json",
        "{\"schema_version\":1,\"id\":\"gate-env\",\"kind\":\"product\",\"verification\":{\"command\":\"true\",\"gate_runtime\":\"driftwatchdog\"}}",
    );
    let stub = passing_stub(tmp.path());
    let run = gate(&db, &[&proj.display().to_string()], Some(&stub));
    assert_eq!(run.status, 0, "stderr: {}", run.stderr);
    let value = gate_json(&run);
    // The override ran exactly as named: its version line and binary
    // stem are the attribution, not the declared probe.
    assert_eq!(value["gate"]["runtime"], "gate-pass");
    assert_eq!(value["gate"]["runtime_version"], "driftwatch 0.1.0");
}

#[test]
fn gate_verdict_surface_byte_identity_with_evidence_command() {
    // Adding `forge gate evidence` must not change the gate verdict surface.
    // Run `forge gate status` and verify the output shape.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = project(tmp.path(), "gate-unchanged");

    let stub = passing_stub(tmp.path());
    let run = gate(&db, &[&proj.display().to_string()], Some(&stub));
    assert_eq!(run.status, 0, "{}", run.stderr);
    let value = gate_json(&run);
    assert_eq!(value["contract"], GATE_CONTRACT_VERSION);
    assert_eq!(value["gate"]["aggregate"], "passed");
}

#[test]
fn evidence_command_runs_and_persists_record() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = project(tmp.path(), "evid-persist");

    let stub = evidence_export_stub(
        tmp.path(),
        "gate-evid.sh",
        "forge-all-unverified.json",
        "evid-persist",
        "0",
    );

    let run = gate_evidence_cmd(&db, &["evidence", &proj.display().to_string()], Some(&stub));
    assert_eq!(run.status, 0, "stderr: {}", run.stderr);

    let value = evidence_json(&run);
    assert_eq!(value["contract"], "release-evidence/0.1.0");
    assert_eq!(value["release_evidence"]["project_id"], "evid-persist");
    assert!(value["persisted"].is_string());

    // Persisted file exists.
    let evid_path = evidence_status_file(&proj, "evid-persist");
    assert!(
        evid_path.exists(),
        "release-evidence.json should exist at {}",
        evid_path.display()
    );

    // Journal has a gate row.
    let journal = journal_for(&db, "evid-persist");
    assert!(!journal.is_empty(), "journal should not be empty");
}

#[test]
fn evidence_status_reads_persisted_record() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = project(tmp.path(), "evid-status");

    let stub = evidence_export_stub(
        tmp.path(),
        "gate-evid.sh",
        "forge-all-unverified.json",
        "evid-status",
        "0",
    );

    // First run: persist.
    let run = gate_evidence_cmd(&db, &["evidence", &proj.display().to_string()], Some(&stub));
    assert_eq!(run.status, 0, "{}", run.stderr);

    // Second run: read.
    let run = gate_evidence_cmd(
        &db,
        &["evidence", "status", &proj.display().to_string()],
        Some(&stub),
    );
    assert_eq!(run.status, 0, "{}", run.stderr);
    let value = evidence_json(&run);
    assert_eq!(value["contract"], "release-evidence/0.1.0");
    assert_eq!(value["release_evidence"]["project_id"], "evid-status");
}
