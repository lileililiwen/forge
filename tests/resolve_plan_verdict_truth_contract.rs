//! CLI contract for `resolve-plan-verdict-truth`.
//!
//! One rule names what a resolution did: a resolution that produced a
//! plan is a planned run, and a request the resolver could not satisfy
//! is a rejected run. A resolution installs nothing, so it is never
//! journalled as a completed action. The component resolver and the UI
//! pattern resolver read the same rule, and every journal row names the
//! run in its detail.
//!
//! - a component resolution that produced a plan journals `planned`;
//! - a UI pattern resolution that produced a plan journals `planned`;
//! - a resolution that satisfied nothing journals `rejected`;
//! - a resolution writes nothing but its journal row.
//!
//! The journal row is read from the registry's own SQLite database, the
//! way the other contract tests read it.

use std::fs;
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
        .env_remove("ANTHROPIC_API_KEY");
    cmd
}

/// Run the CLI in JSON mode and return both the process outcome and
/// the parsed document, so a test can assert the exit code and read the
/// plan from one run.
fn run_json_output(db: &Path, args: &[&str]) -> (std::process::Output, serde_json::Value) {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    cmd.arg("--format").arg("json");
    for a in args {
        cmd.arg(a);
    }
    let out = cmd.output().expect("run forge json");
    let value = serde_json::from_slice(&out.stdout).unwrap_or_else(|err| {
        panic!(
            "invalid json: {err}; stdout={} stderr={}",
            lossy(&out.stdout),
            lossy(&out.stderr)
        )
    });
    (out, value)
}

/// The journal rows of one kind, oldest first.
fn journal_rows(db: &Path, kind: &str) -> Vec<(String, String, String)> {
    let conn = rusqlite::Connection::open(db).expect("open db");
    let mut stmt = conn
        .prepare("SELECT project_id, state, detail FROM operations WHERE kind = ?1 ORDER BY op_id")
        .expect("prepare");
    let mut rows = stmt.query([kind]).expect("query");
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
fn a_component_resolution_is_planned_and_journals_a_planned_row() {
    // A resolution reads the catalog and returns a plan: it installs
    // nothing, so the journal row must not claim a completed
    // resolution.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");

    let (out, value) = run_json_output(
        &db,
        &[
            "component",
            "resolve",
            "--profile",
            "rust-web",
            "--component",
            "paginated-query",
        ],
    );
    assert_eq!(
        out.status.code(),
        Some(0),
        "a resolution exits 0; stderr={}",
        lossy(&out.stderr)
    );
    let steps = value["plan"]["steps"]
        .as_array()
        .expect("the plan has steps");
    assert_eq!(steps.len(), 1, "{steps:?}");
    assert_eq!(steps[0]["action"], "install", "the step names the action");
    let note = value["note"].as_str().expect("the resolution names itself");
    assert!(
        note.contains("verdict=planned"),
        "the output names the run: {note}"
    );

    let rows = journal_rows(&db, "component");
    assert_eq!(rows.len(), 1, "the resolution wrote one journal row");
    let (project_id, state, detail) = &rows[0];
    assert_eq!(project_id, "__component__");
    assert_eq!(
        state, "planned",
        "a resolution installs nothing; detail={detail}"
    );
    assert!(
        detail.contains("verdict=planned"),
        "the detail names the run: {detail}"
    );

    // The resolution wrote nothing but its journal row: no project
    // directory, no receipt, no installed component.
    let written: Vec<String> = fs::read_dir(tmp.path())
        .expect("read dir")
        .map(|entry| {
            entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .to_string()
        })
        .collect();
    assert_eq!(written, vec!["registry.db".to_string()], "{written:?}");
}

#[test]
fn a_ui_pattern_resolution_is_planned_and_journals_a_planned_row() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");

    let (out, value) = run_json_output(
        &db,
        &[
            "ui-pattern",
            "resolve",
            "--profile",
            "react-web",
            "--pattern",
            "billing",
        ],
    );
    assert_eq!(
        out.status.code(),
        Some(0),
        "a resolution exits 0; stderr={}",
        lossy(&out.stderr)
    );
    let steps = value["plan"]["steps"]
        .as_array()
        .expect("the plan has steps");
    assert_eq!(steps.len(), 1, "{steps:?}");
    let note = value["note"].as_str().expect("the resolution names itself");
    assert!(
        note.contains("verdict=planned"),
        "the output names the run: {note}"
    );

    let rows = journal_rows(&db, "ui_pattern");
    assert_eq!(rows.len(), 1, "the resolution wrote one journal row");
    let (project_id, state, detail) = &rows[0];
    assert_eq!(project_id, "__ui_pattern__");
    assert_eq!(
        state, "planned",
        "a resolution installs nothing; detail={detail}"
    );
    assert!(
        detail.contains("verdict=planned"),
        "the detail names the run: {detail}"
    );
}

#[test]
fn a_resolution_that_satisfied_nothing_is_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");

    let (out, value) = run_json_output(
        &db,
        &[
            "component",
            "resolve",
            "--profile",
            "rust-web",
            "--component",
            "made-up",
        ],
    );
    assert_eq!(
        out.status.code(),
        Some(0),
        "a rejected request still reports its plan; stderr={}",
        lossy(&out.stderr)
    );
    let rejections = value["plan"]["rejections"]
        .as_array()
        .expect("the plan carries rejections");
    assert_eq!(rejections.len(), 1, "{rejections:?}");
    assert!(
        rejections[0]["reason"]
            .as_str()
            .unwrap()
            .contains("not in the catalog"),
        "{rejections:?}"
    );

    let rows = journal_rows(&db, "component");
    let (_, state, detail) = &rows[0];
    assert_eq!(state, "rejected", "detail={detail}");
    assert!(
        detail.contains("verdict=rejected"),
        "the detail names the run: {detail}"
    );
}

#[test]
fn both_lanes_read_the_same_rule() {
    // One rule, two lanes: a plan in either lane is a planned run and
    // a refusal in either lane is a rejected run.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");

    let _ = run_json_output(
        &db,
        &[
            "component",
            "resolve",
            "--profile",
            "rust-web",
            "--component",
            "paginated-query",
        ],
    );
    let _ = run_json_output(
        &db,
        &[
            "ui-pattern",
            "resolve",
            "--profile",
            "react-web",
            "--pattern",
            "billing",
        ],
    );
    let _ = run_json_output(
        &db,
        &[
            "component",
            "resolve",
            "--profile",
            "rust-web",
            "--component",
            "made-up",
        ],
    );

    for (kind, expected) in [("component", "planned"), ("ui_pattern", "planned")] {
        let rows = journal_rows(&db, kind);
        let (_, state, detail) = &rows[0];
        assert_eq!(state, expected, "kind={kind} detail={detail}");
    }
    let rows = journal_rows(&db, "component");
    let (_, state, detail) = &rows[1];
    assert_eq!(state, "rejected", "detail={detail}");
}
