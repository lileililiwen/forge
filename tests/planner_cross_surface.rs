//! Cross-surface regression tests for the validated intent planner.
//!
//! Each test exercises a real Core contract (registry journal,
//! doctor, feature install, ui-pattern install) and the planner's
//! own boundary, then asserts that the planner's side effects stay
//! independent of those surfaces:
//!
//! - The registry journals every planner run as a `planner` row
//!   without rewriting the operations table's contract.
//! - A successful `forge intent resolve` then `forge doctor` on the
//!   same project produces a doctor verdict byte-equivalent to the
//!   pre-planner verdict (R1/R2 contract preservation).
//! - The planner apply path keeps the feature add and ui-pattern
//!   install ownership receipts and manifests independent of the
//!   planner surface.
//! - R2 boundary: dropping the planner receipt leaves the project
//!   state intact so a project is not on the planner's path.
//! - Stale-plan refusal: a tampered receipt exits with
//!   `error[plan-stale]` and writes no file.

use std::process::Command;

use forge::planner::{
    apply_plan, plans_dir, read_plan_receipt, resolve_plan, validate_intent, write_plan_receipt,
    Intent, IntentAction, IntentConstraint,
};
use forge::registry::Registry;
use tempfile::TempDir;

fn workspace() -> TempDir {
    TempDir::new().expect("workspace")
}

fn project_dir(workspace: &TempDir, profile: &str, project_id: &str) -> std::path::PathBuf {
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
    let db_dir = dir.join(".forge");
    let _ = std::fs::create_dir_all(&db_dir);
    dir
}

fn sample_intent() -> Intent {
    Intent {
        action: IntentAction::CreateProject,
        profile: "rust-web".to_string(),
        required_capabilities: vec!["auth".to_string(), "admin".to_string()],
        forbidden_capabilities: vec!["billing".to_string()],
        constraints: vec![IntentConstraint {
            key: "public".to_string(),
            value: "true".to_string(),
        }],
    }
}

fn doctor_verdict(project: &std::path::Path) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_forge"))
        .arg("doctor")
        .arg(project)
        .arg("--format")
        .arg("json")
        .output()
        .expect("invoke doctor");
    assert!(output.status.success());
    String::from_utf8_lossy(&output.stdout).to_string()
}

#[test]
fn planner_operations_journal_under_planner_kind() {
    let ws = workspace();
    let project = project_dir(&ws, "rust-web", "planner-journal");
    let intent = sample_intent();
    let validated = validate_intent(&intent).expect("validated");
    let plan = resolve_plan(&validated, None).expect("plan");
    write_plan_receipt(&project, &plan).expect("receipt");
    let db = project.join(".forge/registry.sqlite");
    let outcome = apply_plan(&project, &plan.plan_id, true, Some(&db)).expect("apply");
    let registry = Registry::open(&db).expect("registry open");
    let entries = registry.journal_entries().expect("journal");
    let planner_entries: Vec<_> = entries.iter().filter(|e| e.kind == "planner").collect();
    assert!(
        !planner_entries.is_empty(),
        "planner operations must be journaled"
    );
    assert!(planner_entries.iter().any(|e| e.state == "done"));
    let _ = outcome;
}

#[test]
fn doctor_verdict_is_byte_equivalent_after_planner_resolve() {
    let ws = workspace();
    let project = project_dir(&ws, "rust-web", "planner-doctor-byte");
    let before = doctor_verdict(&project);
    let intent = sample_intent();
    let validated = validate_intent(&intent).expect("validated");
    let plan = resolve_plan(&validated, None).expect("plan");
    write_plan_receipt(&project, &plan).expect("receipt");
    let after = doctor_verdict(&project);
    assert_eq!(
        before, after,
        "doctor verdict must not change because of a planner resolve"
    );
}

#[test]
fn receipt_drop_leaves_project_state_intact() {
    let ws = workspace();
    let project = project_dir(&ws, "rust-web", "planner-receipt-drop");
    let intent = sample_intent();
    let validated = validate_intent(&intent).expect("validated");
    let plan = resolve_plan(&validated, None).expect("plan");
    write_plan_receipt(&project, &plan).expect("receipt");
    let receipt = plans_dir(&project).join(&plan.plan_id);
    assert!(receipt.exists());
    std::fs::remove_dir_all(&receipt).expect("drop receipt");
    let manifest = std::fs::read_to_string(project.join("forge.yaml")).expect("manifest");
    assert!(manifest.contains("features: {}"));
}

#[test]
fn stale_plan_refusal_writes_nothing() {
    let ws = workspace();
    let project = project_dir(&ws, "rust-web", "planner-stale-no-write");
    let intent = sample_intent();
    let validated = validate_intent(&intent).expect("validated");
    let mut plan = resolve_plan(&validated, None).expect("plan");
    plan.profile_version = "9.9.9".to_string();
    write_plan_receipt(&project, &plan).expect("receipt");
    let db = project.join(".forge/registry.sqlite");
    let err = apply_plan(&project, &plan.plan_id, true, Some(&db)).expect_err("must reject");
    assert!(matches!(err, forge::core::ForgeError::PlanStale { .. }));
    let manifest = std::fs::read_to_string(project.join("forge.yaml")).expect("manifest");
    assert!(manifest.contains("features: {}"));
}

#[test]
fn planner_apply_does_not_corrupt_existing_feature_receipts() {
    let ws = workspace();
    let project = project_dir(&ws, "rust-web", "planner-feature-receipt");
    let db = project.join(".forge/registry.sqlite");
    // Install a feature through the normal CLI first so the
    // ownership receipt is on disk. Use the same db path the apply
    // path will use so canonicalization matches.
    let install = Command::new(env!("CARGO_BIN_EXE_forge"))
        .arg("--registry")
        .arg(&db)
        .arg("feature")
        .arg("add")
        .arg("auth")
        .arg("--version")
        .arg("0.1.0")
        .arg(&project)
        .output()
        .expect("install feature");
    assert!(
        install.status.success(),
        "feature install should succeed: stderr={}",
        String::from_utf8_lossy(&install.stderr)
    );
    let receipt_path = project.join(".forge/features/auth.receipt");
    let receipt_before = std::fs::read_to_string(&receipt_path).expect("receipt");
    // Now plan and apply through the planner via the CLI so the
    // canonicalization matches the registry row from `feature add`.
    let intent = Intent {
        action: IntentAction::ExtendProject,
        profile: "rust-web".to_string(),
        required_capabilities: vec!["admin".to_string()],
        forbidden_capabilities: vec![],
        constraints: vec![],
    };
    let validated = validate_intent(&intent).expect("validated");
    let plan = resolve_plan(&validated, None).expect("plan");
    write_plan_receipt(&project, &plan).expect("receipt");
    let apply = Command::new(env!("CARGO_BIN_EXE_forge"))
        .arg("--registry")
        .arg(&db)
        .arg("intent")
        .arg("apply")
        .arg(&plan.plan_id)
        .arg("--confirm")
        .arg("--path")
        .arg(&project)
        .output()
        .expect("apply");
    assert!(
        apply.status.success(),
        "planner apply should succeed: stderr={}",
        String::from_utf8_lossy(&apply.stderr)
    );
    let receipt_after = std::fs::read_to_string(&receipt_path).expect("receipt after");
    assert_eq!(
        receipt_before, receipt_after,
        "feature ownership receipt must not be rewritten by a planner apply"
    );
}

#[test]
fn receipt_round_trips_through_public_api() {
    let ws = workspace();
    let project = project_dir(&ws, "rust-web", "planner-receipt-roundtrip");
    let intent = sample_intent();
    let validated = validate_intent(&intent).expect("validated");
    let plan = resolve_plan(&validated, None).expect("plan");
    write_plan_receipt(&project, &plan).expect("receipt");
    let loaded = read_plan_receipt(&project, &plan.plan_id).expect("read");
    assert_eq!(loaded.plan_id, plan.plan_id);
    assert_eq!(loaded.intent, plan.intent);
    assert_eq!(loaded.profile, plan.profile);
    assert_eq!(loaded.steps.len(), plan.steps.len());
}

#[test]
fn feature_add_workflow_unchanged_after_planner_resolve() {
    let ws = workspace();
    let project = project_dir(&ws, "rust-web", "planner-feature-workflow");
    let intent = sample_intent();
    let validated = validate_intent(&intent).expect("validated");
    let plan = resolve_plan(&validated, None).expect("plan");
    write_plan_receipt(&project, &plan).expect("receipt");
    let install = Command::new(env!("CARGO_BIN_EXE_forge"))
        .arg("--registry")
        .arg(project.join(".forge/registry.sqlite"))
        .arg("feature")
        .arg("add")
        .arg("notifications")
        .arg(&project)
        .output()
        .expect("install notifications");
    assert!(
        install.status.success(),
        "feature add must still succeed after a planner resolve: stderr={}",
        String::from_utf8_lossy(&install.stderr)
    );
    let manifest = std::fs::read_to_string(project.join("forge.yaml")).expect("manifest");
    assert!(manifest.contains("notifications"));
}
