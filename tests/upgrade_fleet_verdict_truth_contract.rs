//! Fleet verdict truth contract: one verdict, every surface.
//!
//! Drives the real `forge` binary through `Command` and pins the claims of
//! the `upgrade-fleet-verdict-truth` capability:
//!
//! - a project the fleet could not judge is `unavailable`, never `skipped`,
//!   and the fleet's verdict is `unavailable`, never `healthy`;
//! - a fleet whose projects were all judged is `healthy` and exits 0;
//! - a fleet with a project that failed or blocked is `not healthy` and
//!   exits 1;
//! - the human renderer, the JSON document and the exit code read the same
//!   verdict;
//! - an already-satisfied fleet keeps its `skipped` result and stays
//!   healthy.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn clean_cmd() -> Command {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY");
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

fn run_json_with_code(db: &Path, args: &[&str]) -> (serde_json::Value, Option<i32>) {
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

fn write_rust_project(dir: &Path, id: &str, features: &str) {
    fs::create_dir_all(dir).unwrap();
    let text = format!(
        "schema: 1\nproject:\n  id: {id}\n  name: Test {id}\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\n{features}"
    );
    fs::write(dir.join("forge.yaml"), &text).unwrap();
}

fn write_clean_receipt(proj: &Path, feature: &str, version: &str) {
    let descriptor = forge::feature::inspect_feature(feature).unwrap();
    let receipt = proj.join(format!(".forge/features/{feature}.receipt"));
    fs::create_dir_all(receipt.parent().unwrap()).unwrap();
    fs::write(
        &receipt,
        forge::feature::expected_receipt(&descriptor, version),
    )
    .unwrap();
}

/// The verdict the human renderer stated, read from its own line.
fn human_verdict(stdout: &str) -> String {
    let line = stdout
        .lines()
        .find(|l| l.starts_with("verdict: "))
        .unwrap_or_else(|| panic!("no verdict line in: {stdout}"));
    line.trim_start_matches("verdict: ").to_string()
}

#[test]
fn an_unavailable_project_makes_the_fleet_unavailable_not_healthy() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("gone-app");
    write_rust_project(&proj, "gone-app", "features:\n  auth: \"0.0.9\"\n");
    let out = run(&db, &["register", proj.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    // The project is registered, but its directory is gone: the fleet can
    // select it and can never judge it.
    fs::remove_dir_all(&proj).unwrap();

    let (value, code) = run_json_with_code(&db, &["upgrade", "--all"]);
    assert_eq!(code, Some(1), "an unjudged fleet must not exit 0");
    let fleet = &value["fleet"];
    assert_eq!(fleet["verdict"], "unavailable");
    assert_eq!(fleet["unavailable"], 1);
    assert_eq!(fleet["skipped"], 0);
    assert_eq!(fleet["succeeded"], 0);
    assert_eq!(fleet["failed"], 0);
    assert_eq!(fleet["blocked"], 0);
    let entry = &fleet["entries"][0];
    assert_eq!(entry["project_id"], "gone-app");
    assert_eq!(entry["status"], "unavailable");
    assert_eq!(entry["changed"], false);
    // The note keeps saying why the project was not judged.
    assert!(entry["note"]
        .as_str()
        .unwrap()
        .contains("project path unavailable"));

    // The human renderer states the same verdict and never claims health.
    let out = run(&db, &["upgrade", "--all"]);
    assert_eq!(out.status.code(), Some(1));
    let stdout = lossy(&out.stdout).to_string();
    assert!(!stdout.contains("verdict: healthy"), "{stdout}");
    assert!(stdout.contains("verdict: unavailable"), "{stdout}");
    assert!(stdout.contains("[unavailable] gone-app:"), "{stdout}");
}

#[test]
fn a_fleet_whose_projects_were_all_judged_is_healthy() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let first = tmp.path().join("first");
    let second = tmp.path().join("second");
    write_rust_project(&first, "judged-one", "features:\n  auth: \"0.0.9\"\n");
    write_rust_project(&second, "judged-two", "features:\n  telemetry: \"0.0.9\"\n");
    for proj in [&first, &second] {
        let out = run(&db, &["register", proj.to_str().unwrap()]);
        assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    }
    write_clean_receipt(&first, "auth", "0.0.9");
    write_clean_receipt(&second, "telemetry", "0.0.9");

    let (value, code) = run_json_with_code(&db, &["upgrade", "--all"]);
    assert_eq!(code, Some(0), "a judged fleet must exit 0");
    let fleet = &value["fleet"];
    assert_eq!(fleet["verdict"], "healthy");
    assert_eq!(fleet["succeeded"], 2);
    assert_eq!(fleet["failed"], 0);
    assert_eq!(fleet["blocked"], 0);
    assert_eq!(fleet["skipped"], 0);
    assert_eq!(fleet["unavailable"], 0);
    for entry in fleet["entries"].as_array().unwrap() {
        assert_eq!(entry["status"], "success");
    }

    let out = run(&db, &["upgrade", "--all"]);
    assert_eq!(out.status.code(), Some(0));
    let stdout = lossy(&out.stdout).to_string();
    assert!(stdout.contains("verdict: healthy"), "{stdout}");
}

#[test]
fn a_fleet_with_a_blocked_project_is_not_healthy() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("drifted-app");
    write_rust_project(&proj, "drifted-app", "features:\n  auth: \"0.0.9\"\n");
    let out = run(&db, &["register", proj.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    write_clean_receipt(&proj, "auth", "0.0.9");
    // An operator-edited receipt is a semantic conflict: the project is
    // blocked before any mutation.
    fs::write(
        proj.join(".forge/features/auth.receipt"),
        "operator runbook notes\n",
    )
    .unwrap();

    let (value, code) = run_json_with_code(&db, &["upgrade", "--all"]);
    assert_eq!(code, Some(1), "a blocked fleet must exit 1");
    let fleet = &value["fleet"];
    assert_eq!(fleet["verdict"], "not healthy");
    assert_eq!(fleet["blocked"], 1);
    assert_eq!(fleet["failed"], 0);
    assert_eq!(fleet["unavailable"], 0);
    let entry = &fleet["entries"][0];
    assert_eq!(entry["status"], "blocked");
    assert_eq!(entry["changed"], false);

    let out = run(&db, &["upgrade", "--all"]);
    assert_eq!(out.status.code(), Some(1));
    let stdout = lossy(&out.stdout).to_string();
    assert!(stdout.contains("verdict: not healthy"), "{stdout}");
    assert!(!stdout.contains("verdict: healthy"), "{stdout}");
}

#[test]
fn a_definite_failure_outranks_an_unavailable_project() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let blocked = tmp.path().join("blocked-app");
    let gone = tmp.path().join("gone-app");
    write_rust_project(&blocked, "blocked-app", "features:\n  auth: \"0.0.9\"\n");
    write_rust_project(&gone, "gone-app", "features:\n  auth: \"0.0.9\"\n");
    for proj in [&blocked, &gone] {
        let out = run(&db, &["register", proj.to_str().unwrap()]);
        assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    }
    write_clean_receipt(&blocked, "auth", "0.0.9");
    fs::write(
        blocked.join(".forge/features/auth.receipt"),
        "operator runbook notes\n",
    )
    .unwrap();
    fs::remove_dir_all(&gone).unwrap();

    let (value, code) = run_json_with_code(&db, &["upgrade", "--all"]);
    assert_eq!(code, Some(1));
    let fleet = &value["fleet"];
    // The definite outcome outranks the unknown: not `unavailable`.
    assert_eq!(fleet["verdict"], "not healthy");
    assert_eq!(fleet["blocked"], 1);
    assert_eq!(fleet["unavailable"], 1);
}

#[test]
fn the_human_renderer_and_the_json_document_state_the_same_verdict() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("gone-app");
    write_rust_project(&proj, "gone-app", "features:\n  auth: \"0.0.9\"\n");
    let out = run(&db, &["register", proj.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    fs::remove_dir_all(&proj).unwrap();

    let (value, json_code) = run_json_with_code(&db, &["upgrade", "--all"]);
    let out = run(&db, &["upgrade", "--all"]);
    let stdout = lossy(&out.stdout).to_string();
    assert_eq!(out.status.code(), json_code);
    assert_eq!(
        human_verdict(&stdout),
        value["fleet"]["verdict"].as_str().unwrap()
    );
    // The unavailable count is stated by both surfaces too.
    assert!(
        stdout.contains(&format!("{} unavailable", value["fleet"]["unavailable"])),
        "{stdout}"
    );
}

#[test]
fn an_already_satisfied_fleet_stays_skipped_and_healthy() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("sat-app");
    write_rust_project(&proj, "sat-app", "features:\n  auth: \"0.0.9\"\n");
    let out = run(&db, &["register", proj.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    write_clean_receipt(&proj, "auth", "0.0.9");

    // The first run upgrades the project.
    let (value, code) = run_json_with_code(&db, &["upgrade", "--all"]);
    assert_eq!(code, Some(0));
    assert_eq!(value["fleet"]["succeeded"], 1);

    // The retry judged the project and chose not to change it: still
    // `skipped`, and the fleet is still healthy.
    let (value, code) = run_json_with_code(&db, &["upgrade", "--all"]);
    assert_eq!(code, Some(0), "an already-satisfied fleet must exit 0");
    let fleet = &value["fleet"];
    assert_eq!(fleet["skipped"], 1);
    assert_eq!(fleet["unavailable"], 0);
    assert_eq!(fleet["verdict"], "healthy");
    let entry = &fleet["entries"][0];
    assert_eq!(entry["status"], "skipped");
    assert_eq!(entry["changed"], false);

    let out = run(&db, &["upgrade", "--all"]);
    assert_eq!(out.status.code(), Some(0));
    let stdout = lossy(&out.stdout).to_string();
    assert!(stdout.contains("verdict: healthy"), "{stdout}");
}

#[test]
fn a_dry_run_fleet_keeps_its_header_and_its_verdict() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("dry-app");
    write_rust_project(&proj, "dry-app", "features:\n  auth: \"0.0.9\"\n");
    let out = run(&db, &["register", proj.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    write_clean_receipt(&proj, "auth", "0.0.9");

    let out = run(&db, &["upgrade", "--all", "--dry-run"]);
    assert_eq!(out.status.code(), Some(0));
    let stdout = lossy(&out.stdout).to_string();
    assert!(stdout.contains("(dry-run)"), "{stdout}");
    assert!(stdout.contains("verdict: healthy"), "{stdout}");

    // A dry-run fleet with an unavailable project still reports it.
    fs::remove_dir_all(&proj).unwrap();
    let (value, code) = run_json_with_code(&db, &["upgrade", "--all", "--dry-run"]);
    assert_eq!(code, Some(1));
    let fleet = &value["fleet"];
    assert_eq!(fleet["dry_run"], true);
    assert_eq!(fleet["verdict"], "unavailable");
    assert_eq!(fleet["unavailable"], 1);
    assert_eq!(fleet["entries"][0]["status"], "unavailable");
}
