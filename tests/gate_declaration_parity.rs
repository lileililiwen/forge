//! The project's declared Gate floor and the plan Forge resolves must be the
//! same set.
//!
//! `forge.yaml` declares the checks a project claims to run; `.ai-gate/gate.yaml`
//! selects the runtime plan. They are separate files with separate authors, so
//! a row can be dropped from the plan while the declaration still names it.
//! Before this test, the only exercise of the recorded plan was a September
//! fixture, which parsed but never compared itself to the live resolution.
//!
//! This test runs the built binary in dry-run mode (it executes no check and
//! writes no evidence), so it observes the same resolution an operator does.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn declared_manifest() -> serde_yaml::Value {
    let path = repo_root().join("forge.yaml");
    let text = fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {path:?}: {err}"));
    serde_yaml::from_str(&text).unwrap_or_else(|err| panic!("parse {path:?}: {err}"))
}

fn declared_stage<'a>(manifest: &'a serde_yaml::Value, key: &str) -> &'a serde_yaml::Value {
    &manifest["verification"][key]
}

fn declared_checks(manifest: &serde_yaml::Value) -> Vec<&serde_yaml::Value> {
    declared_stage(manifest, "checks")
        .as_sequence()
        .expect("forge.yaml declares verification.checks")
        .iter()
        .collect()
}

fn dry_run_plan() -> serde_json::Value {
    let output = Command::new(env!("CARGO_BIN_EXE_forge"))
        .args(["gate", "--dry-run", "--format", "json"])
        .current_dir(repo_root())
        .output()
        .expect("forge gate --dry-run runs");
    assert!(
        output.status.success(),
        "forge gate --dry-run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let document: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("dry-run emits JSON");
    document["gate"]["rehearsal"].clone()
}

#[test]
fn the_declared_floor_equals_the_effective_plan() {
    let manifest = declared_manifest();
    let declared: BTreeSet<String> = declared_checks(&manifest)
        .iter()
        .map(|check| {
            check["id"]
                .as_str()
                .expect("every declared check carries an id")
                .to_string()
        })
        .collect();
    let plan = dry_run_plan();
    let planned: BTreeSet<String> = plan["planned_checks"]
        .as_array()
        .expect("the plan lists planned_checks")
        .iter()
        .map(|check| {
            check["id"]
                .as_str()
                .expect("every planned check carries an id")
                .to_string()
        })
        .collect();
    assert_eq!(
        declared, planned,
        "forge.yaml and the resolved plan disagree; a declared check that never runs does not gate"
    );
}

#[test]
fn the_rule_pack_matches_the_resolved_plan() {
    let manifest = declared_manifest();
    let declared = declared_stage(&manifest, "rule_pack")
        .as_str()
        .expect("forge.yaml declares verification.rule_pack");
    let plan = dry_run_plan();
    assert_eq!(
        declared,
        plan["rule_pack_version"]
            .as_str()
            .expect("the plan names its rule pack"),
        "the declared rule pack differs from the plan Forge resolved"
    );
}

#[test]
fn every_declared_check_names_the_source_the_spec_requires() {
    let manifest = declared_manifest();
    for check in declared_checks(&manifest) {
        let id = check["id"].as_str().unwrap();
        let expected = if id == "source-file-size" {
            "native"
        } else {
            "bound"
        };
        assert_eq!(
            check["source"].as_str(),
            Some(expected),
            "check `{id}` must declare source `{expected}`"
        );
    }
}

#[test]
fn every_declared_project_path_resolves() {
    let manifest = declared_manifest();
    for check in declared_checks(&manifest) {
        let Some(path) = check["path"].as_str() else {
            continue;
        };
        let id = check["id"].as_str().unwrap();
        assert!(
            !Path::new(path).is_absolute() && !path.split('/').any(|part| part == ".."),
            "check `{id}` path `{path}` must be project-relative and confined"
        );
        if check["root"].as_str() == Some("project") {
            let resolved = repo_root().join(path);
            assert!(
                resolved.is_file(),
                "check `{id}` declares `{path}`, which does not resolve to a file"
            );
        }
    }
}

#[test]
fn no_required_check_is_unbound() {
    let plan = dry_run_plan();
    assert_eq!(
        plan["unbound_required_checks"]
            .as_array()
            .expect("the plan lists unbound checks"),
        &Vec::<serde_json::Value>::new(),
        "the plan reports an unbound required check"
    );
}
