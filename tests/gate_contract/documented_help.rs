//! Documented help, dry-run rehearsal, and usage refusals.

use super::*;

#[test]
fn gate_help_surface_documents_the_verbs_and_bounds() {
    let out = Command::new(forge_bin())
        .arg("gate")
        .arg("--help")
        .output()
        .expect("help");
    let text = lossy(&out.stdout);
    assert!(text.contains("status"), "{text}");
    assert!(text.contains("--dry-run"), "{text}");
    assert!(text.contains("--timeout-secs"), "{text}");
    assert!(text.contains("86400"), "{text}");
}

#[test]
fn dry_run_previews_the_plan_without_persisting_or_journaling() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = project(tmp.path(), "gate-preview");
    let plan = fixture("gate-dryrun-plan.txt").display().to_string();
    let stub = write_script(
        tmp.path(),
        "gate-plan.sh",
        &format!(
            "if [ \"$1\" = \"--version\" ]; then echo 'driftwatch 0.1.0'; exit 0; fi\nif [ \"$2\" = \"--dry-run\" ]; then cat '{plan}'; exit 0; fi\necho 'unexpected argv' >&2\nexit 1\n"
        ),
    );

    let before = Registry::open(&db)
        .unwrap()
        .journal_entries()
        .unwrap()
        .len();
    let run = gate(
        &db,
        &["--dry-run", &proj.display().to_string()],
        Some(&stub),
    );
    assert_eq!(run.status, 0, "stderr: {}", run.stderr);
    let value = gate_json(&run);
    assert_eq!(value["dry_run"], true);
    assert_eq!(value["gate"]["persisted"], false);
    assert_eq!(value["gate"]["journaled"], false);
    assert!(value["gate"]["plan"][0]
        .as_str()
        .unwrap_or("")
        .contains("nothing was executed"));
    // The stub proves the argv: rehearsal sends `gate --dry-run`.
    assert!(
        !proj.join(".forge").exists(),
        "rehearsal wrote a .forge tree"
    );
    let after = Registry::open(&db)
        .unwrap()
        .journal_entries()
        .unwrap()
        .len();
    assert_eq!(before, after, "rehearsal journaled a row");
}

#[test]
fn status_flag_conflicts_and_extra_positionals_refuse_before_work() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = project(tmp.path(), "gate-usage");

    let run = gate(&db, &["status", "--dry-run"], None);
    assert!(run.stderr.contains("gate-invalid"), "{}", run.stderr);
    let run = gate(&db, &["status", &proj.display().to_string(), "extra"], None);
    assert!(run.stderr.contains("at most one TARGET"), "{}", run.stderr);
    let run = gate(&db, &["one", "two"], None);
    assert!(
        run.stderr.contains("unexpected arguments"),
        "{}",
        run.stderr
    );
    assert!(journal_for(&db, "gate-usage").is_empty());
}

#[test]
fn evidence_dry_run_and_timeout_flags_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = project(tmp.path(), "evid-flags");

    let stub = evidence_export_stub(
        tmp.path(),
        "gate-evid.sh",
        "forge-all-unverified.json",
        "evid-flags",
        "0",
    );

    let run = gate_evidence_cmd(
        &db,
        &["evidence", "--dry-run", &proj.display().to_string()],
        Some(&stub),
    );
    assert!(run.stderr.contains("--dry-run"), "{}", run.stderr);

    let run = gate_evidence_cmd(
        &db,
        &[
            "evidence",
            "--timeout-secs",
            "300",
            &proj.display().to_string(),
        ],
        Some(&stub),
    );
    assert!(run.stderr.contains("--timeout-secs"), "{}", run.stderr);
}

#[test]
fn evidence_status_rejects_extra_positionals() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = project(tmp.path(), "evid-status-args");

    let stub = evidence_export_stub(
        tmp.path(),
        "gate-evid.sh",
        "forge-all-unverified.json",
        "evid-status-args",
        "0",
    );

    let run = gate_evidence_cmd(
        &db,
        &[
            "evidence",
            "status",
            &proj.display().to_string(),
            "extra-arg",
        ],
        Some(&stub),
    );
    assert_ne!(run.status, 0);
    assert!(
        run.stderr.contains("extra") || run.stderr.contains("at most"),
        "{}",
        run.stderr
    );
}
