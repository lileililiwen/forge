//! Classification derivation contract: `forge classify derive`
//! (`forge-project-classification-derivation`).
//!
//! This suite answers the questions the capability names:
//!
//! - A declared manifest yields `profile` and `lifecycle`
//!   proposals at high confidence, each naming the manifest field
//!   it was derived from.
//! - A maturity gap lowers lifecycle confidence to medium; an
//!   equal current/target keeps it high.
//! - `domain` is low confidence and bounded in length.
//! - Deriving twice on an unchanged project records the same
//!   proposals (determinism).
//! - Deriving writes no project field: `forge.yaml` is
//!   byte-identical before and after.
//! - A project with no `forge.yaml` is refused with a typed error
//!   and nothing is written.
//!
//! No model, no network, no provider is contacted.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn run(dir: &Path, args: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY");
    cmd.env_remove("FORGE_WORKSPACE_REGISTRY");
    cmd.env_remove("FORGE_INVENTORY_SOURCE");
    cmd.env_remove("HTTP_PROXY");
    cmd.env_remove("HTTPS_PROXY");
    cmd.env_remove("ALL_PROXY");
    cmd.arg("--registry").arg(dir.join("registry.db"));
    for arg in args {
        cmd.arg(arg);
    }
    cmd.current_dir(dir);
    cmd.output().expect("run forge")
}

fn run_json(dir: &Path, args: &[&str]) -> Value {
    let out = run(dir, args);
    assert!(
        out.status.success(),
        "{:?} failed: {}",
        args,
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("derive json")
}

fn write_project(dir: &Path, maturity: &str, target: &str) {
    fs::write(
        dir.join("forge.yaml"),
        format!(
            "schema: 1\nproject:\n  id: demo\n  name: Demo Project\n  profile: rust-web\n  maturity: {maturity}\n  target_maturity: {target}\nruntime:\n  language: rust\n"
        ),
    )
    .unwrap();
    fs::write(dir.join("README.md"), "# Demo Project\n\nbody\n").unwrap();
}

fn proposals(page: &Value) -> &Vec<Value> {
    page["proposals"].as_array().expect("proposals array")
}

fn proposal_of<'a>(page: &'a Value, kind: &str) -> &'a Value {
    proposals(page)
        .iter()
        .find(|entry| entry["proposal"]["kind"] == kind)
        .unwrap_or_else(|| panic!("no `{kind}` proposal in {page}"))
}

#[test]
fn declared_manifest_derives_profile_and_lifecycle_at_high_confidence() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    write_project(dir, "L1", "L1");

    let page = run_json(dir, &["classify", "derive", "--format", "json"]);
    assert_eq!(page["contract"], "forge-semantic-proposal/0.1.0");

    let profile = proposal_of(&page, "profile");
    assert_eq!(profile["proposal"]["confidence"], "high");
    assert_eq!(profile["proposal"]["suggested_value"], "rust-web");
    assert_eq!(profile["proposal"]["evidence"][0]["path"], "forge.yaml");
    assert!(
        profile["proposal"]["evidence"][0]["excerpt"]
            .as_str()
            .unwrap()
            .contains("project.profile"),
        "{}",
        profile
    );

    let lifecycle = proposal_of(&page, "lifecycle");
    assert_eq!(lifecycle["proposal"]["confidence"], "high");
    assert_eq!(lifecycle["proposal"]["suggested_value"], "L1");
    assert!(
        lifecycle["proposal"]["evidence"][0]["excerpt"]
            .as_str()
            .unwrap()
            .contains("project.maturity"),
        "{}",
        lifecycle
    );
}

#[test]
fn equal_current_and_target_is_high_a_gap_is_medium() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    write_project(dir, "L1", "L1");
    let page = run_json(dir, &["classify", "derive", "--format", "json"]);
    assert_eq!(
        proposal_of(&page, "lifecycle")["proposal"]["confidence"],
        "high"
    );

    let tmp2 = tempfile::tempdir().unwrap();
    let dir2 = tmp2.path();
    write_project(dir2, "L1", "L3");
    let page2 = run_json(dir2, &["classify", "derive", "--format", "json"]);
    assert_eq!(
        proposal_of(&page2, "lifecycle")["proposal"]["confidence"],
        "medium"
    );
}

#[test]
fn domain_is_low_confidence_and_bounded() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    write_project(dir, "L1", "L1");
    let long_heading = "x".repeat(500);
    fs::write(dir.join("README.md"), format!("# {long_heading}\n")).unwrap();

    let page = run_json(dir, &["classify", "derive", "--format", "json"]);
    let domain = proposal_of(&page, "domain");
    assert_eq!(domain["proposal"]["confidence"], "low");
    let value = domain["proposal"]["suggested_value"].as_str().unwrap();
    assert!(
        value.chars().count() <= 120,
        "domain length {}",
        value.chars().count()
    );
    assert_eq!(domain["proposal"]["evidence"][0]["path"], "README.md");
}

#[test]
fn deriving_twice_is_deterministic() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    write_project(dir, "L1", "L1");

    let first = run_json(dir, &["classify", "derive", "--format", "json"]);
    let second = run_json(dir, &["classify", "derive", "--format", "json"]);

    let ids = |page: &Value| {
        let mut ids: Vec<String> = proposals(page)
            .iter()
            .map(|entry| {
                entry["proposal"]["id"]["hash"]
                    .as_str()
                    .unwrap()
                    .to_string()
            })
            .collect();
        ids.sort();
        ids
    };
    assert_eq!(ids(&first), ids(&second));
    for entry in proposals(&second) {
        assert_eq!(entry["status"], "existing", "{}", entry);
    }
}

#[test]
fn deriving_writes_no_project_field() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    write_project(dir, "L1", "L1");
    let before = fs::read(dir.join("forge.yaml")).unwrap();

    run_json(dir, &["classify", "derive", "--format", "json"]);

    let after = fs::read(dir.join("forge.yaml")).unwrap();
    assert_eq!(before, after);
}

#[test]
fn missing_manifest_is_refused_and_writes_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();

    let out = run(dir, &["classify", "derive", "--format", "json"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("forge.yaml") || stderr.contains("manifest"),
        "{stderr}"
    );
    assert!(!dir.join(".forge").exists(), "nothing may be written");
}
