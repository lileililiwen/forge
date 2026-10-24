//! CLI contract for `analytics-inspect-verdict-truth`.
//!
//! One rule judges every inspection: the report carries the verdict, the
//! verdict names what the inspection did, and the journal follows it.
//!
//! - a dry run is `planned`, is never healthy, exits 0 and journals a
//!   `planned` row;
//! - a healthy inspection is `delivered`, is healthy and journals a `done`
//!   row;
//! - a failing provider is `failed`, is never healthy and journals a
//!   `failed` row;
//! - a disabled block is `delivered`, is healthy and journals a `done` row.

use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

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

/// The `analytics` journal rows, oldest first.
fn analytics_journal(db: &Path) -> Vec<(String, String, String)> {
    let conn = rusqlite::Connection::open(db).expect("open db");
    let mut stmt = conn
        .prepare(
            "SELECT project_id, state, detail FROM operations WHERE kind = 'analytics' ORDER BY op_id",
        )
        .expect("prepare");
    let mut rows = stmt.query([]).expect("query");
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

/// The `analytics` journal row the inspection wrote. The test runs the
/// inspection once per output format, so every row must carry the same
/// verdict.
fn analytics_row(db: &Path) -> (String, String, String) {
    let rows = analytics_journal(db);
    assert!(!rows.is_empty(), "the inspection wrote no journal row");
    let state = rows[0].1.clone();
    assert!(
        rows.iter().all(|row| row.1 == state),
        "every row carries the same verdict: {rows:?}"
    );
    rows.into_iter().last().expect("journal row")
}

/// Stage a project whose `analytics:` block declares one repository
/// provider with the given adapter command.
fn write_analytics_project(dir: &Path, id: &str, adapter_command: Option<&Path>) {
    fs::create_dir_all(dir).unwrap();
    let adapter = match adapter_command {
        Some(path) => format!("      adapter_command: {}\n", path.display()),
        None => String::new(),
    };
    let text = format!(
        "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: rust-web\nanalytics:\n  \
         enabled: true\n  default_window_days: 7\n  repository:\n    - provider: \
         github-analytics\n      enabled: true\n      project_ref: owner/repo\n{adapter}"
    );
    fs::write(dir.join("forge.yaml"), text).unwrap();
}

/// Stage a project whose `analytics:` block is opted out.
fn write_disabled_analytics_project(dir: &Path, id: &str) {
    fs::create_dir_all(dir).unwrap();
    let text = format!(
        "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: rust-web\nanalytics:\n  \
         enabled: false\n  content:\n    - provider: unified-content\n      enabled: true\n      \
         project_ref: slug\n"
    );
    fs::write(dir.join("forge.yaml"), text).unwrap();
}

/// Stage an adapter fixture. The healthy fixture reports the manifest's
/// project reference; the failing fixture exits non-zero.
fn write_script(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    let mut file = fs::File::create(&path).unwrap();
    file.write_all(b"#!/usr/bin/env bash\n").unwrap();
    file.write_all(body.as_bytes()).unwrap();
    let mut perms = fs::metadata(&path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&path, perms).unwrap();
    path
}

#[test]
fn a_dry_run_is_planned_and_journals_a_planned_row() {
    // A plan contacts no provider. The observations must say so, the
    // report must not claim an available provider, the command must exit
    // 0 and the journal must not record a completed inspection.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_analytics_project(&proj, "ana-plan", None);

    let out = run(
        &db,
        &["analytics", "inspect", proj.to_str().unwrap(), "--dry-run"],
    );
    assert_eq!(
        out.status.code(),
        Some(0),
        "a plan must exit 0; stderr={}",
        lossy(&out.stderr)
    );
    let stdout = lossy(&out.stdout);
    assert!(stdout.contains("status=planned"), "stdout={stdout}");
    assert!(!stdout.contains("status=available"), "stdout={stdout}");

    let json = run_json(
        &db,
        &[
            "analytics",
            "inspect",
            proj.to_str().unwrap(),
            "--dry-run",
            "--format",
            "json",
        ],
    );
    let observations = json["report"]["observations"].as_array().unwrap();
    assert!(!observations.is_empty());
    assert!(
        observations.iter().all(|o| o["status"] == "planned"),
        "observations={observations:?}"
    );
    assert_eq!(json["report"]["verdict"], "planned");
    assert_eq!(json["report"]["available"], false);

    let (project_id, state, detail) = analytics_row(&db);
    assert_eq!(project_id, "ana-plan");
    assert_eq!(state, "planned");
    assert!(detail.contains("verdict=planned"), "detail={detail}");
    assert!(detail.contains("healthy=false"), "detail={detail}");
}

#[test]
fn a_healthy_inspection_is_delivered_and_journals_a_done_row() {
    // The adapter reports a healthy provider, so the inspection
    // delivered and the journal records a completed inspection.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    let scripts = tmp.path().join("scripts");
    fs::create_dir_all(&scripts).unwrap();
    let healthy = write_script(
        &scripts,
        "healthy.sh",
        "printf '{\"project_ref\":\"owner/repo\",\"evidence\":[\"stars=42\"],\"note\":\"ok\"}\\n'\n",
    );
    write_analytics_project(&proj, "ana-delivered", Some(&healthy));

    let out = run(&db, &["analytics", "inspect", proj.to_str().unwrap()]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "a delivery must exit 0; stderr={}",
        lossy(&out.stderr)
    );
    let json = run_json(
        &db,
        &[
            "analytics",
            "inspect",
            proj.to_str().unwrap(),
            "--format",
            "json",
        ],
    );
    let observations = json["report"]["observations"].as_array().unwrap();
    let repo = observations
        .iter()
        .find(|o| o["plane"] == "repository")
        .unwrap();
    assert_eq!(repo["status"], "available");
    assert_eq!(json["report"]["verdict"], "delivered");
    assert_eq!(json["report"]["available"], true);

    let (project_id, state, detail) = analytics_row(&db);
    assert_eq!(project_id, "ana-delivered");
    assert_eq!(state, "done");
    assert!(detail.contains("verdict=delivered"), "detail={detail}");
    assert!(detail.contains("healthy=true"), "detail={detail}");
}

#[test]
fn a_failing_provider_is_failed_and_journals_a_failed_row() {
    // The adapter cannot run: the provider is `unavailable`, the run
    // failed and the journal records the failure.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    let scripts = tmp.path().join("scripts");
    fs::create_dir_all(&scripts).unwrap();
    let failing = write_script(&scripts, "failing.sh", "exit 7\n");
    write_analytics_project(&proj, "ana-failed", Some(&failing));

    let out = run(&db, &["analytics", "inspect", proj.to_str().unwrap()]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "a failed provider is a reportable outcome; stderr={}",
        lossy(&out.stderr)
    );
    let json = run_json(
        &db,
        &[
            "analytics",
            "inspect",
            proj.to_str().unwrap(),
            "--format",
            "json",
        ],
    );
    let observations = json["report"]["observations"].as_array().unwrap();
    let repo = observations
        .iter()
        .find(|o| o["plane"] == "repository")
        .unwrap();
    assert_eq!(repo["status"], "unavailable");
    assert_eq!(json["report"]["verdict"], "failed");
    assert_eq!(json["report"]["available"], false);

    let (project_id, state, detail) = analytics_row(&db);
    assert_eq!(project_id, "ana-failed");
    assert_eq!(state, "failed");
    assert!(detail.contains("verdict=failed"), "detail={detail}");
    assert!(detail.contains("healthy=false"), "detail={detail}");
}

#[test]
fn a_disabled_block_is_delivered_and_journals_a_done_row() {
    // An opted-out plane has nothing to plan: the inspection delivered
    // the manifest's evidence that the provider is disabled.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_disabled_analytics_project(&proj, "ana-disabled");

    let out = run(&db, &["analytics", "inspect", proj.to_str().unwrap()]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "a disabled block is not a failure; stderr={}",
        lossy(&out.stderr)
    );
    let json = run_json(
        &db,
        &[
            "analytics",
            "inspect",
            proj.to_str().unwrap(),
            "--format",
            "json",
        ],
    );
    let observations = json["report"]["observations"].as_array().unwrap();
    assert!(!observations.is_empty());
    assert!(
        observations.iter().all(|o| o["status"] == "disabled"),
        "observations={observations:?}"
    );
    assert_eq!(json["report"]["verdict"], "delivered");
    assert_eq!(json["report"]["available"], true);

    let (project_id, state, detail) = analytics_row(&db);
    assert_eq!(project_id, "ana-disabled");
    assert_eq!(state, "done");
    assert!(detail.contains("verdict=delivered"), "detail={detail}");
    assert!(detail.contains("healthy=true"), "detail={detail}");
}

/// The JSON document keeps the report's own verdict and availability
/// fields: a reader never has to re-derive them.
#[test]
fn the_report_carries_the_verdict_and_the_availability() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_analytics_project(&proj, "ana-shape", None);

    let json: Value = run_json(
        &db,
        &[
            "analytics",
            "inspect",
            proj.to_str().unwrap(),
            "--dry-run",
            "--format",
            "json",
        ],
    );
    assert_eq!(json["contract"], "0.1.0");
    assert_eq!(json["project_id"], "ana-shape");
    assert_eq!(json["report"]["contract"], "0.1.0");
    assert_eq!(json["report"]["verdict"], "planned");
    assert_eq!(json["report"]["available"], false);
    assert_eq!(json["report"]["enabled"], true);
}
