//! Portable AI procedures over stable Core operations
//! (`ai-procedure-skills`).
//!
//! Covers the new `forge procedure` command surface end to end through
//! the built binary: catalog discovery (R1 success: the eight named
//! procedures are returned in stable id order with the contract
//! version), per-procedure inspection (R1 success: prerequisites,
//! ordered steps and the verification block are all present), the
//! `report_findings` final-step convention (R2 boundary: every
//! procedure ends with the gap-reporting step), the typed
//! `procedure-invalid` rejection for an unknown id (R1 failure: a
//! missing procedure is refused rather than silently substituted), the
//! `procedure-bypass-refused` rejection for a step that smuggles a
//! bypass marker (R2 failure: a procedure that asks Core to skip a
//! check is refused at the procedure layer), and the JSON / human
//! parity for the `validate` subcommand.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn clean_cmd() -> Command {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY");
    cmd.env_remove("FORGE_DRIFTWATCH_BIN");
    cmd.env_remove("HTTP_PROXY");
    cmd.env_remove("HTTPS_PROXY");
    cmd.env_remove("ALL_PROXY");
    cmd.env_remove("OPENAI_API_KEY");
    cmd.env_remove("ANTHROPIC_API_KEY");
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

fn run_json(db: &Path, args: &[&str]) -> (serde_json::Value, Option<i32>) {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    cmd.arg("--format").arg("json");
    for a in args {
        cmd.arg(a);
    }
    let out = cmd.output().expect("run forge json");
    let code = out.status.code();
    let value = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|err| panic!("invalid json: {err}; stderr={}", lossy(&out.stderr)));
    (value, code)
}

fn lossy(bytes: &[u8]) -> std::borrow::Cow<'_, str> {
    String::from_utf8_lossy(bytes)
}

fn write_procedure_json(dir: &Path, name: &str, spec: &serde_json::Value) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, serde_json::to_string_pretty(spec).unwrap()).unwrap();
    path
}

#[test]
fn procedure_help_lists_the_new_subcommands() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let out = run(&db, &["procedure", "--help"]);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let stdout = lossy(&out.stdout);
    for sub in ["list", "inspect", "validate"] {
        assert!(
            stdout.contains(sub),
            "missing subcommand `{sub}` in help: {stdout}"
        );
    }
}

#[test]
fn top_level_help_includes_procedure_subcommand() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let out = run(&db, &["--help"]);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let stdout = lossy(&out.stdout);
    assert!(
        stdout.contains("procedure"),
        "top-level help should advertise `procedure`; got: {stdout}"
    );
}

#[test]
fn list_returns_eight_named_procedures_in_stable_order() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let (value, code) = run_json(&db, &["procedure", "list"]);
    assert_eq!(code, Some(0));
    assert_eq!(value["contract"], "0.1.0");
    let procedures = value["procedures"].as_array().unwrap();
    let ids: Vec<&str> = procedures
        .iter()
        .map(|p| p["id"].as_str().unwrap())
        .collect();
    assert_eq!(
        ids,
        vec![
            "create-project",
            "deploy-project",
            "fix-quality-findings",
            "mirror-repository",
            "onboard-existing-project",
            "prepare-release",
            "translate-docs",
            "upgrade-project",
        ]
    );
    for entry in procedures {
        assert_eq!(entry["version"], "0.1.0");
        assert!(!entry["title"].as_str().unwrap().is_empty());
        assert!(!entry["description"].as_str().unwrap().is_empty());
    }
}

#[test]
fn inspect_create_project_returns_nine_step_sop_with_report_findings() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let (value, code) = run_json(&db, &["procedure", "inspect", "create-project"]);
    assert_eq!(code, Some(0));
    assert_eq!(value["contract"], "0.1.0");
    let proc = &value["procedure"];
    assert_eq!(proc["id"], "create-project");
    assert_eq!(proc["version"], "0.1.0");
    let steps = proc["steps"].as_array().unwrap();
    assert!(!steps.is_empty());
    let last = steps.last().unwrap();
    assert_eq!(last["op"], "report_findings");
    let ops: Vec<&str> = steps.iter().map(|s| s["op"].as_str().unwrap()).collect();
    for op in [
        "profile_inspect",
        "profile_preflight",
        "component_resolve",
        "ui_pattern_resolve",
        "feature_add",
        "doctor_run",
        "test_run",
        "policy_run",
        "report_findings",
    ] {
        assert!(ops.contains(&op), "missing op `{op}` in {ops:?}");
    }
    let verification = proc["verification"].as_str().unwrap();
    assert!(verification.contains("doctor"));
    assert!(verification.contains("test"));
    let prereqs = proc["prerequisites"].as_array().unwrap();
    assert!(!prereqs.is_empty());
}

#[test]
fn inspect_upgrade_project_includes_spec_generate_handoff() {
    // R2 success scenario: the upgrade SOP routes a semantic
    // conflict through the existing `spec.generate`
    // remediation surface.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let (value, code) = run_json(&db, &["procedure", "inspect", "upgrade-project"]);
    assert_eq!(code, Some(0));
    let proc = &value["procedure"];
    let steps = proc["steps"].as_array().unwrap();
    let ops: Vec<&str> = steps.iter().map(|s| s["op"].as_str().unwrap()).collect();
    assert!(
        ops.contains(&"spec_generate"),
        "upgrade-project must include spec.generate handoff; ops={ops:?}"
    );
    let last = steps.last().unwrap();
    assert_eq!(last["op"], "report_findings");
}

#[test]
fn inspect_unknown_id_typed_rejection() {
    // R1 failure scenario: an unknown id is refused with the
    // typed `procedure-invalid` code before any catalog
    // mutation.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let out = run(&db, &["procedure", "inspect", "no-such-procedure"]);
    assert_eq!(out.status.code(), Some(1), "expected exit 1");
    let stderr = lossy(&out.stderr);
    assert!(
        stderr.contains("error[procedure-invalid]"),
        "stderr should carry the typed `procedure-invalid` code; got: {stderr}"
    );
    assert!(
        stderr.contains("no-such-procedure"),
        "stderr should name the unknown id; got: {stderr}"
    );
}

#[test]
fn inspect_non_kebab_case_id_typed_rejection() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let out = run(&db, &["procedure", "inspect", "CreateProject"]);
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("error[procedure-invalid]"));
    assert!(stderr.contains("kebab-case"));
}

#[test]
fn inspect_empty_id_typed_rejection() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let out = run(&db, &["procedure", "inspect", ""]);
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("error[procedure-invalid]"));
}

#[test]
fn validate_accepts_minimal_valid_procedure() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let spec = serde_json::json!({
        "id": "smoke",
        "version": "0.1.0",
        "title": "Smoke",
        "description": "smoke",
        "prerequisites": [],
        "steps": [
            {"ordinal": 1, "op": "doctor_run", "args": ["."], "description": "doctor"},
            {"ordinal": 2, "op": "report_findings", "args": [], "description": "report"}
        ],
        "verification": "v"
    });
    let path = write_procedure_json(tmp.path(), "smoke.json", &spec);
    let out = run(
        &db,
        &["procedure", "validate", "--path", path.to_str().unwrap()],
    );
    assert_eq!(out.status.code(), Some(0), "stderr: {}", lossy(&out.stderr));
    let stdout = lossy(&out.stdout);
    assert!(stdout.contains("smoke"));
    assert!(stdout.contains("verification"));
}

#[test]
fn validate_accepts_procedure_in_json_format() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let spec = serde_json::json!({
        "id": "smoke",
        "version": "0.1.0",
        "title": "Smoke",
        "description": "smoke",
        "prerequisites": [],
        "steps": [
            {"ordinal": 1, "op": "doctor_run", "args": ["."], "description": "doctor"},
            {"ordinal": 2, "op": "report_findings", "args": [], "description": "report"}
        ],
        "verification": "v"
    });
    let path = write_procedure_json(tmp.path(), "smoke.json", &spec);
    let (value, code) = run_json(
        &db,
        &["procedure", "validate", "--path", path.to_str().unwrap()],
    );
    assert_eq!(code, Some(0));
    assert_eq!(value["contract"], "0.1.0");
    assert_eq!(value["procedure"]["id"], "smoke");
    assert_eq!(
        value["note"].as_str().unwrap(),
        "procedure 'smoke' validated: 2 step(s) and ends with report_findings"
    );
}

#[test]
fn validate_rejects_bypass_marker_with_typed_code() {
    // R2 failure scenario: a procedure that asks Core to
    // skip its own validation is refused at the procedure
    // layer.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let spec = serde_json::json!({
        "id": "smoke-bypass",
        "version": "0.1.0",
        "title": "Smoke",
        "description": "smoke",
        "prerequisites": [],
        "steps": [
            {"ordinal": 1, "op": "doctor_run", "args": ["--force"], "description": "doctor"},
            {"ordinal": 2, "op": "report_findings", "args": [], "description": "report"}
        ],
        "verification": "v"
    });
    let path = write_procedure_json(tmp.path(), "smoke.json", &spec);
    let out = run(
        &db,
        &["procedure", "validate", "--path", path.to_str().unwrap()],
    );
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(
        stderr.contains("error[procedure-bypass-refused]"),
        "expected typed procedure-bypass-refused code; got: {stderr}"
    );
    assert!(stderr.contains("--force"));
}

#[test]
fn validate_rejects_workflow_without_report_findings() {
    // R2 boundary scenario: a workflow that does not end
    // with the gap-reporting step is refused.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let spec = serde_json::json!({
        "id": "no-report",
        "version": "0.1.0",
        "title": "No report",
        "description": "d",
        "prerequisites": [],
        "steps": [
            {"ordinal": 1, "op": "doctor_run", "args": ["."], "description": "doctor"}
        ],
        "verification": "v"
    });
    let path = write_procedure_json(tmp.path(), "no-report.json", &spec);
    let out = run(
        &db,
        &["procedure", "validate", "--path", path.to_str().unwrap()],
    );
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("error[procedure-invalid]"));
    assert!(stderr.contains("report_findings"));
}

#[test]
fn validate_rejects_unknown_operation_with_typed_code() {
    // R1 failure scenario: a procedure that references an
    // unavailable or unstable operation is refused with a
    // typed rejection so the planner never silently
    // schedules it.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let spec = serde_json::json!({
        "id": "future",
        "version": "0.1.0",
        "title": "Future",
        "description": "d",
        "prerequisites": [],
        "steps": [
            {"ordinal": 1, "op": "push", "args": ["."], "description": "d"},
            {"ordinal": 2, "op": "report_findings", "args": [], "description": "d"}
        ],
        "verification": "v"
    });
    let path = write_procedure_json(tmp.path(), "future.json", &spec);
    // The procedure spec is well-formed JSON; `validate`
    // will run the full validator. Push is not in the
    // supported set so the procedure is refused.
    let out = run(
        &db,
        &["procedure", "validate", "--path", path.to_str().unwrap()],
    );
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("error[procedure-invalid]"));
}

#[test]
fn validate_rejects_malformed_json() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let path = tmp.path().join("garbage.json");
    fs::write(&path, b"this is not json").unwrap();
    let out = run(
        &db,
        &["procedure", "validate", "--path", path.to_str().unwrap()],
    );
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("error[procedure-invalid]"));
    assert!(stderr.contains("valid JSON ProcedureSpec"));
}

#[test]
fn every_catalog_procedure_ends_with_report_findings() {
    // R2 boundary scenario: every named SOP must end with
    // the gap-reporting step so the workflow surfaces its
    // findings rather than asserting completion.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let (value, _) = run_json(&db, &["procedure", "list"]);
    let procedures = value["procedures"].as_array().unwrap();
    for entry in procedures {
        let id = entry["id"].as_str().unwrap();
        let (spec_value, _) = run_json(&db, &["procedure", "inspect", id]);
        let proc = &spec_value["procedure"];
        let steps = proc["steps"].as_array().unwrap();
        let last = steps.last().unwrap();
        assert_eq!(
            last["op"], "report_findings",
            "procedure '{id}' does not end with report_findings"
        );
    }
}

#[test]
fn every_catalog_procedure_uses_only_supported_core_operations() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let (list_value, _) = run_json(&db, &["procedure", "list"]);
    let procedures = list_value["procedures"].as_array().unwrap();
    let supported = [
        "profile_inspect",
        "profile_resolve",
        "profile_preflight",
        "feature_resolve",
        "feature_add",
        "feature_remove",
        "feature_upgrade",
        "component_resolve",
        "ui_pattern_resolve",
        "ui_pattern_install",
        "intent_validate",
        "intent_resolve",
        "intent_apply",
        "doctor_run",
        "test_run",
        "commit",
        "policy_run",
        "spec_generate",
        "spec_apply",
        "agent_start",
        "upgrade_apply",
        "upgrade_fleet",
        "import_run",
        "deploy_plan",
        "deploy_apply",
        "deploy_observe",
        "release_prepare",
        "release_apply",
        "docs_translate",
        "mirror_apply",
        "report_findings",
    ];
    for entry in procedures {
        let id = entry["id"].as_str().unwrap();
        let (spec_value, _) = run_json(&db, &["procedure", "inspect", id]);
        let proc = &spec_value["procedure"];
        let steps = proc["steps"].as_array().unwrap();
        for step in steps {
            let op = step["op"].as_str().unwrap();
            assert!(
                supported.contains(&op),
                "procedure '{id}' references unsupported op '{op}'"
            );
        }
    }
}
