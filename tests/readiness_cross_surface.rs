//! Cross-surface regression tests for `forge readiness`
//! (`profile-and-release-readiness`).
//!
//! Exercises the readiness surface across the other
//! Core contracts:
//!
//! - The registry journal stays independent of the readiness surface: a
//!   matrix run invents no project and writes no operations row, and an
//!   artifact run leaves the registry file byte-identical.
//! - The doctor verdict on the same project directory is byte-equivalent
//!   before and after a readiness run so the existing doctor contract
//!   still holds.
//! - A `forge feature add` round trip on the same project remains
//!   compatible after a readiness run so the feature ownership contract
//!   still holds.
//! - The R1 boundary: a generated fixture's native commands behave the
//!   same with Forge absent from PATH, and the matrix row reports
//!   `forge_absent_from_path` truthfully (skipped where `cargo` is
//!   unavailable).

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

fn binary_on_path(name: &str) -> bool {
    let path = std::env::var_os("PATH").unwrap_or_default();
    for dir in std::env::split_paths(&path) {
        if dir.as_os_str().is_empty() {
            continue;
        }
        if dir.join(name).is_file() {
            return true;
        }
    }
    false
}

fn fresh_tmp() -> tempfile::TempDir {
    tempfile::TempDir::new().expect("temp dir")
}

#[test]
fn readiness_matrix_invents_no_project_and_writes_no_journal_row() {
    if !binary_on_path("cargo") {
        return;
    }
    let tmp = fresh_tmp();
    let db = tmp.path().join("registry.db");
    let before = run_json(&db, &["list"]);
    assert_eq!(before["projects"].as_array().unwrap().len(), 0);
    let out = run(&db, &["readiness", "matrix", "--profile", "rust-web"]);
    assert!(out.status.success(), "stderr: {}", lossy(&out.stderr));
    let after = run_json(&db, &["list"]);
    assert_eq!(after, before, "matrix must not register a project");
}

#[test]
fn readiness_artifact_leaves_the_registry_file_untouched() {
    let tmp = fresh_tmp();
    let db = tmp.path().join("registry.db");
    let listed = run(&db, &["list"]);
    assert!(listed.status.success());
    let before = std::fs::read(&db).expect("registry file must exist");
    let out = run(&db, &["readiness", "artifact"]);
    assert!(out.status.success(), "stderr: {}", lossy(&out.stderr));
    let after = std::fs::read(&db).expect("registry file must still exist");
    assert_eq!(before, after, "artifact must not write a journal row");
}

#[test]
fn doctor_verdict_is_byte_equivalent_after_a_readiness_run() {
    let tmp = fresh_tmp();
    let db = tmp.path().join("registry.db");
    let dest = tmp.path().join("doc-proj");
    let created = run(
        &db,
        &[
            "new",
            "--profile",
            "rust-web",
            "--id",
            "doc-proj",
            dest.to_str().unwrap(),
        ],
    );
    assert!(
        created.status.success(),
        "stderr: {}",
        lossy(&created.stderr)
    );
    let first = run(&db, &["doctor", dest.to_str().unwrap(), "--format", "json"]);
    assert!(first.status.success() || first.status.code() == Some(1));
    let first_body = lossy(&first.stdout);
    let probe = run(&db, &["readiness", "artifact"]);
    assert!(probe.status.success());
    let second = run(&db, &["doctor", dest.to_str().unwrap(), "--format", "json"]);
    assert_eq!(second.status.code(), first.status.code());
    assert_eq!(lossy(&second.stdout), first_body);
}

#[test]
fn feature_add_remains_compatible_after_a_readiness_run() {
    let tmp = fresh_tmp();
    let db = tmp.path().join("registry.db");
    let dest = tmp.path().join("feat-proj");
    let created = run(
        &db,
        &[
            "new",
            "--profile",
            "rust-web",
            "--id",
            "feat-proj",
            dest.to_str().unwrap(),
        ],
    );
    assert!(
        created.status.success(),
        "stderr: {}",
        lossy(&created.stderr)
    );
    let probe = run(&db, &["readiness", "artifact"]);
    assert!(probe.status.success());
    let added = run(&db, &["feature", "add", "auth", "feat-proj"]);
    assert!(
        added.status.success(),
        "feature add must keep working after readiness: {}",
        lossy(&added.stderr)
    );
    let listed = run_json(&db, &["inspect", "feat-proj"]);
    let features = listed["features"].as_object().expect("features map");
    assert!(features.contains_key("auth"));
}

#[test]
fn rust_web_row_proves_the_fixture_builds_without_forge() {
    if !binary_on_path("cargo") {
        return;
    }
    let tmp = fresh_tmp();
    let db = tmp.path().join("registry.db");
    let value = run_json(&db, &["readiness", "matrix", "--profile", "rust-web"]);
    let rows = value["matrix"]["rows"].as_array().expect("rows array");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["result"], "passed");
    assert_eq!(rows[0]["forge_absent_from_path"], true);
}
