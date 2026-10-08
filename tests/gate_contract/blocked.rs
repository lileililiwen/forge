//! Blocked documents, contradictions, redaction, and evidence refusals.

use super::*;

#[test]
fn blocked_document_is_evidence_not_failure() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = project(tmp.path(), "gate-blocked");
    let stub = blocked_stub(tmp.path());

    let run = gate(&db, &[&proj.display().to_string()], Some(&stub));
    // Exit mirrors the block (sibling semantics) while the evidence
    // document still prints on stdout, and the unavailable code never
    // appears.
    assert_eq!(run.status, 1);
    assert!(
        !run.stderr.contains("gate-runtime-unavailable"),
        "{}",
        run.stderr
    );
    let value = gate_json(&run);
    assert_eq!(value["gate"]["aggregate"], "blocked");
    assert_eq!(value["gate"]["checks"][0]["state"], "fail");
    let note = value["gate"]["checks"][0]["note"].as_str().unwrap_or("");
    assert!(note.contains("diagnostic: exit 1"), "{note}");
    assert!(
        note.contains("remediation: Fix the failing command"),
        "{note}"
    );
    assert!(evidence_file(&proj, "gate-blocked").is_file());
    let rows = journal_for(&db, "gate-blocked");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].1, "blocked");
}

#[test]
fn contradictory_pass_document_downgrades_never_fabricates_a_pass() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = project(tmp.path(), "gate-contradict");
    let stub = document_stub(
        tmp.path(),
        "gate-lied.sh",
        "gate-status-pass.json",
        "1", // parseable PASS document riding a failure exit
    );

    let run = gate(&db, &[&proj.display().to_string()], Some(&stub));
    assert_eq!(run.status, 1);
    let value = gate_json(&run);
    // Evidence (not unavailable), but the contradiction never passes.
    assert_eq!(value["gate"]["aggregate"], "unknown");
    assert!(!run.stderr.contains("unavailable"), "{}", run.stderr);
    assert!(evidence_file(&proj, "gate-contradict").is_file());
    assert_eq!(journal_for(&db, "gate-contradict")[0].1, "failed");
}

#[test]
fn secrets_and_host_paths_never_escape_the_redaction_pipeline() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = project(tmp.path(), "gate-secret");
    let document = format!(
        "{{\"status\":\"FAIL\",\"blocked\":true,\"failures\":[\"ci\"],\"pending_reviews\":[],\"not_applicable\":[],\"manifest_digest\":\"sha256:x\",\"rule_pack_version\":\"local\",\"results\":[{{\"gate_id\":\"ci\",\"status\":\"FAIL\",\"severity\":\"error\",\"findings\":[],\"evidence\":[],\"missing_evidence\":[],\"diagnostic\":\"exit 1 with token ghp_{} while reading {}\"}}]}}",
        "S".repeat(36),
        proj.display()
    );
    let stub = write_script(
        tmp.path(),
        "gate-secret.sh",
        &format!(
            "if [ \"$1\" = \"--version\" ]; then echo 'driftwatch 0.1.0'; exit 0; fi\ncat <<'DOCEOF'\n{document}\nDOCEOF\nexit 1\n"
        ),
    );

    let run = gate(&db, &[&proj.display().to_string()], Some(&stub));
    assert!(!run.stdout.contains("ghp_"), "{}", run.stdout);
    assert!(run.stdout.contains("[REDACTED]"), "{}", run.stdout);
    let persisted = fs::read_to_string(evidence_file(&proj, "gate-secret")).unwrap();
    assert!(!persisted.contains("ghp_"), "{persisted}");
    for row in journal_for(&db, "gate-secret") {
        assert!(!row.2.contains("ghp_"), "{row:?}");
    }
}

#[test]
fn evidence_export_refused_on_blocked_state() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = project(tmp.path(), "evid-blocked");

    let doc = r#"{
  "schema_version": 1,
  "project_id": "evid-blocked",
  "revision": "deadbeef12345678",
  "toolchain": "driftwatchdog@0.1.0",
  "fields": [{"field": "revision", "state": "blocked"}],
  "gate_run_id": 99
}
"#;
    let stub = {
        let path = tmp.path().join("blocked-export.json");
        fs::write(&path, doc).unwrap();
        let path_str = path.display().to_string();
        write_script(
            tmp.path(),
            "gate-evid.sh",
            &format!(
                "if [ \"$1\" = \"--version\" ]; then echo 'driftwatch 0.1.0'; exit 0; fi
if [ \"$1\" = \"gate\" ] && [ \"$2\" = \"evidence-export\" ]; then cat '{path_str}'; exit 0; fi
echo 'unrecognized'
exit 1
"
            ),
        )
    };
    let run = gate_evidence_cmd(&db, &["evidence", &proj.display().to_string()], Some(&stub));
    assert_ne!(run.status, 0, "{}", run.stderr);
    assert!(run.stderr.contains("blocked"), "{}", run.stderr);
}

#[test]
fn evidence_export_refused_on_unknown_field() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = project(tmp.path(), "evid-refused");

    let stub = evidence_unknown_field_stub(tmp.path(), "gate-evid.sh", "evid-refused");
    let run = gate_evidence_cmd(&db, &["evidence", &proj.display().to_string()], Some(&stub));
    // Refused → non-zero exit.
    assert_ne!(run.status, 0, "{}", run.stderr);
    assert!(
        run.stderr.contains("refused") || run.stderr.contains("unknown"),
        "{}",
        run.stderr
    );
}

#[test]
fn evidence_export_refused_on_publication_contradiction() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = project(tmp.path(), "evid-contradict");

    let stub =
        evidence_publication_contradiction_stub(tmp.path(), "gate-evid.sh", "evid-contradict");
    let run = gate_evidence_cmd(&db, &["evidence", &proj.display().to_string()], Some(&stub));
    assert_ne!(run.status, 0, "{}", run.stderr);
    assert!(
        run.stderr.contains("contradiction") || run.stderr.contains("publication"),
        "{}",
        run.stderr
    );
}

#[test]
fn evidence_refused_count_in_record() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = project(tmp.path(), "evid-counts");

    // Export with mixed states: some verified, some configured, some refused.
    let base: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(evidence_fixture("forge-all-unverified.json")).unwrap(),
    )
    .unwrap();
    let mut doc = base;
    doc["project_id"] = serde_json::json!("evid-counts");
    doc["fields"][0] = serde_json::json!({
        "field": "revision",
        "state": "verified",
        "evidence_ref": "revision-ref"
    });
    doc["fields"][1] = serde_json::json!({
        "field": "version",
        "state": "configured"
    });
    doc["fields"][8] = serde_json::json!({
        "field": "unknown-field-x",
        "state": "unverified"
    });

    let path = tmp.path().join("mixed-export.json");
    fs::write(&path, serde_json::to_string_pretty(&doc).unwrap()).unwrap();
    let path_str = path.display().to_string();
    let stub = write_script(
        tmp.path(),
        "gate-evid.sh",
        &format!(
            "if [ \"$1\" = \"--version\" ]; then echo 'driftwatch 0.1.0'; exit 0; fi
if [ \"$1\" = \"gate\" ] && [ \"$2\" = \"evidence-export\" ]; then cat '{path_str}'; exit 0; fi
echo 'unrecognized'
exit 1
"
        ),
    );

    let run = gate_evidence_cmd(&db, &["evidence", &proj.display().to_string()], Some(&stub));
    // The unknown field should be refused.
    assert_ne!(run.status, 0, "{}", run.stderr);
}
