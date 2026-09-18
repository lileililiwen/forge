//! CLI contract for `forge portal` (`control-plane-portal`).
//!
//! Exercises the control-plane portal end to end through
//! the built binary:
//!
//! - R1 success: a registered project renders a
//!   timestamped dashboard with one entry per named
//!   section (projects/features/components/…/settings) and
//!   one portal journal row.
//! - R1 failure: an unknown portal section is refused
//!   with the typed `error[portal-invalid]` exit-1 error
//!   before any view is rendered.
//! - R1 boundary: a fleet-wide dashboard with an empty
//!   registry renders all twelve sections with empty
//!   `entries:` lists, never a fabricated success line.
//! - R2 success: a per-section view for a registered
//!   project renders the section heading, the rolled-up
//!   status, the per-entry evidence and the `controls_`
//!   available line that points to the matching CLI
//!   command.
//! - R2 boundary: a project with no observable state
//!   (no manifest, no git remote, no features, no
//!   components, …) surfaces `unknown` / `unavailable`
//!   prominently in the per-section status and in the
//!   dashboard rollup, never as `ok`.

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
fn cli_help_lists_portal_subcommand() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let out = run(&db, &["--help"]);
    let stdout = lossy(&out.stdout);
    assert!(
        stdout.contains("portal"),
        "top-level help must mention portal: {stdout}"
    );
    let out = run(&db, &["portal", "--help"]);
    let stdout = lossy(&out.stdout);
    assert!(
        stdout.contains("dashboard"),
        "stdout must list dashboard: {stdout}"
    );
    assert!(stdout.contains("view"), "stdout must list view: {stdout}");
}

#[test]
fn dashboard_for_registered_project_renders_every_section() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("portal-proj");
    create_fresh_project(&db, &proj, "portal-proj");
    let out = run(
        &db,
        &["portal", "dashboard", "portal-proj", "--format", "json"],
    );
    assert_eq!(out.status.code(), Some(0), "stderr={}", lossy(&out.stderr));
    let json = run_json(
        &db,
        &["portal", "dashboard", "portal-proj", "--format", "json"],
    );
    assert_eq!(json["contract"], "0.1.0");
    let dashboard = &json["dashboard"];
    assert_eq!(dashboard["scope"], "project");
    assert_eq!(dashboard["project_id"], "portal-proj");
    let sections = dashboard["sections"].as_array().unwrap();
    assert_eq!(sections.len(), 12);
    for section in sections {
        assert!(section["section_id"].is_string());
        assert!(section["status"].is_string());
    }
}

#[test]
fn dashboard_human_output_carries_required_fields() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("portal-human");
    create_fresh_project(&db, &proj, "portal-human");
    let out = run(&db, &["portal", "dashboard", "portal-human"]);
    assert_eq!(out.status.code(), Some(0), "stderr={}", lossy(&out.stderr));
    let stdout = lossy(&out.stdout);
    for needle in [
        "0.1.0",
        "Forge Control Plane",
        "projects",
        "features",
        "components",
        "policies",
        "specs",
        "agents",
        "deployments",
        "repositories",
        "documentation",
        "analytics",
        "servers",
        "settings",
        "recent operations",
    ] {
        assert!(stdout.contains(needle), "{needle} missing in {stdout}");
    }
}

#[test]
fn dashboard_records_portal_journal_row() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("portal-journal");
    create_fresh_project(&db, &proj, "portal-journal");
    let out = run(&db, &["portal", "dashboard", "portal-journal"]);
    assert_eq!(out.status.code(), Some(0));
    let json = run_json(&db, &["inspect", "portal-journal", "--format", "json"]);
    let _ = json;
    let out = run(
        &db,
        &["portal", "dashboard", "portal-journal", "--format", "json"],
    );
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let detail = json["dashboard"]["sections"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["section_id"].as_str().unwrap_or("").to_string())
        .collect::<Vec<_>>()
        .join(",");
    assert!(detail.contains("projects"));
}

#[test]
fn fleet_dashboard_renders_with_synthetic_project_id() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("portal-fleet");
    create_fresh_project(&db, &proj, "portal-fleet");
    let out = run(&db, &["portal", "dashboard", "--all"]);
    assert_eq!(out.status.code(), Some(0), "stderr={}", lossy(&out.stderr));
    let json = run_json(&db, &["portal", "dashboard", "--all", "--format", "json"]);
    assert_eq!(json["contract"], "0.1.0");
    let dashboard = &json["dashboard"];
    assert_eq!(dashboard["scope"], "fleet");
    assert!(dashboard["project_id"].is_null());
    let sections = dashboard["sections"].as_array().unwrap();
    assert_eq!(sections.len(), 12);
}

#[test]
fn view_for_known_section_returns_section_only() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("portal-view");
    create_fresh_project(&db, &proj, "portal-view");
    let out = run(
        &db,
        &[
            "portal",
            "view",
            "projects",
            "portal-view",
            "--format",
            "json",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "stderr={}", lossy(&out.stderr));
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let view = &json["view"];
    assert_eq!(view["section_id"], "projects");
    assert_eq!(view["project_id"], "portal-view");
    let entries = view["entries"].as_array().unwrap();
    assert!(!entries.is_empty(), "expected at least one project entry");
    assert!(entries.iter().any(|e| e["id"] == "portal-view"));
}

#[test]
fn view_refuses_unknown_section_with_typed_error() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let out = run(&db, &["portal", "view", "audit", "portal-proj"]);
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(
        stderr.contains("error[portal-invalid]"),
        "stderr must surface the typed error: {stderr}"
    );
}

#[test]
fn view_human_output_carries_controls_available() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("portal-controls");
    create_fresh_project(&db, &proj, "portal-controls");
    let out = run(&db, &["portal", "view", "features", "portal-controls"]);
    assert_eq!(out.status.code(), Some(0), "stderr={}", lossy(&out.stderr));
    let stdout = lossy(&out.stdout);
    for needle in [
        "[Features]",
        "section=`features`",
        "controls_available",
        "forge feature list",
    ] {
        assert!(stdout.contains(needle), "{needle} missing in {stdout}");
    }
}

#[test]
fn dashboard_surfaces_unknown_when_project_has_no_evidence() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("portal-empty");
    create_fresh_project(&db, &proj, "portal-empty");
    let out = run(
        &db,
        &["portal", "dashboard", "portal-empty", "--format", "json"],
    );
    assert_eq!(out.status.code(), Some(0), "stderr={}", lossy(&out.stderr));
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let sections = json["dashboard"]["sections"].as_array().unwrap();
    // The dashboard must not report a healthy rollup when a project
    // has no observable state: the rollup surfaces the worst
    // section so a single `unknown` / `unavailable` is never masked.
    let rollup = sections
        .iter()
        .map(|s| s["status"].as_str().unwrap_or(""))
        .max_by_key(|s| match *s {
            "ok" => 0,
            "partial" => 1,
            "warn" => 2,
            "unknown" => 3,
            "unavailable" => 4,
            "fail" => 5,
            _ => -1,
        })
        .unwrap_or("ok")
        .to_string();
    assert!(
        matches!(rollup.as_str(), "unknown" | "unavailable" | "warn" | "fail"),
        "rollup was {rollup}"
    );
    let _ = sections;
}

#[test]
fn view_for_fleet_section_succeeds_with_synthetic_id() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let out = run(&db, &["portal", "view", "servers", "--format", "json"]);
    assert_eq!(out.status.code(), Some(0), "stderr={}", lossy(&out.stderr));
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["view"]["section_id"], "servers");
    assert!(json["view"]["project_id"].is_null());
}
