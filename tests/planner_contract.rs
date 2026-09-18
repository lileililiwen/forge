//! CLI contract tests for the validated intent planner.
//!
//! Each test exercises the public `forge intent` surface and the
//! typed `intent-*` / `plan-*` errors so the CLI's contract is
//! pinned independent of the implementation. The contract maps
//! directly to the `validated-intent-planner` spec scenarios:
//!
//! - R1 success: `forge intent validate --action create_project
//!   --profile rust-web --require auth --require admin --forbid billing`
//!   returns the normalized validated intent with the public constraint
//!   retained and the billing prohibition retained.
//! - R1 failure: a Flutter + postgres intent exits 1 with
//!   `error[intent-invalid]` and a recommended backend boundary.
//! - R1 boundary: an unknown capability exits 1 with
//!   `error[intent-invalid]` and the missing capability named.
//! - R2 success: a compatible resolve returns the deterministic plan
//!   with pinned feature steps and a doctor/test/quality-policy gate.
//! - R2 failure: a stale plan (drifted profile version) is refused by
//!   `apply` with `error[plan-stale]` and the project is untouched.
//! - R2 boundary: an `apply` without `--confirm` exits 1 with
//!   `error[plan-apply-failed]` and no file is written.

use std::process::Command;

use forge::planner::{
    apply_plan, intent_hash, plans_dir, read_plan_receipt, render_apply_human,
    render_intent_validation_human, render_plan_human, resolve_plan, validate_intent,
    write_plan_receipt, Intent, IntentAction, IntentConstraint, PLANNER_CONTRACT_VERSION,
};
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
    let db = dir.join(".forge/registry.sqlite");
    if let Some(parent) = db.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut registry = forge::registry::Registry::open(&db).expect("registry open");
    let _ = registry.register(&dir, None);
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

#[test]
fn help_prints_subcommand_summaries() {
    let output = Command::new(env!("CARGO_BIN_EXE_forge"))
        .arg("intent")
        .arg("--help")
        .output()
        .expect("invoke help");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    for needle in [
        "validate",
        "resolve",
        "apply",
        "list",
        "reviewable deterministic",
    ] {
        assert!(
            stdout.contains(needle),
            "missing '{needle}' in help:\n{stdout}"
        );
    }
}

#[test]
fn contract_version_is_recorded_in_artefacts() {
    let intent = sample_intent();
    let validated = validate_intent(&intent).expect("validated");
    let plan = resolve_plan(&validated, None).expect("plan");
    assert_eq!(PLANNER_CONTRACT_VERSION, "0.1.0");
    assert_eq!(plan.profile_version, "0.1.0");
    // The plan's stored intent is the normalized one, so the hash
    // must be computed from the stored intent to stay deterministic.
    assert_eq!(plan.intent_hash, intent_hash(&plan.intent));
}

#[test]
fn human_renderers_carry_required_and_forbidden_lists() {
    let intent = sample_intent();
    let validated = validate_intent(&intent).expect("validated");
    let outcome = forge::planner::IntentValidationOutcome {
        validated: Some(validated.clone()),
        note: validated.note.clone(),
    };
    let human = render_intent_validation_human(&outcome);
    assert!(human.contains("auth"));
    assert!(human.contains("admin"));
    assert!(human.contains("billing"));
    let plan = resolve_plan(&validated, None).expect("plan");
    let plan_human = render_plan_human(&plan);
    assert!(plan_human.contains("install_feature"));
    assert!(plan_human.contains("doctor"));
    assert!(plan_human.contains("auth@0.1.0"));
}

#[test]
fn cli_validate_returns_normalized_intent_in_json() {
    let output = Command::new(env!("CARGO_BIN_EXE_forge"))
        .arg("intent")
        .arg("validate")
        .arg("--action")
        .arg("create_project")
        .arg("--profile")
        .arg("rust-web")
        .arg("--require")
        .arg("auth")
        .arg("--require")
        .arg("admin")
        .arg("--forbid")
        .arg("billing")
        .arg("--constraint")
        .arg("public=true")
        .arg("--format")
        .arg("json")
        .output()
        .expect("invoke validate");
    assert!(
        output.status.success(),
        "validate should succeed; stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let body: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    let validated = body
        .get("validated")
        .expect("validated field present in JSON");
    assert_eq!(validated["profile_id"], "rust-web");
    let required = validated["intent"]["required_capabilities"]
        .as_array()
        .expect("required array");
    let required: Vec<String> = required
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert!(required.contains(&"auth".to_string()));
    assert!(required.contains(&"admin".to_string()));
    let forbidden = validated["intent"]["forbidden_capabilities"]
        .as_array()
        .expect("forbidden array");
    let forbidden: Vec<String> = forbidden
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert!(forbidden.contains(&"billing".to_string()));
    assert_eq!(body["contract"], PLANNER_CONTRACT_VERSION);
}

#[test]
fn cli_validate_exits_one_for_client_plus_server_capability() {
    let output = Command::new(env!("CARGO_BIN_EXE_forge"))
        .arg("intent")
        .arg("validate")
        .arg("--action")
        .arg("create_project")
        .arg("--profile")
        .arg("flutter-app")
        .arg("--require")
        .arg("auth")
        .arg("--require")
        .arg("postgres")
        .output()
        .expect("invoke validate");
    assert!(!output.status.success(), "validate should fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("error[intent-invalid]"), "stderr: {stderr}");
    assert!(stderr.contains("flutter-app"), "stderr: {stderr}");
    assert!(stderr.contains("postgres"), "stderr: {stderr}");
    assert!(stderr.contains("backend"), "stderr: {stderr}");
}

#[test]
fn cli_validate_exits_one_for_unknown_capability() {
    let output = Command::new(env!("CARGO_BIN_EXE_forge"))
        .arg("intent")
        .arg("validate")
        .arg("--action")
        .arg("create_project")
        .arg("--profile")
        .arg("rust-web")
        .arg("--require")
        .arg("not-a-real-capability")
        .output()
        .expect("invoke validate");
    assert!(!output.status.success(), "validate should fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("error[intent-invalid]"), "stderr: {stderr}");
    assert!(stderr.contains("not-a-real-capability"), "stderr: {stderr}");
}

#[test]
fn unknown_action_is_refused_before_planning() {
    let output = Command::new(env!("CARGO_BIN_EXE_forge"))
        .arg("intent")
        .arg("validate")
        .arg("--action")
        .arg("delete_everything")
        .arg("--profile")
        .arg("rust-web")
        .arg("--require")
        .arg("auth")
        .output()
        .expect("invoke validate");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("error[intent-invalid]"), "stderr: {stderr}");
    assert!(stderr.contains("delete_everything"), "stderr: {stderr}");
}

#[test]
fn cli_resolve_persists_a_receipt_and_json_envelope() {
    let ws = workspace();
    let project = project_dir(&ws, "rust-web", "intent-resolve-ok");
    let output = Command::new(env!("CARGO_BIN_EXE_forge"))
        .arg("intent")
        .arg("resolve")
        .arg("--action")
        .arg("create_project")
        .arg("--profile")
        .arg("rust-web")
        .arg("--require")
        .arg("auth")
        .arg("--require")
        .arg("admin")
        .arg("--forbid")
        .arg("billing")
        .arg("--format")
        .arg("json")
        .arg("--path")
        .arg(&project)
        .output()
        .expect("invoke resolve");
    assert!(
        output.status.success(),
        "resolve should succeed; stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let body: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    let plan = body.get("plan").expect("plan field present in JSON");
    assert_eq!(plan["profile"], "rust-web");
    assert!(!plan["steps"].as_array().expect("steps").is_empty());
    assert!(!plan["intent_hash"]
        .as_str()
        .expect("intent_hash")
        .is_empty());
    assert!(!plan["catalog_hash"]
        .as_str()
        .expect("catalog_hash")
        .is_empty());
    let receipt_path = body["receipt_path"].as_str().expect("receipt_path");
    assert!(
        std::path::Path::new(receipt_path).exists(),
        "receipt must exist on disk"
    );
    // The persisted plan must round-trip through the public API.
    let plan_id = plan["plan_id"].as_str().expect("plan_id");
    let persisted = read_plan_receipt(&project, plan_id).expect("read receipt");
    assert_eq!(persisted.plan_id, plan_id);
    assert_eq!(persisted.intent_hash, plan["intent_hash"]);
}

#[test]
fn cli_apply_refuses_without_confirm() {
    let ws = workspace();
    let project = project_dir(&ws, "rust-web", "intent-apply-no-confirm");
    let intent = sample_intent();
    let validated = validate_intent(&intent).expect("validated");
    let plan = resolve_plan(&validated, None).expect("plan");
    write_plan_receipt(&project, &plan).expect("receipt");
    let db = project.join(".forge/registry.sqlite");
    let outcome = apply_plan(&project, &plan.plan_id, false, Some(&db));
    let err = outcome.expect_err("must refuse");
    match err {
        forge::core::ForgeError::PlanApplyFailed { reason, .. } => {
            assert!(reason.contains("confirm"));
        }
        other => panic!("expected PlanApplyFailed, got {other:?}"),
    }
    // Project state must be untouched: the receipt is the only
    // planner-owned file and the manifest features map is empty.
    let manifest = std::fs::read_to_string(project.join("forge.yaml")).expect("manifest");
    assert!(manifest.contains("features: {}"));
}

#[test]
fn cli_apply_refuses_stale_plan_and_leaves_project_untouched() {
    let ws = workspace();
    let project = project_dir(&ws, "rust-web", "intent-apply-stale");
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
fn apply_human_renderer_lists_steps_and_unresolved() {
    let ws = workspace();
    let project = project_dir(&ws, "rust-web", "intent-apply-render");
    let intent = sample_intent();
    let validated = validate_intent(&intent).expect("validated");
    let plan = resolve_plan(&validated, None).expect("plan");
    write_plan_receipt(&project, &plan).expect("receipt");
    let db = project.join(".forge/registry.sqlite");
    let outcome = apply_plan(&project, &plan.plan_id, true, Some(&db)).expect("apply");
    let human = render_apply_human(&outcome);
    assert!(human.contains("plan"));
    assert!(human.contains("auth"));
    assert!(human.contains("admin"));
    assert!(human.contains("installed") || human.contains("applied"));
}

#[test]
fn list_subcommand_renders_persisted_plans() {
    let ws = workspace();
    let project = project_dir(&ws, "rust-web", "intent-list");
    let intent = sample_intent();
    let validated = validate_intent(&intent).expect("validated");
    let plan = resolve_plan(&validated, None).expect("plan");
    write_plan_receipt(&project, &plan).expect("receipt");
    let output = Command::new(env!("CARGO_BIN_EXE_forge"))
        .arg("intent")
        .arg("list")
        .arg(&project)
        .arg("--format")
        .arg("json")
        .output()
        .expect("invoke list");
    assert!(output.status.success());
    let body: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    let plans = body["plans"].as_array().expect("plans array");
    assert!(
        plans
            .iter()
            .any(|p| p["plan_id"].as_str() == Some(plan.plan_id.as_str())),
        "list should report the persisted plan"
    );
    assert!(plans_dir(&project).exists());
}
