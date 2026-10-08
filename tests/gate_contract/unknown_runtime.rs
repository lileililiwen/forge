//! Unresolvable and unknown runtimes, document-absent runs, timeouts, and unavailable evidence.

use super::*;

#[test]
fn missing_runtime_lists_attempts_and_preserves_prior_evidence() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = project(tmp.path(), "gate-preserve");
    let stub = passing_stub(tmp.path());

    // First run persists real evidence.
    let first = gate(&db, &[&proj.display().to_string()], Some(&stub));
    assert_eq!(first.status, 0);
    let bytes_before = fs::read(evidence_file(&proj, "gate-preserve")).unwrap();

    // A later run with no resolvable runtime fails honestly.
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_GATE_BIN")
        .env_remove("FORGE_DRIFTWATCH_BIN")
        .env("PATH", tmp.path().join("empty-path"));
    let out = cmd
        .arg("--registry")
        .arg(&db)
        .arg("--format")
        .arg("json")
        .arg("gate")
        .arg(&proj)
        .output()
        .expect("run");
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty(), "stdout must stay empty on failure");
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("gate-runtime-unavailable"), "{stderr}");
    assert!(stderr.contains("FORGE_GATE_BIN"), "{stderr}");
    assert!(stderr.contains("driftwatchdog"), "{stderr}");

    // Prior evidence untouched; history preserved with the failure row.
    assert_eq!(
        fs::read(evidence_file(&proj, "gate-preserve")).unwrap(),
        bytes_before
    );
    let rows = journal_for(&db, "gate-preserve");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[1].1, "done");
    assert_eq!(rows[0].1, "failed");
    assert!(rows[0].2.contains("no gate runtime resolvable"), "{rows:?}");
}

#[test]
fn unknown_declared_runtime_refuses_naming_the_declaration() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = project(tmp.path(), "gate-unknown-runtime");
    write_file(
        &proj,
        ".project.json",
        "{\"schema_version\":1,\"id\":\"gate-unknown-runtime\",\"kind\":\"product\",\"verification\":{\"command\":\"true\",\"gate_runtime\":\"jenkins-gate\"}}",
    );

    let run = gate(&db, &[&proj.display().to_string()], None);
    assert_eq!(run.status, 1);
    assert!(run.stdout.is_empty());
    assert!(run.stderr.contains("jenkins-gate"), "{}", run.stderr);
    assert!(run.stderr.contains("driftwatchdog"), "{}", run.stderr);
    assert!(!evidence_file(&proj, "gate-unknown-runtime").exists());
}

#[test]
fn real_run_without_a_document_is_unavailable_never_a_pass() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = project(tmp.path(), "gate-text");
    // The sibling's honest "nothing to gate" answer for an
    // unmanaged project: exit 0, human text, no document.
    let stub = write_script(
        tmp.path(),
        "gate-text.sh",
        "if [ \"$1\" = \"--version\" ]; then echo 'driftwatch 0.1.0'; exit 0; fi\nif [ \"$2\" = \"--dry-run\" ]; then cat <<'EOF'\ngate plan (dry-run; nothing was executed)\nEOF\nexit 0\nfi\necho 'driftwatch gate: no gate.toml or .ai-gate/gate.yaml; nothing to gate.'\nexit 0\n",
    );

    let run = gate(&db, &[&proj.display().to_string()], Some(&stub));
    assert_eq!(run.status, 1);
    assert!(
        run.stderr.contains("gate-runtime-unavailable"),
        "{}",
        run.stderr
    );
    assert!(run.stderr.contains("nothing to gate"), "{}", run.stderr);
    assert!(!proj.join(".forge").exists());
    // The attempt is still history.
    let rows = journal_for(&db, "gate-text");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].1, "failed");

    // The rehearsal of the same runtime still previews honestly.
    let preview = gate(
        &db,
        &["--dry-run", &proj.display().to_string()],
        Some(&stub),
    );
    assert_eq!(preview.status, 0, "stderr: {}", preview.stderr);
}

#[test]
fn timeout_bounds_and_hangs_refuse_before_or_within_the_bounded_wait() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = project(tmp.path(), "gate-timeout");

    // Out-of-range bounds refuse before anything runs.
    for bad in ["0", "86401"] {
        let run = gate(
            &db,
            &["--timeout-secs", bad, &proj.display().to_string()],
            Some(&passing_stub(tmp.path())),
        );
        assert_eq!(run.status, 1);
        assert!(run.stderr.contains("gate-invalid"), "{}", run.stderr);
        assert!(run.stderr.contains("bounded range"), "{}", run.stderr);
    }
    assert!(!proj.join(".forge").exists());
    assert!(journal_for(&db, "gate-timeout").is_empty());

    // A hanging runtime is cut off by the bounded wait, honestly.
    let hang = write_script(
        tmp.path(),
        "gate-hang.sh",
        "if [ \"$1\" = \"--version\" ]; then echo 'driftwatch 0.1.0'; exit 0; fi\nsleep 30\n",
    );
    let run = gate(
        &db,
        &["--timeout-secs", "1", &proj.display().to_string()],
        Some(&hang),
    );
    assert_eq!(run.status, 1);
    assert!(
        run.stderr.contains("gate-runtime-unavailable"),
        "{}",
        run.stderr
    );
    assert!(run.stderr.contains("timeout"), "{}", run.stderr);
    assert_eq!(journal_for(&db, "gate-timeout")[0].1, "failed");
}

#[test]
fn unknown_target_refuses_before_any_invocation() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let run = gate(&db, &["nosuch-project-xyz"], None);
    assert_eq!(run.status, 1);
    assert!(run.stderr.contains("unknown-project"), "{}", run.stderr);
    assert!(run.stdout.is_empty());
}

#[test]
fn evidence_status_absent_without_prior_run() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = project(tmp.path(), "evid-absent");

    let stub = evidence_unavailable_stub(tmp.path(), "gate-evid.sh");
    let run = gate_evidence_cmd(
        &db,
        &["evidence", "status", &proj.display().to_string()],
        Some(&stub),
    );
    eprintln!("DEBUG status={} stdout={}", run.status, run.stdout);
    assert_eq!(run.status, 1, "{}", run.stderr);
    let value = evidence_json(&run);
    assert!(value["release_evidence"].is_null());
    assert_eq!(value["freshness"], "absent");
}

#[test]
fn evidence_export_unavailable_when_runtime_absent() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = project(tmp.path(), "evid-unavail");

    // No gate binary at all.
    let run = gate_evidence_cmd(&db, &["evidence", &proj.display().to_string()], None);
    assert_ne!(run.status, 0, "should fail when no runtime");
    assert!(
        run.stderr.contains("gate-evidence-unavailable"),
        "{}",
        run.stderr
    );
}

#[test]
fn evidence_unavailable_on_malformed_json() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = project(tmp.path(), "evid-malform");

    let stub = evidence_malformed_stub(tmp.path(), "gate-evid.sh");
    let run = gate_evidence_cmd(&db, &["evidence", &proj.display().to_string()], Some(&stub));
    assert_ne!(run.status, 0, "{}", run.stderr);
    assert!(
        run.stderr.contains("gate-evidence-unavailable"),
        "{}",
        run.stderr
    );
}
