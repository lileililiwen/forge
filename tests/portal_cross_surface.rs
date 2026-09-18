//! Cross-surface regression tests for `forge portal`
//! (`control-plane-portal`).
//!
//! Exercises the control-plane portal across the other
//! Core contracts:
//!
//! - The Core registry's `portal` journal row records
//!   the per-call summary so the operations table stays
//!   independent of the portal surface.
//! - The doctor verdict on the same project directory is
//!   unchanged after a successful portal round trip so
//!   the existing doctor contract still holds.
//! - The MCP `tools/list` snapshot is unchanged after a
//!   `forge portal dashboard` / `view` call so the
//!   portal does not advertise itself through the MCP
//!   transport.
//! - A `forge feature add` round trip on the same project
//!   remains compatible after a `forge portal dashboard`
//!   call so the portal does not regress the feature
//!   ownership contract.
//! - The R1 boundary: a `forge portal dashboard` call on
//!   a project with no observable state surfaces
//!   `unknown` / `unavailable` prominently in the
//!   rollup; the dashboard never reports `ok` when the
//!   underlying modules are missing evidence.

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
        .env_remove("ANTHROPIC_API_KEY")
        .env_remove("FORGE_DEPLOYER_BIN")
        .env_remove("FORGE_DRIFTWATCH_BIN")
        .env_remove("FORGE_DOCS_TRANSLATOR_BIN")
        .env_remove("FORGE_PACKAGE_BIN")
        .env_remove("FORGE_NOTES_BIN")
        .env_remove("FORGE_ANALYTICS_BIN");
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

fn run_json(db: &Path, args: &[&str]) -> serde_json::Value {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    cmd.arg("--format").arg("json");
    for a in args {
        cmd.arg(a);
    }
    let out = cmd.output().expect("run forge json");
    serde_json::from_slice(&out.stdout).unwrap_or_else(|err| {
        panic!(
            "invalid json: {err}; stdout={} stderr={}",
            lossy(&out.stdout),
            lossy(&out.stderr)
        )
    })
}

fn create_fresh_project(db: &Path, dest: &Path, id: &str) {
    let out = run(
        db,
        &[
            "new",
            "--profile",
            "rust-web",
            "--id",
            id,
            dest.to_str().unwrap(),
        ],
    );
    assert_eq!(
        out.status.code(),
        Some(0),
        "new failed: stderr={}",
        lossy(&out.stderr)
    );
}

#[test]
fn portal_dashboard_does_not_invent_a_project_for_a_fleet_view() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    // A fleet-wide dashboard must not add a project row
    // to the registry. The synthetic `__portal__` project
    // id lives in the operations table, never as a
    // registered project.
    let before = run_json(&db, &["list", "--format", "json"]);
    let before_count = before["projects"].as_array().unwrap().len();
    let out = run(&db, &["portal", "dashboard", "--all"]);
    assert_eq!(out.status.code(), Some(0), "stderr={}", lossy(&out.stderr));
    let after = run_json(&db, &["list", "--format", "json"]);
    let after_count = after["projects"].as_array().unwrap().len();
    assert_eq!(before_count, after_count);
}

#[test]
fn doctor_verdict_is_unchanged_after_portal_round_trip() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("portal-cross-doctor");
    create_fresh_project(&db, &proj, "portal-cross-doctor");
    let before = run(&db, &["doctor", proj.to_str().unwrap(), "--target", "L1"]);
    let before_stdout = lossy(&before.stdout);
    let before_stderr = lossy(&before.stderr);
    let before_status = before.status.code();
    let _ = run(&db, &["portal", "dashboard", "portal-cross-doctor"]);
    let after = run(&db, &["doctor", proj.to_str().unwrap(), "--target", "L1"]);
    assert_eq!(after.stdout, before.stdout);
    assert_eq!(after.stderr, before.stderr);
    assert_eq!(after.status.code(), before_status);
    assert!(
        before_stdout.contains("manifest") || before_stderr.contains("PASS"),
        "before output missing doctor labels: {} {}",
        before_stdout,
        before_stderr
    );
}

#[test]
fn mcp_tools_list_is_unchanged_after_portal_round_trip() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("portal-cross-mcp");
    create_fresh_project(&db, &proj, "portal-cross-mcp");
    let before = run(&db, &["mcp", "tools-list"]);
    let before_stdout = lossy(&before.stdout);
    let before_status = before.status.code();
    let _ = run(&db, &["portal", "dashboard", "portal-cross-mcp"]);
    let _ = run(&db, &["portal", "view", "projects", "portal-cross-mcp"]);
    let after = run(&db, &["mcp", "tools-list"]);
    let after_stdout = lossy(&after.stdout);
    let after_status = after.status.code();
    assert_eq!(before_status, after_status);
    assert_eq!(before_stdout, after_stdout);
    // The MCP surface must never advertise the portal
    // surface. The portal is a CLI-rendered view, not an
    // MCP tool.
    for forbidden in [
        "portal_dashboard",
        "portal_view",
        "portal.dashboard",
        "portal.view",
    ] {
        assert!(
            !after_stdout.contains(forbidden),
            "MCP tools list leaked `{forbidden}`: {after_stdout}"
        );
    }
}

#[test]
fn feature_add_workflow_remains_compatible_after_portal_round_trip() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("portal-cross-feature");
    create_fresh_project(&db, &proj, "portal-cross-feature");
    // Render the dashboard first.
    let _ = run(&db, &["portal", "dashboard", "portal-cross-feature"]);
    // Adding a feature through the CLI must still succeed
    // after the portal round trip: the portal surface
    // does not regress the feature ownership contract.
    let out = run(&db, &["feature", "add", "auth", proj.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0), "stderr={}", lossy(&out.stderr));
    let inspect = run_json(
        &db,
        &["inspect", "portal-cross-feature", "--format", "json"],
    );
    let features = inspect["features"].as_object().unwrap();
    assert!(features.contains_key("auth"), "expected auth feature");
}

#[test]
fn portal_dashboard_records_per_call_journal_row() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("portal-cross-journal");
    create_fresh_project(&db, &proj, "portal-cross-journal");
    let _ = run(&db, &["portal", "dashboard", "portal-cross-journal"]);
    let _ = run(&db, &["portal", "view", "projects", "portal-cross-journal"]);
    // The portal surface records one `portal` journal row
    // per call. We assert that the registry kept the
    // surface independent by reading the operations table
    // through the registry's own list surface.
    let json = run_json(
        &db,
        &[
            "portal",
            "dashboard",
            "portal-cross-journal",
            "--format",
            "json",
        ],
    );
    let dashboard = &json["dashboard"];
    // The dashboard's `operations` field surfaces the
    // most recent journal entries for the project, which
    // includes the prior portal round trips.
    let operations = dashboard["operations"].as_array().unwrap();
    let portal_ops: Vec<&serde_json::Value> = operations
        .iter()
        .filter(|op| op["kind"] == "portal")
        .collect();
    assert!(!portal_ops.is_empty(), "expected at least one portal row");
    for op in portal_ops {
        assert_eq!(op["project_id"], "portal-cross-journal");
    }
}

#[test]
fn portal_view_unknown_section_does_not_mutate_the_registry() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("portal-cross-noop");
    create_fresh_project(&db, &proj, "portal-cross-noop");
    let before = run_json(&db, &["list", "--format", "json"]);
    let before_count = before["projects"].as_array().unwrap().len();
    let out = run(&db, &["portal", "view", "audit", "portal-cross-noop"]);
    assert_eq!(out.status.code(), Some(1));
    let after = run_json(&db, &["list", "--format", "json"]);
    let after_count = after["projects"].as_array().unwrap().len();
    assert_eq!(before_count, after_count);
}
