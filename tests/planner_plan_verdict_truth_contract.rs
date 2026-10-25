//! CLI contract for `planner-plan-verdict-truth`.
//!
//! The planner names the action it performed. A resolve produces a plan
//! and applies nothing, so it is a planned run and never a completed
//! resolution; it reads the one shared rule, the same rule the component
//! resolver and the UI pattern resolver read. A component the apply
//! resolved is a resolved component, not a verified one: the planner
//! promotes nothing, so it reports no verification.
//!
//! - an intent resolve journals `planned` and names the run;
//! - the resolve row is never a completed resolution;
//! - an applied component step reports `resolved`, never `verified`;
//! - every resolution writer reads the same rule.
//!
//! The journal row is read from the registry's own SQLite database, the
//! way the other contract tests read it.

use std::path::{Path, PathBuf};
use std::process::Command;

use forge::planner::{
    apply_plan, resolve_plan, validate_intent, write_plan_receipt, Intent, IntentAction, PlanStep,
    PlanStepKind,
};
use tempfile::TempDir;

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

/// A project the planner can apply to, with its own registry.
fn project_dir(workspace: &TempDir, profile: &str, project_id: &str) -> PathBuf {
    let dir = workspace.path().join("project");
    std::fs::create_dir_all(&dir).expect("project dir");
    std::fs::write(
        dir.join("forge.yaml"),
        format!(
            "schema: 1\nproject:\n  id: {project_id}\n  name: {project_id}\n  \
             profile: {profile}\nfeatures: {{}}\n"
        ),
    )
    .expect("manifest write");
    let db = dir.join(".forge/registry.sqlite");
    if let Some(parent) = db.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut registry = forge::registry::Registry::open(&db).expect("registry open");
    let _ = registry.register(&dir, None);
    dir
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

/// Run the intent resolver and return the JSON document.
fn resolve_intent(db: &Path, work_dir: &Path, args: &[&str]) -> serde_json::Value {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    cmd.arg("--format").arg("json");
    cmd.arg("intent").arg("resolve");
    cmd.arg("--path").arg(work_dir);
    for a in args {
        cmd.arg(a);
    }
    let out = cmd.output().expect("run forge intent resolve");
    assert_eq!(
        out.status.code(),
        Some(0),
        "a resolve exits 0; stderr={}",
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

#[test]
fn an_intent_resolve_journals_a_planned_run_and_names_it() {
    // A resolve reads the catalog and returns a plan: it applies
    // nothing, so the journal row must not claim a completed
    // resolution.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let work = tmp.path().join("work");
    std::fs::create_dir_all(&work).expect("work dir");

    let value = resolve_intent(
        &db,
        &work,
        &["--action", "create_project", "--profile", "rust-web"],
    );
    let steps = value["plan"]["steps"]
        .as_array()
        .expect("the plan has steps");
    assert!(!steps.is_empty(), "the plan has steps");
    let plan_id = value["plan"]["plan_id"].as_str().expect("plan_id");

    let rows = journal_rows(&db, "planner");
    assert_eq!(rows.len(), 1, "the resolve wrote one journal row");
    let (project_id, state, detail) = &rows[0];
    assert_eq!(project_id, plan_id, "the row names the plan");
    assert_eq!(
        state, "planned",
        "a resolve applies nothing; it is a planned run; detail={detail}"
    );
    assert!(
        detail.contains("verdict=planned"),
        "the detail names the run: {detail}"
    );

    // The command's output names the run too, so a reader never has to
    // open the registry to learn what the resolve did.
    let note = value["note"].as_str().expect("the output names itself");
    assert!(
        note.contains("verdict=planned"),
        "the output names the run: {note}"
    );
}

#[test]
fn an_intent_resolve_is_never_recorded_as_a_completed_resolution() {
    // The row a reader counts and filters on must not be `done`: a
    // plan behind a completed state is still a completed resolution on
    // the record.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let work = tmp.path().join("work");
    std::fs::create_dir_all(&work).expect("work dir");

    resolve_intent(
        &db,
        &work,
        &["--action", "create_project", "--profile", "rust-web"],
    );

    let rows = journal_rows(&db, "planner");
    for (_, state, detail) in &rows {
        assert_ne!(
            state, "done",
            "a resolve applies nothing; it is never a completed resolution; detail={detail}"
        );
    }

    // The plan was produced and nothing was applied: the receipt is
    // the only artifact, and no feature or component was installed.
    let receipts: Vec<String> = std::fs::read_dir(work.join(".forge/planner"))
        .expect("the plan receipt directory exists")
        .map(|entry| {
            entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .to_string()
        })
        .collect();
    assert_eq!(receipts.len(), 1, "{receipts:?}");
    assert!(
        !work.join(".forge/features").exists(),
        "nothing was installed"
    );
}

#[test]
fn an_applied_component_step_reports_resolved_and_never_verified() {
    // The apply executor's component step resolves a component in the
    // catalog. It installs nothing and qualifies nothing, so it does
    // not report a verified component.
    //
    // No supported profile currently declares a component id as a
    // capability, so the deterministic resolver cannot emit a
    // component step on its own. The step is built on a resolved plan
    // here, and the executor is exercised through its public contract.
    let ws = tempfile::tempdir().unwrap();
    let project = project_dir(&ws, "rust-web", "planner-component-step");
    let db = project.join(".forge/registry.sqlite");

    let intent = Intent {
        action: IntentAction::CreateProject,
        profile: "rust-web".to_string(),
        required_capabilities: vec!["auth".to_string()],
        forbidden_capabilities: vec![],
        constraints: vec![],
    };
    let validated = validate_intent(&intent).expect("validated");
    let mut plan = resolve_plan(&validated, None).expect("plan");
    plan.steps = vec![PlanStep {
        kind: PlanStepKind::InstallComponent,
        target: "paginated-query".to_string(),
        version: "0.1.0".to_string(),
        action: "install".to_string(),
        evidence: "component descriptor paginated-query@0.1.0 selected by the resolver".to_string(),
    }];
    write_plan_receipt(&project, &plan).expect("receipt");

    let outcome = apply_plan(&project, &plan.plan_id, true, Some(&db)).expect("apply");
    let step = outcome
        .applied_steps
        .iter()
        .find(|s| s.target == "paginated-query")
        .expect("the component step is reported");
    assert_eq!(
        step.status, "resolved",
        "the step names the action it performed; note={}",
        step.note
    );
    assert_ne!(
        step.status, "verified",
        "the planner resolved the component and qualified nothing"
    );
    assert_ne!(step.status, "installed", "the planner installed nothing");
    assert!(
        step.note.contains("did not auto-qualify"),
        "the note still says what the planner did not do: {}",
        step.note
    );
}

#[test]
fn every_resolution_writer_reads_the_same_rule() {
    // One rule, three lanes: a plan in any lane is a planned run.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let work = tmp.path().join("work");
    std::fs::create_dir_all(&work).expect("work dir");

    for args in [
        vec![
            "component",
            "resolve",
            "--profile",
            "rust-web",
            "--component",
            "paginated-query",
        ],
        vec![
            "ui-pattern",
            "resolve",
            "--profile",
            "react-web",
            "--pattern",
            "billing",
        ],
    ] {
        let mut cmd = clean_cmd();
        cmd.arg("--registry").arg(&db);
        cmd.arg("--format").arg("json");
        for a in &args {
            cmd.arg(a);
        }
        let out = cmd.output().expect("run forge resolve");
        assert_eq!(
            out.status.code(),
            Some(0),
            "a resolution exits 0; stderr={}",
            lossy(&out.stderr)
        );
    }
    resolve_intent(
        &db,
        &work,
        &["--action", "create_project", "--profile", "rust-web"],
    );

    for (kind, expected) in [
        ("component", "planned"),
        ("ui_pattern", "planned"),
        ("planner", "planned"),
    ] {
        let rows = journal_rows(&db, kind);
        assert_eq!(rows.len(), 1, "kind={kind} rows={rows:?}");
        let (_, state, detail) = &rows[0];
        assert_eq!(state, expected, "kind={kind} detail={detail}");
        assert!(
            detail.contains("verdict=planned"),
            "kind={kind} the detail names the run: {detail}"
        );
    }
}
